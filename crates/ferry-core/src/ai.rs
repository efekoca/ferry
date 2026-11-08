//! Suggestions for findings Ferry can't fix on its own. Only the code around a finding leaves the
//! computer, masked first, and a suggestion becomes an ordinary change: previewed, backed up and
//! linted before anything is written.

use crate::analyzer::Finding;
use crate::rector::{AppliedRule, FileChange};
use crate::rules::FixKind;
use crate::version::PhpVersion;
use serde::Serialize;
use std::path::Path;
use std::sync::atomic::AtomicBool;

/// Lines on each side of a finding: enough for the surrounding function in most code.
const AROUND: usize = 20;

const SYSTEM: &str = "You fix PHP code for an upgrade to a newer PHP version. Change only what the problem needs, \
and keep the behavior, style and indentation. A value shown as [redacted] was hidden before sending; leave any \
line that holds one exactly as it is.";

/// Exactly what goes to a provider, so it can be shown before anything is sent.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub system: String,
    pub user: String,
    pub file: String,
    pub first_line: usize,
    pub last_line: usize,
    #[serde(skip)]
    original: String,
    #[serde(skip)]
    masked: Vec<(usize, String)>,
}

/// Anything that turns a prompt into a reply: a cloud model with the user's key, or a local one.
pub trait Provider: Send + Sync {
    fn name(&self) -> String;
    fn complete(&self, prompt: &Prompt, cancel: &AtomicBool) -> Result<String, String>;
}

