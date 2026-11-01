use crate::analyzer::Finding;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub const KEEP: usize = 30;

pub fn fingerprint(f: &Finding) -> String {
    format!("{}|{}|{}", f.rule_id, f.file, f.snippet.trim())
}

pub fn new_findings<'a>(baseline: &[String], findings: &'a [Finding]) -> Vec<&'a Finding> {
    let mut left: HashMap<&str, usize> = HashMap::new();
    for b in baseline {
        *left.entry(b.as_str()).or_default() += 1;
    }
    findings
        .iter()
        .filter(|f| {
            let key = fingerprint(f);
            match left.get_mut(key.as_str()) {
                Some(n) if *n > 0 => {
                    *n -= 1;
                    false
                }
                _ => true,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub at: u64,
    pub target: String,
    pub score: u8,
    pub total: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fingerprints: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub history: Vec<Entry>,
    pub new_since_last: Option<usize>,
    pub fixed_since_last: Option<usize>,
}

pub fn load(file: &Path) -> Vec<Entry> {
    std::fs::read(file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

pub fn record(file: &Path, at: u64, target: &str, score: u8, findings: &[Finding]) -> std::io::Result<Progress> {
    let mut entries = load(file);
    let previous = entries.iter().rev().find(|e| e.target == target).cloned();
    let fingerprints: Vec<String> = findings.iter().map(fingerprint).collect();
    let (new_since_last, fixed_since_last) = match &previous {
        Some(p) => {
            let added = new_findings(&p.fingerprints, findings).len();
            (Some(added), Some((p.total + added).saturating_sub(findings.len())))
        }
        None => (None, None),
    };
    for e in entries.iter_mut().filter(|e| e.target == target) {
        e.fingerprints.clear();
    }
    entries.push(Entry { at, target: target.to_string(), score, total: findings.len(), fingerprints });
    let excess = entries.len().saturating_sub(KEEP * 5);
    entries.drain(..excess);
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(&entries).map_err(std::io::Error::other)?)?;
    std::fs::rename(tmp, file)?;
    let history =
        entries.iter().filter(|e| e.target == target).map(|e| Entry { fingerprints: Vec::new(), ..e.clone() }).collect::<Vec<_>>();
    let history = history[history.len().saturating_sub(KEEP)..].to_vec();
    Ok(Progress { history, new_since_last, fixed_since_last })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::analyze_source;

    #[test]
    fn records_progress_per_target() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("h.json");
        let scan = |src: &str| analyze_source("a.php", src, "8.2".parse().unwrap());
        let first = record(&file, 1, "8.4", 50, &scan("<?php\neach($a);\neach($b);\neach($b);")).unwrap();
        assert_eq!((first.new_since_last, first.fixed_since_last, first.history.len()), (None, None, 1));
        let second = record(&file, 2, "8.4", 60, &scan("<?php\n\neach($b);\nsplit($c);")).unwrap();
        assert_eq!((second.new_since_last, second.fixed_since_last), (Some(1), Some(2)));
        assert_eq!(second.history.iter().map(|e| e.score).collect::<Vec<_>>(), [50, 60]);
        let other = record(&file, 3, "8.5", 10, &[]).unwrap();
        assert_eq!((other.history.len(), other.new_since_last), (1, None));
        assert!(load(&file).iter().filter(|e| e.target == "8.4").map(|e| e.fingerprints.len()).eq([0, 2]));
    }

    #[test]
    fn baseline_comparison_counts_duplicates() {
        let found = analyze_source("a.php", "<?php\neach($a);\neach($a);\neach($b);", "8.2".parse().unwrap());
        let baseline = vec![fingerprint(&found[0])];
        assert_eq!(new_findings(&baseline, &found).len(), 2);
    }
}
