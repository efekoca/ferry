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
    Magento,
    PrestaShop,
    Joomla,
    Moodle,
    MediaWiki,
    Phpbb,
    CodeIgniter,
    Yii,
    CakePhp,
    OpenCart,
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
    pub config: crate::config::Config,
    pub config_error: Option<String>,
}

pub(crate) const PHP_EXTENSIONS: [&str; 13] =
    ["php", "phtml", "inc", "module", "php3", "php4", "php5", "php7", "install", "theme", "profile", "engine", "ctp"];

pub(crate) const ALWAYS_SKIP: [&str; 8] = ["vendor", "node_modules", ".git", ".svn", ".hg", ".idea", ".vscode", ".ferry"];

pub(crate) const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

pub fn detect_framework(root: &Path) -> Framework {
    if root.join("wp-config.php").exists() || root.join("wp-includes").is_dir() || root.join("wp-config-sample.php").exists() {
        return Framework::WordPress;
    }
    if root.join("artisan").is_file() {
        return Framework::Laravel;
    }
    if prestashop_root(root) {
        return Framework::PrestaShop;
    }
    if root.join("app/Mage.php").is_file() || root.join("bin/magento").is_file() {
        return Framework::Magento;
    }
    if root.join("libraries/src/Version.php").is_file() && root.join("administrator").is_dir()
        || root.join("libraries/cms/version/version.php").is_file()
    {
        return Framework::Joomla;
    }
    if ["version.php", "public/version.php"].iter().any(|v| crate::safefs::read_text(root.join(v)).is_ok_and(|t| t.contains("$release")))
        && (root.join("lib/moodlelib.php").is_file() || root.join("public/lib/moodlelib.php").is_file())
    {
        return Framework::Moodle;
    }
    if crate::safefs::read_text(root.join("includes/Defines.php")).is_ok_and(|t| t.contains("MW_VERSION"))
        || crate::safefs::read_text(root.join("includes/DefaultSettings.php")).is_ok_and(|t| t.contains("$wgVersion"))
    {
        return Framework::MediaWiki;
    }
    if ["includes/constants.php", "phpBB/includes/constants.php"]
        .iter()
        .any(|c| crate::safefs::read_text(root.join(c)).is_ok_and(|t| t.contains("PHPBB_VERSION")))
    {
        return Framework::Phpbb;
    }
    if ["", "upload"].iter().any(|d| root.join(d).join("system/startup.php").is_file() && root.join(d).join("catalog/controller").is_dir())
    {
        return Framework::OpenCart;
    }
    if root.join("system/CodeIgniter.php").is_file() || root.join("system/core/CodeIgniter.php").is_file() {
        return Framework::CodeIgniter;
    }
    if root.join("framework/YiiBase.php").is_file() {
        return Framework::Yii;
    }
    if root.join("lib/Cake/VERSION.txt").is_file() {
        return Framework::CakePhp;
    }
    let required = required_packages(root);
    let requires = |wanted: &[&str]| required.iter().any(|name| wanted.contains(&name.as_str()));
    let requires_prefix = |prefix: &str| required.iter().any(|name| name.starts_with(prefix));
    if requires(&["roots/wordpress", "johnpbloch/wordpress"]) || root.join("web/wp/wp-includes").is_dir() {
        return Framework::WordPress;
    }
    if requires(&["codeigniter4/framework"]) {
        return Framework::CodeIgniter;
    }
    if requires(&["yiisoft/yii2", "yiisoft/yii"]) {
        return Framework::Yii;
    }
    if requires(&["cakephp/cakephp"]) {
        return Framework::CakePhp;
    }
    if requires(&["laravel/framework"]) {
        return Framework::Laravel;
    }
    if requires_prefix("magento/product-") || requires(&["magento/magento2-base"]) {
        return Framework::Magento;
    }
    if requires_prefix("drupal/core") || drupal_core_dirs(root).next().is_some() || drupal7_root(root) {
        return Framework::Drupal;
    }
    if requires(&["symfony/framework-bundle", "symfony/symfony"]) || root.join("bin/console").is_file() {
        return Framework::Symfony;
    }
    Framework::Generic
}

