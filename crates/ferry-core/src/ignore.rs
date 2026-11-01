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
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(tmp, file)
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
            Some(i) => ignored.push(IgnoredFinding { reason: i.reason.clone(), finding: f }),
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

fn inline_reason(line: &str, rule_id: &str) -> Option<String> {
    let at = line.find("ferry-ignore")?;
    let rest = line[at + "ferry-ignore".len()..].trim_start_matches(':').trim();
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

pub fn apply(root: &Path, findings: Vec<Finding>, ignores: &[Ignore]) -> Applied {
    let mut sources: std::collections::HashMap<String, Option<Vec<String>>> = std::collections::HashMap::new();
    let mut lines_of = |file: &str| -> Option<Vec<String>> {
        sources
            .entry(file.to_string())
            .or_insert_with(|| {
                std::fs::read(root.join(file)).ok().map(|b| String::from_utf8_lossy(&b).lines().map(str::to_string).collect())
            })
            .clone()
    };
    let (mut kept, mut ignored) = (Vec::new(), Vec::new());
    for f in findings {
        if let Some(i) = ignores.iter().find(|i| i.matches(&f)) {
            ignored.push(IgnoredFinding { reason: i.reason.clone(), finding: f });
            continue;
        }
        let inline = lines_of(&f.file).and_then(|lines| {
            let here = lines.get(f.line.wrapping_sub(1)).and_then(|l| inline_reason(l, &f.rule_id));
            here.or_else(|| f.line.checked_sub(2).and_then(|k| lines.get(k)).and_then(|l| inline_reason(l, &f.rule_id)))
        });
        match inline {
            Some(reason) => ignored.push(IgnoredFinding { reason, finding: f }),
            None => kept.push(f),
        }
    }
    let stale = ignores
        .iter()
        .filter(|i| !lines_of(&i.file).is_some_and(|lines| lines.iter().any(|l| l.trim() == i.snippet || l.contains(i.snippet.as_str()))))
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
}
