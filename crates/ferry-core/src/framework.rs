use crate::analyzer::Finding;
use crate::i18n::Localized;
use crate::project::Framework;
use crate::rules::{FixKind, Severity};
use crate::PhpVersion;
use std::path::Path;

const LARAVEL_MAX_PHP: [(u32, (u32, u32)); 9] = [
    (5, (7, 4)),
    (6, (8, 0)),
    (7, (8, 0)),
    (8, (8, 1)),
    (9, (8, 2)),
    (10, (8, 3)),
    (11, (8, 4)),
    (12, (8, 5)),
    (13, (8, 5)),
];

const WORDPRESS_MIN: [((u32, u32), (u32, u32)); 7] = [
    ((7, 4), (5, 3)),
    ((8, 0), (5, 6)),
    ((8, 1), (5, 9)),
    ((8, 2), (6, 1)),
    ((8, 3), (6, 4)),
    ((8, 4), (6, 7)),
    ((8, 5), (6, 9)),
];

const DRUPAL_MAX_PHP: [((u32, u32), (u32, u32)); 9] = [
    ((8, 0), (7, 4)),
    ((9, 0), (8, 1)),
    ((10, 0), (8, 2)),
    ((10, 2), (8, 3)),
    ((10, 4), (8, 4)),
    ((11, 0), (8, 3)),
    ((11, 1), (8, 4)),
    ((11, 3), (8, 5)),
    ((12, 0), (8, 5)),
];

type Version = (u32, u32);

const SYMFONY_MAINTAINED_UNTIL: [(Version, Option<Version>); 18] = [
    ((2, 0), Some((7, 3))),
    ((3, 0), Some((7, 2))),
    ((3, 4), Some((8, 0))),
    ((4, 0), Some((7, 4))),
    ((4, 4), Some((8, 2))),
    ((5, 0), Some((8, 1))),
    ((5, 4), Some((8, 4))),
    ((6, 0), Some((8, 2))),
    ((6, 3), Some((8, 3))),
    ((6, 4), None),
    ((7, 0), Some((8, 3))),
    ((7, 1), Some((8, 4))),
    ((7, 2), Some((8, 4))),
    ((7, 3), Some((8, 5))),
    ((7, 4), None),
    ((8, 0), None),
    ((8, 1), None),
    ((9, 0), None),
];

pub(crate) struct PackageLimit {
    pub max: PhpVersion,
    pub official: bool,
}

pub(crate) fn package_limit(name: &str, version: &str) -> Option<PackageLimit> {
    let current = version_parts(version)?;
    let laravel = matches!(name, "laravel/framework" | "laravel/lumen-framework") || name.starts_with("illuminate/");
    let (max, official) = if laravel {
        (LARAVEL_MAX_PHP.iter().find(|(m, _)| *m == current.0.max(5))?.1, true)
    } else if matches!(name, "drupal/core" | "drupal/core-recommended") {
        (table_lookup(&DRUPAL_MAX_PHP, current)?, true)
    } else if matches!(name, "symfony/framework-bundle" | "symfony/http-kernel" | "symfony/symfony") {
        (table_lookup(&SYMFONY_MAINTAINED_UNTIL, current)??, false)
    } else {
        return None;
    };
    Some(PackageLimit {
        max: PhpVersion::new(max.0, max.1),
        official,
    })
}

pub fn check(root: &Path, framework: Framework, target: PhpVersion) -> Vec<Finding> {
    match framework {
        Framework::Laravel => laravel(root, target).into_iter().collect(),
        Framework::WordPress => wordpress(root, target).into_iter().collect(),
        Framework::Drupal => drupal(root, target).into_iter().collect(),
        Framework::Symfony => symfony(root, target).into_iter().collect(),
        Framework::Generic => Vec::new(),
    }
}

fn locked_version(root: &Path, names: &[&str]) -> Option<(String, String)> {
    let lock: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("composer.lock")).ok()?).ok()?;
    let packages = lock["packages"].as_array()?;
    names.iter().find_map(|name| {
        let p = packages.iter().find(|p| p["name"] == *name)?;
        Some((name.to_string(), p["version"].as_str()?.to_string()))
    })
}

