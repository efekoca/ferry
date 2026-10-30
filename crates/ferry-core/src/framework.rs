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

pub fn check(root: &Path, framework: Framework, target: PhpVersion) -> Vec<Finding> {
    match framework {
        Framework::Laravel => laravel(root, target).into_iter().collect(),
        Framework::WordPress => wordpress(root, target).into_iter().collect(),
        _ => Vec::new(),
    }
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
fn finding(file: &str, line: usize, snippet: String, rule: &str, title: String, since: PhpVersion, message: Localized, replacement: Localized) -> Finding {
    Finding {
        id: format!("{file}:{line}:1:{rule}"),
        file: file.to_string(),
        line,
        column: 1,
        rule_id: rule.to_string(),
        title: Localized::same(title),
        message,
        since,
        severity: Severity::Breaking,
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
