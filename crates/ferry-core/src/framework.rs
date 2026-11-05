use crate::analyzer::Finding;
use crate::i18n::Localized;
use crate::project::Framework;
use crate::rules::{FixKind, Severity};
use crate::PhpVersion;
use std::path::Path;

const LARAVEL_MAX_PHP: [(u32, (u32, u32)); 9] =
    [(5, (7, 4)), (6, (8, 0)), (7, (8, 0)), (8, (8, 1)), (9, (8, 2)), (10, (8, 3)), (11, (8, 4)), (12, (8, 5)), (13, (8, 5))];

const WORDPRESS_MIN: [((u32, u32), (u32, u32)); 7] =
    [((7, 4), (5, 3)), ((8, 0), (5, 6)), ((8, 1), (5, 9)), ((8, 2), (6, 1)), ((8, 3), (6, 4)), ((8, 4), (6, 7)), ((8, 5), (6, 9))];

const DRUPAL_MAX_PHP: [((u32, u32), (u32, u32)); 13] = [
    ((7, 0), (8, 0)),
    ((7, 92), (8, 1)),
    ((7, 94), (8, 2)),
    ((7, 103), (8, 3)),
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
    Some(PackageLimit { max: PhpVersion::new(max.0, max.1), official })
}

pub fn check(root: &Path, framework: Framework, target: PhpVersion) -> Vec<Finding> {
    match framework {
        Framework::Laravel => laravel(root, target).into_iter().collect(),
        Framework::WordPress => wordpress(root, target).into_iter().collect(),
        Framework::Drupal => drupal(root, target).into_iter().collect(),
        Framework::Symfony => symfony(root, target).into_iter().collect(),
        Framework::Magento => magento(root, target).into_iter().collect(),
        Framework::PrestaShop => prestashop(root, target).into_iter().collect(),
        Framework::Joomla => joomla(root, target).into_iter().collect(),
        Framework::Moodle => moodle(root, target).into_iter().collect(),
        Framework::MediaWiki => mediawiki(root, target).into_iter().collect(),
        Framework::Phpbb => phpbb(root, target).into_iter().collect(),
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
    let d7 = std::fs::read_to_string(root.join("includes/bootstrap.inc")).ok().and_then(|text| {
        let (line, snippet) = line_of(&text, "define('VERSION'")?;
        let version = snippet.split(['\'', '"']).nth(3)?.to_string();
        Some((line, snippet, version))
    });
    if let Some((line, snippet, version)) = d7 {
        return drupal7(target, line, snippet, &version);
    }
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
    let max_s_acc = crate::i18n::tr_accusative(&max_s);
    Some(finding(
        &file,
        line,
        snippet,
        "framework.drupal_version",
        format!("Drupal {cur}"),
        since,
        Severity::Breaking,
        Localized {
            tr: format!("Drupal {cur} en fazla PHP {max_s_acc} destekliyor. PHP {t} için Drupal'ı en az {needed} sürümüne yükseltin."),
            en: format!("Drupal {cur} supports PHP up to {max_s}. PHP {t} needs at least Drupal {needed}."),
        },
        Localized::same(format!("composer require drupal/core-recommended:^{needed} --with-all-dependencies")),
    ))
}

fn drupal7(target: PhpVersion, line: usize, snippet: String, version: &str) -> Option<Finding> {
    let current = version_parts(version)?;
    let max = table_lookup(&DRUPAL_MAX_PHP, current)?;
    let since = next_minor(max);
    if target < since {
        return None;
    }
    let cur = format!("{}.{}", current.0, current.1);
    let t = target.to_string();
    let within = DRUPAL_MAX_PHP.iter().find(|(v, m)| v.0 == 7 && PhpVersion::new(m.0, m.1) >= target).map(|(v, _)| format!("7.{}", v.1));
    let t_acc = crate::i18n::tr_accusative(&target.to_string());
    let (tr, en, fix) = match within {
        Some(need) => (
            format!("Drupal {cur}, PHP {t} ile çalışmaz. PHP {t} için Drupal 7'yi en az {need} sürümüne güncelleyin."),
            format!("Drupal {cur} doesn't run on PHP {t}. PHP {t} needs at least Drupal {need}."),
            format!("Drupal {need}+"),
        ),
        None => (
            format!("Hiçbir Drupal 7 sürümü PHP {t_acc} desteklemiyor; Drupal 7'nin desteği 5 Ocak 2025'te bitti. PHP {t} için siteyi Drupal 10 ya da 11'e taşımanız gerekiyor."),
            format!("No Drupal 7 release supports PHP {t}, and Drupal 7 reached end of life on 5 January 2025. PHP {t} needs the site moved to Drupal 10 or 11."),
            "Drupal 10.4+ / 11.3+".to_string(),
        ),
    };
    Some(finding(
        "includes/bootstrap.inc",
        line,
        snippet,
        "framework.drupal_version",
        format!("Drupal {cur}"),
        since,
        Severity::Breaking,
        Localized { tr, en },
        Localized::same(fix),
    ))
}

type Release = (u32, u32, u32);

const MAGENTO2_MAX_PHP: [(Release, (u32, u32)); 6] =
    [((2, 0, 0), (7, 4)), ((2, 4, 4), (8, 1)), ((2, 4, 6), (8, 2)), ((2, 4, 7), (8, 3)), ((2, 4, 8), (8, 4)), ((2, 4, 9), (8, 5))];

const PRESTASHOP_MAX_PHP: [(Release, (u32, u32)); 3] = [((1, 6, 0), (7, 4)), ((8, 0, 0), (8, 1)), ((9, 0, 0), (8, 5))];

const JOOMLA_MAX_PHP: [(Release, (u32, u32)); 8] = [
    ((3, 0, 0), (8, 0)),
    ((4, 0, 0), (8, 0)),
    ((4, 0, 5), (8, 1)),
    ((4, 3, 0), (8, 2)),
    ((5, 0, 0), (8, 3)),
    ((5, 3, 0), (8, 4)),
    ((5, 4, 0), (8, 5)),
    ((6, 0, 0), (8, 4)),
];

const MOODLE_MAX_PHP: [(Release, (u32, u32)); 19] = [
    ((3, 0, 0), (7, 0)),
    ((3, 2, 0), (7, 1)),
    ((3, 4, 0), (7, 2)),
    ((3, 6, 0), (7, 2)),
    ((3, 6, 4), (7, 3)),
    ((3, 8, 0), (7, 3)),
    ((3, 8, 3), (7, 4)),
    ((3, 11, 0), (7, 4)),
    ((3, 11, 8), (8, 0)),
    ((4, 0, 0), (7, 4)),
    ((4, 0, 2), (8, 0)),
    ((4, 1, 0), (8, 0)),
    ((4, 1, 2), (8, 1)),
    ((4, 2, 0), (8, 1)),
    ((4, 2, 3), (8, 2)),
    ((4, 3, 0), (8, 2)),
    ((4, 4, 0), (8, 3)),
    ((5, 0, 0), (8, 4)),
    ((5, 4, 0), (8, 5)),
];

const MEDIAWIKI_MAX_PHP: [(Release, (u32, u32)); 5] =
    [((1, 31, 0), (7, 3)), ((1, 35, 0), (7, 4)), ((1, 39, 0), (8, 1)), ((1, 42, 0), (8, 3)), ((1, 46, 0), (8, 5))];

const PHPBB_MAX_PHP: [(Release, (u32, u32)); 4] = [((3, 2, 0), (7, 2)), ((3, 3, 0), (7, 4)), ((3, 3, 6), (8, 1)), ((3, 3, 12), (8, 3))];

struct Located {
    file: String,
    line: usize,
    snippet: String,
    version: String,
}

fn locate(root: &Path, sources: &[(&str, &str)]) -> Option<Located> {
    sources.iter().find_map(|(rel, needle)| {
        let text = std::fs::read_to_string(root.join(rel)).ok()?;
        let (line, snippet) = line_of(&text, needle)?;
        let raw = snippet.split(['\'', '"']).find(|p| p.starts_with(|c: char| c.is_ascii_digit()))?;
        let version: String = raw.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
        Some(Located { file: rel.to_string(), line, snippet, version: version.trim_end_matches('.').to_string() })
    })
}

fn release_check(
    found: Located,
    table: &[(Release, (u32, u32))],
    product: &str,
    rule: &str,
    severity: Severity,
    target: PhpVersion,
) -> Option<Finding> {
    let current = release_parts(&found.version)?;
    let max = table.iter().rev().find(|(v, _)| *v <= current)?.1;
    let since = next_minor(max);
    if target < since {
        return None;
    }
    let needed =
        table.iter().find(|(v, m)| *v > current && PhpVersion::new(m.0, m.1) >= target).map(|(v, _)| format!("{}.{}.{}", v.0, v.1, v.2));
    let (cur, t, t_acc) = (found.version.clone(), target.to_string(), crate::i18n::tr_accusative(&target.to_string()));
    let (tr, en, replacement) = match &needed {
        Some(n) => (
            format!("{product} {cur} resmi olarak PHP {t_acc} desteklemiyor. PHP {t} için {product}'u en az {n} sürümüne yükseltin."),
            format!("{product} {cur} doesn't officially support PHP {t}. PHP {t} needs at least {product} {n}."),
            format!("{product} {n}+"),
        ),
        None => (
            format!("{product} {cur} resmi olarak PHP {t_acc} desteklemiyor ve bunu destekleyen bir sürümü henüz yayımlanmadı."),
            format!("{product} {cur} doesn't officially support PHP {t}, and no release that does has been published yet."),
            format!("PHP {}", max.0.to_string() + "." + &max.1.to_string()),
        ),
    };
    Some(finding(
        &found.file,
        found.line,
        found.snippet,
        rule,
        format!("{product} {cur}"),
        since,
        severity,
        Localized { tr, en },
        Localized::same(replacement),
    ))
}

fn joomla(root: &Path, target: PhpVersion) -> Option<Finding> {
    let found = std::fs::read_to_string(root.join("libraries/src/Version.php"))
        .ok()
        .and_then(|text| {
            let num = |name: &str| {
                line_of(&text, name)
                    .and_then(|(l, s)| Some((l, s.clone(), s.split(['=', ';']).nth(1)?.trim().trim_matches(['\'', '"']).to_string())))
            };
            let (line, snippet, major) = num("const MAJOR_VERSION")?;
            let (_, _, minor) = num("const MINOR_VERSION")?;
            let (_, _, patch) = num("const PATCH_VERSION")?;
            Some(Located { file: "libraries/src/Version.php".into(), line, snippet, version: format!("{major}.{minor}.{patch}") })
        })
        .or_else(|| {
            let rel = "libraries/cms/version/version.php";
            let text = std::fs::read_to_string(root.join(rel)).ok()?;
            let value = |name: &str| line_of(&text, name).and_then(|(l, s)| Some((l, s.clone(), s.split(['\'', '"']).nth(1)?.to_string())));
            let (line, snippet, release) = value("RELEASE")?;
            let level = value("DEV_LEVEL").map(|v| v.2).unwrap_or_else(|| "0".into());
            Some(Located { file: rel.into(), line, snippet, version: format!("{release}.{level}") })
        })?;
    let major = release_parts(&found.version)?.0;
    let severity = if major <= 4 { Severity::Breaking } else { Severity::Behavior };
    release_check(found, &JOOMLA_MAX_PHP, "Joomla", "framework.joomla_version", severity, target)
}

fn moodle(root: &Path, target: PhpVersion) -> Option<Finding> {
    let found = locate(root, &[("version.php", "$release"), ("public/version.php", "$release")])?;
    release_check(found, &MOODLE_MAX_PHP, "Moodle", "framework.moodle_version", Severity::Breaking, target)
}

fn mediawiki(root: &Path, target: PhpVersion) -> Option<Finding> {
    let found = locate(root, &[("includes/Defines.php", "'MW_VERSION'"), ("includes/DefaultSettings.php", "$wgVersion")])?;
    release_check(found, &MEDIAWIKI_MAX_PHP, "MediaWiki", "framework.mediawiki_version", Severity::Behavior, target)
}

fn phpbb(root: &Path, target: PhpVersion) -> Option<Finding> {
    let found = locate(root, &[("includes/constants.php", "'PHPBB_VERSION'"), ("phpBB/includes/constants.php", "'PHPBB_VERSION'")])?;
    release_check(found, &PHPBB_MAX_PHP, "phpBB", "framework.phpbb_version", Severity::Breaking, target)
}

fn release_parts(v: &str) -> Option<Release> {
    let mut parts = v.trim_start_matches(['v', 'V']).split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty());
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    Some((major, minor, patch))
}

