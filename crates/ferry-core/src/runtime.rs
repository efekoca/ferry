use crate::analyzer::Finding;
use crate::i18n::Localized;
use crate::rules::{FixKind, Severity};
use crate::version::PhpVersion;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Fatal,
    Warning,
    Deprecated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub path: String,
    pub line: usize,
    pub level: Level,
    pub message: String,
    pub count: usize,
}

fn throwable(lower: &str) -> bool {
    lower.split(':').next().is_some_and(|head| head.ends_with("error") && !head.contains(' '))
}

fn level_of(text: &str) -> Option<Level> {
    let t = text.to_ascii_lowercase();
    if t.contains("fatal error") || t.contains("uncaught") || t.contains("parse error") {
        Some(Level::Fatal)
    } else if t.contains("deprecat") {
        Some(Level::Deprecated)
    } else if throwable(&t) {
        Some(Level::Fatal)
    } else if t.contains("warning") {
        Some(Level::Warning)
    } else {
        None
    }
}

/// PHP 8 messages can name other places first, as in `called in a.php on line 3 and defined in
/// a.php:2`, so the location that counts is the one that ends the line.
fn split_location(rest: &str) -> Option<(String, String, usize)> {
    let rest = rest.trim();
    if let Some(at) = rest.rfind(" in ") {
        if let Some((path, line)) = rest[at + 4..].trim().rsplit_once(':') {
            if let (Ok(line), false) = (line.parse(), path.contains(" on line ")) {
                return Some((rest[..at].trim().to_string(), path.to_string(), line));
            }
        }
    }
    let at = rest.rfind(" on line ")?;
    let digits: String = rest[at + 9..].trim_start().chars().take_while(char::is_ascii_digit).collect();
    let line = digits.parse().ok()?;
    let head = &rest[..at];
    let at_in = head.rfind(" in ")?;
    Some((head[..at_in].trim().to_string(), head[at_in + 4..].trim().to_string(), line))
}

/// A record from Monolog's JsonFormatter. Records about the application itself, such as a failed
/// login, have no PHP error in them and are left out.
fn monolog_record(text: &str) -> Option<(String, usize, Level, String)> {
    let record: serde_json::Value = serde_json::from_str(text).ok()?;
    record.get("level_name").or(record.get("level"))?;
    let exception = record.pointer("/context/exception");
    let message = exception.and_then(|e| e["message"].as_str()).or(record["message"].as_str())?.to_string();
    let class = exception.and_then(|e| e["class"].as_str());
    let code = exception.and_then(|e| e["code"].as_i64());
    let level = match code {
        Some(8192 | 16384) if class == Some("ErrorException") => Some(Level::Deprecated),
        Some(2 | 512) if class == Some("ErrorException") => Some(Level::Warning),
        _ => record["message"].as_str().and_then(level_of).or_else(|| level_of(&format!("{}: {message}", class.unwrap_or_default()))),
    }?;
    if let Some((path, line)) = exception.and_then(|e| e["file"].as_str()?.rsplit_once(':')) {
        return Some((path.to_string(), line.parse().ok()?, level, message));
    }
    ["/context", "/extra"].iter().find_map(|key| {
        let place = record.pointer(key)?;
        Some((place["file"].as_str()?.to_string(), place["line"].as_u64()? as usize, level, message.clone()))
    })
}

/// What servers add after the PHP message: nginx's request details, Apache's referer and its
/// `\\n` for the stack trace PHP wrote on the same line.
fn without_server_tail(text: &str) -> &str {
    let end = ["\" while ", "\", client: ", ", client: ", ", referer: ", "\\nStack trace:"].iter().filter_map(|m| text.find(m)).min();
    text[..end.unwrap_or(text.len())].trim_end_matches(['"', ' '])
}

fn php_line(text: &str) -> Option<(String, usize, Level, String)> {
    let text = without_server_tail(text);
    let body = text.find("PHP ").map_or(text, |i| &text[i + 4..]);
    let colon = body.find(':')?;
    let level = level_of(&body[..colon])?;
    let (message, path, line) = split_location(&body[colon + 1..])?;
    Some((path, line, level, message))
}

