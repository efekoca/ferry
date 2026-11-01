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
}
