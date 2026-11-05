use crate::i18n::Localized;
use crate::version::{php_allows, PhpVersion};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyStatus {
    Compatible,
    Incompatible,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub php_constraint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub php_conflict: Option<String>,
    pub status: DependencyStatus,
    pub dev: bool,
    pub note: Option<Localized>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyReport {
    pub target: PhpVersion,
    pub root_constraint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_conflict: Option<String>,
    pub root_status: DependencyStatus,
    pub packages: Vec<Dependency>,
    pub has_lock_file: bool,
    pub lock_unreadable: bool,
    pub extensions: Vec<crate::extensions::ExtensionUse>,
    pub wordpress: Vec<crate::wordpress::Component>,
}

fn status_for(constraint: Option<&str>, conflict: Option<&str>, target: PhpVersion) -> DependencyStatus {
    if constraint.is_none() && conflict.is_none() {
        return DependencyStatus::Unknown;
    }
    match php_allows(constraint, conflict, target) {
        Some(true) => DependencyStatus::Compatible,
        Some(false) => DependencyStatus::Incompatible,
        None => DependencyStatus::Unknown,
    }
}

fn read_json(path: &Path) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn check(root: &Path, target: PhpVersion) -> DependencyReport {
    let mut report = check_packages(root, target);
    let excluded = crate::project::inspect(root).map(|i| i.excluded_dirs).unwrap_or_default();
    report.extensions = crate::extensions::used(root, &crate::project::collect_php_files(root, &excluded));
    report.wordpress = crate::wordpress::inventory(root);
    report
}

/// Polyfills do nothing once PHP itself has the feature. Only ones the project asks for itself
/// can be removed: the rest come with other packages.
fn needless_polyfill(name: &str, target: PhpVersion) -> Option<Localized> {
    let digits = name.strip_prefix("symfony/polyfill-php");
    let built_in = match digits.and_then(|d| d.parse::<u32>().ok()) {
        Some(n) => PhpVersion::new(n / 10, n % 10),
        None if name == "paragonie/random_compat" => PhpVersion::new(7, 0),
        None => return None,
    };
    (built_in <= target).then(|| Localized {
        tr: format!(
            "PHP {built_in} bunu zaten içeriyor. Projenin PHP kısıtını yükselttikten sonra composer remove {name} ile kaldırabilirsiniz."
        ),
        en: format!(
            "PHP {built_in} already includes this. Once the project's PHP constraint is raised, composer remove {name} takes it out."
        ),
    })
}

pub fn check_packages(root: &Path, target: PhpVersion) -> DependencyReport {
    let manifest = read_json(&root.join("composer.json"));
    let field = |v: Option<&serde_json::Value>, path: &str| v.and_then(|j| j.pointer(path)).and_then(|v| v.as_str()).map(str::to_string);
    let root_constraint = field(manifest.as_ref(), "/require/php");
    let root_conflict = field(manifest.as_ref(), "/conflict/php");
    let lock = read_json(&root.join("composer.lock"));
    let direct =
        |name: &str| ["/require", "/require-dev"].iter().any(|key| manifest.as_ref().and_then(|m| m.pointer(key)?.get(name)).is_some());
    let mut packages = Vec::new();
    if let Some(lock) = &lock {
        for (key, dev) in [("packages", false), ("packages-dev", true)] {
            let Some(list) = lock.get(key).and_then(|v| v.as_array()) else {
                continue;
            };
            for pkg in list {
                let name = pkg.get("name").and_then(|v| v.as_str()).unwrap_or("?").to_string();
                let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or("?").to_string();
                let php_constraint = field(Some(pkg), "/require/php");
                let php_conflict = field(Some(pkg), "/conflict/php");
                let mut status = status_for(php_constraint.as_deref(), php_conflict.as_deref(), target);
                let mut note = None;
                if status == DependencyStatus::Incompatible
                    && status_for(php_constraint.as_deref(), None, target) == DependencyStatus::Compatible
                {
                    let (c, t) = (php_conflict.clone().unwrap_or_default(), target.to_string());
                    note = Some(Localized {
                        tr: format!("Paketin conflict alanı ({c}) PHP {t} ile kurulmasını engelliyor."),
                        en: format!("The package's conflict field ({c}) keeps it from installing on PHP {t}."),
                    });
                }
                if note.is_none() && direct(&name) {
                    note = needless_polyfill(&name, target);
                }
                if let Some(limit) = crate::framework::package_limit(&name, &version).filter(|l| target > l.max) {
                    let (max, t) = (limit.max.to_string(), target.to_string());
                    if limit.official {
                        status = DependencyStatus::Incompatible;
                        note = Some(Localized {
                            tr: format!("Bu sürüm resmi olarak en fazla PHP {} destekliyor.", crate::i18n::tr_accusative(&max.to_string())),
                            en: format!("This version officially supports PHP up to {max}."),
                        });
                    } else {
                        if status == DependencyStatus::Compatible {
                            status = DependencyStatus::Unknown;
                        }
                        note = Some(Localized {
                            tr: format!("Bu sürümün bakımı PHP {t} çıkmadan bitti; bu sürümle hiç test edilmedi."),
                            en: format!("This version stopped receiving fixes before PHP {t} came out and was never tested on it."),
                        });
                    }
                }
                packages.push(Dependency { name, version, php_constraint, php_conflict, status, dev, note });
            }
        }
    }
    packages.sort_by(|a, b| {
        let rank = |s: DependencyStatus| match s {
            DependencyStatus::Incompatible => 0,
            DependencyStatus::Unknown => 1,
            DependencyStatus::Compatible => 2,
        };
        (rank(a.status), &a.name).cmp(&(rank(b.status), &b.name))
    });
    DependencyReport {
        target,
        root_status: status_for(root_constraint.as_deref(), root_conflict.as_deref(), target),
        root_constraint,
        root_conflict,
        packages,
        has_lock_file: lock.is_some(),
        lock_unreadable: lock.is_none() && root.join("composer.lock").is_file(),
        extensions: Vec::new(),
        wordpress: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn conflict_fields_block_the_target() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("composer.json"), r#"{"require":{"php":"^8.0"},"conflict":{"php":">=8.4"}}"#).unwrap();
        fs::write(
            dir.path().join("composer.lock"),
            r#"{"packages": [
                {"name": "a/blocked", "version": "1.0.0", "require": {"php": "^8.0"}, "conflict": {"php": ">=8.4"}},
                {"name": "a/patch", "version": "1.0.0", "require": {"php": "^8.0"}, "conflict": {"php": "8.4.0"}},
                {"name": "a/only-conflict", "version": "1.0.0", "conflict": {"php": "<8.0"}}
            ]}"#,
        )
        .unwrap();
        let report = check_packages(dir.path(), PhpVersion::new(8, 4));
        assert_eq!((report.root_status, report.root_conflict.as_deref()), (DependencyStatus::Incompatible, Some(">=8.4")));
        let status = |n: &str| report.packages.iter().find(|p| p.name == n).unwrap();
        assert_eq!(status("a/blocked").status, DependencyStatus::Incompatible);
        assert!(status("a/blocked").note.as_ref().unwrap().en.contains("conflict"));
        assert_eq!(status("a/patch").status, DependencyStatus::Compatible);
        assert_eq!(status("a/only-conflict").status, DependencyStatus::Compatible);
        assert_eq!(check_packages(dir.path(), PhpVersion::new(8, 3)).root_status, DependencyStatus::Compatible);
    }

    #[test]
    fn classifies_locked_packages() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("composer.json"), r#"{"require":{"php":"^7.4"}}"#).unwrap();
        fs::write(
            dir.path().join("composer.lock"),
            r#"{
              "packages": [
                {"name": "old/lib", "version": "1.0.0", "require": {"php": ">=5.6 <8.0"}},
                {"name": "good/lib", "version": "2.1.0", "require": {"php": "^7.4 || ^8.0"}},
                {"name": "plain/lib", "version": "0.3.0"}
              ],
              "packages-dev": [
                {"name": "dev/tool", "version": "9.0.0", "require": {"php": ">=8.1"}}
              ]
            }"#,
        )
        .unwrap();
        let r = check(dir.path(), "8.3".parse().unwrap());
        assert!(r.has_lock_file);
        assert_eq!(r.root_status, DependencyStatus::Incompatible);
        let summary: Vec<_> = r.packages.iter().map(|p| (p.name.as_str(), p.status, p.dev)).collect();
        assert_eq!(
            summary,
            vec![
                ("old/lib", DependencyStatus::Incompatible, false),
                ("plain/lib", DependencyStatus::Unknown, false),
                ("dev/tool", DependencyStatus::Compatible, true),
                ("good/lib", DependencyStatus::Compatible, false),
            ]
        );
    }

    #[test]
    fn known_framework_limits_beat_loose_constraints() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("composer.lock"),
            r#"{
              "packages": [
                {"name": "laravel/framework", "version": "v9.52.16", "require": {"php": "^8.0.2"}},
                {"name": "illuminate/support", "version": "v11.30.0", "require": {"php": "^8.2"}},
                {"name": "symfony/http-kernel", "version": "v5.4.40", "require": {"php": ">=7.2.5"}},
                {"name": "symfony/console", "version": "v5.4.40", "require": {"php": ">=7.2.5"}}
              ]
            }"#,
        )
        .unwrap();
        let r = check(dir.path(), "8.5".parse().unwrap());
        let row = |n: &str| r.packages.iter().find(|p| p.name == n).unwrap();
        assert_eq!(row("laravel/framework").status, DependencyStatus::Incompatible);
        assert!(row("laravel/framework").note.as_ref().unwrap().en.contains("up to 8.2"));
        assert_eq!(row("illuminate/support").status, DependencyStatus::Incompatible);
        assert_eq!(row("symfony/http-kernel").status, DependencyStatus::Unknown);
        assert!(row("symfony/console").note.is_none(), "only core packages carry the Symfony limit");
        let ok = check(dir.path(), "8.2".parse().unwrap());
        assert!(ok.packages.iter().find(|p| p.name == "laravel/framework").unwrap().note.is_none());
    }

    #[test]
    fn polyfills_the_project_asks_for_can_go() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("composer.json"),
            r#"{"require":{"symfony/polyfill-php80":"^1.0","symfony/polyfill-php85":"^1.0","paragonie/random_compat":"^9.0"}}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("composer.lock"),
            r#"{"packages": [
                {"name": "symfony/polyfill-php80", "version": "v1.31.0"},
                {"name": "symfony/polyfill-php85", "version": "v1.33.0"},
                {"name": "symfony/polyfill-php81", "version": "v1.31.0"},
                {"name": "paragonie/random_compat", "version": "v9.99.100"}
            ]}"#,
        )
        .unwrap();
        let r = check_packages(dir.path(), "8.4".parse().unwrap());
        let note = |n: &str| r.packages.iter().find(|p| p.name == n).unwrap().note.as_ref().map(|l| l.en.clone());
        assert!(note("symfony/polyfill-php80").unwrap().contains("PHP 8.0 already includes this"));
        assert!(note("paragonie/random_compat").unwrap().contains("PHP 7.0"));
        assert_eq!(note("symfony/polyfill-php85"), None, "the target doesn't have it yet");
        assert_eq!(note("symfony/polyfill-php81"), None, "it came with another package");
    }

    #[test]
    fn missing_files_are_not_errors() {
        let dir = tempfile::tempdir().unwrap();
        let r = check(dir.path(), "8.3".parse().unwrap());
        assert!(!r.has_lock_file);
        assert!(r.packages.is_empty());
        assert_eq!(r.root_status, DependencyStatus::Unknown);
    }
}
