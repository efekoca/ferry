use crate::version::{Constraint, PhpVersion};
use serde::Serialize;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Framework {
    WordPress,
    Laravel,
    Symfony,
    Drupal,
    Generic,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub root: String,
    pub name: String,
    pub framework: Framework,
    pub php_files: usize,
    pub php_constraint: Option<String>,
    pub current_version: Option<PhpVersion>,
    pub has_composer_lock: bool,
    pub has_vendor: bool,
    pub is_git_repo: bool,
    pub excluded_dirs: Vec<String>,
}

pub(crate) const PHP_EXTENSIONS: [&str; 14] = [
    "php", "phtml", "inc", "module", "php3", "php4", "php5", "php7", "phpt", "install", "theme", "profile", "engine", "ctp",
];

const ALWAYS_SKIP: [&str; 8] = [
    "vendor", "node_modules", ".git", ".svn", ".hg", ".idea", ".vscode", ".ferry",
];

pub(crate) const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

pub fn detect_framework(root: &Path) -> Framework {
    if root.join("wp-config.php").exists()
        || root.join("wp-includes").is_dir()
        || root.join("wp-config-sample.php").exists()
    {
        return Framework::WordPress;
    }
    if root.join("artisan").is_file() {
        return Framework::Laravel;
    }
    let composer = std::fs::read_to_string(root.join("composer.json")).unwrap_or_default();
    if composer.contains("\"laravel/framework\"") {
        return Framework::Laravel;
    }
    if composer.contains("\"drupal/core") || drupal_core_dirs(root).next().is_some() {
        return Framework::Drupal;
    }
    if composer.contains("\"symfony/framework-bundle\"") || root.join("bin/console").is_file() {
        return Framework::Symfony;
    }
    Framework::Generic
}

pub fn framework_excludes(framework: Framework) -> Vec<String> {
    let dirs: &[&str] = match framework {
        Framework::WordPress => &["wp-admin", "wp-includes"],
        Framework::Laravel => &["storage", "bootstrap/cache"],
        Framework::Symfony => &["var"],
        Framework::Drupal => &["core", "sites/default/files"],
        Framework::Generic => &[],
    };
    dirs.iter().map(|d| d.to_string()).collect()
}

const DRUPAL_DOCROOTS: [&str; 5] = ["", "web", "docroot", "html", "public"];

fn drupal_core_dirs(root: &Path) -> impl Iterator<Item = &'static str> + '_ {
    DRUPAL_DOCROOTS
        .into_iter()
        .filter(move |d| root.join(d).join("core/lib/Drupal.php").is_file())
}

fn project_excludes(root: &Path, framework: Framework) -> Vec<String> {
    let mut dirs = framework_excludes(framework);
    if framework == Framework::Drupal {
        for docroot in drupal_core_dirs(root).filter(|d| !d.is_empty()) {
            dirs.push(format!("{docroot}/core"));
            dirs.push(format!("{docroot}/sites/default/files"));
        }
    }
    dirs
}

fn is_php_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| PHP_EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

pub fn collect_php_files(root: &Path, excluded: &[String]) -> Vec<PathBuf> {
    let excluded: Vec<PathBuf> = excluded.iter().map(|d| root.join(d)).collect();
    let keep = |e: &DirEntry| {
        if e.depth() == 0 {
            return true;
        }
        if e.file_type().is_dir() {
            let name = e.file_name().to_string_lossy();
            if name.starts_with('.') || ALWAYS_SKIP.iter().any(|s| *s == name) {
                return false;
            }
            return !excluded.iter().any(|x| x == e.path());
        }
        true
    };
    let mut files: Vec<PathBuf> = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(keep)
        .filter_map(|e| match e {
            Ok(e) => (e.file_type().is_file() && is_php_file(e.path())).then(|| e.into_path()),
            Err(e) => e.path().filter(|p| p.is_dir() || is_php_file(p)).map(Path::to_path_buf),
        })
        .collect();
    files.sort();
    files
}

