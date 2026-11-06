use crate::analyzer::{relative_path, FileError, Finding};
use crate::i18n::{Localized, Text};
use crate::rector::{AppliedRule, FileChange, RectorMode, RectorPreview};
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

const MAX_ENTRIES: usize = 2_000_000;
const HIDDEN_DIRS: [&str; 4] = [".github", ".ddev", ".upsun", ".platform"];

fn visit_files(root: &Path, mut visit: impl FnMut(String, Kind, String)) -> Vec<FileError> {
    let exclude = crate::config::load(root).unwrap_or_default().exclude;
    let excluded: Vec<std::path::PathBuf> = exclude.iter().map(|d| root.join(d.trim_end_matches('/'))).collect();
    let keep = |e: &walkdir::DirEntry| {
        if e.depth() == 0 {
            return true;
        }
        if excluded.iter().any(|x| x == e.path()) {
            return false;
        }
        if !e.file_type().is_dir() {
            return true;
        }
        let name = e.file_name().to_string_lossy();
        (e.depth() == 1 && HIDDEN_DIRS.contains(&name.as_ref()))
            || (!name.starts_with('.') && !crate::project::ALWAYS_SKIP.contains(&name.as_ref()))
    };
    let mut errors = Vec::new();
    let mut seen = 0;
    let mut files = 0;
    for entry in WalkDir::new(root).follow_links(false).into_iter().filter_entry(keep) {
        seen += 1;
        if seen > MAX_ENTRIES || files > MAX_FILES {
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
        let unix = rel.replace('\\', "/");
        let in_workflows = unix.starts_with(".github/workflows/");
        let yaml = name.ends_with(".yml") || name.ends_with(".yaml");
        let hosting = (unix.starts_with(".ddev/") && name == "config.yaml")
            || ((unix.starts_with(".upsun/") || unix.starts_with(".platform/")) && yaml)
            || (entry.depth() == 1 && matches!(name.as_str(), ".lando.yml" | ".lando.yaml" | ".platform.app.yaml"));
        let kind = match name.as_str() {
            _ if hosting => Kind::Hosting,
            ".htaccess" => Kind::Htaccess,
            ".user.ini" | "php.ini" => Kind::Ini,
            "composer.json" if entry.depth() == 1 => Kind::Composer,
            "composer.lock" if entry.depth() == 1 => Kind::ComposerLock,
            ".tool-versions" | ".php-version" if entry.depth() == 1 => Kind::Pin,
            ".gitlab-ci.yml" | "bitbucket-pipelines.yml" | ".travis.yml" => Kind::Ci,
            "caddyfile" => Kind::Socket,
            n if n.ends_with(".conf") => Kind::Socket,
            n if n == "dockerfile" || n.starts_with("dockerfile.") || n.ends_with(".dockerfile") => Kind::Docker,
            n if (n.starts_with("docker-compose") || n.starts_with("compose")) && yaml => Kind::Docker,
            _ if in_workflows && yaml => Kind::Ci,
            _ => continue,
        };
        if std::fs::metadata(entry.path()).is_ok_and(|m| m.len() > crate::project::MAX_FILE_BYTES) {
            errors.push(FileError {
                file: rel,
                message: crate::t!("Atlandı: dosya çok büyük (2 MB üstü).", "Skipped: file is too large (over 2 MB)."),
            });
            continue;
        }
        match std::fs::read_to_string(entry.path()) {
            Ok(text) => {
                files += 1;
                visit(rel, kind, text);
            }
            Err(e) => errors.push(FileError { file: rel, message: crate::i18n::io_error(&e) }),
        }
    }
    errors
}

pub fn scan(root: &Path, target: PhpVersion) -> (Vec<Finding>, Vec<FileError>) {
    let mut out = Vec::new();
    let errors = visit_files(root, |rel, kind, text| {
        let hits = match kind {
            Kind::Htaccess => htaccess(&text),
            Kind::Ini => ini(&text),
            Kind::Composer => composer(&text, target),
            Kind::ComposerLock => composer_lock(&text, target),
            Kind::Pin | Kind::Docker | Kind::Ci | Kind::Hosting | Kind::Socket => pins(&text, kind, target),
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
    });
    (out, errors)
}

/// Every pin that `scan` reports, moved to the target. Nothing is written here: the changes go
/// through the same preview, backup and undo as Rector's.
pub fn bump(root: &Path, target: PhpVersion) -> RectorPreview {
    let started = std::time::Instant::now();
    let mut changes = Vec::new();
    let errors = visit_files(root, |rel, kind, text| {
        let Some((after, note)) = bumped(&text, kind, target) else { return };
        let rule = AppliedRule {
            class: note.class().to_string(),
            name: note.class().to_string(),
            title: note.title(target),
            since: Some(target),
            fix: FixKind::Likely,
        };
        changes.push(FileChange {
            file: rel,
            rules: vec![rule],
            fix: FixKind::Likely,
            diff: crate::diff::compute(&text, &after, 3),
            original_hash: crate::snapshot::fnv1a(text.as_bytes()),
            new_content: after,
        });
    });
    changes.sort_by(|a, b| a.file.cmp(&b.file));
    RectorPreview {
        target,
        mode: RectorMode::Pins,
        changes,
        errors: errors.into_iter().map(|e| format!("{}: {}", e.file, e.message)).collect(),
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BumpNote {
    Moved,
    Exact,
    Composer,
}

impl BumpNote {
    fn class(self) -> &'static str {
        match self {
            BumpNote::Moved => "server.php_version_pin",
            BumpNote::Exact => "server.php_version_exact",
            BumpNote::Composer => "server.composer_platform",
        }
    }

    fn title(self, target: PhpVersion) -> Localized {
        let text = match self {
            BumpNote::Moved => (format!("PHP sürümü {target} olur"), format!("PHP version becomes {target}")),
            BumpNote::Exact => (
                format!("PHP {target}.0 yazılır; kullandığınız yama sürümüyle değiştirin"),
                format!("Writes PHP {target}.0; change it to the patch release you use"),
            ),
            BumpNote::Composer => (
                format!("config.platform.php {target} olur; ardından composer update çalıştırın"),
                format!("config.platform.php becomes {target}; run composer update afterwards"),
            ),
        };
        Localized { tr: text.0, en: text.1 }
    }
}

fn bumped(text: &str, kind: Kind, target: PhpVersion) -> Option<(String, BumpNote)> {
    if kind == Kind::Composer {
        let (_, found) = composer_platform(text)?;
        return (found < target).then(|| bumped_composer(text, target)).flatten().map(|after| (after, BumpNote::Composer));
    }
    if !matches!(kind, Kind::Pin | Kind::Docker | Kind::Ci | Kind::Hosting | Kind::Socket) {
        return None;
    }
    let lines = stale_lines(text, kind, target);
    let mut exact = false;
    let after = edit_lines(text, |k, raw| {
        let (_, spans) = lines.iter().find(|(line, _)| *line == k + 1)?;
        let (new, full) = bumped_line(raw, spans, kind, target)?;
        exact |= full;
        Some(new)
    });
    (after != text).then_some((after, if exact { BumpNote::Exact } else { BumpNote::Moved }))
}

fn edit_lines(text: &str, mut edit: impl FnMut(usize, &str) -> Option<String>) -> String {
    text.split_inclusive('\n').enumerate().map(|(k, raw)| edit(k, raw).unwrap_or_else(|| raw.to_string())).collect()
}

/// The byte range of a string value at a JSON path, found by walking the document so that a key
/// of the same name elsewhere can't be mistaken for it.
fn json_string_span(text: &str, path: &[&str]) -> Option<std::ops::Range<usize>> {
    struct Reader<'a> {
        bytes: &'a [u8],
        at: usize,
    }
    impl Reader<'_> {
        fn skip_space(&mut self) {
            while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
                self.at += 1;
            }
        }
        fn string(&mut self) -> Option<std::ops::Range<usize>> {
            self.at += 1;
            let start = self.at;
            while *self.bytes.get(self.at)? != b'"' {
                self.at += if self.bytes[self.at] == b'\\' { 2 } else { 1 };
            }
            self.at += 1;
            Some(start..self.at - 1)
        }
        fn skip_value(&mut self) -> Option<()> {
            self.skip_space();
            match *self.bytes.get(self.at)? {
                b'"' => self.string().map(|_| ()),
                b'{' | b'[' => {
                    let mut depth = 0usize;
                    loop {
                        match *self.bytes.get(self.at)? {
                            b'"' => {
                                self.string()?;
                                continue;
                            }
                            b'{' | b'[' => depth += 1,
                            b'}' | b']' => depth -= 1,
                            _ => {}
                        }
                        self.at += 1;
                        if depth == 0 {
                            return Some(());
                        }
                    }
                }
                _ => {
                    while !matches!(*self.bytes.get(self.at)?, b',' | b'}' | b']') {
                        self.at += 1;
                    }
                    Some(())
                }
            }
        }
        fn find(&mut self, path: &[&str], text: &str) -> Option<std::ops::Range<usize>> {
            self.skip_space();
            let Some((first, rest)) = path.split_first() else {
                return (*self.bytes.get(self.at)? == b'"').then(|| self.string()).flatten();
            };
            if *self.bytes.get(self.at)? != b'{' {
                return None;
            }
            self.at += 1;
            let mut found = None;
            loop {
                self.skip_space();
                match *self.bytes.get(self.at)? {
                    b'}' => return found,
                    b',' => self.at += 1,
                    b'"' => {
                        let key = self.string()?;
                        self.skip_space();
                        self.at += 1;
                        if text[key] == **first {
                            found = self.find(rest, text).or(found);
                        } else {
                            self.skip_value()?;
                        }
                    }
                    _ => return None,
                }
            }
        }
    }
    Reader { bytes: text.as_bytes(), at: 0 }.find(path, text)
}