fn package_line(root: &Path, package: &str, fallback: &str) -> (String, usize, String) {
    std::fs::read_to_string(root.join("composer.json"))
        .ok()
        .and_then(|t| line_of(&t, &format!("\"{package}\"")))
        .map(|(l, s)| ("composer.json".to_string(), l, s))
        .unwrap_or_else(|| ("composer.lock".to_string(), 1, fallback.to_string()))
}

fn table_lookup<T: Copy>(table: &[((u32, u32), T)], version: (u32, u32)) -> Option<T> {
    table.iter().rev().find(|(v, _)| *v <= version).map(|(_, t)| *t)
}

fn drupal(root: &Path, target: PhpVersion) -> Option<Finding> {
    let (package, version) = locked_version(root, &["drupal/core", "drupal/core-recommended", "drupal/drupal"]).or_else(|| {
        ["", "web/", "docroot/", "html/", "public/"].iter().find_map(|base| {
            let text = std::fs::read_to_string(root.join(format!("{base}core/lib/Drupal.php"))).ok()?;
            let (_, line) = line_of(&text, "const VERSION")?;
            Some(("drupal/core".to_string(), line.split(['\'', '"']).nth(1)?.to_string()))
        })
    })?;
    let current = version_parts(&version)?;
    let max = table_lookup(&DRUPAL_MAX_PHP, current)?;
    let since = next_minor(max);
    if target < since {
        return None;
    }
    let needed = if target >= PhpVersion::new(8, 5) { "11.3" } else { "10.4" };
    let (file, line, snippet) = package_line(root, &package, &version);
    let (cur, max_s, t) = (format!("{}.{}", current.0, current.1), format!("{}.{}", max.0, max.1), target.to_string());
    Some(finding(
        &file,
        line,
        snippet,
        "framework.drupal_version",
        format!("Drupal {cur}"),
        since,
        Severity::Breaking,
        Localized {
            tr: format!("Drupal {cur} en fazla PHP {max_s}'i destekliyor. PHP {t} için Drupal'ı en az {needed} sürümüne yükseltin."),
            en: format!("Drupal {cur} supports PHP up to {max_s}. PHP {t} needs at least Drupal {needed}."),
        },
        Localized::same(format!("composer require drupal/core-recommended:^{needed} --with-all-dependencies")),
    ))
}

fn symfony(root: &Path, target: PhpVersion) -> Option<Finding> {
    let (package, version) = locked_version(root, &["symfony/framework-bundle", "symfony/http-kernel"])?;
    let current = version_parts(&version)?;
    let max = table_lookup(&SYMFONY_MAINTAINED_UNTIL, current)??;
    let since = next_minor(max);
    if target < since {
        return None;
    }
    let (file, line, snippet) = package_line(root, &package, &version);
    let (cur, t) = (format!("{}.{}", current.0, current.1), target.to_string());
    let after = since.to_string();
    Some(finding(
        &file,
        line,
        snippet,
        "framework.symfony_version",
        format!("Symfony {cur}"),
        since,
        Severity::Behavior,
        Localized {
            tr: format!("Symfony {cur}'in bakımı PHP {after} çıkmadan bitti; bu sürüm PHP {t} ile hiç test edilmedi. Symfony 6.4 ya da 7.4 LTS'ye geçmeniz önerilir."),
            en: format!("Symfony {cur} stopped receiving fixes before PHP {after} came out, so it was never tested on PHP {t}. Moving to Symfony 6.4 or 7.4 LTS is recommended."),
        },
        Localized::same("composer require symfony/framework-bundle:^6.4"),
    ))
}

fn version_parts(v: &str) -> Option<(u32, u32)> {
    let mut parts = v.trim_start_matches(['v', 'V']).split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty());
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    Some((major, minor))
}

fn next_minor((major, minor): (u32, u32)) -> PhpVersion {
    if major == 7 && minor == 4 {
        PhpVersion::new(8, 0)
    } else {
        PhpVersion::new(major, minor + 1)
    }
}