/// `require` says what the project runs on, and `name` that it's the framework's own repository.
/// `suggest`, `conflict` and `require-dev` say neither.
fn required_packages(root: &Path) -> Vec<String> {
    let text = crate::safefs::read_text(root.join("composer.json")).unwrap_or_default();
    let manifest: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    let required = manifest.get("require").and_then(|r| r.as_object()).into_iter().flat_map(|r| r.keys().map(String::as_str));
    required.chain(manifest.get("name").and_then(|n| n.as_str())).map(str::to_ascii_lowercase).collect()
}

pub fn framework_excludes(framework: Framework) -> Vec<String> {
    let dirs: &[&str] = match framework {
        Framework::WordPress => &["wp-admin", "wp-includes", "web/wp"],
        Framework::Laravel => &["storage", "bootstrap/cache"],
        Framework::Symfony => &["var"],
        Framework::Drupal => &["core", "sites/default/files"],
        Framework::Magento => &["generated", "var", "pub/static", "setup", "dev", "lib/internal", "app/code/core", "lib", "downloader"],
        Framework::PrestaShop => {
            &["classes", "controllers", "src", "Core", "Adapter", "app", "cache", "var", "install", "install-dev", "tests"]
        }
        Framework::Joomla => &["cache", "tmp", "administrator/cache", "libraries/vendor", "media"],
        Framework::Moodle => &["vendor", "lib/vendor", "public/lib/vendor"],
        Framework::MediaWiki => &["includes", "languages", "maintenance", "resources", "tests", "vendor", "cache"],
        Framework::Phpbb => &[
            "phpbb",
            "includes",
            "cache",
            "store",
            "install",
            "vendor",
            "phpBB/phpbb",
            "phpBB/includes",
            "phpBB/cache",
            "phpBB/store",
            "phpBB/install",
            "phpBB/vendor",
        ],
        Framework::CodeIgniter => &["system", "writable"],
        Framework::Yii => &["framework", "runtime", "web/assets", "protected/runtime"],
        Framework::CakePhp => &["lib/Cake", "tmp", "logs", "app/tmp"],
        Framework::OpenCart => &["system", "image", "install", "upload/system", "upload/image", "upload/install"],
        Framework::Generic => &[],
    };
    dirs.iter().map(|d| d.to_string()).collect()
}

const DRUPAL_DOCROOTS: [&str; 5] = ["", "web", "docroot", "html", "public"];

pub(crate) fn drupal7_root(root: &Path) -> bool {
    crate::safefs::read_text(root.join("includes/bootstrap.inc")).is_ok_and(|t| t.contains("define('VERSION', '7."))
}

pub(crate) fn prestashop_root(root: &Path) -> bool {
    crate::safefs::read_text(root.join("config/settings.inc.php")).is_ok_and(|t| t.contains("_PS_VERSION_"))
        || crate::safefs::read_text(root.join("app/AppKernel.php")).is_ok_and(|t| t.contains("PrestaShop"))
        || root.join("classes/PrestaShopAutoload.php").is_file()
}

fn drupal_core_dirs(root: &Path) -> impl Iterator<Item = &'static str> + '_ {
    DRUPAL_DOCROOTS.into_iter().filter(move |d| root.join(d).join("core/lib/Drupal.php").is_file())
}

fn project_excludes(root: &Path, framework: Framework) -> Vec<String> {
    let mut dirs = framework_excludes(framework);
    if framework == Framework::Drupal && drupal7_root(root) {
        dirs = [
            "includes",
            "misc",
            "modules",
            "scripts",
            "themes",
            "profiles/minimal",
            "profiles/standard",
            "profiles/testing",
            "sites/default/files",
        ]
        .iter()
        .map(|d| d.to_string())
        .collect();
    }
    if framework == Framework::Drupal {
        for docroot in drupal_core_dirs(root).filter(|d| !d.is_empty()) {
            dirs.push(format!("{docroot}/core"));
            dirs.push(format!("{docroot}/sites/default/files"));
        }
    }
    dirs
}

/// Changes when the code or the files that decide how it runs change. Size and modification time
/// are enough: this answers "is that result still about these files", not "are they identical".
pub fn stamp(root: &Path, files: &[PathBuf]) -> String {
    stamp_without(root, files, &[])
}