/// `[2024-01-02 10:00:00] production.ERROR: message {"exception":"[object] (Class(code: 0): text at /a.php:3)`.
fn laravel_line(text: &str) -> Option<(String, usize, Level, String)> {
    let rest = text.strip_prefix('[')?.split_once("] ")?.1;
    let (channel, rest) = rest.split_once(": ")?;
    let (_, severity) = channel.split_once('.')?;
    if !["DEBUG", "INFO", "NOTICE", "WARNING", "ERROR", "CRITICAL", "ALERT", "EMERGENCY"].contains(&severity) {
        return None;
    }
    let Some((_, object)) = rest.split_once("[object] (") else {
        let level = level_of(rest)?;
        let (message, path, line) = split_location(rest)?;
        return Some((path, line, level, message));
    };
    let (class, after) = object.split_once("(code: ")?;
    let inner = after.split_once("): ")?.1;
    let at = inner.find(".php:").and_then(|dot| inner[..dot].rfind(" at "))?;
    let (message, location) = (&inner[..at], &inner[at + 4..]);
    let (path, line) = location.split_once(')')?.0.rsplit_once(':')?;
    let level = if class == "ErrorException" { Level::Warning } else { level_of(&format!("{class}: {message}"))? };
    Some((path.to_string(), line.parse().ok()?, level, message.to_string()))
}

/// True on a heading of the Symfony PHPUnit bridge's summary that lists deprecations in the
/// project's own reach. Indirect and legacy ones come from packages or are silenced on purpose.
fn bridge_heading(text: &str) -> Option<bool> {
    let start = text.find(|c: char| c.is_ascii_uppercase())?;
    let rest = &text[start..];
    let (head, _) = rest.split_once(" deprecation notices (")?;
    Some(matches!(head, "Remaining self" | "Remaining direct" | "Other" | "Unsilenced"))
}

fn bridge_notice(text: &str) -> Option<(usize, &str)> {
    let (count, message) = text.split_once("x: ")?;
    Some((count.parse().ok()?, message.trim()))
}

