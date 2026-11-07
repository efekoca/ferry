use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub enum SnapshotError {
    InvalidPath(String),
    NotFound(String),
    Corrupt(String),
    Io { path: String, source: std::io::Error },
    Restore { path: String, source: std::io::Error, unrecovered: Vec<String> },
    Manifest(serde_json::Error),
}

impl From<serde_json::Error> for SnapshotError {
    fn from(e: serde_json::Error) -> Self {
        SnapshotError::Manifest(e)
    }
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            SnapshotError::InvalidPath(p) => crate::t!("Geçersiz dosya yolu: {p}", "Invalid file path: {p}"),
            SnapshotError::NotFound(id) => crate::t!("Yedek bulunamadı: {id}", "Snapshot not found: {id}"),
            SnapshotError::Corrupt(p) => crate::t!(
                "Yedekteki {p} kopyası bozuk ya da eksik; hiçbir dosya değiştirilmedi.",
                "The backup copy of {p} is damaged or missing; no files were changed."
            ),
            SnapshotError::Io { path, source } => {
                let reason = crate::i18n::io_error(source);
                crate::t!("Dosya işlemi başarısız ({path}): {reason}", "File operation failed ({path}): {reason}")
            }
            SnapshotError::Restore { path, source, unrecovered } => {
                let reason = crate::i18n::io_error(source);
                if unrecovered.is_empty() {
                    crate::t!(
                        "Geri yükleme başarısız ({path}): {reason}. Dosyalar önceki hâlinde bırakıldı.",
                        "Restore failed ({path}): {reason}. The files were left as they were."
                    )
                } else {
                    let files = unrecovered.join(", ");
                    crate::t!(
                        "Geri yükleme başarısız ({path}): {reason}. Şu dosyalar önceki hâline döndürülemedi: {files}",
                        "Restore failed ({path}): {reason}. These files couldn't be put back as they were: {files}"
                    )
                }
            }
            SnapshotError::Manifest(e) => crate::t!("Yedek bilgisi okunamadı: {e}", "Couldn't read snapshot info: {e}"),
        };
        f.write_str(&msg)
    }
}

impl std::error::Error for SnapshotError {}

fn io_err(path: &Path) -> impl FnOnce(std::io::Error) -> SnapshotError + '_ {
    move |source| SnapshotError::Io { path: path.display().to_string(), source }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotFile {
    pub path: String,
    pub existed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    #[serde(default)]
    pub version: u32,
    pub id: String,
    pub created_at: u64,
    pub project_root: String,
    pub label: String,
    pub files: Vec<SnapshotFile>,
}

pub struct SnapshotStore {
    base: PathBuf,
}

pub const VERSION: u32 = 2;

pub fn content_hash(bytes: &[u8]) -> String {
    format!("{:016x}", fnv1a(bytes))
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), SnapshotError> {
    use std::io::Write;
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir).map_err(io_err(dir))?;
    tmp.write_all(bytes).and_then(|_| tmp.as_file().sync_all()).map_err(io_err(path))?;
    tmp.persist(path).map_err(|e| io_err(path)(e.error))?;
    Ok(())
}

fn put(target: &Path, content: Option<&[u8]>) -> std::io::Result<()> {
    match content {
        Some(bytes) => {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            crate::safefs::replace(target, bytes)
        }
        None => match std::fs::remove_file(target) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            other => other,
        },
    }
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, SnapshotError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_err(path)(e)),
    }
}

const WRITING_MARKER: &str = "writing";

pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn safe_relative(rel: &str) -> Result<PathBuf, SnapshotError> {
    let path = PathBuf::from(rel);
    let ok = !rel.is_empty() && path.components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir));
    if ok {
        Ok(path)
    } else {
        Err(SnapshotError::InvalidPath(rel.to_string()))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub snapshots: usize,
    pub projects: usize,
    pub bytes: u64,
}

fn dir_size(dir: &Path) -> u64 {
    walkdir::WalkDir::new(dir).into_iter().flatten().filter_map(|e| e.metadata().ok()).filter(|m| m.is_file()).map(|m| m.len()).sum()
}