const PLATFORM_PHP: [&str; 3] = ["config", "platform", "php"];

/// Edits the value itself, then checks that nothing else in the document moved.
fn bumped_composer(text: &str, target: PhpVersion) -> Option<String> {
    let span = json_string_span(text, &PLATFORM_PHP)?;
    let value = &text[span.clone()];
    let start = span.start + value.find(|c: char| c.is_ascii_digit())?;
    let end = start + version_len(&text[start..]);
    let full = text[start..end].matches('.').count() == 2;
    let version = if full { format!("{target}.0") } else { target.to_string() };
    let after = format!("{}{version}{}", &text[..start], &text[end..]);
    let mut wanted: serde_json::Value = serde_json::from_str(text).ok()?;
    let edited = format!("{}{version}{}", &text[span.start..start], &text[end..span.end]);
    *wanted.pointer_mut("/config/platform/php")? = serde_json::Value::String(edited);
    (serde_json::from_str::<serde_json::Value>(&after).ok()? == wanted).then_some(after)
}

/// Returns the edited line and whether an exact release was written. Images pinned by digest stay
/// as they are: the digest wouldn't match a new tag. In a list of versions the target is added,
/// so CI keeps testing what it tested.
fn bumped_line(raw: &str, spans: &[Span], kind: Kind, target: PhpVersion) -> Option<(String, bool)> {
    if raw.contains("@sha256:") {
        return None;
    }
    let last = spans.last()?;
    if spans.len() > 1 {
        let before = raw[..last.0.start].chars().next_back().filter(|c| matches!(c, '\'' | '"'));
        let after = raw[last.0.end..].chars().next().filter(|c| Some(*c) == before);
        let quote = after.map_or(String::new(), String::from);
        let end = last.0.end + quote.len();
        return Some((format!("{}, {quote}{target}{quote}{}", &raw[..end], &raw[end..]), false));
    }
    let full = kind == Kind::Pin && raw[last.0.clone()].matches('.').count() == 2;
    let version = if full { format!("{target}.0") } else { target.to_string() };
    Some((format!("{}{version}{}", &raw[..last.0.start], &raw[last.0.end..]), full))
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
    Hosting,
    Socket,
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

type Span = (std::ops::Range<usize>, PhpVersion);

fn version_len(s: &str) -> usize {
    s.bytes().take_while(|c| c.is_ascii_digit() || *c == b'.').count()
}

fn version_at(s: &str) -> Option<PhpVersion> {
    let mut parts = s[..version_len(s)].split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    (5..=9).contains(&major).then(|| PhpVersion::new(major, minor))
}

fn span_at(line: &str, start: usize) -> Option<Span> {
    let rest = &line[start..];
    let found = version_at(rest)?;
    Some((start..start + rest[..version_len(rest)].trim_end_matches('.').len(), found))
}

fn image_version(line: &str) -> Option<Span> {
    let lower = line.to_ascii_lowercase();
    for marker in ["php:", ":php", "/php"] {
        let mut from = 0;
        while let Some(k) = lower[from..].find(marker) {
            let start = from + k + marker.len();
            if let Some(span) = span_at(line, start) {
                return Some(span);
            }
            from = start;
        }
    }
    None
}

fn listed_versions(line: &str, from: usize) -> Vec<Span> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    for k in from..bytes.len() {
        if bytes[k].is_ascii_digit() && (k == 0 || !(bytes[k - 1].is_ascii_alphanumeric() || bytes[k - 1] == b'.')) {
            out.extend(span_at(line, k));
        }
    }
    out
}