/// The stamp of everything but these files, for telling whether anything else changed.
pub fn stamp_without(root: &Path, files: &[PathBuf], left_out: &[String]) -> String {
    let extra = ["composer.json", "composer.lock", ".ferry/config.json"].map(|f| root.join(f));
    let left_out: Vec<PathBuf> = left_out.iter().map(|f| root.join(f)).collect();
    let mut parts: Vec<String> = files
        .iter()
        .chain(&extra)
        .filter(|path| !left_out.contains(path))
        .filter_map(|path| {
            let meta = std::fs::metadata(path).ok()?;
            let changed = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
            Some(format!("{}:{}:{changed}", path.display(), meta.len()))
        })
        .collect();
    parts.sort();
    format!("{:016x}", crate::snapshot::fnv1a(parts.join("\n").as_bytes()))
}

pub fn scanned_files(root: &Path) -> Vec<PathBuf> {
    inspect(root).map(|i| collect_php_files(root, &i.excluded_dirs)).unwrap_or_default()
}

pub fn is_php_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| PHP_EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// A folder that merely has this name, like `src/vendor`, can hold the project's own code.
fn composer_vendor(dir: &Path) -> bool {
    dir.join("autoload.php").is_file() || dir.join("composer").is_dir() || dir.parent().is_some_and(|p| p.join("composer.json").is_file())
}

pub(crate) fn nested_composer_vendors(root: &Path) -> Vec<String> {
    let keep = |e: &DirEntry| {
        let name = e.file_name().to_string_lossy();
        e.depth() == 0 || (e.file_type().is_dir() && !name.starts_with('.') && !ALWAYS_SKIP.iter().any(|s| *s == name && name != "vendor"))
    };
    let walk = WalkDir::new(root).follow_links(false).into_iter().filter_entry(keep).flatten();
    let found = walk.filter(|e| e.depth() > 1 && e.file_name() == "vendor" && composer_vendor(e.path()));
    let mut dirs: Vec<String> = found.map(|e| crate::analyzer::relative_path(root, e.path())).collect();
    dirs.retain(|d| !d.starts_with("vendor/"));
    dirs
}

/// `legacy`, `legacy/`, `./legacy/*` and `legacy/**` all mean the folder.
fn exclude_path(entry: &str) -> String {
    let path = entry.trim().replace('\\', "/");
    let path = path.trim_start_matches("./");
    path.strip_suffix("/**").or_else(|| path.strip_suffix("/*")).unwrap_or(path).trim_end_matches('/').to_string()
}

pub fn collect_php_files(root: &Path, excluded: &[String]) -> Vec<PathBuf> {
    let excluded: Vec<PathBuf> = excluded.iter().map(|d| root.join(d)).collect();
    let keep = |e: &DirEntry| {
        if e.depth() == 0 {
            return true;
        }
        if e.file_type().is_dir() {
            let name = e.file_name().to_string_lossy();
            let bundled = name == "vendor" && e.depth() > 1 && !composer_vendor(e.path());
            if name.starts_with('.') || (ALWAYS_SKIP.iter().any(|s| *s == name) && !bundled) {
                return false;
            }
            return !excluded.iter().any(|x| x == e.path());
        }
        !excluded.iter().any(|x| x == e.path())
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
    files.dedup();
    files
}

fn composer_php_constraint(root: &Path) -> Option<String> {
    let text = crate::safefs::read_text(root.join("composer.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let php = json.get("require")?.get("php")?.as_str()?;
    Some(php.to_string())
}

fn composer_platform_php(root: &Path) -> Option<PhpVersion> {
    let text = crate::safefs::read_text(root.join("composer.json")).ok()?;
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
    let (config, config_error) = match crate::config::load(root) {
        Ok(config) => (config, None),
        Err(e) => (Default::default(), Some(e)),
    };
    let mut excluded_dirs = project_excludes(root, framework);
    excluded_dirs.extend(config.exclude.iter().map(|d| exclude_path(d)));
    let php_files = collect_php_files(root, &excluded_dirs).iter().filter(|p| !p.is_dir()).count();
    let php_constraint = composer_php_constraint(root);
    let current_version =
        composer_platform_php(root).or_else(|| php_constraint.as_deref().and_then(Constraint::parse).and_then(|c| c.lowest_known_line()));
    let name = root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| root.to_string_lossy().to_string());
    Ok(ProjectInfo {
        root: root.to_string_lossy().to_string(),
        name,
        framework,
        php_files,
        php_constraint,
        current_version,
        has_composer_lock: root.join("composer.lock").is_file(),
        has_vendor: root.join("vendor").is_dir(),
        is_git_repo: root.ancestors().any(|dir| dir.join(".git").exists()),
        excluded_dirs,
        config,
        config_error,
    })
}

#[cfg(test)]
mod config_tests {
    use super::*;
    use std::fs;

    #[test]
    fn excludes_from_the_config_keep_files_out_of_the_scan() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("legacy")).unwrap();
        fs::write(dir.path().join("legacy/a.php"), "<?php").unwrap();
        fs::write(dir.path().join("b.php"), "<?php").unwrap();
        assert_eq!(inspect(dir.path()).unwrap().php_files, 2);
        let config = crate::config::Config { exclude: vec!["legacy/".into()], ..Default::default() };
        crate::config::save(dir.path(), &config).unwrap();
        let info = inspect(dir.path()).unwrap();
        assert_eq!((info.php_files, info.config_error), (1, None));
    }

    #[test]
    fn a_broken_config_is_reported_without_blocking_the_project() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(".ferry")).unwrap();
        fs::write(crate::config::path(dir.path()), "{").unwrap();
        assert!(inspect(dir.path()).unwrap().config_error.is_some());
    }
}