impl SnapshotStore {
    pub fn usage(&self) -> Usage {
        let mut usage = Usage::default();
        let Ok(projects) = std::fs::read_dir(&self.base) else { return usage };
        for project in projects.flatten().filter(|e| e.path().is_dir()) {
            let count = std::fs::read_dir(project.path())
                .map(|entries| entries.flatten().filter(|e| e.path().join("manifest.json").is_file()).count())
                .unwrap_or(0);
            if count > 0 {
                usage.projects += 1;
                usage.snapshots += count;
            }
            usage.bytes += dir_size(&project.path());
        }
        usage
    }

    pub fn delete_all(&self) -> Result<(), SnapshotError> {
        if !self.base.exists() {
            return Ok(());
        }
        let trash = self.base.with_file_name(format!(".trash-snapshots-{}", std::process::id()));
        std::fs::rename(&self.base, &trash).map_err(io_err(&self.base))?;
        let _ = std::fs::remove_dir_all(&trash);
        Ok(())
    }

    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }

    fn project_dir(&self, project_root: &Path) -> PathBuf {
        let key = fnv1a(project_root.to_string_lossy().as_bytes());
        self.base.join(format!("{key:016x}"))
    }

    pub fn create(&self, project_root: &Path, rel_paths: &[String], label: &str) -> Result<Snapshot, SnapshotError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        let project_dir = self.project_dir(project_root);
        std::fs::create_dir_all(&project_dir).map_err(io_err(&project_dir))?;
        let mut id = now.to_string();
        let mut n = 1;
        loop {
            match std::fs::create_dir(project_dir.join(&id)) {
                Ok(()) => break,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    id = format!("{now}-{n}");
                    n += 1;
                }
                Err(e) => return Err(io_err(&project_dir)(e)),
            }
        }
        let dir = project_dir.join(&id);
        let result = self.fill(project_root, &dir, id, now, rel_paths, label);
        if result.is_err() {
            let _ = std::fs::remove_dir_all(&dir);
        }
        result
    }

    fn fill(
        &self,
        project_root: &Path,
        dir: &Path,
        id: String,
        now: u64,
        rel_paths: &[String],
        label: &str,
    ) -> Result<Snapshot, SnapshotError> {
        let files_dir = dir.join("files");
        std::fs::create_dir_all(&files_dir).map_err(io_err(&files_dir))?;

        let mut files = Vec::new();
        for rel in rel_paths {
            let rel_path = safe_relative(rel)?;
            let src = crate::safefs::resolve(project_root, rel).map_err(io_err(&project_root.join(&rel_path)))?;
            let content = read_optional(&src)?;
            if let Some(bytes) = &content {
                let dest = files_dir.join(&rel_path);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).map_err(io_err(parent))?;
                }
                write_new(&dest, bytes)?;
            }
            files.push(SnapshotFile { path: rel.clone(), existed: content.is_some(), hash: content.as_deref().map(content_hash) });
        }
        let snapshot = Snapshot {
            version: VERSION,
            id,
            created_at: now,
            project_root: project_root.to_string_lossy().to_string(),
            label: label.to_string(),
            files,
        };
        write_new(&dir.join("manifest.json"), &serde_json::to_vec_pretty(&snapshot)?)?;
        Ok(snapshot)
    }

    pub fn list(&self, project_root: &Path) -> Result<Vec<Snapshot>, SnapshotError> {
        let dir = self.project_dir(project_root);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(".trash-") {
                continue;
            }
            let manifest = entry.path().join("manifest.json");
            if let Ok(bytes) = std::fs::read(&manifest) {
                if let Ok(s) = serde_json::from_slice::<Snapshot>(&bytes) {
                    out.push(s);
                }
            }
        }
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
        Ok(out)
    }

    /// `newest` always stays: dates alone can't tell, because the clock may have been set back.
    pub fn prune(&self, project_root: &Path, keep: usize, newest: &str) -> Result<Vec<String>, SnapshotError> {
        let mut removed = Vec::new();
        let mut list = self.list(project_root)?;
        if let Some(at) = list.iter().position(|s| s.id == newest) {
            list[..=at].rotate_right(1);
        }
        for old in list.into_iter().skip(keep).filter(|s| !self.was_interrupted(project_root, &s.id)) {
            self.delete(project_root, &old.id)?;
            removed.push(old.id);
        }
        Ok(removed)
    }

    fn load(&self, project_root: &Path, id: &str) -> Result<(Snapshot, PathBuf), SnapshotError> {
        safe_relative(id)?;
        let dir = self.project_dir(project_root).join(id);
        let bytes = std::fs::read(dir.join("manifest.json")).map_err(|_| SnapshotError::NotFound(id.to_string()))?;
        Ok((serde_json::from_slice(&bytes)?, dir))
    }

    /// Left in the backup while files are written. It only survives if the app stops mid-way.
    pub fn mark_writing(&self, project_root: &Path, id: &str) -> Result<(), SnapshotError> {
        let (_, dir) = self.load(project_root, id)?;
        std::fs::write(dir.join(WRITING_MARKER), b"").map_err(io_err(&dir))
    }

    pub fn clear_writing(&self, project_root: &Path, id: &str) {
        if let Ok((_, dir)) = self.load(project_root, id) {
            let _ = std::fs::remove_file(dir.join(WRITING_MARKER));
        }
    }

    fn was_interrupted(&self, project_root: &Path, id: &str) -> bool {
        self.load(project_root, id).is_ok_and(|(_, dir)| dir.join(WRITING_MARKER).is_file())
    }

    /// Backups whose write never finished. The mark stays until the backup is restored or the
    /// user says the files are fine, so a warning that went unseen comes back.
    pub fn interrupted(&self, project_root: &Path) -> Vec<Snapshot> {
        let found = self.list(project_root).unwrap_or_default();
        found.into_iter().filter(|s| self.was_interrupted(project_root, &s.id)).collect()
    }

    pub fn restore(&self, project_root: &Path, id: &str) -> Result<Vec<String>, SnapshotError> {
        self.restore_files(project_root, id, None)
    }

    pub fn get(&self, project_root: &Path, id: &str) -> Result<Snapshot, SnapshotError> {
        self.load(project_root, id).map(|(snapshot, _)| snapshot)
    }

    pub fn restore_files(&self, project_root: &Path, id: &str, only: Option<&[String]>) -> Result<Vec<String>, SnapshotError> {
        let (snapshot, dir) = self.load(project_root, id)?;
        let mut plan = Vec::new();
        for file in snapshot.files.iter().filter(|f| only.is_none_or(|o| o.contains(&f.path))) {
            let rel = safe_relative(&file.path)?;
            let target = crate::safefs::resolve(project_root, &file.path).map_err(io_err(&project_root.join(&rel)))?;
            let backup = if file.existed {
                let bytes = std::fs::read(dir.join("files").join(&rel)).map_err(|_| SnapshotError::Corrupt(file.path.clone()))?;
                if file.hash.as_ref().is_some_and(|h| *h != content_hash(&bytes)) {
                    return Err(SnapshotError::Corrupt(file.path.clone()));
                }
                Some(bytes)
            } else {
                None
            };
            let current = read_optional(&target)?;
            plan.push((file.path.clone(), target, backup, current));
        }
        for (i, (_, target, backup, current)) in plan.iter().enumerate() {
            if backup == current {
                continue;
            }
            if let Err(e) = put(target, backup.as_deref()) {
                let unrecovered = plan[..=i]
                    .iter()
                    .rev()
                    .filter(|(_, target, _, current)| {
                        let unchanged = read_optional(target).is_ok_and(|now| now == *current);
                        !unchanged && put(target, current.as_deref()).is_err()
                    })
                    .map(|(path, ..)| path.clone())
                    .collect();
                return Err(SnapshotError::Restore { path: plan[i].0.clone(), source: e, unrecovered });
            }
        }
        Ok(plan.into_iter().map(|(path, ..)| path).collect())
    }

    pub fn write_attachment(&self, project_root: &Path, id: &str, name: &str, bytes: &[u8]) -> Result<(), SnapshotError> {
        let (_, dir) = self.load(project_root, id)?;
        write_new(&dir.join(safe_relative(name)?), bytes)
    }

    pub fn read_attachment(&self, project_root: &Path, id: &str, name: &str) -> Result<Option<Vec<u8>>, SnapshotError> {
        let (_, dir) = self.load(project_root, id)?;
        read_optional(&dir.join(safe_relative(name)?))
    }

    pub fn originals(&self, project_root: &Path, id: &str) -> Result<Vec<(String, Option<String>)>, SnapshotError> {
        let (snapshot, dir) = self.load(project_root, id)?;
        snapshot
            .files
            .iter()
            .map(|file| {
                let saved = dir.join("files").join(safe_relative(&file.path)?);
                let content = file
                    .existed
                    .then(|| std::fs::read(&saved).map(|b| String::from_utf8_lossy(&b).into_owned()))
                    .transpose()
                    .map_err(io_err(&saved))?;
                Ok((file.path.clone(), content))
            })
            .collect()
    }

    pub fn delete(&self, project_root: &Path, id: &str) -> Result<(), SnapshotError> {
        let (_, dir) = self.load(project_root, id)?;
        let project_dir = self.project_dir(project_root);
        let trash = project_dir.join(format!(".trash-{id}"));
        std::fs::rename(&dir, &trash).map_err(io_err(&dir))?;
        for entry in std::fs::read_dir(&project_dir).into_iter().flatten().flatten() {
            if entry.file_name().to_string_lossy().starts_with(".trash-") {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn usage_counts_every_project_and_delete_all_clears_them() {
        let store_dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(store_dir.path().join("snapshots"));
        assert_eq!(store.usage(), Usage::default());
        let (one, two) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        fs::write(one.path().join("a.php"), "12345").unwrap();
        fs::write(two.path().join("b.php"), "1234567890").unwrap();
        store.create(one.path(), &["a.php".into()], "x").unwrap();
        store.create(one.path(), &["a.php".into()], "y").unwrap();
        store.create(two.path(), &["b.php".into()], "z").unwrap();
        let usage = store.usage();
        assert_eq!((usage.snapshots, usage.projects), (3, 2));
        assert!(usage.bytes >= 20, "{usage:?}");

        store.delete_all().unwrap();
        assert_eq!(store.usage(), Usage::default());
        assert!(store.list(one.path()).unwrap().is_empty());
        store.delete_all().unwrap();
        store.create(one.path(), &["a.php".into()], "again").unwrap();
        assert_eq!(store.usage().snapshots, 1);
    }

    #[test]
    fn prune_keeps_the_backup_just_made_when_the_clock_went_back() {
        let project = tempfile::tempdir().unwrap();
        let store_dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(store_dir.path());
        let root = project.path();
        fs::write(root.join("a.php"), "x").unwrap();
        let older = store.create(root, &["a.php".to_string()], "older").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let newest = store.create(root, &["a.php".to_string()], "newest").unwrap();
        let manifest = store.project_dir(root).join(&older.id).join("manifest.json");
        let mut json: serde_json::Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
        json["createdAt"] = serde_json::json!(newest.created_at + 60_000);
        fs::write(&manifest, serde_json::to_vec(&json).unwrap()).unwrap();
        assert_eq!(store.prune(root, 1, &newest.id).unwrap(), [older.id]);
        assert_eq!(store.list(root).unwrap()[0].id, newest.id);
    }

    #[test]
    fn prune_keeps_the_newest_snapshots() {
        let project = tempfile::tempdir().unwrap();
        let store_dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(store_dir.path());
        let root = project.path();
        fs::write(root.join("a.php"), "x").unwrap();
        let ids: Vec<String> = (0..4).map(|n| store.create(root, &["a.php".into()], &format!("s{n}")).unwrap().id).collect();
        let newest: Vec<String> = store.list(root).unwrap().into_iter().map(|s| s.id).collect();
        assert_eq!(newest.len(), 4);

        let mut removed = store.prune(root, 2, "").unwrap();
        removed.sort();
        let mut expected = newest[2..].to_vec();
        expected.sort();
        assert_eq!(removed, expected);
        let left: Vec<String> = store.list(root).unwrap().into_iter().map(|s| s.id).collect();
        assert_eq!(left, newest[..2]);
        assert!(store.prune(root, 2, "").unwrap().is_empty());
        assert!(ids.iter().all(|id| left.contains(id) || removed.contains(id)));
    }

    #[test]
    fn create_list_restore_delete() {
        let project = tempfile::tempdir().unwrap();
        let store_dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(store_dir.path());
        let root = project.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/a.php"), "original").unwrap();

        let snap = store.create(root, &["src/a.php".into(), "src/new.php".into()], "Rector").unwrap();
        assert_eq!(snap.files.iter().map(|f| f.existed).collect::<Vec<_>>(), vec![true, false]);

        fs::write(root.join("src/a.php"), "changed").unwrap();
        fs::write(root.join("src/new.php"), "created").unwrap();

        let second = store.create(root, &["src/a.php".into()], "ikinci").unwrap();
        let list = store.list(root).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, second.id, "newest first");

        let restored = store.restore(root, &snap.id).unwrap();
        assert_eq!(restored, vec!["src/a.php", "src/new.php"]);
        assert_eq!(fs::read_to_string(root.join("src/a.php")).unwrap(), "original");
        assert!(!root.join("src/new.php").exists());

        store.write_attachment(root, &snap.id, "changes.json", b"[]").unwrap();
        assert_eq!(store.read_attachment(root, &snap.id, "changes.json").unwrap().as_deref(), Some(&b"[]"[..]));
        assert_eq!(store.read_attachment(root, &second.id, "changes.json").unwrap(), None);
        assert_eq!(
            store.originals(root, &snap.id).unwrap(),
            vec![("src/a.php".to_string(), Some("original".to_string())), ("src/new.php".to_string(), None)]
        );

        store.delete(root, &snap.id).unwrap();
        assert_eq!(store.list(root).unwrap().len(), 1);
        assert!(matches!(store.restore(root, &snap.id), Err(SnapshotError::NotFound(_))));
    }

    #[test]
    fn restores_read_only_files_and_subsets() {
        let project = tempfile::tempdir().unwrap();
        let store_dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(store_dir.path());
        let root = project.path();
        fs::write(root.join("a.php"), "a").unwrap();
        fs::write(root.join("b.php"), "b").unwrap();
        let snap = store.create(root, &["a.php".into(), "b.php".into()], "x").unwrap();
        fs::write(root.join("a.php"), "A").unwrap();
        fs::write(root.join("b.php"), "B").unwrap();
        let mut perms = fs::metadata(root.join("a.php")).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(root.join("a.php"), perms).unwrap();

        assert_eq!(store.restore_files(root, &snap.id, Some(&["a.php".to_string()])).unwrap(), vec!["a.php"]);
        assert_eq!(fs::read_to_string(root.join("a.php")).unwrap(), "a");
        assert_eq!(fs::read_to_string(root.join("b.php")).unwrap(), "B");
    }

    #[test]
    fn rejects_escaping_paths() {
        let project = tempfile::tempdir().unwrap();
        let store_dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(store_dir.path());
        for bad in ["../x.php", "/etc/passwd", ""] {
            assert!(matches!(store.create(project.path(), &[bad.to_string()], "x"), Err(SnapshotError::InvalidPath(_))));
        }
        assert!(store.restore(project.path(), "../../x").is_err());
    }

    #[test]
    fn restore_checks_every_backup_before_writing() {
        let project = tempfile::tempdir().unwrap();
        let store_dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(store_dir.path());
        let root = project.path();
        fs::write(root.join("a.php"), "a").unwrap();
        fs::write(root.join("b.php"), "b").unwrap();
        let snap = store.create(root, &["a.php".into(), "b.php".into()], "x").unwrap();
        fs::write(root.join("a.php"), "A").unwrap();
        fs::write(root.join("b.php"), "B").unwrap();
        let backup = store.project_dir(root).join(&snap.id).join("files/b.php");

        fs::write(&backup, "tampered").unwrap();
        assert!(matches!(store.restore(root, &snap.id), Err(SnapshotError::Corrupt(p)) if p == "b.php"));
        fs::remove_file(&backup).unwrap();
        assert!(matches!(store.restore(root, &snap.id), Err(SnapshotError::Corrupt(_))));
        assert_eq!(fs::read_to_string(root.join("a.php")).unwrap(), "A", "nothing is written when a backup is bad");
        assert_eq!(fs::read_to_string(root.join("b.php")).unwrap(), "B");
    }

    #[cfg(unix)]
    #[test]
    fn restore_rolls_back_when_a_write_fails() {
        use std::os::unix::fs::PermissionsExt;
        let project = tempfile::tempdir().unwrap();
        let store_dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(store_dir.path());
        let root = project.path();
        fs::create_dir(root.join("z")).unwrap();
        fs::write(root.join("a.php"), "a").unwrap();
        fs::write(root.join("z/b.php"), "b").unwrap();
        let snap = store.create(root, &["a.php".into(), "z/b.php".into()], "x").unwrap();
        fs::write(root.join("a.php"), "A").unwrap();
        fs::write(root.join("z/b.php"), "B").unwrap();
        fs::set_permissions(root.join("z"), fs::Permissions::from_mode(0o555)).unwrap();
        if fs::write(root.join("z/.probe"), "").is_ok() {
            return;
        }
        let result = store.restore(root, &snap.id);
        fs::set_permissions(root.join("z"), fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(&result, Err(SnapshotError::Restore { unrecovered, .. }) if unrecovered.is_empty()), "{result:?}");
        assert_eq!(fs::read_to_string(root.join("a.php")).unwrap(), "A");
        assert_eq!(fs::read_to_string(root.join("z/b.php")).unwrap(), "B");
    }

    #[cfg(unix)]
    #[test]
    fn restore_never_writes_through_links() {
        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let store_dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(store_dir.path());
        let root = project.path();
        fs::write(root.join("a.php"), "backup").unwrap();
        let snap = store.create(root, &["a.php".into()], "x").unwrap();
        fs::remove_file(root.join("a.php")).unwrap();
        fs::write(outside.path().join("victim.php"), "victim").unwrap();
        std::os::unix::fs::symlink(outside.path().join("victim.php"), root.join("a.php")).unwrap();
        assert!(store.restore(root, &snap.id).is_err());
        assert_eq!(fs::read_to_string(outside.path().join("victim.php")).unwrap(), "victim");
    }

    #[test]
    fn listing_never_deletes_and_deleting_sweeps_leftovers() {
        let project = tempfile::tempdir().unwrap();
        let store_dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(store_dir.path());
        let root = project.path();
        fs::write(root.join("a.php"), "a").unwrap();
        let snap = store.create(root, &["a.php".into()], "x").unwrap();
        let leftover = store.project_dir(root).join(".trash-old");
        fs::create_dir(&leftover).unwrap();
        assert_eq!(store.list(root).unwrap().len(), 1);
        assert!(leftover.exists(), "listing only reads");
        store.delete(root, &snap.id).unwrap();
        assert!(!leftover.exists());
    }

    #[test]
    fn empty_list_for_unknown_project() {
        let store_dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(store_dir.path());
        assert!(store.list(Path::new("/nope")).unwrap().is_empty());
    }

    #[test]
    fn an_unfinished_write_is_reported_once() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("p");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("a.php"), "a").unwrap();
        let store = SnapshotStore::new(dir.path().join("s"));
        let finished = store.create(&root, &["a.php".to_string()], "one").unwrap();
        store.mark_writing(&root, &finished.id).unwrap();
        store.clear_writing(&root, &finished.id);
        assert!(store.interrupted(&root).is_empty());
        store.mark_writing(&root, &finished.id).unwrap();
        for _ in 0..2 {
            assert_eq!(store.interrupted(&root).iter().map(|s| s.label.as_str()).collect::<Vec<_>>(), ["one"]);
        }
        store.create(&root, &["a.txt".to_string()], "two").unwrap();
        store.prune(&root, 1, "").unwrap();
        assert_eq!(store.interrupted(&root).len(), 1, "an unresolved backup outlives the limit");
        store.clear_writing(&root, &finished.id);
        assert!(store.interrupted(&root).is_empty());
    }
}
