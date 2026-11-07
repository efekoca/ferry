use std::io::{Error, ErrorKind, Result, Write};
use std::path::{Component, Path, PathBuf};

fn refuse(msg: String) -> Error {
    Error::new(ErrorKind::InvalidInput, msg)
}

#[cfg(unix)]
fn hard_linked(meta: &std::fs::Metadata) -> bool {
    std::os::unix::fs::MetadataExt::nlink(meta) > 1
}

#[cfg(not(unix))]
fn hard_linked(_: &std::fs::Metadata) -> bool {
    false
}

pub fn resolve(root: &Path, rel: &str) -> Result<PathBuf> {
    let rel_path = Path::new(rel);
    let parts: Vec<Component> = rel_path.components().filter(|c| !matches!(c, Component::CurDir)).collect();
    if parts.is_empty() || !parts.iter().all(|c| matches!(c, Component::Normal(_))) {
        return Err(refuse(crate::t!("Proje dışını gösteren yol: {rel}", "Path points outside the project: {rel}")));
    }
    let mut path = root.to_path_buf();
    for (i, part) in parts.iter().enumerate() {
        path.push(part);
        let last = i + 1 == parts.len();
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(root.join(rel_path)),
            Err(e) => return Err(e),
        };
        if meta.file_type().is_symlink() {
            return Err(refuse(crate::t!(
                "{rel}: yolda sembolik bağlantı var; hedef dosya elle düzenlenmeli",
                "{rel}: the path goes through a symbolic link; edit the target file by hand"
            )));
        }
        if !last && !meta.is_dir() {
            return Err(refuse(crate::t!("{rel}: üst yol bir klasör değil", "{rel}: a parent path isn't a folder")));
        }
        if last && !meta.is_file() {
            return Err(refuse(crate::t!("{rel}: normal bir dosya değil", "{rel}: not a regular file")));
        }
        if last && hard_linked(&meta) {
            return Err(refuse(crate::t!(
                "{rel}: dosyanın başka adları da var (hard link); hepsi birlikte değişeceği için elle düzenlenmeli",
                "{rel}: the file has other names (hard link); edit it by hand, since every name would change"
            )));
        }
    }
    Ok(path)
}

const MAX_SOURCE: u64 = 4 * 1024 * 1024;

pub fn lines_around(root: &Path, rel: &str, line: usize, around: usize) -> Result<Vec<(usize, String)>> {
    let path = resolve(root, rel)?;
    if std::fs::metadata(&path)?.len() > MAX_SOURCE {
        return Err(refuse(crate::t!("{rel}: dosya çok büyük", "{rel}: the file is too large")));
    }
    let bytes = std::fs::read(&path)?;
    let text = String::from_utf8_lossy(&bytes);
    let first = line.saturating_sub(around).max(1);
    let last = line.saturating_add(around);
    Ok(text
        .lines()
        .enumerate()
        .map(|(i, text)| (i + 1, text))
        .filter(|(n, _)| (first..=last).contains(n))
        .map(|(n, text)| (n, text.to_string()))
        .collect())
}

pub fn replace(path: &Path, bytes: &[u8]) -> Result<()> {
    let meta = std::fs::symlink_metadata(path).ok();
    if meta.as_ref().is_some_and(|m| m.file_type().is_symlink()) {
        return Err(refuse(crate::t!(
            "sembolik bağlantı; hedef dosya elle düzenlenmeli",
            "it's a symbolic link; edit the target file by hand"
        )));
    }
    if meta.as_ref().is_some_and(hard_linked) {
        return Err(refuse(crate::t!(
            "dosyanın başka adları da var (hard link); elle düzenlenmeli",
            "the file has other names (hard link); edit it by hand"
        )));
    }
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    let readonly = meta.as_ref().is_some_and(|m| m.permissions().readonly());
    if let Some(meta) = &meta {
        let mut perms = meta.permissions();
        if cfg!(windows) {
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
        }
        let _ = tmp.as_file().set_permissions(perms);
    } else {
        set_new_file_mode(tmp.as_file());
    }
    if readonly && cfg!(windows) {
        set_readonly(path, false)?;
    }
    if let Err(e) = tmp.persist(path) {
        if readonly && cfg!(windows) {
            let _ = set_readonly(path, true);
        }
        return Err(e.error);
    }
    if readonly {
        set_readonly(path, true)?;
    }
    Ok(())
}