fn socket_version(line: &str) -> Option<Span> {
    let lower = line.to_ascii_lowercase();
    lower.match_indices("php").find_map(|(k, _)| {
        let span = span_at(line, k + 3)?;
        lower[span.0.end..].starts_with("-fpm").then_some(span)
    })
}

fn yaml_key(line: &str) -> (&str, usize) {
    let start = line.len() - line.trim_start_matches(|c: char| c.is_whitespace() || c == '-').len();
    let Some(colon) = line[start..].find(':') else { return ("", 0) };
    (line[start..start + colon].trim().trim_matches(|c| c == '"' || c == '\''), start + colon + 1)
}

fn line_spans(line: &str, kind: Kind) -> Vec<Span> {
    let trimmed = line.trim_start();
    let lower = trimmed.to_ascii_lowercase();
    let indent = line.len() - trimmed.len();
    let (key, value) = yaml_key(line);
    let key = key.to_ascii_lowercase();
    match kind {
        Kind::Pin => span_at(line, indent + if lower.starts_with("php ") { 4 } else { 0 }).into_iter().collect(),
        Kind::Docker => {
            if let Some(at) =
                lower.find("php_version=").filter(|_| lower.starts_with("arg ") || lower.starts_with("env ") || lower.starts_with("- "))
            {
                return span_at(line, indent + at + "php_version=".len()).into_iter().collect();
            }
            if key == "php_version" {
                return listed_versions(line, value);
            }
            let image = lower.starts_with("from ") || key == "image";
            image.then(|| image_version(line)).flatten().into_iter().collect()
        }
        Kind::Socket => socket_version(line).into_iter().collect(),
        Kind::Hosting if matches!(key.as_str(), "type" | "via") => image_version(line).into_iter().collect(),
        _ if matches!(key.as_str(), "php" | "php-version" | "php-versions" | "php_version") => listed_versions(line, value),
        _ if matches!(key.as_str(), "image" | "container") => image_version(line).into_iter().collect(),
        _ if key == "name" && lower.contains("php:") => image_version(line).into_iter().collect(),
        _ if lower.contains("docker run") => image_version(line).into_iter().collect(),
        _ => Vec::new(),
    }
}