fn composer_php_constraint(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("composer.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let php = json.get("require")?.get("php")?.as_str()?;
    Some(php.to_string())
}

fn composer_platform_php(root: &Path) -> Option<PhpVersion> {
    let text = std::fs::read_to_string(root.join("composer.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    json.pointer("/config/platform/php")?.as_str()?.parse().ok()
}

pub fn inspect(root: &Path) -> std::io::Result<ProjectInfo> {
    let meta = std::fs::metadata(root)?;
    if !meta.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            crate::t!("Seçilen yol bir klasör değil.", "The selected path isn't a folder."),
        ));
    }
    let framework = detect_framework(root);
    let excluded_dirs = project_excludes(root, framework);
    let php_files = collect_php_files(root, &excluded_dirs)
        .iter()
        .filter(|p| !p.is_dir())
        .count();
    let php_constraint = composer_php_constraint(root);
    let current_version = composer_platform_php(root).or_else(|| {
        php_constraint
            .as_deref()
            .and_then(Constraint::parse)
            .and_then(|c| c.lowest_known_line())
    });
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| root.to_string_lossy().to_string());
    Ok(ProjectInfo {
        root: root.to_string_lossy().to_string(),
        name,
        framework,
        php_files,
        php_constraint,
        current_version,
        has_composer_lock: root.join("composer.lock").is_file(),
        has_vendor: root.join("vendor").is_dir(),
        is_git_repo: root.join(".git").exists(),
        excluded_dirs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_wordpress_and_skips_core() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("wp-config.php"), "<?php").unwrap();
        fs::create_dir_all(root.join("wp-includes")).unwrap();
        fs::write(root.join("wp-includes/core.php"), "<?php").unwrap();
        fs::create_dir_all(root.join("wp-content/plugins/x")).unwrap();
        fs::write(root.join("wp-content/plugins/x/x.php"), "<?php").unwrap();
        fs::create_dir_all(root.join("vendor/a")).unwrap();
        fs::write(root.join("vendor/a/a.php"), "<?php").unwrap();
        fs::write(root.join("readme.txt"), "x").unwrap();

        let info = inspect(root).unwrap();
        assert_eq!(info.framework, Framework::WordPress);
        assert_eq!(info.php_files, 2);
        let files = collect_php_files(root, &info.excluded_dirs);
        let names: Vec<_> = files.iter().map(|f| crate::analyzer::relative_path(root, f)).collect();
        assert_eq!(names, vec!["wp-config.php", "wp-content/plugins/x/x.php"]);
    }

    #[test]
    fn reads_composer_constraint() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(
            root.join("composer.json"),
            r#"{"require": {"php": "^7.4 || ^8.0", "laravel/framework": "^8.0"}}"#,
        )
        .unwrap();
        let info = inspect(root).unwrap();
        assert_eq!(info.framework, Framework::Laravel);
        assert_eq!(info.php_constraint.as_deref(), Some("^7.4 || ^8.0"));
        assert_eq!(info.current_version, Some(PhpVersion::new(7, 4)));
    }

    #[test]
    fn platform_overrides_constraint() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("composer.json"),
            r#"{"require": {"php": ">=7.1"}, "config": {"platform": {"php": "8.0.30"}}}"#,
        )
        .unwrap();
        assert_eq!(inspect(dir.path()).unwrap().current_version, Some(PhpVersion::new(8, 0)));
    }

    #[test]
    fn drupal_docroot_core_is_excluded() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("composer.json"), r#"{"require": {"drupal/core-recommended": "^10"}}"#).unwrap();
        fs::create_dir_all(root.join("web/core/lib")).unwrap();
        fs::write(root.join("web/core/lib/Drupal.php"), "<?php").unwrap();
        fs::create_dir_all(root.join("web/modules/custom/m")).unwrap();
        fs::write(root.join("web/modules/custom/m/m.module"), "<?php").unwrap();
        let info = inspect(root).unwrap();
        assert_eq!(info.framework, Framework::Drupal);
        assert!(info.excluded_dirs.contains(&"web/core".to_string()));
        assert_eq!(info.php_files, 1);
    }

    #[test]
    fn oversized_files_are_kept_for_reporting() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("big.php"), vec![b' '; MAX_FILE_BYTES as usize + 1]).unwrap();
        assert_eq!(collect_php_files(dir.path(), &[]).len(), 1);
    }

    #[test]
    fn rejects_files() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.php");
        fs::write(&f, "<?php").unwrap();
        assert!(inspect(&f).is_err());
    }
}
