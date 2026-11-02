use crate::analyzer::Finding;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ignore {
    pub rule_id: String,
    pub file: String,
    pub snippet: String,
    pub reason: String,
    #[serde(default)]
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoredFinding {
    pub finding: Finding,
    pub reason: String,
    pub inline: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct IgnoreFile {
    #[serde(default)]
    ignores: Vec<Ignore>,
}

pub fn path(root: &Path) -> PathBuf {
    root.join(".ferry").join("ignore.json")
}

pub fn load(root: &Path) -> Vec<Ignore> {
    std::fs::read(path(root)).ok().and_then(|b| serde_json::from_slice::<IgnoreFile>(&b).ok()).map(|f| f.ignores).unwrap_or_default()
}

pub fn save(root: &Path, ignores: &[Ignore]) -> std::io::Result<()> {
    let file = path(root);
    std::fs::create_dir_all(file.parent().unwrap())?;
    let mut json = serde_json::to_vec_pretty(&IgnoreFile { ignores: ignores.to_vec() }).map_err(std::io::Error::other)?;
    json.push(b'\n');
    let mut tmp = tempfile::NamedTempFile::new_in(file.parent().unwrap())?;
    std::io::Write::write_all(&mut tmp, &json)?;
    tmp.persist(file).map(|_| ()).map_err(|e| e.error)
}

impl Ignore {
    pub fn for_finding(finding: &Finding, reason: &str, created_at: u64) -> Self {
        Self {
            rule_id: finding.rule_id.clone(),
            file: finding.file.clone(),
            snippet: finding.snippet.trim().to_string(),
            reason: reason.trim().to_string(),
            created_at,
        }
    }

    pub fn matches(&self, finding: &Finding) -> bool {
        self.rule_id == finding.rule_id && self.file == finding.file && self.snippet == finding.snippet.trim()
    }
}

pub fn split(findings: Vec<Finding>, ignores: &[Ignore]) -> (Vec<Finding>, Vec<IgnoredFinding>) {
    let mut kept = Vec::new();
    let mut ignored = Vec::new();
    for f in findings {
        match ignores.iter().find(|i| i.matches(&f)) {
            Some(i) => ignored.push(IgnoredFinding { reason: i.reason.clone(), finding: f, inline: false }),
            None => kept.push(f),
        }
    }
    (kept, ignored)
}

pub struct Applied {
    pub kept: Vec<Finding>,
    pub ignored: Vec<IgnoredFinding>,
    pub stale: Vec<Ignore>,
}

fn directive(comment: &str, rule_id: &str) -> Option<String> {
    let at = comment.find("ferry-ignore")?;
    let rest = comment[at + "ferry-ignore".len()..].trim_start_matches(':').trim();
    let rest = rest.split("*/").next().unwrap_or_default().split("-->").next().unwrap_or_default().trim();
    let (first, tail) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let names_rule = first.contains('.') || first.ends_with('*');
    if names_rule {
        let matches = first.strip_suffix('*').map_or(first == rule_id, |prefix| rule_id.starts_with(prefix));
        matches.then(|| tail.trim().to_string())
    } else {
        Some(rest.to_string())
    }
}

struct Source {
    lines: Vec<String>,
    trimmed: std::collections::HashSet<String>,
    comments: std::collections::HashMap<usize, Vec<String>>,
}

impl Source {
    fn read(path: &Path) -> Option<Self> {
        let text = String::from_utf8_lossy(&std::fs::read(path).ok()?).into_owned();
        let lexed = crate::lexer::lex(&text);
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        let line_of = |pos: usize| starts.partition_point(|&s| s <= pos);
        let mut comments: std::collections::HashMap<usize, Vec<String>> = std::collections::HashMap::new();
        let mut add = |start: usize, end: usize| {
            let body = &text[start..end];
            if body.contains("ferry-ignore") {
                for line in line_of(start)..=line_of(end.saturating_sub(1).max(start)) {
                    comments.entry(line).or_default().push(body.to_string());
                }
            }
        };
        for &(start, end) in &lexed.comments {
            add(start, end);
        }
        for &(start, end) in &lexed.html {
            let mut from = start;
            while let Some(open) = text[from..end].find("<!--") {
                let open = from + open;
                let close = text[open..end].find("-->").map_or(end, |c| open + c + 3);
                add(open, close);
                from = close;
            }
        }
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        let trimmed = lines.iter().map(|l| l.trim().to_string()).collect();
        Some(Self { lines, trimmed, comments })
    }

    fn inline_reason(&self, line: usize, rule_id: &str) -> Option<String> {
        let on = |l: usize| self.comments.get(&l).and_then(|cs| cs.iter().find_map(|c| directive(c, rule_id)));
        on(line).or_else(|| line.checked_sub(1).filter(|&l| l > 0).and_then(on))
    }

    fn has_snippet(&self, snippet: &str) -> bool {
        match snippet.strip_suffix('…') {
            Some(prefix) => self.lines.iter().any(|l| l.trim().starts_with(prefix)),
            None => self.trimmed.contains(snippet),
        }
    }
}

pub fn apply(root: &Path, findings: Vec<Finding>, ignores: &[Ignore]) -> Applied {
    let mut sources: std::collections::HashMap<String, Option<Source>> = std::collections::HashMap::new();
    let (mut kept, mut ignored) = (Vec::new(), Vec::new());
    for f in findings {
        if let Some(i) = ignores.iter().find(|i| i.matches(&f)) {
            ignored.push(IgnoredFinding { reason: i.reason.clone(), finding: f, inline: false });
            continue;
        }
        let source = sources.entry(f.file.clone()).or_insert_with(|| Source::read(&root.join(&f.file)));
        match source.as_ref().and_then(|s| s.inline_reason(f.line, &f.rule_id)) {
            Some(reason) => ignored.push(IgnoredFinding { reason, finding: f, inline: true }),
            None => kept.push(f),
        }
    }
    let stale = ignores
        .iter()
        .filter(|i| {
            let source = sources.entry(i.file.clone()).or_insert_with(|| Source::read(&root.join(&i.file)));
            !source.as_ref().is_some_and(|s| s.has_snippet(&i.snippet))
        })
        .cloned()
        .collect();
    Applied { kept, ignored, stale }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::analyze_source;

    #[test]
    fn ignores_survive_line_moves_and_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let before = analyze_source("a.php", "<?php\neach($a);\neach($b);", "8.0".parse().unwrap());
        save(dir.path(), &[Ignore::for_finding(&before[0], " vendored copy ", 1)]).unwrap();
        let ignores = load(dir.path());
        assert_eq!(ignores[0].reason, "vendored copy");
        let after = analyze_source("a.php", "<?php\n\n\neach($a);\neach($b);", "8.0".parse().unwrap());
        let (kept, ignored) = split(after, &ignores);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].line, 5);
        assert_eq!((ignored[0].finding.line, ignored[0].reason.as_str()), (4, "vendored copy"));
        assert!(load(&dir.path().join("missing")).is_empty());
    }

    #[test]
    fn inline_comments_and_stale_entries() {
        let dir = tempfile::tempdir().unwrap();
        let src = "<?php\n// ferry-ignore removed.each only runs on the PHP 5 server\neach($a);\neach($b); // ferry-ignore\n/* ferry-ignore removed.* legacy */ each($c);\n// ferry-ignore deprecated.utf8_encode wrong rule\neach($d);\n";
        std::fs::write(dir.path().join("a.php"), src).unwrap();
        let findings = analyze_source("a.php", src, "8.2".parse().unwrap());
        let gone = Ignore {
            rule_id: "removed.each".into(),
            file: "a.php".into(),
            snippet: "each($old);".into(),
            reason: "x".into(),
            created_at: 0,
        };
        let missing = Ignore { file: "deleted.php".into(), ..gone.clone() };
        let applied = apply(dir.path(), findings, &[gone.clone(), missing.clone()]);
        let reasons: Vec<(usize, &str)> = applied.ignored.iter().map(|i| (i.finding.line, i.reason.as_str())).collect();
        assert_eq!(reasons, [(3, "only runs on the PHP 5 server"), (4, ""), (5, "legacy")]);
        assert_eq!(applied.kept.iter().map(|f| f.line).collect::<Vec<_>>(), [7]);
        assert_eq!(applied.stale, [gone, missing]);
    }

    #[test]
    fn only_real_comments_are_directives() {
        let dir = tempfile::tempdir().unwrap();
        let src = "<?php\n$text = 'ferry-ignore'; each($a);\n$t = \"// ferry-ignore\";\neach($b);\n/*\n * ferry-ignore removed.each block\n */ each($c);\n?>\n<!-- ferry-ignore removed.each template -->\n<?php each($d); ?>\n";
        std::fs::write(dir.path().join("a.php"), src).unwrap();
        let findings = analyze_source("a.php", src, "8.2".parse().unwrap());
        let applied = apply(dir.path(), findings, &[]);
        assert_eq!(applied.kept.iter().map(|f| f.line).collect::<Vec<_>>(), [2, 4]);
        let reasons: Vec<(usize, &str, bool)> = applied.ignored.iter().map(|i| (i.finding.line, i.reason.as_str(), i.inline)).collect();
        assert_eq!(reasons, [(7, "block", true), (10, "template", true)]);
    }

    #[test]
    fn long_lines_keep_their_ignore() {
        let dir = tempfile::tempdir().unwrap();
        let src = format!("<?php\neach($a); // {}\n", "x".repeat(300));
        std::fs::write(dir.path().join("a.php"), &src).unwrap();
        let findings = analyze_source("a.php", &src, "8.2".parse().unwrap());
        assert!(findings[0].snippet.ends_with('…'));
        let ignores = [Ignore::for_finding(&findings[0], "kept", 0)];
        let applied = apply(dir.path(), findings, &ignores);
        assert_eq!((applied.ignored.len(), applied.stale.len()), (1, 0));
        assert!(!applied.ignored[0].inline);
    }

    #[test]
    fn many_findings_in_one_file_stay_fast() {
        let dir = tempfile::tempdir().unwrap();
        let src = format!("<?php\n{}", "each($a);\n".repeat(4000));
        std::fs::write(dir.path().join("a.php"), &src).unwrap();
        let findings = analyze_source("a.php", &src, "8.2".parse().unwrap());
        let ignores: Vec<Ignore> =
            (0..200).map(|n| Ignore { snippet: format!("gone{n}();"), ..Ignore::for_finding(&findings[0], "x", 0) }).collect();
        let started = std::time::Instant::now();
        let applied = apply(dir.path(), findings, &ignores);
        assert_eq!((applied.kept.len(), applied.stale.len()), (4000, 200));
        assert!(started.elapsed() < std::time::Duration::from_secs(2), "{:?}", started.elapsed());
    }
}
