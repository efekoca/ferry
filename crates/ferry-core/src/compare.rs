use crate::analyzer;
use crate::deps::{self, DependencyStatus};
use crate::version::PhpVersion;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSummary {
    pub target: PhpVersion,
    pub summary: crate::report::Summary,
    pub blocking_packages: usize,
    pub root_blocks: bool,
    pub incomplete: bool,
    pub error: Option<String>,
}

pub fn compare(root: &Path, files: &[PathBuf], targets: &[PhpVersion]) -> Vec<TargetSummary> {
    let ignores = crate::ignore::load(root);
    std::thread::scope(|s| {
        let handles: Vec<_> = targets
            .iter()
            .map(|&target| {
                let ignores = &ignores;
                s.spawn(move || {
                    let scan = analyzer::analyze_files(root, files, target, |_, _, _| {});
                    let kept = crate::ignore::apply(root, scan.findings, ignores).kept;
                    let packages = deps::check_packages(root, target);
                    TargetSummary {
                        target,
                        summary: crate::report::summarize(&kept),
                        blocking_packages: packages.packages.iter().filter(|p| p.status == DependencyStatus::Incompatible).count(),
                        root_blocks: packages.root_status == DependencyStatus::Incompatible,
                        incomplete: !scan.errors.is_empty(),
                        error: None,
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .zip(targets)
            .map(|(h, &target)| {
                h.join().unwrap_or_else(|panic| {
                    let reason = panic
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_default();
                    TargetSummary {
                        target,
                        summary: crate::report::Summary::default(),
                        blocking_packages: 0,
                        root_blocks: false,
                        incomplete: true,
                        error: Some(crate::t!("Bu hedef karşılaştırılamadı: {reason}", "This target couldn't be compared: {reason}")),
                    }
                })
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_targets_never_have_fewer_breaking_findings() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.php"),
            "<?php\nfunction f(int $a = null) {}\n$s = utf8_encode($x);\nlist($a, $b) = each($arr);\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("composer.json"), r#"{"require":{"php":">=7.4 <8.4"}}"#).unwrap();
        let files = vec![dir.path().join("a.php")];
        let targets: Vec<PhpVersion> = ["8.2", "8.3", "8.4", "8.5"].iter().map(|t| t.parse().unwrap()).collect();
        let result = compare(dir.path(), &files, &targets);
        assert_eq!(result.iter().map(|r| r.target.to_string()).collect::<Vec<_>>(), ["8.2", "8.3", "8.4", "8.5"]);
        assert!(result.windows(2).all(|w| w[0].summary.total <= w[1].summary.total));
        assert_eq!(result.iter().map(|r| r.root_blocks).collect::<Vec<_>>(), [false, false, true, true]);
        assert!(result.iter().all(|r| !r.incomplete && r.error.is_none()));
        std::fs::write(dir.path().join("big.php"), format!("<?php\n//{}", "x".repeat(3 * 1024 * 1024))).unwrap();
        let files = vec![dir.path().join("a.php"), dir.path().join("big.php")];
        assert!(compare(dir.path(), &files, &targets[..1])[0].incomplete);
    }
}
