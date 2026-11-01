use crate::analyzer::{relative_path, Finding};
use crate::i18n::{Localized, Text};
use crate::rules::{FixKind, Severity, DEPRECATED_INI, REMOVED_INI};
use crate::version::PhpVersion;
use std::path::Path;
use walkdir::WalkDir;

const MAX_FILES: usize = 2000;

struct Hit {
    line: usize,
    rule: &'static str,
    title: Localized,
    since: PhpVersion,
    severity: Severity,
    message: Text,
    replacement: Option<Localized>,
}

pub fn scan(root: &Path, target: PhpVersion) -> Vec<Finding> {
    let keep = |e: &walkdir::DirEntry| {
        if e.depth() == 0 || !e.file_type().is_dir() {
            return true;
        }
        let name = e.file_name().to_string_lossy();
        (e.depth() == 1 && name == ".github") || (!name.starts_with('.') && !crate::project::ALWAYS_SKIP.contains(&name.as_ref()))
    };
    let mut out = Vec::new();
    for entry in WalkDir::new(root).max_depth(8).follow_links(false).into_iter().filter_entry(keep).flatten().take(MAX_FILES * 20) {
        if !entry.file_type().is_file() || std::fs::metadata(entry.path()).is_ok_and(|m| m.len() > crate::project::MAX_FILE_BYTES) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let rel = relative_path(root, entry.path());
        let in_workflows = rel.replace('\\', "/").starts_with(".github/workflows/");
        let kind = match name.as_str() {
            ".htaccess" => Kind::Htaccess,
            ".user.ini" | "php.ini" => Kind::Ini,
            "composer.json" if entry.depth() == 1 => Kind::Composer,
            ".tool-versions" | ".php-version" if entry.depth() == 1 => Kind::Pin,
            ".gitlab-ci.yml" | "bitbucket-pipelines.yml" | ".travis.yml" => Kind::Ci,
            n if n == "dockerfile" || n.starts_with("dockerfile.") || n.ends_with(".dockerfile") => Kind::Docker,
            n if (n.starts_with("docker-compose") || n.starts_with("compose")) && (n.ends_with(".yml") || n.ends_with(".yaml")) => {
                Kind::Docker
            }
            n if in_workflows && (n.ends_with(".yml") || n.ends_with(".yaml")) => Kind::Ci,
            _ => continue,
        };
        let Ok(text) = std::fs::read_to_string(entry.path()) else { continue };
        let hits = match kind {
            Kind::Htaccess => htaccess(&text),
            Kind::Ini => ini(&text),
            Kind::Composer => composer(&text, target),
            Kind::Pin | Kind::Docker | Kind::Ci => pins(&text, kind, target),
        };
        let lines: Vec<&str> = text.lines().collect();
        for h in hits.into_iter().filter(|h| h.since <= target) {
            out.push(Finding {
                id: format!("{rel}:{}:1:{}", h.line, h.rule),
                file: rel.clone(),
                line: h.line,
                column: 1,
                rule_id: h.rule.to_string(),
                title: h.title,
                message: h.message.into(),
                since: h.since,
                severity: h.severity,
                fix: FixKind::Manual,
                replacement: h.replacement,
                snippet: lines.get(h.line - 1).map_or(String::new(), |l| l.trim().to_string()),
            });
        }
        if out.len() > MAX_FILES {
            break;
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Htaccess,
    Ini,
    Composer,
    Pin,
    Docker,
    Ci,
}

fn truthy(value: &str) -> bool {
    let v = value.trim().trim_matches(|c| c == '"' || c == '\'').to_ascii_lowercase();
    !matches!(v.as_str(), "" | "0" | "off" | "false" | "no" | "none")
}

fn setting(name: &str, value: &str, line: usize) -> Option<Hit> {
    let name = name.to_ascii_lowercase();
    if let Some(&(_, a, b)) = REMOVED_INI.iter().find(|(k, _, _)| *k == name) {
        return truthy(value).then(|| Hit {
            line,
            rule: "removed.ini_setting",
            title: Localized::same(format!("{name} = {}", value.trim())),
            since: PhpVersion::new(a, b),
            severity: Severity::Breaking,
            message: REMOVED_SETTING,
            replacement: Some(Text::new("satırı silin", "delete the line").into()),
        });
    }
    let &(_, a, b) = DEPRECATED_INI.iter().find(|(k, _, _)| *k == name)?;
    Some(Hit {
        line,
        rule: "deprecated.ini_setting",
        title: Localized::same(name.clone()),
        since: PhpVersion::new(a, b),
        severity: Severity::Deprecated,
        message: crate::rules::patterns::INI_SETTING.note,
        replacement: Some(Text::new("ayarı silin", "remove the setting").into()),
    })
}

const REMOVED_SETTING: Text = Text::new(
    "Bu ayar PHP'den kaldırıldı. Açık bırakıldığında PHP \"Directive is no longer available\" hatasıyla başlamaz, yani site hiç açılmaz.",
    "PHP removed this setting. Left on, PHP refuses to start with \"Directive is no longer available\", so the site doesn't load at all.",
);

const PHP_VALUE_FPM: Text = Text::new(
    "php_value ve php_flag yalnızca Apache'nin mod_php modülüyle çalışır. PHP 8 sunucuları çoğunlukla PHP-FPM kullanır; orada bu satır 500 Internal Server Error verir. Ayarı .user.ini dosyasına taşıyın ya da satırı <IfModule mod_php.c> içine alın.",
    "php_value and php_flag only work with Apache's mod_php. PHP 8 hosting mostly runs PHP-FPM, where this line causes a 500 Internal Server Error. Move the setting to .user.ini or wrap it in <IfModule mod_php.c>.",
);

const OLD_MODULE: Text = Text::new(
    "PHP 8'in Apache modülünün adı mod_php.c'dir. Eski modül adını kontrol eden bu bloğun içindeki ayarlar yeni sürümde sessizce uygulanmaz.",
    "PHP 8's Apache module is called mod_php.c. Settings inside this block check the old module name and silently stop applying on the new version.",
);

fn htaccess(text: &str) -> Vec<Hit> {
    let mut out = Vec::new();
    let mut stack: Vec<bool> = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("<ifmodule") {
            let module = rest.trim().trim_end_matches('>').trim();
            let negated = module.starts_with('!');
            let old = match module {
                "mod_php4.c" | "mod_php5.c" | "php5_module" | "php4_module" => Some((7, 0)),
                "mod_php7.c" | "php7_module" => Some((8, 0)),
                _ => None,
            };
            if let Some((a, b)) = old {
                out.push(Hit {
                    line: i + 1,
                    rule: "server.old_php_module",
                    title: Localized::same(line.to_string()),
                    since: PhpVersion::new(a, b),
                    severity: Severity::Behavior,
                    message: OLD_MODULE,
                    replacement: Some(Localized::same("<IfModule mod_php.c>")),
                });
            }
            stack.push(!negated && module.contains("php"));
            continue;
        }
        if lower.starts_with("</ifmodule") {
            stack.pop();
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(directive) = parts.next().map(str::to_ascii_lowercase) else { continue };
        if !matches!(directive.as_str(), "php_value" | "php_flag" | "php_admin_value" | "php_admin_flag") {
            continue;
        }
        let name = parts.next().unwrap_or_default();
        let value = parts.collect::<Vec<_>>().join(" ");
        if let Some(hit) = setting(name, &value, i + 1) {
            out.push(hit);
        }
        if !stack.contains(&true) {
            out.push(Hit {
                line: i + 1,
                rule: "server.php_value_outside_mod_php",
                title: Localized::same(format!("{directive} {name}")),
                since: PhpVersion::new(8, 0),
                severity: Severity::Behavior,
                message: PHP_VALUE_FPM,
                replacement: Some(Localized::same(format!("{name} = {value}"))),
            });
        }
    }
    out
}

fn ini(text: &str) -> Vec<Hit> {
    text.lines()
        .enumerate()
        .filter_map(|(i, raw)| {
            let line = raw.trim();
            if line.starts_with(';') || line.starts_with('#') || line.starts_with('[') {
                return None;
            }
            let (name, value) = line.split_once('=')?;
            let value = value.split(';').next().unwrap_or_default();
            setting(name.trim(), value, i + 1)
        })
        .collect()
}

fn version_at(s: &str) -> Option<PhpVersion> {
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    let mut parts = digits.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    (5..=9).contains(&major).then(|| PhpVersion::new(major, minor))
}

fn image_version(line: &str) -> Option<PhpVersion> {
    let lower = line.to_ascii_lowercase();
    for marker in ["php:", ":php", "/php"] {
        let mut from = 0;
        while let Some(k) = lower[from..].find(marker) {
            let start = from + k + marker.len();
            if let Some(v) = version_at(&lower[start..]) {
                return Some(v);
            }
            from = start;
        }
    }
    None
}

fn listed_versions(line: &str) -> Vec<PhpVersion> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    for k in 0..bytes.len() {
        if bytes[k].is_ascii_digit() && (k == 0 || !(bytes[k - 1].is_ascii_alphanumeric() || bytes[k - 1] == b'.')) {
            if let Some(v) = version_at(&line[k..]) {
                out.push(v);
            }
        }
    }
    out
}

const PINNED: Text = Text::new(
    "Bu dosya hâlâ eski bir PHP sürümünü sabitliyor. Kodu hedef sürüme taşısanız bile bu ortam, CI ya da konteyner eski sürümle çalışmaya devam eder. Sürümü hedefe güncelleyin.",
    "This file still pins an older PHP version. Even after the code moves to the target, this environment, CI job or container keeps running the old one. Update it to the target.",
);

const PLATFORM: Text = Text::new(
    "composer.json'daki config.platform.php, Composer'ın paketleri bu eski sürüme göre seçmesine yol açar. Hedef sürümde desteklenmeyen paket sürümleri kurulabilir. Değeri hedef sürüme güncelleyin.",
    "config.platform.php in composer.json makes Composer pick packages for this older version, so it can install releases that don't support the target. Update it to the target.",
);

fn pin_hit(line: usize, found: PhpVersion, target: PhpVersion, message: Text) -> Hit {
    Hit {
        line,
        rule: "server.php_version_pin",
        title: Localized { tr: format!("PHP {found} sabitlenmiş"), en: format!("PHP {found} pinned") },
        since: target,
        severity: Severity::Behavior,
        message,
        replacement: Some(Localized::same(format!("PHP {target}"))),
    }
}

fn pins(text: &str, kind: Kind, target: PhpVersion) -> Vec<Hit> {
    let mut out: Vec<(usize, PhpVersion)> = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.starts_with('#') {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        let found = match kind {
            Kind::Pin => {
                let rest = lower.strip_prefix("php ").unwrap_or(&lower);
                version_at(rest.trim()).into_iter().collect()
            }
            Kind::Docker => {
                let image = lower.starts_with("from ") || lower.starts_with("image:") || lower.starts_with("- image:");
                image.then(|| image_version(line)).flatten().into_iter().collect()
            }
            _ => {
                let key =
                    lower.trim_start_matches("- ").split(':').next().unwrap_or_default().trim().trim_matches(|c| c == '"' || c == '\'');
                if matches!(key, "php" | "php-version" | "php-versions" | "php_version") {
                    listed_versions(line.split_once(':').map_or("", |(_, v)| v))
                } else if key == "image" || key == "- image" {
                    image_version(line).into_iter().collect()
                } else {
                    Vec::new()
                }
            }
        };
        if let Some(&max) = found.iter().max() {
            out.push((i + 1, max));
        }
    }
    let old = |&(_, v): &(usize, PhpVersion)| v < target;
    if kind == Kind::Ci {
        let best = out.iter().copied().max_by_key(|&(line, v)| (v, std::cmp::Reverse(line)));
        return best.filter(old).map(|(line, v)| pin_hit(line, v, target, PINNED)).into_iter().collect();
    }
    out.into_iter().filter(old).map(|(line, v)| pin_hit(line, v, target, PINNED)).collect()
}

fn composer(text: &str, target: PhpVersion) -> Vec<Hit> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(text) else { return Vec::new() };
    let Some(pinned) = json.pointer("/config/platform/php").and_then(|v| v.as_str()) else { return Vec::new() };
    let Some(found) = version_at(pinned.trim_start_matches(['^', '~', '>', '=', ' '])) else { return Vec::new() };
    if found >= target {
        return Vec::new();
    }
    let lines: Vec<&str> = text.lines().collect();
    let platform = lines.iter().position(|l| l.contains("\"platform\"")).unwrap_or(0);
    let line = (platform..lines.len()).find(|&k| lines[k].contains("\"php\"")).map_or(platform + 1, |k| k + 1);
    vec![pin_hit(line, found, target, PLATFORM)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(found: &[Finding]) -> Vec<(String, usize, String)> {
        let mut v: Vec<_> = found.iter().map(|f| (f.file.replace('\\', "/"), f.line, f.rule_id.clone())).collect();
        v.sort();
        v
    }

    #[test]
    fn server_files_and_version_pins() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let write = |path: &str, text: &str| {
            let p = root.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        write(".htaccess", "php_value memory_limit 256M\n<IfModule mod_php7.c>\n  php_flag magic_quotes_gpc off\n  php_flag register_globals on\n</IfModule>\n<IfModule mod_php.c>\n  php_value upload_max_filesize 8M\n</IfModule>\n");
        write("public/.user.ini", "; comment\nallow_url_include = On\nsafe_mode = Off\ntrack_errors = 1\n");
        write("Dockerfile", "FROM php:7.4-fpm-alpine\nFROM node:20 AS assets\n");
        write("docker-compose.yml", "services:\n  app:\n    image: wordpress:php8.4-apache\n  old:\n    image: php:8.1-cli\n");
        write(".github/workflows/ci.yml", "jobs:\n  test:\n    strategy:\n      matrix:\n        php: ['7.4', '8.0']\n    steps:\n      - uses: shivammathur/setup-php@v2\n        with:\n          php-version: ${{ matrix.php }}\n  modern:\n    steps:\n      - with:\n          php-version: '8.4'\n");
        write(".github/workflows/legacy.yml", "matrix:\n  include:\n    - php: 7.4\n    - php: 7.3\n");
        write(".tool-versions", "nodejs 20.1.0\nphp 8.2.10\n");
        write("composer.json", "{\n  \"config\": {\n    \"platform\": {\n      \"php\": \"7.4.33\"\n    }\n  }\n}\n");
        write("vendor/pkg/.htaccess", "php_value x 1\n");
        let found = scan(root, PhpVersion::new(8, 4));
        let expected = [
            (".github/workflows/legacy.yml", 3, "server.php_version_pin"),
            (".htaccess", 1, "server.php_value_outside_mod_php"),
            (".htaccess", 2, "server.old_php_module"),
            (".htaccess", 4, "removed.ini_setting"),
            (".tool-versions", 2, "server.php_version_pin"),
            ("Dockerfile", 1, "server.php_version_pin"),
            ("composer.json", 4, "server.php_version_pin"),
            ("docker-compose.yml", 5, "server.php_version_pin"),
            ("public/.user.ini", 2, "deprecated.ini_setting"),
            ("public/.user.ini", 4, "removed.ini_setting"),
        ];
        let expected: Vec<_> = expected.iter().map(|(f, l, r)| (f.to_string(), *l, r.to_string())).collect();
        assert_eq!(ids(&found), expected);
        let docker = found.iter().find(|f| f.file == "Dockerfile").unwrap();
        assert_eq!((docker.title.en.as_str(), docker.since.to_string()), ("PHP 7.4 pinned", "8.4".to_string()));
        assert!(scan(root, PhpVersion::new(8, 2)).iter().all(|f| f.file != ".tool-versions"));
    }
}