fn line_of(text: &str, needle: &str) -> Option<(usize, String)> {
    text.lines().enumerate().find(|(_, l)| l.contains(needle)).map(|(i, l)| (i + 1, l.trim().to_string()))
}

#[allow(clippy::too_many_arguments)]
fn finding(
    file: &str,
    line: usize,
    snippet: String,
    rule: &str,
    title: String,
    since: PhpVersion,
    severity: Severity,
    message: Localized,
    replacement: Localized,
) -> Finding {
    Finding {
        id: format!("{file}:{line}:1:{rule}"),
        file: file.to_string(),
        line,
        column: 1,
        rule_id: rule.to_string(),
        title: Localized::same(title),
        message,
        since,
        severity,
        fix: FixKind::Manual,
        replacement: Some(replacement),
        snippet,
    }
}

fn laravel(root: &Path, target: PhpVersion) -> Option<Finding> {
    let lock: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("composer.lock")).ok()?).ok()?;
    let version = lock["packages"]
        .as_array()?
        .iter()
        .find(|p| p["name"] == "laravel/framework")?["version"]
        .as_str()?
        .to_string();
    let (major, _) = version_parts(&version)?;
    let &(_, max) = LARAVEL_MAX_PHP.iter().find(|(m, _)| *m == major.max(5))?;
    let since = next_minor(max);
    if target < since {
        return None;
    }
    let needed = LARAVEL_MAX_PHP
        .iter()
        .find(|(_, m)| PhpVersion::new(m.0, m.1) >= target)
        .map(|(m, _)| *m)?;
    let (file, (line, snippet)) = std::fs::read_to_string(root.join("composer.json"))
        .ok()
        .and_then(|t| line_of(&t, "\"laravel/framework\""))
        .map(|l| ("composer.json", l))
        .unwrap_or(("composer.lock", (1, version.clone())));
    let (max_s, t) = (format!("{}.{}", max.0, max.1), target.to_string());
    Some(finding(
        file,
        line,
        snippet,
        "framework.laravel_version",
        format!("Laravel {major}"),
        since,
        Severity::Breaking,
        Localized {
            tr: format!("Laravel {major} resmi olarak en fazla PHP {max_s}'i destekliyor. PHP {t} için Laravel'i en az {needed}. sürüme yükseltmeniz gerekiyor."),
            en: format!("Laravel {major} officially supports PHP up to {max_s}. PHP {t} needs at least Laravel {needed}."),
        },
        Localized::same(format!("composer require laravel/framework:^{needed}.0 --with-all-dependencies")),
    ))
}

