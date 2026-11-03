use crate::analyzer::{relative_path, FileError, Finding};
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

const MAX_ENTRIES: usize = MAX_FILES * 20;

pub fn scan(root: &Path, target: PhpVersion) -> (Vec<Finding>, Vec<FileError>) {
    let keep = |e: &walkdir::DirEntry| {
        if e.depth() == 0 || !e.file_type().is_dir() {
            return true;
        }
        let name = e.file_name().to_string_lossy();
        (e.depth() == 1 && name == ".github") || (!name.starts_with('.') && !crate::project::ALWAYS_SKIP.contains(&name.as_ref()))
    };
    let mut out = Vec::new();
    let mut errors = Vec::new();
    let mut seen = 0;
    for entry in WalkDir::new(root).max_depth(8).follow_links(false).into_iter().filter_entry(keep) {
        seen += 1;
        if seen > MAX_ENTRIES || out.len() > MAX_FILES {
            errors.push(FileError {
                file: ".".into(),
                message: crate::t!(
                    "Sunucu yapılandırması araması {MAX_ENTRIES} girişte durdu; daha sonraki dosyalar kontrol edilmedi.",
                    "The server configuration search stopped after {MAX_ENTRIES} entries; later files weren't checked."
                ),
            });
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                let file = e.path().map_or_else(|| ".".to_string(), |p| relative_path(root, p));
                errors.push(FileError { file, message: e.to_string() });
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let rel = relative_path(root, entry.path());
        let in_workflows = rel.replace('\\', "/").starts_with(".github/workflows/");
        let kind = match name.as_str() {
            ".htaccess" => Kind::Htaccess,
            ".user.ini" | "php.ini" => Kind::Ini,
            "composer.json" if entry.depth() == 1 => Kind::Composer,
            "composer.lock" if entry.depth() == 1 => Kind::ComposerLock,
            ".tool-versions" | ".php-version" if entry.depth() == 1 => Kind::Pin,
            ".gitlab-ci.yml" | "bitbucket-pipelines.yml" | ".travis.yml" => Kind::Ci,
            n if n == "dockerfile" || n.starts_with("dockerfile.") || n.ends_with(".dockerfile") => Kind::Docker,
            n if (n.starts_with("docker-compose") || n.starts_with("compose")) && (n.ends_with(".yml") || n.ends_with(".yaml")) => {
                Kind::Docker
            }
            n if in_workflows && (n.ends_with(".yml") || n.ends_with(".yaml")) => Kind::Ci,
            _ => continue,
        };
        if std::fs::metadata(entry.path()).is_ok_and(|m| m.len() > crate::project::MAX_FILE_BYTES) {
            errors.push(FileError {
                file: rel,
                message: crate::t!("Atlandı: dosya çok büyük (2 MB üstü).", "Skipped: file is too large (over 2 MB)."),
            });
            continue;
        }
        let text = match std::fs::read_to_string(entry.path()) {
            Ok(text) => text,
            Err(e) => {
                errors.push(FileError { file: rel, message: crate::i18n::io_error(&e) });
                continue;
            }
        };
        let hits = match kind {
            Kind::Htaccess => htaccess(&text),
            Kind::Ini => ini(&text),
            Kind::Composer => composer(&text, target),
            Kind::ComposerLock => composer_lock(&text, target),
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
                runtime_hits: 0,
            });
        }
    }
    (out, errors)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Htaccess,
    Ini,
    Composer,
    ComposerLock,
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
    let Some(&(_, a, b)) = DEPRECATED_INI.iter().find(|(k, _, _)| *k == name) else {
        let generated =
            crate::symbols::REMOVED_INI.binary_search_by(|r| r.0.cmp(name.as_str())).ok().map(|i| crate::symbols::REMOVED_INI[i].1);
        let ignored = crate::rules::IGNORED_INI.iter().find(|(k, _, _)| *k == name).map(|&(_, a, b)| (a, b));
        let (a, b) = generated.or(ignored)?;
        return Some(Hit {
            line,
            rule: "removed.builtin_ini",
            title: Localized::same(name.clone()),
            since: PhpVersion::new(a, b),
            severity: Severity::Behavior,
            message: IGNORED_SETTING,
            replacement: Some(Text::new("satırı silin", "delete the line").into()),
        });
    };
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

