use crate::analyzer::Finding;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ignore {
    pub rule_id: String,
    pub file: String,
    pub snippet: String,
    /// Only on a shortened snippet, so long lines that start alike stay apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_hash: Option<String>,
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
    crate::safefs::read_bytes(path(root))
        .ok()
        .and_then(|b| serde_json::from_slice::<IgnoreFile>(&b).ok())
        .map(|f| f.ignores)
        .unwrap_or_default()
}

/// For anything that's about to rewrite the file: a broken file must not turn into an empty list,
/// or the next save would wipe what the team shares in it.
pub fn load_for_update(root: &Path) -> Result<Vec<Ignore>, String> {
    let bytes = match crate::safefs::read_bytes(path(root)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(crate::t!(
                ".ferry/ignore.json okunamadı, önce düzeltin: {e}",
                "Couldn't read .ferry/ignore.json, fix it first: {e}"
            ))
        }
    };
    serde_json::from_slice::<IgnoreFile>(&bytes)
        .map(|f| f.ignores)
        .map_err(|e| crate::t!(".ferry/ignore.json okunamadı, önce düzeltin: {e}", "Couldn't read .ferry/ignore.json, fix it first: {e}"))
}

pub fn save(root: &Path, ignores: &[Ignore]) -> std::io::Result<()> {
    crate::safefs::resolve(root, ".ferry/ignore.json")?;
    let file = path(root);
    std::fs::create_dir_all(file.parent().unwrap())?;
    let mut json = serde_json::to_vec_pretty(&IgnoreFile { ignores: ignores.to_vec() }).map_err(std::io::Error::other)?;
    json.push(b'\n');
    let mut tmp = tempfile::NamedTempFile::new_in(file.parent().unwrap())?;
    std::io::Write::write_all(&mut tmp, &json)?;
    crate::safefs::persist(tmp, &file)
}

fn hash_line(line: &str) -> String {
    format!("{:016x}", crate::snapshot::fnv1a(line.as_bytes()))
}

fn clean_line(line: &str) -> &str {
    line.trim_start_matches('\u{feff}').trim()
}

fn line_at(root: &Path, finding: &Finding) -> Option<String> {
    let bytes = crate::safefs::read_bytes(root.join(&finding.file)).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    text.lines().nth(finding.line.checked_sub(1)?).map(|line| clean_line(line).to_string())
}

impl Ignore {
    pub fn for_finding(root: &Path, finding: &Finding, reason: &str, created_at: u64) -> Self {
        let snippet = finding.snippet.trim().to_string();
        let line_hash = snippet
            .ends_with('…')
            .then(|| line_at(root, finding))
            .flatten()
            .filter(|line| crate::analyzer::clip_snippet(line) == snippet)
            .map(|line| hash_line(&line));
        Self {
            rule_id: finding.rule_id.clone(),
            file: finding.file.clone(),
            snippet,
            line_hash,
            reason: reason.trim().to_string(),
            created_at,
        }
    }

    pub fn matches(&self, finding: &Finding) -> bool {
        self.rule_id == finding.rule_id && self.file == finding.file && self.snippet == finding.snippet.trim()
    }

    fn same_line(&self, line: Option<&str>) -> bool {
        match (&self.line_hash, line) {
            (Some(want), Some(line)) => *want == hash_line(line),
            _ => true,
        }
    }