pub fn parse(log: &str) -> Vec<Event> {
    let mut found: BTreeMap<(String, usize, String), (Level, usize)> = BTreeMap::new();
    let mut add_many = |path: String, line: usize, level: Level, message: String, count: usize| {
        found.entry((path, line, crate::redact::text(&message))).and_modify(|e| e.1 += count).or_insert((level, count));
    };
    let lines: Vec<&str> = log.lines().collect();
    let mut section = Level::Deprecated;
    let mut bridge = None;
    for (k, raw) in lines.iter().enumerate() {
        let text = raw.trim();
        if text.starts_with('{') {
            if let Some((path, line, level, message)) = monolog_record(text) {
                add_many(path, line, level, message, 1);
                continue;
            }
        }
        if let Some((path, line, level, message)) = laravel_line(text) {
            add_many(path, line, level, message, 1);
            continue;
        }
        if text.contains("PHP message: ") {
            let parts: Vec<_> = text.split("PHP message: ").skip(1).filter_map(php_line).collect();
            if !parts.is_empty() {
                parts.into_iter().for_each(|(path, line, level, message)| add_many(path, line, level, message, 1));
                continue;
            }
        }
        if let Some(reachable) = bridge_heading(text) {
            bridge = Some(reachable);
            continue;
        }
        if let (Some(reachable), Some((count, message))) = (bridge, bridge_notice(text)) {
            if reachable {
                add_many(String::new(), 0, Level::Deprecated, message.to_string(), count);
            }
            continue;
        }
        let mut add = |path: String, line: usize, level: Level, message: String| add_many(path, line, level, message, 1);
        if text.ends_with(':') && (text.contains(" triggered ") || text.starts_with("There w")) {
            section = level_of(text).unwrap_or(Level::Warning);
            continue;
        }
        if let Some((path, line, level, message)) = php_line(text) {
            add(path, line, level, message);
            continue;
        }
        if let Some(location) = text.strip_prefix("Location:") {
            let message = k.checked_sub(1).and_then(|p| lines[p].trim().strip_prefix("Message:")).map(str::trim);
            if let (Some(message), Some((path, line))) = (message, location.trim().rsplit_once(':')) {
                if let (Some(level), Ok(line)) = (level_of(message), line.parse::<usize>()) {
                    add(path.to_string(), line, level, message.to_string());
                }
            }
            continue;
        }
        let bare = text
            .rsplit_once(':')
            .filter(|(p, n)| p.starts_with('/') && p.ends_with(".php") && !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
        if let Some((path, line)) = bare {
            let previous = (0..k).rev().map(|p| lines[p].trim()).find(|l| !l.is_empty());
            let first_frame = previous.is_some_and(|p| !p.ends_with(|c: char| c.is_ascii_digit()) || !p.contains(".php:"));
            if let (true, Some(message)) = (first_frame, previous) {
                if let Some(level) = level_of(message) {
                    add(path.to_string(), line.parse().unwrap_or(0), level, message.to_string());
                }
            }
            continue;
        }
        let numbered = text.split_once(") ").filter(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
        if let Some((_, location)) = numbered {
            let Some((path, line)) = location.rsplit_once(':') else { continue };
            let Ok(line) = line.trim().parse::<usize>() else { continue };
            if !path.ends_with(".php") && !path.ends_with(".inc") {
                continue;
            }
            let message = lines.get(k + 1).map(|m| m.trim().to_string()).unwrap_or_default();
            let level = section;
            add(path.trim().to_string(), line, level, message);
        }
    }
    found.into_iter().map(|((path, line, message), (level, count))| Event { path, line, level, message, count }).collect()
}

struct Located {
    prefix: String,
    rel: String,
}

fn path_parts(path: &str) -> Vec<String> {
    path.replace('\\', "/").split('/').filter(|p| !p.is_empty() && *p != ".").map(str::to_string).collect()
}

/// Splits the project roots someone typed for a log's server or container.
pub fn parse_roots(text: &str) -> Vec<String> {
    text.split([',', '\n']).map(str::trim).filter(|r| !path_parts(r).is_empty()).map(str::to_string).collect()
}

fn under_root(root: &Path, path: &str, remote_roots: &[String]) -> Option<String> {
    let parts = path_parts(path);
    if parts.iter().any(|p| p == "..") {
        return None;
    }
    remote_roots.iter().find_map(|remote| {
        let remote = path_parts(remote);
        let drive = remote[0].ends_with(':');
        let same = |(a, b): (&String, &String)| a == b || drive && a.eq_ignore_ascii_case(b);
        if parts.len() <= remote.len() || !parts.iter().zip(&remote).all(same) {
            return None;
        }
        let rel = parts[remote.len()..].join("/");
        crate::safefs::resolve(root, &rel).is_ok_and(|p| p.is_file()).then_some(rel)
    })
}

fn locate(root: &Path, path: &str) -> Option<Located> {
    let parts = path_parts(path);
    let parts: Vec<&str> = parts.iter().map(String::as_str).collect();
    if parts.contains(&"..") {
        return None;
    }
    let k = (0..parts.len()).find(|&k| crate::safefs::resolve(root, &parts[k..].join("/")).is_ok_and(|p| p.is_file()))?;
    Some(Located { prefix: parts[..k].join("/").to_ascii_lowercase(), rel: parts[k..].join("/") })
}

fn project_files(root: &Path, events: &[Event], remote_roots: &[String]) -> Vec<Option<String>> {
    if !remote_roots.is_empty() {
        return events.iter().map(|e| under_root(root, &e.path, remote_roots)).collect();
    }
    let found: Vec<Option<Located>> = events.iter().map(|e| locate(root, &e.path)).collect();
    let mut files_by_prefix: std::collections::HashMap<&str, std::collections::HashSet<&str>> = std::collections::HashMap::new();
    for l in found.iter().flatten() {
        files_by_prefix.entry(l.prefix.as_str()).or_default().insert(l.rel.as_str());
    }
    let strong: std::collections::HashSet<&str> = files_by_prefix
        .iter()
        .filter(|(_, files)| files.len() > 1 || files.iter().any(|f| f.contains('/')))
        .map(|(prefix, _)| *prefix)
        .collect();
    let folder = root.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    found
        .iter()
        .map(|l| {
            let l = l.as_ref()?;
            let trusted = l.rel.contains('/') || strong.contains(l.prefix.as_str()) || l.prefix.rsplit('/').next() == Some(folder.as_str());
            trusted.then(|| l.rel.clone())
        })
        .collect()
}

fn symbol(f: &Finding) -> Option<String> {
    let name = f.title.en.split_once('(')?.0.trim();
    let valid = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | ':' | '\\'));
    valid.then(|| name.rsplit('\\').next().unwrap_or(name).to_ascii_lowercase())
}

fn names_symbol(message: &str, name: &str) -> bool {
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    message
        .match_indices(name)
        .any(|(at, _)| !message[..at].chars().next_back().is_some_and(word) && !message[at + name.len()..].chars().next().is_some_and(word))
}

pub struct Merged {
    pub confirmed: usize,
    pub added: usize,
    pub outside: usize,
    pub outside_fatals: usize,
}

/// With `remote_roots`, only paths under one of those roots count as part of the project.
pub fn merge(root: &Path, findings: &mut Vec<Finding>, events: &[Event], target: PhpVersion, remote_roots: &[String]) -> Merged {
    merge_from(root, findings, events, target, remote_roots, None)
}

/// The PHP version PHPUnit printed in its header, when the log has one.
pub fn php_version(log: &str) -> Option<PhpVersion> {
    log.lines().find_map(|line| line.trim().strip_prefix("Runtime:")?.trim().strip_prefix("PHP ")?.parse().ok())
}