fn line_break(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

pub fn prompt(root: &Path, finding: &Finding, target: PhpVersion) -> Result<Prompt, String> {
    if finding.rule_id.starts_with("framework.") {
        return Err(crate::t!(
            "Bu bulgu kodla değil, framework ya da CMS güncellenerek çözülür.",
            "This finding is solved by updating the framework or CMS, not the code."
        ));
    }
    if !crate::project::is_php_file(Path::new(&finding.file)) {
        return Err(crate::t!("AI önerileri yalnızca PHP dosyaları için.", "AI suggestions are for PHP files only."));
    }
    let path = crate::safefs::resolve(root, &finding.file).map_err(|e| e.to_string())?;
    let bytes = crate::safefs::read_bytes(&path).map_err(|e| e.to_string())?;
    let original = String::from_utf8(bytes)
        .map_err(|_| crate::t!("Dosya UTF-8 değil, bu yüzden AI'a gönderilmedi.", "The file isn't UTF-8, so it wasn't sent to AI."))?;
    let lines: Vec<&str> = original.lines().collect();
    if finding.line == 0 || finding.line > lines.len() {
        return Err(crate::t!(
            "Bulgunun satırı dosyada yok; projeyi yeniden tarayın.",
            "The finding's line isn't in the file; scan again."
        ));
    }
    let first = finding.line.saturating_sub(AROUND).max(1);
    let last = (finding.line + AROUND).min(lines.len());
    let mut masked = Vec::new();
    let excerpt: Vec<String> = (first..=last)
        .map(|n| {
            let line = lines[n - 1];
            let shown = crate::redact::text(line);
            if shown != line {
                masked.push((n, shown.clone()));
            }
            shown
        })
        .collect();
    let hint = finding.replacement.as_ref().map_or(String::new(), |r| format!("\nSuggested direction: {}", r.en));
    let user = format!(
        "Ferry found this problem in a project moving to PHP {target}:\n{}: {}{hint}\n\nFile {}, lines {first}-{last}. \
         The problem is on line {}.\n```php\n{}\n```\n\nReply with lines {first}-{last} corrected, as one ```php block and \
         nothing else. Keep every line that doesn't need to change exactly as it is.",
        finding.title.en,
        finding.message.en,
        finding.file,
        finding.line,
        excerpt.join("\n"),
    );
    Ok(Prompt { system: SYSTEM.to_string(), user, file: finding.file.clone(), first_line: first, last_line: last, original, masked })
}

fn code_block(reply: &str) -> Option<&str> {
    let open = reply.find("```")?;
    let body = &reply[open + 3..];
    let body = &body[body.find('\n')? + 1..];
    let close = body.rfind("```")?;
    Some(body[..close].strip_suffix('\n').unwrap_or(&body[..close]))
}

/// The reply as a change to the file. A line the model was shown masked has to come back
/// unchanged, and then gets its real value back.
pub fn suggestion(finding: &Finding, prompt: &Prompt, reply: &str) -> Result<FileChange, String> {
    let code =
        code_block(reply).ok_or_else(|| crate::t!("Yanıtta kod bulunamadı.", "The reply had no code in it."))?.trim_end_matches('\r');
    let lines: Vec<&str> = prompt.original.lines().collect();
    let window = prompt.last_line - prompt.first_line + 1;
    let mut replaced = Vec::new();
    for line in code.lines() {
        let line = line.trim_end_matches('\r');
        if line.contains(crate::redact::MASK) {
            let Some(&(n, _)) = prompt.masked.iter().find(|(_, shown)| shown == line) else {
                return Err(crate::t!(
                    "Öneri gizlenmiş bir değeri içeren satırı değiştirdi; uygulanmadı.",
                    "The suggestion changed a line that held a hidden value, so it wasn't used."
                ));
            };
            replaced.push(lines[n - 1].to_string());
        } else {
            replaced.push(line.to_string());
        }
    }
    if replaced.len() > window * 3 + 20 {
        return Err(crate::t!(
            "Öneri beklenenden çok uzun; uygulanmadı.",
            "The suggestion was much longer than expected, so it wasn't used."
        ));
    }
    let mut out: Vec<String> = lines[..prompt.first_line - 1].iter().map(|l| l.to_string()).collect();
    out.extend(replaced);
    out.extend(lines[prompt.last_line..].iter().map(|l| l.to_string()));
    let newline = line_break(&prompt.original);
    let mut new_content = out.join(newline);
    if prompt.original.ends_with('\n') {
        new_content.push_str(newline);
    }
    if new_content == prompt.original {
        return Err(crate::t!("Öneri kodu değiştirmedi.", "The suggestion didn't change the code."));
    }
    let rule = AppliedRule {
        class: "ai".to_string(),
        name: finding.rule_id.clone(),
        title: finding.title.clone(),
        since: Some(finding.since),
        fix: FixKind::Manual,
    };
    Ok(FileChange {
        file: finding.file.clone(),
        rules: vec![rule],
        fix: FixKind::Manual,
        diff: crate::diff::compute(&prompt.original, &new_content, 3),
        original_hash: crate::snapshot::fnv1a(prompt.original.as_bytes()),
        new_content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(root: &Path, src: &str) -> Finding {
        std::fs::write(root.join("a.php"), src).unwrap();
        crate::analyzer::analyze_source("a.php", src, PhpVersion::new(8, 4)).into_iter().find(|f| f.rule_id == "removed.each").unwrap()
    }

    #[test]
    fn only_the_lines_around_a_finding_are_sent_and_secrets_are_masked() {
        let dir = tempfile::tempdir().unwrap();
        let mut src = String::from("<?php\n");
        for n in 0..60 {
            src.push_str(&format!("$v{n} = {n};\n"));
        }
        src.push_str("$password = 'hunter2';\nwhile (list($k, $v) = each($a)) {}\n");
        let f = finding(dir.path(), &src);
        let p = prompt(dir.path(), &f, PhpVersion::new(8, 4)).unwrap();
        assert_eq!((p.first_line, p.last_line), (f.line - AROUND, f.line));
        assert!(!p.user.contains("$v0 = 0;") && p.user.contains("each($a)"));
        assert!(!p.user.contains("hunter2") && p.user.contains("[redacted]"));
    }

    #[test]
    fn a_reply_becomes_a_change_that_keeps_hidden_values_and_line_endings() {
        let dir = tempfile::tempdir().unwrap();
        let src = "<?php\r\n$password = 'hunter2';\r\nwhile (list($k, $v) = each($a)) {}\r\necho 1;\r\n";
        let f = finding(dir.path(), src);
        let p = prompt(dir.path(), &f, PhpVersion::new(8, 4)).unwrap();
        let reply = "Here:\n```php\n<?php\n$password = '[redacted]';\nforeach ($a as $k => $v) {}\necho 1;\n```";
        let change = suggestion(&f, &p, reply).unwrap();
        assert_eq!(change.new_content, "<?php\r\n$password = 'hunter2';\r\nforeach ($a as $k => $v) {}\r\necho 1;\r\n");
        assert_eq!(change.original_hash, crate::snapshot::fnv1a(src.as_bytes()));
        assert_eq!(change.rules[0].name, "removed.each");
    }

    #[test]
    fn a_reply_that_touches_a_hidden_value_or_says_nothing_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let f = finding(dir.path(), "<?php\n$password = 'hunter2';\nwhile (list($k, $v) = each($a)) {}\n");
        let p = prompt(dir.path(), &f, PhpVersion::new(8, 4)).unwrap();
        let changed = "```php\n<?php\n$password = '[redacted]x';\nforeach ($a as $k => $v) {}\n```";
        assert!(suggestion(&f, &p, changed).is_err());
        assert!(suggestion(&f, &p, "No code, sorry.").is_err());
        let same = "```php\n<?php\n$password = '[redacted]';\nwhile (list($k, $v) = each($a)) {}\n```";
        assert!(suggestion(&f, &p, same).is_err());
    }

    #[test]
    fn files_that_arent_php_or_utf8_are_never_sent() {
        let dir = tempfile::tempdir().unwrap();
        let mut f = finding(dir.path(), "<?php\nwhile (list($k, $v) = each($a)) {}\n");
        std::fs::write(dir.path().join("a.php"), b"<?php\n\xff each($a);\n").unwrap();
        assert!(prompt(dir.path(), &f, PhpVersion::new(8, 4)).is_err());
        f.file = ".env".into();
        std::fs::write(dir.path().join(".env"), "DB_PASSWORD=x\n").unwrap();
        assert!(prompt(dir.path(), &f, PhpVersion::new(8, 4)).is_err());
    }
}
