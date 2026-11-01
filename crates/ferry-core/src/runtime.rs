use crate::analyzer::Finding;
use crate::i18n::Localized;
use crate::rules::{FixKind, Severity};
use crate::version::PhpVersion;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
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

fn level_of(text: &str) -> Option<Level> {
    let t = text.to_ascii_lowercase();
    if t.contains("deprecat") {
        Some(Level::Deprecated)
    } else if t.contains("fatal error") || t.contains("uncaught") || t.contains("parse error") {
        Some(Level::Fatal)
    } else if t.contains("warning") {
        Some(Level::Warning)
    } else {
        None
    }
}

fn split_location(rest: &str) -> Option<(String, String, usize)> {
    let Some(at) = rest.rfind(" on line ") else {
        let at_in = rest.rfind(" in ")?;
        let (path, line) = rest[at_in + 4..].trim().rsplit_once(':')?;
        return Some((rest[..at_in].trim().to_string(), path.to_string(), line.trim().parse().ok()?));
    };
    let line: usize = rest[at + 9..].trim().trim_end_matches(|c: char| !c.is_ascii_digit()).parse().ok()?;
    let head = &rest[..at];
    let at_in = head.rfind(" in ")?;
    Some((head[..at_in].trim().to_string(), head[at_in + 4..].trim().to_string(), line))
}

pub fn parse(log: &str) -> Vec<Event> {
    let mut found: BTreeMap<(String, usize, String), (Level, usize)> = BTreeMap::new();
    let mut add = |path: String, line: usize, level: Level, message: String| {
        found.entry((path, line, message)).and_modify(|e| e.1 += 1).or_insert((level, 1));
    };
    let lines: Vec<&str> = log.lines().collect();
    let mut section = Level::Deprecated;
    for (k, raw) in lines.iter().enumerate() {
        let text = raw.trim();
        if text.ends_with(':') && (text.contains(" triggered ") || text.starts_with("There w")) {
            section = level_of(text).unwrap_or(Level::Warning);
            continue;
        }
        let body = text.find("PHP ").map_or(text, |i| &text[i + 4..]);
        if let Some(colon) = body.find(':') {
            if let Some(level) = level_of(&body[..colon]) {
                if let Some((message, path, line)) = split_location(&body[colon + 1..]) {
                    add(path, line, level, message);
                    continue;
                }
            }
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

fn relative(root: &Path, path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|p| !p.is_empty()).collect();
    (0..parts.len()).map(|k| parts[k..].join("/")).find(|rel| root.join(rel).is_file())
}

pub struct Merged {
    pub confirmed: usize,
    pub added: usize,
    pub outside: usize,
}

pub fn merge(root: &Path, findings: &mut Vec<Finding>, events: &[Event], target: PhpVersion) -> Merged {
    let mut merged = Merged { confirmed: 0, added: 0, outside: 0 };
    for e in events {
        let skipped = |rel: &str| rel.split('/').any(|part| crate::project::ALWAYS_SKIP.contains(&part));
        let Some(file) = relative(root, &e.path).filter(|rel| !skipped(rel)) else {
            merged.outside += 1;
            continue;
        };
        let close = |f: &Finding| {
            let kind = match e.level {
                Level::Fatal => Severity::Breaking,
                Level::Warning => Severity::Behavior,
                Level::Deprecated => Severity::Deprecated,
            };
            f.line == e.line || (f.line.abs_diff(e.line) == 1 && f.severity == kind)
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
        if let Some(f) = findings.iter_mut().find(|f| f.file == file && f.line == e.line && f.rule_id == rule) {
            f.runtime_hits += e.count;
            continue;
        }
        let snippet = std::fs::read_to_string(root.join(&file))
            .ok()
            .and_then(|t| t.lines().nth(e.line.saturating_sub(1)).map(|l| l.trim().chars().take(200).collect()))
            .unwrap_or_default();
        findings.push(Finding {
            id: format!("{file}:{}:1:{rule}", e.line),
            file,
            line: e.line,
            column: 1,
            rule_id: rule.to_string(),
            title: Localized::same(e.message.chars().take(120).collect::<String>()),
            message: Localized {
                tr: format!("Bu mesaj çalışma anı log'unda {} kez görüldü; statik tarama burada bir şey bulmadı.", e.count),
                en: format!("This message appeared {} times in the runtime log; the static scan found nothing here.", e.count),
            },
            since: target,
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
        let merged = merge(dir.path(), &mut findings, &events, "8.2".parse().unwrap());
        assert_eq!((merged.confirmed, merged.added, merged.outside), (1, 4, 1));
        let each = findings.iter().find(|f| f.rule_id == "removed.each").unwrap();
        assert_eq!(each.runtime_hits, 1);
        let warning = findings.iter().find(|f| f.rule_id == "runtime.warning" && f.line == 4).unwrap();
        assert_eq!((warning.line, warning.snippet.as_str()), (4, "echo $row['id'];"));
    }
}
