use crate::i18n::Localized;
use crate::project::Framework;
use crate::runtime::{Event, Level};
use crate::version::PhpVersion;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OriginKind {
    Project,
    Core,
    Package,
    Extension,
}

/// Where a group of messages comes from, which decides who can fix them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Origin {
    pub kind: OriginKind,
    pub name: String,
    pub version: Option<String>,
    pub messages: usize,
    pub fatal: usize,
    pub advice: Option<Localized>,
}

const GENERATED: [&str; 9] = ["cache", "tmp", "var", "storage", "store", "files", "static", "generated", "media"];

fn framework_name(framework: Framework) -> String {
    match framework {
        Framework::Phpbb => "phpBB".to_string(),
        Framework::CakePhp => "CakePHP".to_string(),
        other => format!("{other:?}"),
    }
}

fn in_core(rel: &str, framework: Framework) -> bool {
    if framework == Framework::WordPress {
        return ["wp-admin", "wp-includes"].iter().any(|d| rel.starts_with(&format!("{d}/")) || rel.contains(&format!("/{d}/")));
    }
    crate::project::framework_excludes(framework)
        .iter()
        .filter(|dir| !GENERATED.contains(&dir.rsplit('/').next().unwrap_or(dir)) && !dir.ends_with("vendor"))
        .any(|dir| rel.starts_with(&format!("{dir}/")))
}

fn package_of(rel: &str) -> Option<String> {
    let (_, rest) = rel.split_once("vendor/").filter(|(before, _)| before.is_empty() || before.ends_with('/'))?;
    let mut parts = rest.split('/');
    let (vendor, name) = (parts.next()?, parts.next()?);
    parts.next()?;
    Some(format!("{vendor}/{name}"))
}

/// Groups messages by what ships the file they came from, worst group first.
pub fn group(root: &Path, events: &[Event], target: PhpVersion) -> Vec<Origin> {
    let framework = crate::project::detect_framework(root);
    let packages = crate::deps::check_packages(root, target).packages;
    let components = crate::wordpress::inventory(root);
    let core_advice = crate::framework::check(root, framework, target).into_iter().next().map(|f| f.message);
    let mut groups: Vec<Origin> = Vec::new();
    for event in events {
        let rel = event.path.strip_prefix("/app/").unwrap_or(&event.path);
        let (kind, name, version, advice) = if let Some(name) = package_of(rel) {
            let found = packages.iter().find(|p| p.name == name);
            (OriginKind::Package, name, found.map(|p| p.version.clone()), found.and_then(|p| p.note.clone()))
        } else if let Some(c) = components.iter().find(|c| rel.starts_with(&format!("{}/", c.dir))) {
            (OriginKind::Extension, c.name.clone(), c.version.clone(), None)
        } else if in_core(rel, framework) {
            (OriginKind::Core, framework_name(framework), None, core_advice.clone())
        } else {
            (OriginKind::Project, String::new(), None, None)
        };
        let index = match groups.iter().position(|g| g.kind == kind && g.name == name) {
            Some(index) => index,
            None => {
                groups.push(Origin { kind, name, version, messages: 0, fatal: 0, advice });
                groups.len() - 1
            }
        };
        groups[index].messages += 1;
        groups[index].fatal += usize::from(event.level == Level::Fatal);
    }
    groups.sort_by_key(|g| (std::cmp::Reverse(g.fatal), std::cmp::Reverse(g.messages)));
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(path: &str, level: Level) -> Event {
        Event { path: format!("/app/{path}"), line: 1, level, message: "m".into(), count: 1 }
    }

    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (rel, body) in files {
            let path = dir.path().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, body).unwrap();
        }
        dir
    }

    #[test]
    fn wordpress_core_plugins_and_own_code_are_told_apart() {
        let dir = project(&[
            ("wp-includes/version.php", "<?php\n$wp_version = '4.9.8';\n"),
            ("wp-content/plugins/shop/shop.php", "<?php\n/*\nPlugin Name: Shop\nVersion: 1.2\n*/\n"),
        ]);
        let events = [
            event("wp-includes/load.php", Level::Fatal),
            event("wp-content/plugins/shop/cart.php", Level::Warning),
            event("wp-content/mu-plugins/mine.php", Level::Warning),
        ];
        let groups = group(dir.path(), &events, PhpVersion::new(8, 4));
        assert_eq!(groups[0].kind, OriginKind::Core);
        assert_eq!((groups[0].name.as_str(), groups[0].fatal), ("WordPress", 1));
        assert!(groups[0].advice.as_ref().is_some_and(|a| a.en.contains("update WordPress")), "{groups:?}");
        let plugin = groups.iter().find(|g| g.kind == OriginKind::Extension).unwrap();
        assert_eq!((plugin.name.as_str(), plugin.version.as_deref()), ("Shop", Some("1.2")));
        assert!(groups.iter().any(|g| g.kind == OriginKind::Project));
    }

    #[test]
    fn composer_packages_carry_their_locked_version() {
        let lock = r#"{"packages":[{"name":"acme/mail","version":"1.4.0"}]}"#;
        let dir = project(&[("composer.lock", lock)]);
        let events = [event("vendor/acme/mail/src/A.php", Level::Warning), event("vendor/acme/mail/src/B.php", Level::Warning)];
        let groups = group(dir.path(), &events, PhpVersion::new(8, 4));
        assert_eq!(groups.len(), 1);
        assert_eq!(
            (groups[0].kind, groups[0].name.as_str(), groups[0].version.as_deref(), groups[0].messages),
            (OriginKind::Package, "acme/mail", Some("1.4.0"), 2)
        );
    }

    #[test]
    fn generated_folders_are_not_framework_core() {
        assert!(!in_core("storage/framework/views/a.php", Framework::Laravel));
        assert!(in_core("core/lib/Drupal.php", Framework::Drupal));
        assert!(!in_core("modules/custom/a.php", Framework::Drupal));
        assert!(in_core("web/wp/wp-includes/load.php", Framework::WordPress));
    }

    #[test]
    fn a_vendor_folder_must_be_a_whole_path_segment() {
        assert_eq!(package_of("lib/vendor/a/b/c.php").as_deref(), Some("a/b"));
        assert_eq!(package_of("myvendor/a/b/c.php"), None);
        assert_eq!(package_of("vendor/a.php"), None);
    }
}