#[cfg(test)]
mod detection_tests {
    use super::*;
    use std::fs;

    fn with_composer(json: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("composer.json"), json).unwrap();
        dir
    }

    #[test]
    fn only_required_packages_name_the_framework() {
        let dir = with_composer(
            r#"{"suggest": {"laravel/framework": "x"}, "conflict": {"cakephp/cakephp": "*"}, "require-dev": {"yiisoft/yii2": "*"}}"#,
        );
        assert_eq!(detect_framework(dir.path()), Framework::Generic);
        let dir = with_composer(r#"{"require": {"laravel/framework": "^11"}}"#);
        assert_eq!(detect_framework(dir.path()), Framework::Laravel);
    }

    #[test]
    fn bedrock_is_wordpress_and_its_core_is_left_out() {
        let dir = with_composer(r#"{"require": {"roots/wordpress": "^6"}}"#);
        assert_eq!(detect_framework(dir.path()), Framework::WordPress);
        assert!(framework_excludes(Framework::WordPress).contains(&"web/wp".to_string()));
    }

    #[test]
    fn a_folder_called_vendor_is_only_skipped_when_composer_made_it() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for (folder, file) in
            [("vendor/a", "a.php"), ("src/vendor", "mine.php"), ("pkg/vendor/b", "b.php"), ("mod/vendor/composer", "c.php")]
        {
            fs::create_dir_all(root.join(folder)).unwrap();
            fs::write(root.join(folder).join(file), "<?php\n").unwrap();
        }
        fs::write(root.join("pkg/composer.json"), "{}").unwrap();
        let found: Vec<String> = collect_php_files(root, &[]).iter().map(|p| crate::analyzer::relative_path(root, p)).collect();
        assert_eq!(found, ["src/vendor/mine.php"]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn a_folder_inside_a_repository_counts_as_one() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        std::fs::create_dir_all(dir.path().join("apps/site")).unwrap();
        std::fs::write(dir.path().join("apps/site/a.php"), "<?php\n").unwrap();
        assert!(inspect(&dir.path().join("apps/site")).unwrap().is_git_repo);
    }

    #[test]
    fn exclude_entries_name_folders_in_any_form_and_single_files() {
        for entry in ["legacy", "legacy/", "./legacy", "legacy/*", "legacy/**", " legacy/ "] {
            assert_eq!(exclude_path(entry), "legacy", "{entry}");
        }
        assert_eq!(exclude_path("src\\old"), "src/old");
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("legacy")).unwrap();
        for file in ["legacy/a.php", "b.php", "c.php"] {
            std::fs::write(dir.path().join(file), "<?php\n").unwrap();
        }
        let files = collect_php_files(dir.path(), &["legacy".into(), "b.php".into()]);
        assert_eq!(files.iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect::<Vec<_>>(), ["c.php"]);
    }

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
        fs::write(root.join("composer.json"), r#"{"require": {"php": "^7.4 || ^8.0", "laravel/framework": "^8.0"}}"#).unwrap();
        let info = inspect(root).unwrap();
        assert_eq!(info.framework, Framework::Laravel);
        assert_eq!(info.php_constraint.as_deref(), Some("^7.4 || ^8.0"));
        assert_eq!(info.current_version, Some(PhpVersion::new(7, 4)));
    }

    #[test]
    fn platform_overrides_constraint() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("composer.json"), r#"{"require": {"php": ">=7.1"}, "config": {"platform": {"php": "8.0.30"}}}"#).unwrap();
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