/// The line up to a YAML comment. A `#` inside quotes or a word stays.
fn without_comment(line: &str) -> &str {
    let mut quote = None;
    for (at, c) in line.char_indices() {
        match (quote, c) {
            (Some(open), _) if open == c => quote = None,
            (None, '"' | '\'') => quote = Some(c),
            (None, '#') if at == 0 || line[..at].ends_with(char::is_whitespace) => return &line[..at],
            _ => {}
        }
    }
    line
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Matrix {
    Include,
    Exclude,
}

/// Each line with its versions, and whether it sits under a CI matrix's `include:`. Those lines
/// adjust jobs the matrix already has, and `exclude:` lines remove jobs, so neither is a pin.
fn pin_lines(text: &str, kind: Kind) -> Vec<(usize, Vec<Span>, bool)> {
    let mut out = Vec::new();
    let mut list = None;
    let mut matrix: Option<(usize, Matrix)> = None;
    for (i, raw) in text.lines().enumerate().filter(|(_, raw)| !raw.trim().is_empty() && !raw.trim_start().starts_with('#')) {
        let raw = without_comment(raw);
        let indent = raw.len() - raw.trim_start().len();
        let item = raw.trim_start().starts_with("- ");
        if matrix.is_some_and(|(open, _)| indent < open || (indent == open && !item)) {
            matrix = None;
        }
        if kind == Kind::Ci && matrix.is_none() {
            let (key, value) = yaml_key(raw);
            matrix = match key.to_ascii_lowercase().as_str() {
                _ if !raw[value..].trim().is_empty() => None,
                "include" => Some((indent, Matrix::Include)),
                "exclude" => Some((indent, Matrix::Exclude)),
                _ => None,
            };
            if matrix.is_some() {
                continue;
            }
        }
        if matrix.is_some_and(|(_, m)| m == Matrix::Exclude) {
            continue;
        }
        let included = matrix.is_some();
        if list.is_some_and(|open: usize| item && indent >= open) {
            let spans = listed_versions(raw, 0);
            if !spans.is_empty() {
                out.push((i + 1, spans, included));
            }
            continue;
        }
        list = None;
        let (key, value) = yaml_key(raw);
        let starts_list = matches!(kind, Kind::Ci | Kind::Hosting)
            && matches!(key.to_ascii_lowercase().as_str(), "php" | "php-version" | "php-versions" | "php_version")
            && raw[value..].trim().is_empty();
        if starts_list {
            list = Some(indent);
            continue;
        }
        let spans = line_spans(raw, kind);
        if !spans.is_empty() {
            out.push((i + 1, spans, included));
        }
    }
    out
}

fn newest(spans: &[Span]) -> PhpVersion {
    spans.iter().map(|s| s.1).max().unwrap_or(PhpVersion::new(0, 0))
}

/// A CI file passes as long as one job reaches the target, so only its newest pins count as stale.
fn stale_lines(text: &str, kind: Kind, target: PhpVersion) -> Vec<(usize, Vec<Span>)> {
    let lines = pin_lines(text, kind);
    let tested = lines.iter().map(|(_, spans, _)| newest(spans)).max();
    let has_own = lines.iter().any(|(_, _, included)| !included);
    let lines: Vec<(usize, Vec<Span>)> =
        lines.into_iter().filter(|(_, _, included)| !has_own || !included).map(|(line, spans, _)| (line, spans)).collect();
    let best = lines.iter().map(|(_, spans)| newest(spans)).max();
    match kind {
        Kind::Ci if tested.is_some_and(|b| b >= target) => Vec::new(),
        Kind::Ci => lines.into_iter().filter(|(_, spans)| Some(newest(spans)) == best).collect(),
        _ => lines.into_iter().filter(|(_, spans)| newest(spans) < target).collect(),
    }
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
    let stale = stale_lines(text, kind, target);
    let reported = if kind == Kind::Ci { &stale[..stale.len().min(1)] } else { &stale[..] };
    reported.iter().map(|(line, spans)| pin_hit(*line, newest(spans), target, PINNED)).collect()
}

const PLATFORM_CHECK: Text = Text::new(
    "config.platform-check kapalı. Composer, kurulu paketlerin sunucudaki PHP sürümüyle ve eklentilerle uyumlu olduğunu çalışırken artık kontrol etmiyor; uyumsuz bir paket ancak bir hata çıktığında fark edilir. Geçiş sırasında açık tutun.",
    "config.platform-check is turned off, so Composer no longer checks at runtime that the installed packages fit the server's PHP version and extensions. An incompatible package only shows up when something fails. Keep it on during the upgrade.",
);

const PLATFORM_EXT: Text = Text::new(
    "config.platform bu eklentiyi eski bir sürümde sabitliyor. Composer sunucuya bakmadan paketleri bu varsayıma göre seçer; eklenti hedef PHP'de yoksa ya da sürümü farklıysa hata ancak çalışırken çıkar. Satırı silin ya da hedef sunucudaki sürümü yazın.",
    "config.platform pins this extension to an old version. Composer picks packages from that assumption without looking at the server, so if the extension is missing or different on the target PHP, the error only shows up at runtime. Delete the line or write the version on the target server.",
);

const OLD_PLUGIN: Text = Text::new(
    "Bu Composer eklentisi yalnızca Composer 1 ile çalışır. Composer 2 onu devre dışı bırakır ya da kurulumu durdurur; PHP 8 uyumlu paketler için Composer 2 gerekir. Eklentiyi Composer 2 destekleyen bir sürüme yükseltin ya da kaldırın.",
    "This Composer plugin only works with Composer 1. Composer 2 disables it or stops the install, and PHP 8 compatible packages need Composer 2. Upgrade the plugin to a release that supports Composer 2, or remove it.",
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
    if let Some((line, found)) = composer_platform(text).filter(|(_, found)| *found < target) {
        out.push(pin_hit(line, found, target, PLATFORM));
    }
    let platform = json.pointer("/config/platform").and_then(|p| p.as_object());
    for (name, value) in platform.into_iter().flatten().filter(|(name, _)| name.starts_with("ext-")) {
        let pinned = value.as_str().unwrap_or_default();
        if version_at(pinned).is_some_and(|found| found < target) {
            out.push(Hit {
                line: line_of(text, &format!("\"{name}\"")),
                rule: "server.composer_platform_ext",
                title: Localized::same(format!("config.platform.{name}: {pinned}")),
                since: target,
                severity: Severity::Behavior,
                message: PLATFORM_EXT,
                replacement: Some(Text::new("satırı silin", "delete the line").into()),
            });
        }
    }
    out
}

fn composer_platform(text: &str) -> Option<(usize, PhpVersion)> {
    let json = serde_json::from_str::<serde_json::Value>(text).ok()?;
    let pinned = json.pointer("/config/platform/php")?.as_str()?;
    let found = version_at(pinned.trim_start_matches(['^', '~', '>', '=', ' ']))?;
    let span = json_string_span(text, &PLATFORM_PHP)?;
    let line = text[..span.start].matches('\n').count() + 1;
    Some((line, found))
}

/// True when no alternative in the constraint can match Composer 2's plugin API.
fn composer1_only(constraint: &str) -> bool {
    constraint.split('|').map(str::trim).filter(|alt| !alt.is_empty()).all(|alt| {
        let bare = alt.trim_start_matches(['^', '~', '=', 'v', ' ']);
        let major = bare.split('.').next().and_then(|m| m.parse::<u32>().ok());
        major.is_some_and(|m| m < 2) || (alt.starts_with('>') && alt.contains("<2"))
    })
}

fn composer_lock(text: &str, target: PhpVersion) -> Vec<Hit> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(text) else { return Vec::new() };
    let mut out = Vec::new();
    let packages = ["packages", "packages-dev"].iter().filter_map(|key| json.get(key)?.as_array()).flatten();
    for package in packages.filter(|p| p["type"] == "composer-plugin") {
        let (Some(name), Some(api)) = (package["name"].as_str(), package.pointer("/require/composer-plugin-api").and_then(|v| v.as_str()))
        else {
            continue;
        };
        if composer1_only(api) {
            out.push(Hit {
                line: line_of(text, &format!("\"name\": \"{name}\"")),
                rule: "server.composer_plugin_api",
                title: Localized::same(format!("{name} (composer-plugin-api {api})")),
                since: target,
                severity: Severity::Behavior,
                message: OLD_PLUGIN,
                replacement: Some(Localized::same(format!("composer update {name}"))),
            });
        }
    }
    if json.get("plugin-api-version").is_none() && json.get("packages").is_some() && json.get("content-hash").is_some() {
        out.push(Hit {
            line: 1,
            rule: "server.composer1_lock",
            title: Localized { tr: "Composer 1 kilit dosyası".into(), en: "Composer 1 lock file".into() },
            since: target,
            severity: Severity::Behavior,
            message: COMPOSER1_LOCK,
            replacement: Some(Localized::same("composer update (Composer 2)")),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excluded_folders_are_left_out_of_server_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("old/.ferry")).unwrap();
        std::fs::create_dir_all(root.join(".ferry")).unwrap();
        std::fs::write(root.join("old/Dockerfile"), "FROM php:7.4\n").unwrap();
        std::fs::write(root.join("Dockerfile"), "FROM php:7.4\n").unwrap();
        let before = scan(root, PhpVersion::new(8, 4)).0.len();
        std::fs::write(root.join(".ferry/config.json"), r#"{"exclude": ["old/"]}"#).unwrap();
        assert_eq!(scan(root, PhpVersion::new(8, 4)).0.len(), before - 1);
    }

    #[test]
    fn matrix_extras_and_comments_are_not_pins() {
        let target = PhpVersion::new(8, 4);
        let yaml = "jobs:\n  t:\n    strategy:\n      matrix:\n        php: ['8.1', '8.2'] # 8.5 soon\n        include:\n          - php: '8.2'\n            coverage: true\n        exclude:\n          - php: '8.1'\n    steps:\n      - run: x\n";
        let lines: Vec<usize> = stale_lines(yaml, Kind::Ci, target).into_iter().map(|(line, _)| line).collect();
        assert_eq!(lines, [5]);
        let spans = &stale_lines(yaml, Kind::Ci, target)[0].1;
        assert_eq!(spans.len(), 2, "the 8.5 in the comment isn't a version");
        let only_include = "matrix:\n  include:\n    - php: 7.4\n";
        assert_eq!(stale_lines(only_include, Kind::Ci, target).len(), 1);
        let job_on_target = "matrix:\n  php: ['8.1']\n  include:\n    - php: '8.5'\n";
        assert!(stale_lines(job_on_target, Kind::Ci, target).is_empty());
        assert_eq!(without_comment("php: '8.1#x' # note"), "php: '8.1#x' ");
    }

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

    fn write_all(root: &Path, files: &[(&str, &str)]) {
        for (path, text) in files {
            let p = root.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        }
    }

    #[test]
    fn composer_platform_extensions_and_old_plugins() {
        let dir = tempfile::tempdir().unwrap();
        write_all(
            dir.path(),
            &[
                (
                    "composer.json",
                    "{\n  \"config\": {\n    \"platform\": {\n      \"ext-intl\": \"7.4.33\",\n      \"ext-gd\": \"8.4.0\",\n      \"ext-custom\": \"12.0\"\n    }\n  }\n}\n",
                ),
                (
                    "composer.lock",
                    "{\n  \"plugin-api-version\": \"2.6.0\",\n  \"content-hash\": \"a\",\n  \"packages\": [\n    {\n      \"name\": \"old/plugin\",\n      \"type\": \"composer-plugin\",\n      \"require\": {\"composer-plugin-api\": \"^1.0\"}\n    },\n    {\n      \"name\": \"new/plugin\",\n      \"type\": \"composer-plugin\",\n      \"require\": {\"composer-plugin-api\": \"^1.1 || ^2.0\"}\n    }\n  ],\n  \"packages-dev\": [\n    {\n      \"name\": \"dev/plugin\",\n      \"type\": \"composer-plugin\",\n      \"require\": {\"composer-plugin-api\": \">=1.0 <2.0\"}\n    },\n    {\n      \"name\": \"any/plugin\",\n      \"type\": \"composer-plugin\",\n      \"require\": {\"composer-plugin-api\": \"*\"}\n    }\n  ]\n}\n",
                ),
            ],
        );
        let found = ids(&scan(dir.path(), PhpVersion::new(8, 4)).0);
        let expected = [
            ("composer.json", 4, "server.composer_platform_ext"),
            ("composer.lock", 6, "server.composer_plugin_api"),
            ("composer.lock", 18, "server.composer_plugin_api"),
        ];
        assert_eq!(found, expected.map(|(f, l, r)| (f.to_string(), l, r.to_string())));
    }

    #[test]
    fn composer_pins_change_only_the_platform_value() {
        let target = PhpVersion::new(8, 4);
        for (before, after) in [
            (
                r#"{"name":"acme2/x","require":{"php":"^7.4"},"config":{"platform":{"php":"7.4.33"}}}"#,
                r#"{"name":"acme2/x","require":{"php":"^7.4"},"config":{"platform":{"php":"8.4.0"}}}"#,
            ),
            (
                r#"{"keywords":["platform"],"require":{"php":">=7.4"},"config":{"platform":{"ext-redis2":"5.3","php":"7.4"}}}"#,
                r#"{"keywords":["platform"],"require":{"php":">=7.4"},"config":{"platform":{"ext-redis2":"5.3","php":"8.4"}}}"#,
            ),
            (
                "{\n  \"config\": {\n    \"platform\": {\n      \"php\": \"7.4.33\"\n    }\n  }\n}\n",
                "{\n  \"config\": {\n    \"platform\": {\n      \"php\": \"8.4.0\"\n    }\n  }\n}\n",
            ),
        ] {
            assert_eq!(bumped_composer(before, target).as_deref(), Some(after));
        }
        assert_eq!(composer_platform(r#"{"keywords":["platform","php"]}"#), None);
        let line = composer_platform(
            "{\n  \"keywords\": [\"platform\"],\n  \"config\": {\n    \"platform\": {\n      \"php\": \"7.4\"\n    }\n  }\n}",
        )
        .unwrap()
        .0;
        assert_eq!(line, 5);
    }

    #[test]
    fn hosting_files_and_fpm_sockets_are_pins_too() {
        let dir = tempfile::tempdir().unwrap();
        write_all(
            dir.path(),
            &[
                (".ddev/config.yaml", "name: site\nphp_version: \"7.4\"\n"),
                (".lando.yml", "config:\n  php: '7.4'\nservices:\n  app:\n    type: php:8.0\n"),
                (".platform.app.yaml", "name: app\ntype: 'php:8.1'\n"),
                (".upsun/config.yaml", "applications:\n  app:\n    type: 'php:8.2'\n"),
                ("docker/nginx/site.conf", "# php7.0-fpm.sock\nfastcgi_pass unix:/run/php/php7.4-fpm.sock;\n"),
                ("Caddyfile", "php_fastcgi unix//run/php/php8.1-fpm.sock\n"),
                ("other.conf", "listen = /run/php/php8.4-fpm.sock\nserver_name php7.example;\n"),
            ],
        );
        let found = scan(dir.path(), PhpVersion::new(8, 4)).0;
        let pins: Vec<_> = ids(&found).into_iter().map(|(f, l, _)| (f, l)).collect();
        let expected = [
            (".ddev/config.yaml", 2),
            (".lando.yml", 2),
            (".lando.yml", 5),
            (".platform.app.yaml", 2),
            (".upsun/config.yaml", 3),
            ("Caddyfile", 1),
            ("docker/nginx/site.conf", 2),
        ];
        assert_eq!(pins, expected.map(|(f, l)| (f.to_string(), l)));
    }

    #[test]
    fn pins_in_lists_containers_and_build_arguments() {
        let dir = tempfile::tempdir().unwrap();
        write_all(
            dir.path(),
            &[
                (".travis.yml", "language: php\nphp:\n  - 7.1\n  - 7.2\nscript: phpunit\n"),
                (".github/workflows/a.yml", "jobs:\n  c:\n    container: php:7.1-cli\n"),
                (".github/workflows/b.yml", "jobs:\n  c:\n    steps:\n      - run: |\n          docker run php:7.0-cli php -v\n"),
                (".gitlab-ci.yml", "job:\n  image:\n    name: php:7.3-cli\n"),
                ("Dockerfile", "ARG PHP_VERSION=7.3\nFROM php:${PHP_VERSION}-cli\n"),
                ("docker-compose.yml", "services:\n  app:\n    build:\n      args:\n        PHP_VERSION: \"7.4\"\n  b:\n    build:\n      args:\n        - PHP_VERSION=7.2\n"),
            ],
        );
        let target = PhpVersion::new(8, 4);
        let pins: Vec<_> = ids(&scan(dir.path(), target).0).into_iter().map(|(f, l, _)| (f, l)).collect();
        let expected = [
            (".github/workflows/a.yml", 3),
            (".github/workflows/b.yml", 5),
            (".gitlab-ci.yml", 3),
            (".travis.yml", 4),
            ("Dockerfile", 1),
            ("docker-compose.yml", 5),
            ("docker-compose.yml", 9),
        ];
        assert_eq!(pins, expected.map(|(f, l)| (f.to_string(), l)));
        let after = bump(dir.path(), target);
        let text = |name: &str| after.changes.iter().find(|c| c.file.replace('\\', "/") == name).unwrap().new_content.clone();
        assert_eq!(text("Dockerfile"), "ARG PHP_VERSION=8.4\nFROM php:${PHP_VERSION}-cli\n");
        assert!(text(".travis.yml").contains("  - 7.1\n  - 8.4\n"));
        assert!(text("docker-compose.yml").contains("PHP_VERSION: \"8.4\"") && text("docker-compose.yml").contains("- PHP_VERSION=8.4"));
        assert!(text(".github/workflows/b.yml").contains("docker run php:8.4-cli"));
    }

    #[test]
    fn pins_move_to_the_target() {
        let dir = tempfile::tempdir().unwrap();
        write_all(
            dir.path(),
            &[
                (".php-version", "8.2.10\n"),
                (".tool-versions", "nodejs 20.1.0\nphp 8.2\n"),
                ("Dockerfile", "FROM php:7.4-fpm-alpine AS app\r\nFROM node:20\r\n"),
                ("Dockerfile.locked", "FROM php:7.4@sha256:abc\n"),
                ("docker-compose.yml", "services:\n  app:\n    image: wordpress:php7.4.33-apache\n"),
                (
                    ".github/workflows/ci.yml",
                    "jobs:\n  a:\n    php: ['7.4', '8.0']\n  b:\n    php-versions: \"7.4, 8.0\"\n  c:\n    php-version: 7.3\n",
                ),
                (".github/workflows/done.yml", "php: '8.4'\nother:\n  php: 7.4\n"),
                ("composer.json", "{\n  \"config\": {\n    \"platform\": {\n      \"php\": \"7.4.33\"\n    }\n  }\n}\n"),
                (".ddev/config.yaml", "php_version: \"7.4\"\n"),
                ("nginx.conf", "fastcgi_pass unix:/run/php/php7.4-fpm.sock;\n"),
            ],
        );
        let preview = bump(dir.path(), PhpVersion::new(8, 4));
        let after: std::collections::HashMap<_, _> =
            preview.changes.iter().map(|c| (c.file.replace('\\', "/"), (c.new_content.as_str(), c.rules[0].class.as_str()))).collect();
        assert_eq!(after.len(), 8, "{:?}", after.keys());
        assert_eq!(after[".php-version"], ("8.4.0\n", "server.php_version_exact"));
        assert_eq!(after[".tool-versions"], ("nodejs 20.1.0\nphp 8.4\n", "server.php_version_pin"));
        assert_eq!(after["Dockerfile"].0, "FROM php:8.4-fpm-alpine AS app\r\nFROM node:20\r\n");
        assert_eq!(after["docker-compose.yml"].0, "services:\n  app:\n    image: wordpress:php8.4-apache\n");
        assert_eq!(
            after[".github/workflows/ci.yml"].0,
            "jobs:\n  a:\n    php: ['7.4', '8.0', '8.4']\n  b:\n    php-versions: \"7.4, 8.0, 8.4\"\n  c:\n    php-version: 7.3\n"
        );
        assert_eq!(
            after["composer.json"],
            ("{\n  \"config\": {\n    \"platform\": {\n      \"php\": \"8.4.0\"\n    }\n  }\n}\n", "server.composer_platform")
        );
        assert_eq!(after[".ddev/config.yaml"].0, "php_version: \"8.4\"\n");
        assert_eq!(after["nginx.conf"].0, "fastcgi_pass unix:/run/php/php8.4-fpm.sock;\n");
    }

    #[test]
    fn a_bumped_project_has_no_pins_left() {
        let dir = tempfile::tempdir().unwrap();
        write_all(
            dir.path(),
            &[
                (".php-version", "8.2.10\n"),
                ("Dockerfile", "FROM php:7.4-cli\n"),
                (".github/workflows/ci.yml", "php: [7.4, 8.0]\n"),
                (".lando.yml", "services:\n  app:\n    type: php:8.0\n"),
                ("composer.json", "{\"config\":{\"platform\":{\"php\":\"7.4\"}}}"),
            ],
        );
        let target = PhpVersion::new(8, 4);
        for change in bump(dir.path(), target).changes {
            std::fs::write(dir.path().join(&change.file), &change.new_content).unwrap();
        }
        assert_eq!(ids(&scan(dir.path(), target).0), Vec::new());
        assert!(bump(dir.path(), target).changes.is_empty());
    }

    #[test]
    fn config_files_are_found_at_any_depth() {
        let dir = tempfile::tempdir().unwrap();
        let deep = dir.path().join("a/b/c/d/e/f/g/h/i");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(deep.join("php.ini"), "mbstring.func_overload=1\n").unwrap();
        let (findings, errors) = scan(dir.path(), PhpVersion::new(8, 5));
        assert!(errors.is_empty());
        assert!(findings.iter().any(|f| f.file.ends_with("php.ini")));
    }

    #[test]
    fn oversized_config_files_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".htaccess"), "#".repeat(3 * 1024 * 1024)).unwrap();
        let (_, errors) = scan(dir.path(), PhpVersion::new(8, 4));
        assert_eq!(errors.iter().map(|e| e.file.as_str()).collect::<Vec<_>>(), [".htaccess"]);
    }
}