const IGNORED_SETTING: Text = Text::new(
    "Bu ayar PHP'den kaldırıldı. PHP satırı yok sayar, yani ayarın yaptığı iş artık yapılmaz.",
    "PHP removed this setting. It ignores the line, so whatever the setting did no longer happens.",
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
    let mut out = Vec::new();
    let mut jit_buffer = None;
    let mut jit_mode = false;
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix('#') {
            out.push(Hit {
                line: i + 1,
                rule: "removed.ini_hash_comment",
                title: Localized::same(line.chars().take(40).collect::<String>()),
                since: PhpVersion::new(7, 0),
                severity: Severity::Breaking,
                message: HASH_COMMENT,
                replacement: Some(Localized::same(format!(";{rest}"))),
            });
            continue;
        }
        if line.starts_with(';') || line.starts_with('[') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else { continue };
        let value = value.split(';').next().unwrap_or_default();
        let key = name.trim().to_ascii_lowercase();
        match key.as_str() {
            "opcache.jit" => jit_mode = true,
            "opcache.jit_buffer_size" if truthy(value) => jit_buffer = Some(i + 1),
            "zend_extension" if value.to_ascii_lowercase().contains("opcache") => out.push(Hit {
                line: i + 1,
                rule: "behavior.opcache_zend_extension",
                title: Localized::same(line.to_string()),
                since: PhpVersion::new(8, 5),
                severity: Severity::Behavior,
                message: OPCACHE_BUILT_IN,
                replacement: Some(Text::new("satırı silin", "delete the line").into()),
            }),
            _ => {}
        }
        if let Some(hit) = setting(name.trim(), value, i + 1) {
            out.push(hit);
        }
    }
    if let Some(line) = jit_buffer.filter(|_| !jit_mode) {
        out.push(Hit {
            line,
            rule: "behavior.opcache_jit_default",
            title: Localized::same("opcache.jit_buffer_size".to_string()),
            since: PhpVersion::new(8, 4),
            severity: Severity::Behavior,
            message: JIT_DEFAULT,
            replacement: Some(Localized::same("opcache.jit = tracing".to_string())),
        });
    }
    out
}

const HASH_COMMENT: Text = Text::new(
    "PHP 7.0'dan beri ini dosyalarında # yorum başlatmaz; satır ayar olarak okunur ve dosya hata verebilir. Yorum için ; kullanın.",
    "Since PHP 7.0 # doesn't start a comment in ini files; the line is read as a setting and the file can fail to parse. Use ; for comments.",
);

const OPCACHE_BUILT_IN: Text = Text::new(
    "PHP 8.5'te OPcache her zaman PHP'nin içinde gelir; onu zend_extension ile yüklemek başlangıçta uyarı verir.",
    "In PHP 8.5 OPcache is always built in, and loading it with zend_extension raises a warning at startup.",
);

const JIT_DEFAULT: Text = Text::new(
    "PHP 8.4'te opcache.jit'in varsayılanı disable oldu. opcache.jit ayarlanmadan yalnızca jit_buffer_size verilirse JIT artık sessizce kapalı kalır.",
    "In PHP 8.4 opcache.jit defaults to disable. Setting only jit_buffer_size, without opcache.jit, now silently leaves the JIT off.",
);

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

const PLATFORM_CHECK: Text = Text::new(
    "config.platform-check kapalı. Composer, kurulu paketlerin sunucudaki PHP sürümüyle ve eklentilerle uyumlu olduğunu çalışırken artık kontrol etmiyor; uyumsuz bir paket ancak bir hata çıktığında fark edilir. Geçiş sırasında açık tutun.",
    "config.platform-check is turned off, so Composer no longer checks at runtime that the installed packages fit the server's PHP version and extensions. An incompatible package only shows up when something fails. Keep it on during the upgrade.",
);

const COMPOSER1_LOCK: Text = Text::new(
    "Bu composer.lock Composer 1 ile oluşturulmuş (plugin-api-version yok). Composer 1 artık güncellenmiyor ve Packagist'in yeni meta verisini okuyamıyor. PHP 8 uyumlu paket sürümlerine geçmeden önce Composer 2 ile composer update çalıştırın.",
    "This composer.lock was written by Composer 1 (there is no plugin-api-version). Composer 1 is no longer maintained and can't read Packagist's current metadata. Run composer update with Composer 2 before moving to PHP 8 compatible package versions.",
);

fn line_of(text: &str, needle: &str) -> usize {
    text.lines().position(|l| l.contains(needle)).map_or(1, |k| k + 1)
}

fn composer(text: &str, target: PhpVersion) -> Vec<Hit> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(text) else { return Vec::new() };
    let mut out = Vec::new();
    if json.pointer("/config/platform-check").is_some_and(|v| v == &serde_json::Value::Bool(false) || v.as_str() == Some("false")) {
        out.push(Hit {
            line: line_of(text, "\"platform-check\""),
            rule: "server.composer_platform_check",
            title: Localized::same("config.platform-check: false"),
            since: target,
            severity: Severity::Behavior,
            message: PLATFORM_CHECK,
            replacement: Some(Localized::same("\"platform-check\": true")),
        });
    }
    let pinned = json.pointer("/config/platform/php").and_then(|v| v.as_str());
    if let Some(found) = pinned.and_then(|p| version_at(p.trim_start_matches(['^', '~', '>', '=', ' ']))).filter(|f| *f < target) {
        let lines: Vec<&str> = text.lines().collect();
        let platform = lines.iter().position(|l| l.contains("\"platform\"")).unwrap_or(0);
        let line = (platform..lines.len()).find(|&k| lines[k].contains("\"php\"")).map_or(platform + 1, |k| k + 1);
        out.push(pin_hit(line, found, target, PLATFORM));
    }
    out
}

