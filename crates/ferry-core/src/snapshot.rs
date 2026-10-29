use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub enum SnapshotError {
    InvalidPath(String),
    NotFound(String),
    Io { path: String, source: std::io::Error },
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
            SnapshotError::Io { path, source } => {
                crate::t!("Dosya işlemi başarısız ({path}): {source}", "File operation failed ({path}): {source}")
            }
            SnapshotError::Manifest(e) => crate::t!("Yedek bilgisi okunamadı: {e}", "Couldn't read snapshot info: {e}"),
        };
        f.write_str(&msg)
    }
}

impl std::error::Error for SnapshotError {}

fn io_err(path: &Path) -> impl FnOnce(std::io::Error) -> SnapshotError + '_ {
    move |source| SnapshotError::Io {
        path: path.display().to_string(),
        source,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotFile {
    pub path: String,
    pub existed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub id: String,
    pub created_at: u64,
    pub project_root: String,
    pub label: String,
    pub files: Vec<SnapshotFile>,
}

pub struct SnapshotStore {
    base: PathBuf,
}

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
    let ok = !rel.is_empty()
        && path
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir));
    if ok {
        Ok(path)
    } else {
        Err(SnapshotError::InvalidPath(rel.to_string()))
    }
}

impl SnapshotStore {
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }

    fn project_dir(&self, project_root: &Path) -> PathBuf {
        let key = fnv1a(project_root.to_string_lossy().as_bytes());
        self.base.join(format!("{key:016x}"))
    }

    pub fn create(
        &self,
        project_root: &Path,
        rel_paths: &[String],
        label: &str,
    ) -> Result<Snapshot, SnapshotError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
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
            let src = project_root.join(&rel_path);
            let existed = src.is_file();
            if existed {
                let dest = files_dir.join(&rel_path);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).map_err(io_err(parent))?;
                }
                std::fs::copy(&src, &dest).map_err(io_err(&src))?;
            }
            files.push(SnapshotFile {
                path: rel.clone(),
                existed,
            });
        }
        let snapshot = Snapshot {
            id,
            created_at: now,
            project_root: project_root.to_string_lossy().to_string(),
            label: label.to_string(),
            files,
        };
        let manifest = dir.join("manifest.json");
        std::fs::write(&manifest, serde_json::to_vec_pretty(&snapshot)?).map_err(io_err(&manifest))?;
        Ok(snapshot)
    }

    pub fn list(&self, project_root: &Path) -> Result<Vec<Snapshot>, SnapshotError> {
        let dir = self.project_dir(project_root);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
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

    fn load(&self, project_root: &Path, id: &str) -> Result<(Snapshot, PathBuf), SnapshotError> {
        safe_relative(id)?;
        let dir = self.project_dir(project_root).join(id);
        let bytes = std::fs::read(dir.join("manifest.json"))
            .map_err(|_| SnapshotError::NotFound(id.to_string()))?;
        Ok((serde_json::from_slice(&bytes)?, dir))
    }

    pub fn restore(&self, project_root: &Path, id: &str) -> Result<Vec<String>, SnapshotError> {
        let (snapshot, dir) = self.load(project_root, id)?;
        let mut restored = Vec::new();
        let mut first_error = None;
        for file in &snapshot.files {
            let result = (|| {
                let rel = safe_relative(&file.path)?;
                let target = project_root.join(&rel);
                if file.existed {
                    let saved = dir.join("files").join(&rel);
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent).map_err(io_err(parent))?;
                    }
                    std::fs::copy(&saved, &target).map_err(io_err(&saved))?;
                } else if target.exists() {
                    std::fs::remove_file(&target).map_err(io_err(&target))?;
                }
                Ok::<_, SnapshotError>(())
            })();
            match result {
                Ok(()) => restored.push(file.path.clone()),
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }
        match first_error {
            Some(e) => Err(e),
            None => Ok(restored),
        }
    }

    pub fn write_attachment(&self, project_root: &Path, id: &str, name: &str, bytes: &[u8]) -> Result<(), SnapshotError> {
        let (_, dir) = self.load(project_root, id)?;
        let path = dir.join(safe_relative(name)?);
        std::fs::write(&path, bytes).map_err(io_err(&path))
    }

    pub fn read_attachment(&self, project_root: &Path, id: &str, name: &str) -> Result<Option<Vec<u8>>, SnapshotError> {
        let (_, dir) = self.load(project_root, id)?;
        Ok(std::fs::read(dir.join(safe_relative(name)?)).ok())
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
        std::fs::remove_dir_all(&dir).map_err(io_err(&dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn create_list_restore_delete() {
        let project = tempfile::tempdir().unwrap();
        let store_dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(store_dir.path());
        let root = project.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/a.php"), "original").unwrap();

        let snap = store
            .create(root, &["src/a.php".into(), "src/new.php".into()], "Rector")
            .unwrap();
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
    fn rejects_escaping_paths() {
        let project = tempfile::tempdir().unwrap();
        let store_dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(store_dir.path());
        for bad in ["../x.php", "/etc/passwd", ""] {
            assert!(matches!(
                store.create(project.path(), &[bad.to_string()], "x"),
                Err(SnapshotError::InvalidPath(_))
            ));
        }
        assert!(store.restore(project.path(), "../../x").is_err());
    }

    #[test]
    fn empty_list_for_unknown_project() {
        let store_dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(store_dir.path());
        assert!(store.list(Path::new("/nope")).unwrap().is_empty());
    }
}