fn wordpress(root: &Path, target: PhpVersion) -> Option<Finding> {
    let (rel, text) = ["", "wp", "web/wp", "public/wp", "wordpress"].iter().find_map(|base| {
        let rel = if base.is_empty() { "wp-includes/version.php".to_string() } else { format!("{base}/wp-includes/version.php") };
        std::fs::read_to_string(root.join(&rel)).ok().map(|t| (rel, t))
    })?;
    let (line, snippet) = line_of(&text, "$wp_version")?;
    let quoted = snippet.split(['\'', '"']).nth(1)?;
    let current = version_parts(quoted)?;
    let &(breaks_at, _) = WORDPRESS_MIN.iter().find(|(_, min)| current < *min)?;
    let since = PhpVersion::new(breaks_at.0, breaks_at.1);
    if target < since {
        return None;
    }
    let &(_, needed) = WORDPRESS_MIN.iter().rev().find(|(php, _)| PhpVersion::new(php.0, php.1) <= target)?;
    let (cur, need, t) = (format!("{}.{}", current.0, current.1), format!("{}.{}", needed.0, needed.1), target.to_string());
    Some(finding(
        &rel,
        line,
        snippet.clone(),
        "framework.wordpress_version",
        format!("WordPress {cur}"),
        since,
        Severity::Breaking,
        Localized {
            tr: format!("WordPress {cur}, PHP {t} ile çalışmaz. PHP {t} için en az WordPress {need} gerekiyor; önce WordPress'i güncelleyin."),
            en: format!("WordPress {cur} doesn't run on PHP {t}. PHP {t} needs at least WordPress {need}; update WordPress first."),
        },
        Localized::same(format!("WordPress {need}+")),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn laravel_version_against_target() {
        let dir = project(&[
            ("composer.json", "{\n  \"require\": {\n    \"laravel/framework\": \"^10.48\"\n  }\n}\n"),
            ("composer.lock", r#"{"packages":[{"name":"laravel/framework","version":"v10.48.22"}]}"#),
        ]);
        assert!(check(dir.path(), Framework::Laravel, PhpVersion::new(8, 3)).is_empty());
        let f = &check(dir.path(), Framework::Laravel, PhpVersion::new(8, 5))[0];
        assert_eq!((f.file.as_str(), f.line, f.since), ("composer.json", 3, PhpVersion::new(8, 4)));
        assert!(f.message.en.contains("at least Laravel 12"), "{}", f.message.en);
    }

    #[test]
    fn drupal_version_from_lock_or_core() {
        let dir = project(&[
            ("composer.json", "{\n  \"require\": {\n    \"drupal/core-recommended\": \"^10.3\"\n  }\n}\n"),
            ("composer.lock", r#"{"packages":[{"name":"drupal/core-recommended","version":"10.3.6"}]}"#),
        ]);
        assert!(check(dir.path(), Framework::Drupal, PhpVersion::new(8, 3)).is_empty());
        let f = &check(dir.path(), Framework::Drupal, PhpVersion::new(8, 4))[0];
        assert_eq!((f.file.as_str(), f.line, f.since), ("composer.json", 3, PhpVersion::new(8, 4)));
        assert!(f.message.en.contains("at least Drupal 10.4"), "{}", f.message.en);
        let old = project(&[("web/core/lib/Drupal.php", "<?php\nclass Drupal {\n  const VERSION = '9.5.11';\n}\n")]);
        assert_eq!(check(old.path(), Framework::Drupal, PhpVersion::new(8, 2))[0].since, PhpVersion::new(8, 2));
        let new = project(&[("composer.lock", r#"{"packages":[{"name":"drupal/core","version":"11.3.1"}]}"#)]);
        assert!(check(new.path(), Framework::Drupal, PhpVersion::new(8, 5)).is_empty());
    }

    #[test]
    fn symfony_versions_outside_maintenance_are_risky() {
        let dir = project(&[("composer.lock", r#"{"packages":[{"name":"symfony/framework-bundle","version":"v4.4.51"}]}"#)]);
        assert!(check(dir.path(), Framework::Symfony, PhpVersion::new(8, 2)).is_empty());
        let f = &check(dir.path(), Framework::Symfony, PhpVersion::new(8, 3))[0];
        assert_eq!((f.severity, f.since), (Severity::Behavior, PhpVersion::new(8, 3)));
        let lts = project(&[("composer.lock", r#"{"packages":[{"name":"symfony/framework-bundle","version":"v6.4.13"}]}"#)]);
        assert!(check(lts.path(), Framework::Symfony, PhpVersion::new(8, 5)).is_empty());
    }

    #[test]
    fn wordpress_core_version_against_target() {
        let dir = project(&[("wp-includes/version.php", "<?php\n$wp_version = '6.2.2';\n")]);
        assert!(check(dir.path(), Framework::WordPress, PhpVersion::new(8, 2)).is_empty());
        let f = &check(dir.path(), Framework::WordPress, PhpVersion::new(8, 4))[0];
        assert_eq!((f.file.as_str(), f.line, f.since), ("wp-includes/version.php", 2, PhpVersion::new(8, 3)));
        assert!(f.message.en.contains("at least WordPress 6.7"), "{}", f.message.en);
        let old = project(&[("web/wp/wp-includes/version.php", "<?php\n$wp_version = \"4.9.8\";\n")]);
        assert_eq!(check(old.path(), Framework::WordPress, PhpVersion::new(8, 1))[0].since, PhpVersion::new(7, 4));
    }
}