/// A temporary file starts private. A file that's being restored must be readable like any other.
#[cfg(unix)]
fn set_new_file_mode(file: &std::fs::File) {
    use std::os::unix::fs::PermissionsExt;
    let _ = file.set_permissions(std::fs::Permissions::from_mode(0o644));
}

#[cfg(not(unix))]
fn set_new_file_mode(_: &std::fs::File) {}

pub fn set_readonly(path: &Path, readonly: bool) -> Result<()> {
    let mut perms = std::fs::metadata(path)?.permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(readonly);
    std::fs::set_permissions(path, perms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn reads_the_lines_around_a_finding() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.php"), "one\r\ntwo\nthree\nfour\nfive\n").unwrap();
        let lines = |line| lines_around(dir.path(), "a.php", line, 1).unwrap();
        assert_eq!(lines(3), vec![(2, "two".to_string()), (3, "three".to_string()), (4, "four".to_string())]);
        assert_eq!(lines(1), vec![(1, "one".to_string()), (2, "two".to_string())]);
        assert_eq!(lines(5), vec![(4, "four".to_string()), (5, "five".to_string())]);
        assert!(lines(9).is_empty());
    }

    #[test]
    fn stays_inside_the_project_when_reading_lines() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("project");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(dir.path().join("secret.txt"), "x").unwrap();
        assert!(lines_around(&root, "../secret.txt", 1, 1).is_err());
        assert!(lines_around(&root, "/etc/hosts", 1, 1).is_err());
    }

    #[test]
    fn replace_keeps_read_only_files_read_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.php");
        fs::write(&path, "old").unwrap();
        set_readonly(&path, true).unwrap();
        replace(&path, b"new").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        assert!(fs::metadata(&path).unwrap().permissions().readonly());
        set_readonly(&path, false).unwrap();
    }

    #[test]
    fn resolve_rejects_paths_leaving_the_root() {
        let dir = tempfile::tempdir().unwrap();
        for bad in ["../x.php", "/etc/passwd", "", ".", "a/../../x.php"] {
            assert!(resolve(dir.path(), bad).is_err(), "{bad}");
        }
        assert_eq!(resolve(dir.path(), "src/new.php").unwrap(), dir.path().join("src/new.php"));
        fs::create_dir(dir.path().join("src")).unwrap();
        assert!(resolve(dir.path(), "src").is_err(), "a folder isn't a file");
    }

    #[cfg(unix)]
    #[test]
    fn resolve_refuses_links_anywhere_on_the_path() {
        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("a.php"), "victim").unwrap();
        std::os::unix::fs::symlink(outside.path(), project.path().join("src")).unwrap();
        std::os::unix::fs::symlink(outside.path().join("a.php"), project.path().join("b.php")).unwrap();
        assert!(resolve(project.path(), "src/a.php").is_err());
        assert!(resolve(project.path(), "b.php").is_err());

        fs::write(project.path().join("c.php"), "x").unwrap();
        fs::hard_link(project.path().join("c.php"), outside.path().join("alias.php")).unwrap();
        assert!(resolve(project.path(), "c.php").is_err());
        assert!(replace(&project.path().join("c.php"), b"new").is_err());
        assert_eq!(fs::read_to_string(outside.path().join("alias.php")).unwrap(), "x");
    }

    #[cfg(unix)]
    #[test]
    fn a_replaced_file_keeps_its_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.php");
        for mode in [0o644, 0o600, 0o640, 0o755, 0o444] {
            std::fs::write(&file, "old").unwrap();
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(mode)).unwrap();
            replace(&file, b"new").unwrap();
            assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, mode);
            std::fs::remove_file(&file).unwrap();
        }
        replace(&file, b"restored").unwrap();
        assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o644);
    }
}