/// With `log_php`, messages that come from a newer PHP than the target say nothing about the
/// target: they count as outside, and the rest carry the version that printed them.
pub fn merge_from(
    root: &Path,
    findings: &mut Vec<Finding>,
    events: &[Event],
    target: PhpVersion,
    remote_roots: &[String],
    log_php: Option<PhpVersion>,
) -> Merged {
    if log_php.is_some_and(|php| php > target) {
        let outside_fatals = events.iter().filter(|e| e.level == Level::Fatal).count();
        return Merged { confirmed: 0, added: 0, outside: events.len(), outside_fatals };
    }
    let mut merged = Merged { confirmed: 0, added: 0, outside: 0, outside_fatals: 0 };
    let files = project_files(root, events, remote_roots);
    for (e, file) in events.iter().zip(files) {
        if e.path.is_empty() {
            let message = e.message.to_ascii_lowercase();
            let mut same = findings.iter_mut().filter(|f| symbol(f).is_some_and(|s| names_symbol(&message, &s))).peekable();
            if same.peek().is_none() {
                merged.outside += 1;
            } else {
                merged.confirmed += 1;
            }
            same.for_each(|f| f.runtime_hits += e.count);
            continue;
        }
        let skipped = |rel: &str| rel.split('/').any(|part| crate::project::ALWAYS_SKIP.contains(&part));
        let Some(file) = file.filter(|rel| !skipped(rel)) else {
            merged.outside += 1;
            merged.outside_fatals += usize::from(e.level == Level::Fatal);
            continue;
        };
        let message = e.message.to_ascii_lowercase();
        let close = |f: &Finding| {
            let compatible = match e.level {
                Level::Fatal => f.severity == Severity::Breaking,
                Level::Warning => matches!(f.severity, Severity::Behavior | Severity::Breaking),
                Level::Deprecated => matches!(f.severity, Severity::Deprecated | Severity::Breaking),
            };
            let named = symbol(f).map(|s| names_symbol(&message, &s));
            compatible
                && match f.line.abs_diff(e.line) {
                    0 => named == Some(true) || (named.is_none() && e.level != Level::Fatal),
                    1 => named == Some(true),
                    _ => false,
                }
        };
        if let Some(f) = findings.iter_mut().find(|f| f.file == file && close(f) && !f.rule_id.starts_with("runtime.")) {
            f.runtime_hits += e.count;
            merged.confirmed += 1;
            continue;
        }
        let (rule, severity) = match e.level {
            Level::Fatal => ("runtime.fatal", Severity::Breaking),
            Level::Warning => ("runtime.warning", Severity::Behavior),
            Level::Deprecated => ("runtime.deprecated", Severity::Deprecated),
        };
        let title = e.message.chars().take(120).collect::<String>();
        if let Some(f) = findings.iter_mut().find(|f| f.file == file && f.line == e.line && f.rule_id == rule && f.title.en == title) {
            f.runtime_hits += e.count;
            continue;
        }
        let snippet = std::fs::read_to_string(root.join(&file))
            .ok()
            .and_then(|t| t.lines().nth(e.line.saturating_sub(1)).map(crate::analyzer::clip_snippet))
            .unwrap_or_default();
        let id = format!("{file}:{}:1:{rule}", e.line);
        let taken = findings.iter().any(|f| f.id == id);
        findings.push(Finding {
            id: if taken { format!("{id}:{:x}", crate::snapshot::fnv1a(title.as_bytes())) } else { id },
            file,
            line: e.line,
            column: 1,
            rule_id: rule.to_string(),
            title: Localized::same(title),
            message: Localized {
                tr: format!("Bu mesaj çalışma anı log'unda {} kez görüldü; statik tarama burada bir şey bulmadı.", e.count),
                en: format!("This message appeared {} times in the runtime log; the static scan found nothing here.", e.count),
            },
            since: log_php.unwrap_or(target),
            severity,
            fix: FixKind::Manual,
            replacement: None,
            snippet,
            runtime_hits: e.count,
        });
        merged.added += 1;
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_log_formats_keep_every_message() {
        let nginx = r#"2024/01/02 10:00:00 [error] 12#12: *1 FastCGI sent in stderr: "PHP message: PHP Warning:  Undefined variable $x in /app/a.php on line 3PHP message: PHP Fatal error:  Uncaught Error: nope in /app/b.php:9" while reading response header from upstream, client: 10.0.0.1, server: in x, request: "GET / HTTP/1.1""#;
        let events = parse(nginx);
        let places: Vec<_> = events.iter().map(|e| (e.path.as_str(), e.line, e.level)).collect();
        assert_eq!(places, [("/app/a.php", 3, Level::Warning), ("/app/b.php", 9, Level::Fatal)]);

        let fpm = r#"[02-Jan-2024 10:00:00] WARNING: [pool www] child 12 said into stderr: "NOTICE: PHP message: PHP Deprecated:  old() is deprecated in /app/a.php on line 4""#;
        assert_eq!(parse(fpm)[0].line, 4);

        let apache = "[Tue Jan 02 10:00:00.1 2024] [php:warn] [pid 1] [client 10.0.0.1:5] PHP Warning:  Undefined variable $x in /app/a.php:5, referer: http://x.test/in in/";
        assert_eq!(parse(apache)[0].line, 5);
        let trace = r"[php:error] [client 1.1.1.1:5] PHP Fatal error:  Uncaught Exception: boom in /app/a.php:6\nStack trace:\n#0 {main}\n  thrown in /app/a.php on line 6, referer: http://x.test/";
        let event = &parse(trace)[0];
        assert_eq!((event.line, event.message.as_str()), (6, "Uncaught Exception: boom"));
    }

    #[test]
    fn laravel_logs_are_read() {
        let warning = r#"[2024-01-02 10:00:00] production.ERROR: Undefined variable $x {"userId":1,"exception":"[object] (ErrorException(code: 0): Undefined variable $x at /app/Foo.php:12)
[stacktrace]
#0 /app/vendor/x.php(1): y()"}"#;
        let events = parse(warning);
        assert_eq!(events.len(), 1);
        assert_eq!((events[0].path.as_str(), events[0].line, events[0].level), ("/app/Foo.php", 12, Level::Warning));

        let fatal = r#"[2024-01-02 10:00:00] local.ERROR: Bad {"exception":"[object] (TypeError(code: 0): strlen(): Argument #1 must be string at /app/Bar.php:7)"#;
        assert_eq!(parse(fatal)[0].level, Level::Fatal);
        let app =
            r#"[2024-01-02 10:00:00] local.ERROR: Nope {"exception":"[object] (App\\Exceptions\\Denied(code: 0): Nope at /app/Baz.php:7)"#;
        assert!(parse(app).is_empty());
        assert_eq!(parse("[2024-01-02 10:00:00] local.WARNING: old() is deprecated in /app/a.php on line 2")[0].level, Level::Deprecated);
    }

    #[test]
    fn messages_lose_personal_details() {
        let events = parse("PHP Warning:  Mail to ada@example.com failed in /app/a.php on line 3");
        assert_eq!(events[0].message, "Mail to [redacted] failed");
    }
    use crate::analyzer::analyze_source;

    const LOG: &str = "[01-Nov-2026 10:00:00 UTC] PHP Deprecated:  Creation of dynamic property Cart::$total is deprecated in /var/www/site/src/Cart.php on line 3
[01-Nov-2026 10:00:01 UTC] PHP Deprecated:  Creation of dynamic property Cart::$total is deprecated in /var/www/site/src/Cart.php on line 3
PHP Warning:  Undefined array key \"id\" in /var/www/site/src/Cart.php on line 4
PHP Deprecated:  strftime() is deprecated in /var/www/site/vendor/x/y.php on line 9
PHP Fatal error:  Uncaught TypeError: count(): Argument #1 must be of type Countable|array, null given in /home/me/site/src/Cart.php:5
1 test triggered 1 PHP deprecation:

1) /home/runner/work/site/src/Cart.php:2
each() is deprecated

1 test triggered 1 PHP warning:

1) /home/runner/work/site/src/Cart.php:3
Undefined array key \"total\"