fn release_label((major, minor, patch): Release) -> String {
    format!("{major}.{minor}.{patch}")
}

fn magento(root: &Path, target: PhpVersion) -> Option<Finding> {
    if let Ok(text) = std::fs::read_to_string(root.join("app/Mage.php")) {
        if text.contains("getOpenMageVersionInfo") || locked_version(root, &["openmage/magento-lts"]).is_some() {
            return None;
        }
        let since = PhpVersion::new(8, 0);
        if target < since {
            return None;
        }
        let (line, snippet) = line_of(&text, "function getVersionInfo").unwrap_or((1, "Mage".to_string()));
        let t = target.to_string();
        let t_acc = crate::i18n::tr_accusative(&target.to_string());
        return Some(finding(
            "app/Mage.php",
            line,
            snippet,
            "framework.magento_version",
            "Magento 1".to_string(),
            since,
            Severity::Breaking,
            Localized {
                tr: format!("Magento 1'in desteği Haziran 2020'de, PHP 8 çıkmadan bitti; hiçbir Magento 1 sürümü PHP {t_acc} resmi olarak desteklemiyor. Bakımı süren OpenMage LTS'ye geçmeniz ya da Magento 2'ye taşınmanız gerekiyor."),
                en: format!("Magento 1 reached end of life in June 2020, before PHP 8 came out, and no Magento 1 release officially supports PHP {t}. Move to the maintained OpenMage LTS fork or migrate to Magento 2."),
            },
            Localized::same("composer require openmage/magento-lts"),
        ));
    }
    let (package, version) =
        locked_version(root, &["magento/product-community-edition", "magento/product-enterprise-edition", "magento/magento2-base"])?;
    let current = release_parts(&version)?;
    let max = MAGENTO2_MAX_PHP.iter().rev().find(|(v, _)| *v <= current)?.1;
    let since = next_minor(max);
    if target < since {
        return None;
    }
    let needed = MAGENTO2_MAX_PHP.iter().find(|(_, m)| PhpVersion::new(m.0, m.1) >= target).map(|(v, _)| release_label(*v))?;
    let (file, line, snippet) = package_line(root, &package, &version);
    let (cur, t) = (release_label(current), target.to_string());
    let t_acc = crate::i18n::tr_accusative(&target.to_string());
    Some(finding(
        &file,
        line,
        snippet,
        "framework.magento_version",
        format!("Magento {cur}"),
        since,
        Severity::Breaking,
        Localized {
            tr: format!("Magento {cur} resmi olarak PHP {t_acc} desteklemiyor. PHP {t} için Magento'yu en az {needed} sürümüne yükseltin."),
            en: format!("Magento {cur} doesn't officially support PHP {t}. PHP {t} needs at least Magento {needed}."),
        },
        Localized::same(format!("composer require {package}:{needed} --no-update && composer update")),
    ))
}