    /// `matches`, and the same full line when the snippet was shortened.
    pub fn covers(&self, root: &Path, finding: &Finding) -> bool {
        self.matches(finding) && (self.line_hash.is_none() || self.same_line(line_at(root, finding).as_deref()))
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

const RULE_GROUPS: [&str; 7] = ["behavior", "deprecated", "framework", "removed", "runtime", "server", "syntax"];

/// A directive starts a comment, or a line of a block comment. Prose that mentions it, or a word
/// that only begins the same way, isn't one.
fn directive_text(comment: &str) -> Option<&str> {
    comment.lines().find_map(|line| {
        let rest = line.trim_start_matches(|c: char| c.is_whitespace() || "/#;*!<-".contains(c)).strip_prefix("ferry-ignore")?;
        (rest.is_empty() || rest.starts_with(|c: char| c.is_whitespace() || matches!(c, ':' | '*' | '-'))).then_some(rest)
    })
}

fn directive(comment: &str, rule_id: &str) -> Option<String> {
    let rest = directive_text(comment)?.trim_start_matches(':').trim();
    let rest = rest.split("*/").next().unwrap_or_default().split("-->").next().unwrap_or_default().trim();
    let (first, tail) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let names_rule =
        first.ends_with('*') || first.split_once('.').is_some_and(|(group, rest)| RULE_GROUPS.contains(&group) && !rest.is_empty());
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
    comments: std::collections::HashMap<usize, Vec<(String, bool)>>,
}

impl Source {
    fn read(path: &Path) -> Option<Self> {
        let text = String::from_utf8_lossy(&crate::safefs::read_bytes(path).ok()?).into_owned();
        let lexed = crate::lexer::lex(&text);
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        let line_of = |pos: usize| starts.partition_point(|&s| s <= pos);
        let mut comments: std::collections::HashMap<usize, Vec<(String, bool)>> = std::collections::HashMap::new();
        let mut add = |start: usize, end: usize| {
            let body = &text[start..end];
            if body.contains("ferry-ignore") {
                let own_line = text[..start].rsplit('\n').next().is_some_and(|before| before.trim().is_empty());
                for line in line_of(start)..=line_of(end.saturating_sub(1).max(start)) {
                    comments.entry(line).or_default().push((body.to_string(), own_line));
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
        if !crate::project::is_php_file(path) {
            for (i, line) in text.lines().enumerate() {
                let start =
                    line.char_indices().find(|&(at, c)| matches!(c, '#' | ';') && (at == 0 || line[..at].ends_with(char::is_whitespace)));
                if let Some((at, _)) = start.filter(|&(at, _)| line[at..].contains("ferry-ignore")) {
                    comments.entry(i + 1).or_default().push((line[at..].to_string(), line[..at].trim().is_empty()));
                }
            }
        }
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        let trimmed = lines.iter().map(|l| clean_line(l).to_string()).collect();
        Some(Self { lines, trimmed, comments })
    }

    fn inline_reason(&self, line: usize, rule_id: &str) -> Option<String> {
        let on = |l: usize, own_line: bool| {
            self.comments.get(&l).and_then(|cs| cs.iter().filter(|c| c.1 || !own_line).find_map(|c| directive(&c.0, rule_id)))
        };
        on(line, false).or_else(|| line.checked_sub(1).filter(|&l| l > 0).and_then(|l| on(l, true)))
    }

    fn has(&self, ignore: &Ignore) -> bool {
        if let Some(want) = &ignore.line_hash {
            return self.trimmed.iter().any(|line| hash_line(line) == *want);
        }
        match ignore.snippet.strip_suffix('…') {
            Some(prefix) => self.lines.iter().any(|l| l.trim().starts_with(prefix)),
            None => self.trimmed.contains(&ignore.snippet),
        }
    }
}

pub fn apply(root: &Path, findings: Vec<Finding>, ignores: &[Ignore]) -> Applied {
    let mut sources: std::collections::HashMap<String, Option<Source>> = std::collections::HashMap::new();
    let (mut kept, mut ignored) = (Vec::new(), Vec::new());
    for f in findings {
        let hit = ignores.iter().find(|i| {
            i.matches(&f) && {
                let source = sources.entry(f.file.clone()).or_insert_with(|| Source::read(&root.join(&f.file)));
                i.same_line(source.as_ref().and_then(|s| s.lines.get(f.line.wrapping_sub(1))).map(|line| clean_line(line)))
            }
        });
        if let Some(i) = hit {
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
        .filter(|i| i.line_hash.is_some() || !ignored.iter().any(|g: &IgnoredFinding| !g.inline && i.matches(&g.finding)))
        .filter(|i| {
            let source = sources.entry(i.file.clone()).or_insert_with(|| Source::read(&root.join(&i.file)));
            !source.as_ref().is_some_and(|s| s.has(i))
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
        save(dir.path(), &[Ignore::for_finding(dir.path(), &before[0], " vendored copy ", 1)]).unwrap();
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
    fn a_directive_works_in_files_that_are_not_php() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Dockerfile"), "# ferry-ignore server.php_version_pin kept for the old worker\nFROM php:7.4-cli\n")
            .unwrap();
        let (findings, _) = crate::server::scan(dir.path(), "8.4".parse().unwrap());
        assert_eq!(findings.len(), 1);
        let applied = apply(dir.path(), findings, &[]);
        assert!(applied.kept.is_empty());
        assert_eq!(applied.ignored[0].reason, "kept for the old worker");
    }

    #[test]
    fn a_dotted_word_in_a_reason_is_not_a_rule_name() {
        assert_eq!(
            directive("// ferry-ignore docs.example.test says it is fine", "removed.each").as_deref(),
            Some("docs.example.test says it is fine")
        );
        assert_eq!(directive("// ferry-ignore removed.split not this one", "removed.each"), None);
        assert_eq!(directive("// ferry-ignore removed.each known", "removed.each").as_deref(), Some("known"));
        assert_eq!(directive("// ferry-ignore removed.* legacy", "removed.each").as_deref(), Some("legacy"));
    }

    #[test]
    fn only_a_real_directive_hides_a_finding() {
        let dir = tempfile::tempdir().unwrap();
        let src = "<?php\n// ferry-ignored by nobody\neach($a);\n// see ferry-ignore docs\neach($b);\n$x = 1; // ferry-ignore\neach($c);\n/**\n * ferry-ignore removed.each old\n */\neach($d);\n";
        std::fs::write(dir.path().join("a.php"), src).unwrap();
        let findings = analyze_source("a.php", src, "8.2".parse().unwrap());
        let applied = apply(dir.path(), findings, &[]);
        let lines: Vec<usize> = applied.kept.iter().map(|f| f.line).collect();
        assert_eq!(lines, [3, 5, 7]);
        assert_eq!(applied.ignored.iter().map(|i| i.finding.line).collect::<Vec<_>>(), [11]);
    }

    #[test]
    fn an_ignore_on_a_bom_line_is_not_stale() {
        let dir = tempfile::tempdir().unwrap();
        let src = "\u{feff}<?php each($a);\n";
        std::fs::write(dir.path().join("a.php"), src).unwrap();
        let findings = analyze_source("a.php", src, "8.2".parse().unwrap());
        let ignores = [Ignore::for_finding(dir.path(), &findings[0], "old", 1)];
        let applied = apply(dir.path(), findings, &ignores);
        assert_eq!((applied.ignored.len(), applied.stale.len()), (1, 0));
    }

    #[cfg(unix)]
    #[test]
    fn a_file_that_cant_be_read_is_not_treated_as_empty_when_saving() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        save(
            dir.path(),
            &[Ignore {
                rule_id: "r".into(),
                file: "a.php".into(),
                snippet: "s".into(),
                line_hash: None,
                reason: "x".into(),
                created_at: 0,
            }],
        )
        .unwrap();
        std::fs::set_permissions(path(dir.path()), std::fs::Permissions::from_mode(0o000)).unwrap();
        let enforced = std::fs::File::open(path(dir.path())).is_err();
        let loaded = load_for_update(dir.path());
        std::fs::set_permissions(path(dir.path()), std::fs::Permissions::from_mode(0o644)).unwrap();
        if enforced {
            assert!(loaded.is_err());
        }
    }

    #[test]
    fn a_broken_file_is_not_treated_as_empty_when_saving() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_for_update(dir.path()).unwrap().is_empty());
        std::fs::create_dir(dir.path().join(".ferry")).unwrap();
        std::fs::write(path(dir.path()), "<<<<<<< HEAD\n{}").unwrap();
        assert!(load_for_update(dir.path()).is_err());
    }

    #[test]
    fn inline_comments_and_stale_entries() {
        let dir = tempfile::tempdir().unwrap();
        let src = "<?php\n// ferry-ignore removed.each only runs on the PHP 5 server\neach($a);\neach($b); // ferry-ignore\n/* ferry-ignore removed.* legacy */ each($c);\n// ferry-ignore deprecated.utf8_encode wrong rule\neach($d);\n";
        std::fs::write(dir.path().join("a.php"), src).unwrap();
        let findings = analyze_source("a.php", src, "8.2".parse().unwrap());
        let gone = Ignore {
            line_hash: None,
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
        let ignores = [Ignore::for_finding(dir.path(), &findings[0], "kept", 0)];
        let applied = apply(dir.path(), findings, &ignores);
        assert_eq!((applied.ignored.len(), applied.stale.len()), (1, 0));
        assert!(!applied.ignored[0].inline);
    }

    fn long_twins() -> (tempfile::TempDir, Vec<Finding>) {
        let dir = tempfile::tempdir().unwrap();
        let start = format!("each($a); // {}", "x".repeat(300));
        let src = format!("<?php\n{start} one\n{start} two\n");
        std::fs::write(dir.path().join("a.php"), &src).unwrap();
        let findings = analyze_source("a.php", &src, "8.2".parse().unwrap());
        assert_eq!(findings[0].snippet, findings[1].snippet);
        (dir, findings)
    }

    #[test]
    fn an_ignore_keeps_to_its_own_long_line() {
        let (dir, findings) = long_twins();
        let first = Ignore::for_finding(dir.path(), &findings[0], "kept", 0);
        assert!(first.covers(dir.path(), &findings[0]) && !first.covers(dir.path(), &findings[1]));
        let applied = apply(dir.path(), findings.clone(), std::slice::from_ref(&first));
        assert_eq!((applied.kept.len(), applied.kept[0].line), (1, 3));
        assert_eq!((applied.ignored.len(), applied.stale.len()), (1, 0));

        let src = std::fs::read_to_string(dir.path().join("a.php")).unwrap().replacen(" one\n", " changed\n", 1);
        std::fs::write(dir.path().join("a.php"), src).unwrap();
        let applied = apply(dir.path(), findings, std::slice::from_ref(&first));
        assert_eq!(applied.stale, [first]);
    }

    #[test]
    fn an_older_entry_still_covers_every_line_it_used_to() {
        let (dir, findings) = long_twins();
        let old = Ignore { line_hash: None, ..Ignore::for_finding(dir.path(), &findings[0], "kept", 0) };
        save(dir.path(), std::slice::from_ref(&old)).unwrap();
        assert!(!std::fs::read_to_string(path(dir.path())).unwrap().contains("lineHash"));
        let applied = apply(dir.path(), findings, &load(dir.path()));
        assert_eq!((applied.kept.len(), applied.ignored.len(), applied.stale.len()), (0, 2, 0));
    }

    #[test]
    fn many_findings_in_one_file_stay_fast() {
        let dir = tempfile::tempdir().unwrap();
        let src = format!("<?php\n{}", "each($a);\n".repeat(4000));
        std::fs::write(dir.path().join("a.php"), &src).unwrap();
        let findings = analyze_source("a.php", &src, "8.2".parse().unwrap());
        let ignores: Vec<Ignore> =
            (0..200).map(|n| Ignore { snippet: format!("gone{n}();"), ..Ignore::for_finding(dir.path(), &findings[0], "x", 0) }).collect();
        let started = std::time::Instant::now();
        let applied = apply(dir.path(), findings, &ignores);
        assert_eq!((applied.kept.len(), applied.stale.len()), (4000, 200));
        assert!(started.elapsed() < std::time::Duration::from_secs(2), "{:?}", started.elapsed());
    }
}