";

    #[test]
    fn phpunit_9_and_bootstrap_errors() {
        let log = "1) OAuthTest::testOptions\nMethod ReflectionProperty::setAccessible() is deprecated since 8.5\n\n/app/test/OAuth/OAuthTest.php:39\n/app/vendor/phpunit/phpunit/src/Framework/TestCase.php:1200\n\nAn error occurred inside PHPUnit.\n\nMessage:  Tests\\Base::wrap(): Implicitly marking parameter $dt as nullable is deprecated\nLocation: /app/tests/AbstractTestCase.php:228\n\n2) FooTest::testBar\nFailed asserting that false is true.\n\n/app/tests/FooTest.php:12\n";
        let events: Vec<(String, usize, Level)> = parse(log).into_iter().map(|e| (e.path, e.line, e.level)).collect();
        assert_eq!(
            events,
            [
                ("/app/test/OAuth/OAuthTest.php".to_string(), 39, Level::Deprecated),
                ("/app/tests/AbstractTestCase.php".to_string(), 228, Level::Deprecated)
            ]
        );
    }

    #[test]
    fn parses_logs_and_merges_with_findings() {
        let events = parse(LOG);
        let mut summary: Vec<_> =
            events.iter().map(|e| (e.path.rsplit('/').next().unwrap().to_string(), e.line, e.level, e.count)).collect();
        summary.sort();
        assert_eq!(
            summary,
            [
                ("Cart.php".to_string(), 2, Level::Deprecated, 1),
                ("Cart.php".to_string(), 3, Level::Warning, 1),
                ("Cart.php".to_string(), 3, Level::Deprecated, 2),
                ("Cart.php".to_string(), 4, Level::Warning, 1),
                ("Cart.php".to_string(), 5, Level::Fatal, 1),
                ("y.php".to_string(), 9, Level::Deprecated, 1),
            ]
        );
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        let src = "<?php\neach($a);\n$cart->total = 1;\necho $row['id'];\n";
        std::fs::write(dir.path().join("src/Cart.php"), src).unwrap();
        std::fs::create_dir_all(dir.path().join("vendor/x")).unwrap();
        std::fs::write(dir.path().join("vendor/x/y.php"), "<?php\n").unwrap();
        let mut findings = analyze_source("src/Cart.php", src, "8.2".parse().unwrap());
        let merged = merge(dir.path(), &mut findings, &events, "8.2".parse().unwrap(), &[]);
        assert_eq!((merged.confirmed, merged.added, merged.outside), (1, 4, 1));
        let each = findings.iter().find(|f| f.rule_id == "removed.each").unwrap();
        assert_eq!(each.runtime_hits, 1);
        let warning = findings.iter().find(|f| f.rule_id == "runtime.warning" && f.line == 4).unwrap();
        assert_eq!((warning.line, warning.snippet.as_str()), (4, "echo $row['id'];"));
    }

    #[test]
    fn unrelated_runtime_events_stay_separate() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("app");
        std::fs::create_dir(&root).unwrap();
        let src = "<?php utf8_encode($s);\n$x = 1;\n";
        std::fs::write(root.join("a.php"), src).unwrap();
        let target = "8.4".parse().unwrap();
        let log = "PHP Fatal error:  Uncaught RuntimeException: independent failure in /srv/app/a.php on line 1
