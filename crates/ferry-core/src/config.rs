use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

/// Project settings meant to be committed, so a team and CI share them.
/// Nothing here connects to a server: a server copy always needs someone to confirm it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub target: Option<String>,
    pub exclude: Vec<String>,
    pub baseline: Option<String>,
    pub docker: DockerConfig,
    pub git: GitConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GitConfig {
    /// Refuse to write while the project has uncommitted changes.
    pub require_clean: bool,
    /// Offer each batch of changes on its own branch, with its own commit.
    pub branch_per_batch: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DockerConfig {
    pub source: Option<String>,
    pub extensions: Option<bool>,
    pub pages: Option<Vec<String>>,
    pub dump: Option<String>,
    pub prepare: Option<String>,
    pub test: Option<String>,
}

pub fn path(root: &Path) -> PathBuf {
    root.join(".ferry").join("config.json")
}

fn inside(relative: &str) -> bool {
    let path = Path::new(relative);
    !relative.is_empty() && path.components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

fn check(config: &Config) -> Result<(), String> {
    let paths = config.exclude.iter().chain(&config.baseline).chain(&config.docker.dump);
    match paths.into_iter().find(|p| !inside(p)) {
        Some(path) => Err(crate::t!("Yol proje içinde olmalı: {path}", "The path must be inside the project: {path}")),
        None => Ok(()),
    }
}

/// A missing file is an empty config. A broken one is an error, so settings are never dropped silently.
pub fn load(root: &Path) -> Result<Config, String> {
    let file = path(root);
    let Ok(bytes) = std::fs::read(&file) else { return Ok(Config::default()) };
    let config: Config = serde_json::from_slice(&bytes)
        .map_err(|e| crate::t!(".ferry/config.json okunamadı: {e}", "Couldn't read .ferry/config.json: {e}"))?;
    check(&config).map_err(|e| format!(".ferry/config.json: {e}"))?;
    Ok(config)
}

pub fn save(root: &Path, config: &Config) -> Result<(), String> {
    check(config)?;
    let file = path(root);
    let dir = file.parent().unwrap();
    let write = || -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let mut json = serde_json::to_vec_pretty(config).map_err(std::io::Error::other)?;
        json.push(b'\n');
        let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
        std::io::Write::write_all(&mut tmp, &json)?;
        tmp.persist(&file).map(|_| ()).map_err(|e| e.error)
    };
    write().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_config_survives_saving_and_loading() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()).unwrap(), Config::default());
        let config = Config {
            target: Some("8.4".into()),
            exclude: vec!["legacy".into()],
            git: GitConfig { require_clean: true, branch_per_batch: false },
            baseline: Some(".ferry/baseline.json".into()),
            docker: DockerConfig { pages: Some(vec!["/login".into()]), test: Some("vendor/bin/pest".into()), ..DockerConfig::default() },
        };
        save(dir.path(), &config).unwrap();
        assert_eq!(load(dir.path()).unwrap(), config);
    }

    #[test]
    fn unknown_fields_are_kept_out_of_the_way() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".ferry")).unwrap();
        std::fs::write(path(dir.path()), r#"{"target":"8.5","later":true,"docker":{"test":"t"}}"#).unwrap();
        let config = load(dir.path()).unwrap();
        assert_eq!((config.target.as_deref(), config.docker.test.as_deref()), (Some("8.5"), Some("t")));
    }

    #[test]
    fn broken_files_and_outside_paths_are_errors() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".ferry")).unwrap();
        std::fs::write(path(dir.path()), "{").unwrap();
        assert!(load(dir.path()).is_err());
        for bad in ["../secret", "/etc/passwd", ""] {
            let config = Config { docker: DockerConfig { dump: Some(bad.into()), ..DockerConfig::default() }, ..Config::default() };
            assert!(save(dir.path(), &config).is_err(), "{bad}");
        }
    }
}
