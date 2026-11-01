use crate::i18n::Localized;
use crate::version::{constraint_allows, PhpVersion};
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
    pub status: DependencyStatus,
    pub dev: bool,
    pub note: Option<Localized>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyReport {
    pub target: PhpVersion,
    pub root_constraint: Option<String>,
    pub root_status: DependencyStatus,
    pub packages: Vec<Dependency>,
    pub has_lock_file: bool,
}

fn status_for(constraint: Option<&str>, target: PhpVersion) -> DependencyStatus {
    match constraint.and_then(|c| constraint_allows(c, target)) {
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
    let root_constraint =
        read_json(&root.join("composer.json")).and_then(|j| j.pointer("/require/php").and_then(|v| v.as_str()).map(str::to_string));
    let lock = read_json(&root.join("composer.lock"));
    let mut packages = Vec::new();
    if let Some(lock) = &lock {
        for (key, dev) in [("packages", false), ("packages-dev", true)] {
            let Some(list) = lock.get(key).and_then(|v| v.as_array()) else {
                continue;
            };
            for pkg in list {
                let name = pkg.get("name").and_then(|v| v.as_str()).unwrap_or("?").to_string();
                let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or("?").to_string();
                let php_constraint = pkg.pointer("/require/php").and_then(|v| v.as_str()).map(str::to_string);
                let mut status = status_for(php_constraint.as_deref(), target);
                let mut note = None;
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
                packages.push(Dependency { name, version, php_constraint, status, dev, note });
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
        root_status: status_for(root_constraint.as_deref(), target),
        root_constraint,
        packages,
        has_lock_file: lock.is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
    fn missing_files_are_not_errors() {
        let dir = tempfile::tempdir().unwrap();
        let r = check(dir.path(), "8.3".parse().unwrap());
        assert!(!r.has_lock_file);
        assert!(r.packages.is_empty());
        assert_eq!(r.root_status, DependencyStatus::Unknown);
    }
}