PHP Deprecated:  Creation of dynamic property A::$b is deprecated in /srv/app/a.php on line 1
PHP Deprecated:  Function utf8_encode() is deprecated in /srv/app/a.php on line 2
";
        let mut findings = analyze_source("a.php", src, target);
        assert_eq!(findings.len(), 1);
        let merged = merge(&root, &mut findings, &parse(log), target, &[]);
        assert_eq!((merged.confirmed, merged.added), (1, 2));
        assert_eq!(findings.iter().find(|f| f.rule_id == "deprecated.utf8_encode").unwrap().runtime_hits, 1);
        let fatal = findings.iter().find(|f| f.rule_id == "runtime.fatal").expect("a fatal is never folded into a deprecation");
        assert_eq!((fatal.line, fatal.severity), (1, Severity::Breaking));
        assert!(findings.iter().any(|f| f.rule_id == "runtime.deprecated" && f.line == 1));
    }

    #[test]
    fn php_8_messages_that_name_two_places() {
        let log = "PHP Fatal error:  Uncaught TypeError: f(): Argument #1 ($a) must be of type int, string given, called in /srv/app/a.php on line 3 and defined in /srv/app/a.php:2\n\
            PHP Fatal error:  Uncaught ArgumentCountError: Too few arguments to function g(), 1 passed in /srv/app/b.php on line 5 and exactly 2 expected in /srv/app/b.php:7\n\
            2026/11/06 10:00:00 [error] 1#1: *1 FastCGI sent in stderr: \"PHP message: PHP Warning:  Undefined variable $x in /srv/app/c.php on line 4\" while reading response\n\
            [Fri Nov 06 10:00:00 2026] [php:error] PHP Warning:  Undefined array key 1 in /srv/app/d.php on line 9, referer: http://example.com/page/2\n";
        let mut found: Vec<_> = parse(log).into_iter().map(|e| (e.path, e.line, e.level)).collect();
        found.sort();
        assert_eq!(
            found,
            [
                ("/srv/app/a.php".to_string(), 2, Level::Fatal),
                ("/srv/app/b.php".to_string(), 7, Level::Fatal),
                ("/srv/app/c.php".to_string(), 4, Level::Warning),
                ("/srv/app/d.php".to_string(), 9, Level::Warning),
            ]
        );
    }

    #[test]
    fn a_fatal_needs_a_finding_that_names_it() {
        let target = PhpVersion::new(8, 4);
        let mut findings = analyze_source("a.php", "<?php\necho $s{0}; run();\n", target);
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.php"), "<?php\necho $s{0}; run();\n").unwrap();
        findings[0].file = "src/a.php".into();
        let events = parse("PHP Fatal error:  Uncaught RuntimeException: boom in /srv/app/src/a.php:2\n");
        let merged = merge(dir.path(), &mut findings, &events, target, &["/srv/app".to_string()]);
        assert_eq!((merged.confirmed, merged.added), (0, 1), "{findings:?}");
        assert!(findings.iter().any(|f| f.rule_id == "runtime.fatal"));
    }

    #[test]
    fn a_critical_record_about_deprecation_is_still_fatal() {
        let log = r#"{"message":"Uncaught PHP Exception RuntimeException: \"call to deprecated endpoint\"","context":{"exception":{"class":"RuntimeException","message":"call to deprecated endpoint","code":0,"file":"/a/b.php:3"}},"level":500,"level_name":"CRITICAL"}"#;
        assert_eq!(parse(log)[0].level, Level::Fatal);
    }

    #[test]
    fn a_log_from_a_newer_php_says_nothing_about_the_target() {
        let log = "PHPUnit 12.0.0 by Sebastian Bergmann and contributors.\n\nRuntime:       PHP 8.5.12\n\nPHP Deprecated:  old in /srv/app/src/a.php on line 2\n";
        assert_eq!(php_version(log), Some(PhpVersion::new(8, 5)));
        let events = parse(log);
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.php"), "<?php\nf();\n").unwrap();
        let roots = ["/srv/app".to_string()];
        let mut findings = Vec::new();
        let merged = merge_from(dir.path(), &mut findings, &events, PhpVersion::new(8, 2), &roots, php_version(log));
        assert_eq!((merged.added, merged.outside, findings.len()), (0, 1, 0));
        let merged = merge_from(dir.path(), &mut findings, &events, PhpVersion::new(8, 5), &roots, php_version(log));
        assert_eq!((merged.added, findings[0].since), (1, PhpVersion::new(8, 5)));
        let later = merge_from(dir.path(), &mut Vec::new(), &events, PhpVersion::new(8, 5), &roots, Some(PhpVersion::new(8, 4)));
        assert_eq!(later.added, 1);
    }

    #[test]
    fn monolog_json_records() {
        let log = [
            r#"{"message":"Deprecated: strftime() is deprecated","context":{"exception":{"class":"ErrorException","message":"Deprecated: strftime() is deprecated","code":8192,"file":"/srv/app/src/A.php:12"}},"level":200,"level_name":"INFO","channel":"php","datetime":"2026-11-05T10:00:00+00:00","extra":{}}"#,
            r#"{"message":"Uncaught PHP Exception TypeError: bad","context":{"exception":{"class":"TypeError","message":"bad","code":0,"file":"/srv/app/src/B.php:3"}},"level":500,"level_name":"CRITICAL","channel":"request","extra":{}}"#,
            r#"{"message":"Warning: Undefined variable $x","context":{},"level":300,"level_name":"WARNING","channel":"php","extra":{"file":"/srv/app/src/C.php","line":8}}"#,
            r#"{"message":"Payment declined","context":{"exception":{"class":"App\PaymentFailed","message":"Payment declined","code":0,"file":"/srv/app/src/D.php:1"}},"level":400,"level_name":"ERROR","channel":"app","extra":{}}"#,
            r#"{"hello":"world"}"#,
        ]
        .join("\n");
        let events = parse(&log);
        let summary: Vec<_> = events.iter().map(|e| (e.path.as_str(), e.line, e.level)).collect();
        assert_eq!(
            summary,
            [
                ("/srv/app/src/A.php", 12, Level::Deprecated),
                ("/srv/app/src/B.php", 3, Level::Fatal),
                ("/srv/app/src/C.php", 8, Level::Warning)
            ]
        );
    }

    #[test]
    fn bridge_summaries_confirm_findings_by_name() {
        let log = "OK (3 tests)\n\n\u{1b}[37;41mRemaining self deprecation notices (3)\u{1b}[0m\n\n  2x: Function strftime() is deprecated\n    2x in ATest::testOne from App\\Tests\n\n  1x: Something this project never heard of\n    1x in ATest::testTwo from App\\Tests\n\nRemaining indirect deprecation notices (1)\n\n  4x: Function utf8_encode() is deprecated\n    4x in BTest::test from App\\Tests\n\nLegacy deprecation notices (1)\n\n  1x: Old thing\n";
        let events = parse(log);
        let notices: Vec<_> = events.iter().map(|e| (e.path.as_str(), e.message.as_str(), e.count)).collect();
        assert_eq!(notices, [("", "Function strftime() is deprecated", 2), ("", "Something this project never heard of", 1)]);

        let target = PhpVersion::new(8, 4);
        let mut findings = analyze_source("a.php", "<?php\nstrftime('%Y');\n", target);
        let dir = tempfile::tempdir().unwrap();
        let merged = merge(dir.path(), &mut findings, &events, target, &[]);
        assert_eq!((merged.confirmed, merged.added, merged.outside), (1, 0, 1));
        assert_eq!(findings.iter().find(|f| f.title.en.contains("strftime")).unwrap().runtime_hits, 2);
    }

    #[test]
    fn phpunit_errors_count_as_fatal() {
        let log = "1) OldTest::testFirst\nError: Call to undefined function each()\n\n/app/src/Old.php:3\n/app/tests/OldTest.php:4\n\n2) T::x\nTypeError: count() bad\n\n/app/a.php:9\n\n3) T::y\nFailed asserting that false is true.\n\n/app/b.php:1\n";
        let events = parse(log);
        let found: Vec<_> = events.iter().map(|e| (e.path.as_str(), e.line, e.level)).collect();
        assert_eq!(found, [("/app/a.php", 9, Level::Fatal), ("/app/src/Old.php", 3, Level::Fatal)]);
    }

    #[test]
    fn remote_roots_decide_what_belongs_to_the_project() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("shop");
        std::fs::create_dir_all(root.join("src")).unwrap();
        for f in ["a.php", "src/c.php"] {
            std::fs::write(root.join(f), "<?php\n$x = 1;\n").unwrap();
        }
        let warn = |path: &str| format!("PHP Warning:  Undefined variable $y in {path} on line 2\n");
        let log = [
            "/var/www/site/a.php",
            "/other/src/c.php",
            "/var/www/site2/a.php",
            "/var/www/site/missing.php",
            "C:\\inetpub\\site\\src\\c.php",
        ]
        .map(warn)
        .concat();
        let events = parse(&log);
        let mut guessed = Vec::new();
        assert_eq!(merge(&root, &mut guessed, &events, "8.4".parse().unwrap(), &[]).outside, 3);
        let mut findings = Vec::new();
        let roots = parse_roots(" /var/www/site/ , c:/INETPUB/site\n,/ ");
        assert_eq!(roots, ["/var/www/site/", "c:/INETPUB/site"]);
        let merged = merge(&root, &mut findings, &events, "8.4".parse().unwrap(), &roots);
        assert_eq!((merged.added, merged.outside), (2, 3));
        let mut files: Vec<&str> = findings.iter().map(|f| f.file.as_str()).collect();
        files.sort();
        assert_eq!(files, ["a.php", "src/c.php"]);
    }

    #[test]
    fn events_from_another_project_stay_outside() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("shop");
        std::fs::create_dir_all(root.join("src")).unwrap();
        for f in ["a.php", "b.php", "src/c.php"] {
            std::fs::write(root.join(f), "<?php\n$x = 1;\n").unwrap();
        }
        let target = "8.4".parse().unwrap();
        let merged_with = |log: &str| {
            let mut findings = Vec::new();
            let merged = merge(&root, &mut findings, &parse(log), target, &[]);
            (merged.added, merged.outside, findings.into_iter().map(|f| f.file).collect::<Vec<_>>())
        };
        let warn = |path: &str| format!("PHP Warning:  Undefined variable $y in {path} on line 2\n");
        assert_eq!(merged_with(&warn("/other-project/a.php")), (0, 1, vec![]));
        assert_eq!(merged_with(&warn("/srv/shop/a.php")), (1, 0, vec!["a.php".to_string()]));
        assert_eq!(merged_with(&format!("{}{}", warn("/srv/www/a.php"), warn("/srv/www/b.php"))).0, 2);
        assert_eq!(merged_with(&format!("{}{}", warn("/srv/www/src/c.php"), warn("/srv/www/a.php"))).0, 2);
        assert_eq!(
            merged_with(&format!("{}{}", warn("/srv/www/src/c.php"), warn("/elsewhere/a.php"))),
            (1, 1, vec!["src/c.php".to_string()])
        );
        assert_eq!(merged_with(&warn("/srv/shop/../shop/a.php")).1, 1);
        assert_eq!(merged_with(&warn("C:\\inetpub\\shop\\src\\c.php")).0, 1);
    }

    #[test]
    fn a_fatal_outside_the_project_is_counted() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("mysite");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("index.php"), "<?php\necho 1;\n").unwrap();
        let events = parse("PHP Fatal error:  Boom in /var/www/html/index.php on line 2\n");
        let merged = merge(&root, &mut Vec::new(), &events, PhpVersion::new(8, 5), &[]);
        assert_eq!((merged.outside, merged.outside_fatals, merged.added), (1, 1, 0));
    }

    #[test]
    fn a_symbol_only_matches_whole_words() {
        assert!(!names_symbol("reached the limit", "each"));
        assert!(names_symbol("call to undefined function each()", "each"));
    }

    #[test]
    fn different_messages_on_a_line_stay_apart() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir(root.join("src")).unwrap();
        std::fs::write(root.join("src/a.php"), format!("<?php\n{}\n", "x".repeat(300))).unwrap();
        let events = parse("PHP Warning:  First in /w/src/a.php on line 2\nPHP Warning:  Second in /w/src/a.php on line 2\n");
        let mut findings = Vec::new();
        merge(root, &mut findings, &events, PhpVersion::new(8, 5), &[]);
        assert_eq!(findings.len(), 2);
        assert_ne!(findings[0].id, findings[1].id);
        assert!(findings[0].snippet.ends_with('…'));
    }
}