fn composer_lock(text: &str, target: PhpVersion) -> Vec<Hit> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(text) else { return Vec::new() };
    if json.get("plugin-api-version").is_some() || json.get("packages").is_none() || json.get("content-hash").is_none() {
        return Vec::new();
    }
    vec![Hit {
        line: 1,
        rule: "server.composer1_lock",
        title: Localized { tr: "Composer 1 kilit dosyası".into(), en: "Composer 1 lock file".into() },
        since: target,
        severity: Severity::Behavior,
        message: COMPOSER1_LOCK,
        replacement: Some(Localized::same("composer update (Composer 2)")),
    }]
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
        write("public/.user.ini", "; comment\nallow_url_include = On\nsafe_mode = Off\ntrack_errors = 1\nsession.hash_function = 0\n# old comment\nopcache.load_comments = 0\nzend_extension = opcache.so\nopcache.jit_buffer_size = 64M\n");
        write("Dockerfile", "FROM php:7.4-fpm-alpine\nFROM node:20 AS assets\n");
        write("docker-compose.yml", "services:\n  app:\n    image: wordpress:php8.4-apache\n  old:\n    image: php:8.1-cli\n");
        write(".github/workflows/ci.yml", "jobs:\n  test:\n    strategy:\n      matrix:\n        php: ['7.4', '8.0']\n    steps:\n      - uses: shivammathur/setup-php@v2\n        with:\n          php-version: ${{ matrix.php }}\n  modern:\n    steps:\n      - with:\n          php-version: '8.4'\n");
        write(".github/workflows/legacy.yml", "matrix:\n  include:\n    - php: 7.4\n    - php: 7.3\n");
        write(".tool-versions", "nodejs 20.1.0\nphp 8.2.10\n");
        write(
            "composer.json",
            "{\n  \"config\": {\n    \"platform-check\": false,\n    \"platform\": {\n      \"php\": \"7.4.33\"\n    }\n  }\n}\n",
        );
        write("composer.lock", "{\n  \"content-hash\": \"abc\",\n  \"packages\": []\n}\n");
        write("vendor/pkg/.htaccess", "php_value x 1\n");
        let found = scan(root, PhpVersion::new(8, 4)).0;
        let expected = [
            (".github/workflows/legacy.yml", 3, "server.php_version_pin"),
            (".htaccess", 1, "server.php_value_outside_mod_php"),
            (".htaccess", 2, "server.old_php_module"),
            (".htaccess", 4, "removed.ini_setting"),
            (".tool-versions", 2, "server.php_version_pin"),
            ("Dockerfile", 1, "server.php_version_pin"),
            ("composer.json", 3, "server.composer_platform_check"),
            ("composer.json", 5, "server.php_version_pin"),
            ("composer.lock", 1, "server.composer1_lock"),
            ("docker-compose.yml", 5, "server.php_version_pin"),
            ("public/.user.ini", 2, "deprecated.ini_setting"),
            ("public/.user.ini", 4, "removed.ini_setting"),
            ("public/.user.ini", 5, "removed.builtin_ini"),
            ("public/.user.ini", 6, "removed.ini_hash_comment"),
            ("public/.user.ini", 7, "removed.builtin_ini"),
            ("public/.user.ini", 9, "behavior.opcache_jit_default"),
        ];
        let expected: Vec<_> = expected.iter().map(|(f, l, r)| (f.to_string(), *l, r.to_string())).collect();
        assert_eq!(ids(&found), expected);
        let docker = found.iter().find(|f| f.file == "Dockerfile").unwrap();
        assert_eq!((docker.title.en.as_str(), docker.since.to_string()), ("PHP 7.4 pinned", "8.4".to_string()));
        assert!(scan(root, PhpVersion::new(8, 2)).0.iter().all(|f| f.file != ".tool-versions"));
    }

    #[test]
    fn oversized_config_files_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".htaccess"), "#".repeat(3 * 1024 * 1024)).unwrap();
        let (_, errors) = scan(dir.path(), PhpVersion::new(8, 4));
        assert_eq!(errors.iter().map(|e| e.file.as_str()).collect::<Vec<_>>(), [".htaccess"]);
    }
}