fn prestashop(root: &Path, target: PhpVersion) -> Option<Finding> {
    let (rel, line, snippet, version) = [
        ("app/AppKernel.php", "const VERSION"),
        ("config/settings.inc.php", "_PS_VERSION_"),
        ("install/install_version.php", "_PS_INSTALL_VERSION_"),
        ("install-dev/install_version.php", "_PS_INSTALL_VERSION_"),
    ]
    .iter()
    .find_map(|(rel, needle)| {
        let text = std::fs::read_to_string(root.join(rel)).ok()?;
        let (line, snippet) = line_of(&text, needle)?;
        let version = snippet.split(['\'', '"']).find(|p| p.starts_with(|c: char| c.is_ascii_digit()))?.to_string();
        Some((rel.to_string(), line, snippet.clone(), version))
    })?;
    let current = release_parts(&version)?;
    let max = PRESTASHOP_MAX_PHP.iter().rev().find(|(v, _)| *v <= current)?.1;
    let since = next_minor(max);
    if target < since {
        return None;
    }
    let needed = PRESTASHOP_MAX_PHP.iter().find(|(_, m)| PhpVersion::new(m.0, m.1) >= target).map(|(v, _)| format!("{}.{}", v.0, v.1))?;
    let (cur, t) = (version.clone(), target.to_string());
    let t_acc = crate::i18n::tr_accusative(&target.to_string());
    Some(finding(
        &rel,
        line,
        snippet,
        "framework.prestashop_version",
        format!("PrestaShop {cur}"),
        since,
        Severity::Breaking,
        Localized {
            tr: format!(
                "PrestaShop {cur} resmi olarak PHP {t_acc} desteklemiyor. PHP {t} için PrestaShop'u en az {needed} sürümüne yükseltin."
            ),
            en: format!("PrestaShop {cur} doesn't officially support PHP {t}. PHP {t} needs at least PrestaShop {needed}."),
        },
        Localized::same(format!("PrestaShop {needed}+")),
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
        runtime_hits: 0,
    }
}

fn laravel(root: &Path, target: PhpVersion) -> Option<Finding> {
    let lock: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("composer.lock")).ok()?).ok()?;
    let version = lock["packages"].as_array()?.iter().find(|p| p["name"] == "laravel/framework")?["version"].as_str()?.to_string();
    let (major, _) = version_parts(&version)?;
    let &(_, max) = LARAVEL_MAX_PHP.iter().find(|(m, _)| *m == major.max(5))?;
    let since = next_minor(max);
    if target < since {
        return None;
    }
    let needed = LARAVEL_MAX_PHP.iter().find(|(_, m)| PhpVersion::new(m.0, m.1) >= target).map(|(m, _)| *m)?;
    let (file, (line, snippet)) = std::fs::read_to_string(root.join("composer.json"))
        .ok()
        .and_then(|t| line_of(&t, "\"laravel/framework\""))
        .map(|l| ("composer.json", l))
        .unwrap_or(("composer.lock", (1, version.clone())));
    let (max_s, t) = (format!("{}.{}", max.0, max.1), target.to_string());
    let max_s_acc = crate::i18n::tr_accusative(&max_s);
    Some(finding(
        file,
        line,
        snippet,
        "framework.laravel_version",
        format!("Laravel {major}"),
        since,
        Severity::Breaking,
        Localized {
            tr: format!("Laravel {major} resmi olarak en fazla PHP {max_s_acc} destekliyor. PHP {t} için Laravel'i en az {needed}. sürüme yükseltmeniz gerekiyor."),
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
    let (line, snippet) = line_of(&text, "$wp_version =")?;
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
            tr: format!(
                "WordPress {cur}, PHP {t} ile çalışmaz. PHP {t} için en az WordPress {need} gerekiyor; önce WordPress'i güncelleyin."
            ),
            en: format!("WordPress {cur} doesn't run on PHP {t}. PHP {t} needs at least WordPress {need}; update WordPress first."),
        },
        Localized::same(format!("WordPress {need}+")),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project;

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
    fn drupal7_version_against_target() {
        let dir = project(&[("includes/bootstrap.inc", "<?php\n\ndefine('VERSION', '7.98');\n")]);
        assert_eq!(project::detect_framework(dir.path()), Framework::Drupal);
        assert!(check(dir.path(), Framework::Drupal, PhpVersion::new(8, 2)).is_empty());
        let f = &check(dir.path(), Framework::Drupal, PhpVersion::new(8, 3))[0];
        assert_eq!((f.file.as_str(), f.line, f.since), ("includes/bootstrap.inc", 3, PhpVersion::new(8, 3)));
        assert!(f.message.en.contains("at least Drupal 7.103"), "{}", f.message.en);
        let f = &check(dir.path(), Framework::Drupal, PhpVersion::new(8, 4))[0];
        assert!(f.message.en.contains("No Drupal 7 release supports PHP 8.4"), "{}", f.message.en);
        assert!(f.message.tr.contains("PHP 8.4'ü"), "{}", f.message.tr);
    }

    #[test]
    fn magento_versions_against_target() {
        let m1 = project(&[("app/Mage.php", "<?php\nfinal class Mage {\n    public static function getVersionInfo() {}\n}\n")]);
        assert_eq!(project::detect_framework(m1.path()), Framework::Magento);
        let f = &check(m1.path(), Framework::Magento, PhpVersion::new(8, 2))[0];
        assert_eq!((f.file.as_str(), f.line, f.since), ("app/Mage.php", 3, PhpVersion::new(8, 0)));
        let openmage =
            project(&[("app/Mage.php", "<?php\nfinal class Mage {\n    public static function getOpenMageVersionInfo() {}\n}\n")]);
        assert!(check(openmage.path(), Framework::Magento, PhpVersion::new(8, 4)).is_empty());
        let m2 = project(&[
            ("composer.json", "{\n  \"require\": {\n    \"magento/product-community-edition\": \"2.4.7-p3\"\n  }\n}\n"),
            ("composer.lock", r#"{"packages":[{"name":"magento/product-community-edition","version":"2.4.7-p3"}]}"#),
        ]);
        assert_eq!(project::detect_framework(m2.path()), Framework::Magento);
        assert!(check(m2.path(), Framework::Magento, PhpVersion::new(8, 3)).is_empty());
        let f = &check(m2.path(), Framework::Magento, PhpVersion::new(8, 5))[0];
        assert_eq!((f.file.as_str(), f.line, f.since), ("composer.json", 3, PhpVersion::new(8, 4)));
        assert!(f.message.en.contains("at least Magento 2.4.9"), "{}", f.message.en);
    }

    #[test]
    fn prestashop_versions_against_target() {
        let old = project(&[("config/settings.inc.php", "<?php\ndefine('_PS_VERSION_', '1.6.1.24');\n")]);
        assert_eq!(project::detect_framework(old.path()), Framework::PrestaShop);
        let f = &check(old.path(), Framework::PrestaShop, PhpVersion::new(8, 2))[0];
        assert_eq!((f.file.as_str(), f.line, f.since), ("config/settings.inc.php", 2, PhpVersion::new(8, 0)));
        assert!(f.message.en.contains("at least PrestaShop 9.0"), "{}", f.message.en);
        let eight = project(&[("app/AppKernel.php", "<?php\nclass AppKernel {\n    const VERSION = '8.1.7';\n} // PrestaShop\n")]);
        assert_eq!(project::detect_framework(eight.path()), Framework::PrestaShop);
        assert_eq!(check(eight.path(), Framework::PrestaShop, PhpVersion::new(8, 2))[0].since, PhpVersion::new(8, 2));
        let nine = project(&[("app/AppKernel.php", "<?php\nclass AppKernel {\n    const VERSION = '9.0.1';\n} // PrestaShop\n")]);
        assert!(check(nine.path(), Framework::PrestaShop, PhpVersion::new(8, 5)).is_empty());
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
        let documented = project(&[("wp-includes/version.php", "<?php\n/**\n * @global string $wp_version\n */\n$wp_version = '4.9';\n")]);
        assert_eq!(check(documented.path(), Framework::WordPress, PhpVersion::new(8, 4))[0].line, 5);
        let old = project(&[("web/wp/wp-includes/version.php", "<?php\n$wp_version = \"4.9.8\";\n")]);
        assert_eq!(check(old.path(), Framework::WordPress, PhpVersion::new(8, 1))[0].since, PhpVersion::new(7, 4));
    }

    #[test]
    fn joomla_moodle_mediawiki_and_phpbb_versions() {
        let joomla4 = project(&[
            ("libraries/src/Version.php", "<?php\nnamespace Joomla\\CMS;\nfinal class Version {\n    public const MAJOR_VERSION = 4;\n    public const MINOR_VERSION = 0;\n    public const PATCH_VERSION = 4;\n}\n"),
            ("administrator/index.php", "<?php\n"),
        ]);
        assert_eq!(project::detect_framework(joomla4.path()), Framework::Joomla);
        let f = &check(joomla4.path(), Framework::Joomla, PhpVersion::new(8, 1))[0];
        assert_eq!((f.title.en.as_str(), f.severity, f.line), ("Joomla 4.0.4", Severity::Breaking, 4));
        assert!(f.message.en.contains("at least Joomla 4.0.5"), "{}", f.message.en);
        let joomla3 = project(&[(
            "libraries/cms/version/version.php",
            "<?php\nfinal class JVersion {\n    const RELEASE = '3.9';\n    const DEV_LEVEL = '28';\n}\n",
        )]);
        assert_eq!(check(joomla3.path(), Framework::Joomla, PhpVersion::new(8, 2))[0].title.en, "Joomla 3.9.28");

        let moodle = project(&[
            ("version.php", "<?php\n$version  = 2024100700.00;\n$release  = '4.5.1+ (Build: 20250103)';\n"),
            ("lib/moodlelib.php", "<?php\n"),
        ]);
        assert_eq!(project::detect_framework(moodle.path()), Framework::Moodle);
        assert!(check(moodle.path(), Framework::Moodle, PhpVersion::new(8, 3)).is_empty());
        let f = &check(moodle.path(), Framework::Moodle, PhpVersion::new(8, 4))[0];
        assert_eq!((f.title.en.as_str(), f.since), ("Moodle 4.5.1", PhpVersion::new(8, 4)));
        assert!(f.message.en.contains("at least Moodle 5.0.0"), "{}", f.message.en);

        let wiki = project(&[("includes/Defines.php", "<?php\ndefine( 'MW_VERSION', '1.43.3' );\n")]);
        assert_eq!(project::detect_framework(wiki.path()), Framework::MediaWiki);
        let f = &check(wiki.path(), Framework::MediaWiki, PhpVersion::new(8, 4))[0];
        assert_eq!((f.severity, f.since), (Severity::Behavior, PhpVersion::new(8, 4)));

        let bb = project(&[("phpBB/includes/constants.php", "<?php\n@define('PHPBB_VERSION', '3.3.5');\n")]);
        assert_eq!(project::detect_framework(bb.path()), Framework::Phpbb);
        let f = &check(bb.path(), Framework::Phpbb, PhpVersion::new(8, 0))[0];
        assert!(f.message.en.contains("at least phpBB 3.3.6"), "{}", f.message.en);
        let latest = project(&[("includes/constants.php", "<?php\n@define('PHPBB_VERSION', '3.3.19');\n")]);
        let f = &check(latest.path(), Framework::Phpbb, PhpVersion::new(8, 4))[0];
        assert!(f.message.en.contains("no release that does has been published yet"), "{}", f.message.en);
    }
}
