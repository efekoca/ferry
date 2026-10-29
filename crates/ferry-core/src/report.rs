use crate::analyzer::{Finding, ScanResult};
use crate::deps::{DependencyReport, DependencyStatus};
use crate::i18n::{Lang, Text};
use crate::project::ProjectInfo;
use crate::rules::{FixKind, Severity};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub total: usize,
    pub breaking: usize,
    pub deprecated: usize,
    pub behavior: usize,
    pub auto: usize,
    pub likely: usize,
    pub manual: usize,
    pub files_affected: usize,
    pub score: u8,
}

fn weight(f: &Finding) -> f64 {
    let base = match f.severity {
        Severity::Breaking => 4.0,
        Severity::Deprecated => 1.0,
        Severity::Behavior => 1.5,
    };
    let effort = match f.fix {
        FixKind::Auto => 0.25,
        FixKind::Likely => 0.6,
        FixKind::Manual => 1.0,
    };
    base * effort
}

pub fn score(findings: &[Finding]) -> u8 {
    let penalty: f64 = findings.iter().map(weight).sum();
    (100.0 * (-penalty / 40.0).exp()).round().clamp(0.0, 100.0) as u8
}

pub fn summarize(findings: &[Finding]) -> Summary {
    let mut s = Summary {
        total: findings.len(),
        score: score(findings),
        ..Summary::default()
    };
    let mut files = BTreeSet::new();
    for f in findings {
        match f.severity {
            Severity::Breaking => s.breaking += 1,
            Severity::Deprecated => s.deprecated += 1,
            Severity::Behavior => s.behavior += 1,
        }
        match f.fix {
            FixKind::Auto => s.auto += 1,
            FixKind::Likely => s.likely += 1,
            FixKind::Manual => s.manual += 1,
        }
        files.insert(f.file.as_str());
    }
    s.files_affected = files.len();
    s
}

fn severity_label(s: Severity) -> Text {
    match s {
        Severity::Breaking => Text::new("Kritik", "Breaking"),
        Severity::Deprecated => Text::new("Eskimiş", "Deprecated"),
        Severity::Behavior => Text::new("Riskli", "Risky"),
    }
}

fn fix_label(f: FixKind) -> Text {
    match f {
        FixKind::Auto => Text::new("Otomatik", "Automatic"),
        FixKind::Likely => Text::new("Gözden geçir", "Review"),
        FixKind::Manual => Text::new("Elle düzelt", "Manual"),
    }
}

fn md_escape(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

pub fn to_markdown(project: &ProjectInfo, scan: &ScanResult, deps: Option<&DependencyReport>, lang: Lang) -> String {
    let l = |t: Text| t.get(lang);
    let s = summarize(&scan.findings);
    let current = project
        .current_version
        .map_or_else(|| l(Text::new("bilinmiyor", "unknown")).to_string(), |v| format!("PHP {v}"));
    let mut md = String::new();
    md.push_str(&format!("# {}: {}\n\n", l(Text::new("Ferry raporu", "Ferry report")), project.name));
    md.push_str(&format!(
        "- **{}:** PHP {}\n- **{}:** {}\n- **{}:** {} ({} {})\n- **{}:** {}/100\n\n",
        l(Text::new("Hedef sürüm", "Target version")),
        scan.target,
        l(Text::new("Şu anki sürüm (tahmini)", "Current version (estimated)")),
        current,
        l(Text::new("Taranan dosya", "Files scanned")),
        scan.files_scanned,
        scan.lines_scanned,
        l(Text::new("satır", "lines")),
        l(Text::new("Hazırlık skoru", "Readiness score")),
        s.score
    ));
    md.push_str(&format!(
        "## {}\n\n| | {} |\n|---|---:|\n",
        l(Text::new("Özet", "Summary")),
        l(Text::new("Adet", "Count"))
    ));
    for (label, n) in [
        (severity_label(Severity::Breaking), s.breaking),
        (severity_label(Severity::Deprecated), s.deprecated),
        (severity_label(Severity::Behavior), s.behavior),
        (fix_label(FixKind::Auto), s.auto),
        (fix_label(FixKind::Likely), s.likely),
        (fix_label(FixKind::Manual), s.manual),
    ] {
        md.push_str(&format!("| {} | {n} |\n", l(label)));
    }
    md.push('\n');

    if scan.findings.is_empty() {
        md.push_str(l(Text::new(
            "Hedef sürümle uyumsuz bir şey bulunmadı.\n\n",
            "Nothing incompatible with the target version was found.\n\n",
        )));
    } else {
        md.push_str(&format!("## {}\n\n", l(Text::new("Bulgular", "Findings"))));
        md.push_str(l(Text::new(
            "| Dosya | Satır | Sorun | Önem | Düzeltme | Öneri |\n",
            "| File | Line | Issue | Severity | Fix | Suggestion |\n",
        )));
        md.push_str("|---|---:|---|---|---|---|\n");
        for f in &scan.findings {
            md.push_str(&format!(
                "| `{}` | {} | {} (PHP {}) | {} | {} | {} |\n",
                md_escape(&f.file),
                f.line,
                md_escape(f.title.get(lang)),
                f.since,
                l(severity_label(f.severity)),
                l(fix_label(f.fix)),
                md_escape(f.replacement.as_ref().map_or("—", |r| r.get(lang))),
            ));
        }
        md.push('\n');
    }

    if let Some(deps) = deps {
        let bad: Vec<_> = deps
            .packages
            .iter()
            .filter(|p| p.status == DependencyStatus::Incompatible)
            .collect();
        md.push_str(&format!("## {}\n\n", l(Text::new("Bağımlılıklar", "Dependencies"))));
        if deps.root_status == DependencyStatus::Incompatible {
            let constraint = deps.root_constraint.as_deref().unwrap_or("");
            md.push_str(&match lang {
                Lang::Tr => format!("composer.json'daki PHP kısıtı (`{constraint}`) PHP {} ile uyumlu değil.\n\n", deps.target),
                Lang::En => format!("The PHP constraint in composer.json (`{constraint}`) doesn't allow PHP {}.\n\n", deps.target),
            });
        }
        if !deps.has_lock_file {
            md.push_str(l(Text::new("composer.lock bulunamadı.\n\n", "No composer.lock found.\n\n")));
        } else if bad.is_empty() {
            md.push_str(&match lang {
                Lang::Tr => format!("{} paketin hepsi PHP {} ile uyumlu görünüyor.\n\n", deps.packages.len(), deps.target),
                Lang::En => format!("All {} packages look compatible with PHP {}.\n\n", deps.packages.len(), deps.target),
            });
        } else {
            md.push_str(l(Text::new(
                "| Paket | Sürüm | PHP gereksinimi |\n",
                "| Package | Version | PHP requirement |\n",
            )));
            md.push_str("|---|---|---|\n");
            for p in bad {
                md.push_str(&format!(
                    "| `{}` | {} | `{}` |\n",
                    p.name.replace('|', "\\|"),
                    p.version.replace('|', "\\|"),
                    p.php_constraint.as_deref().unwrap_or("—").replace('|', "\\|")
                ));
            }
            md.push('\n');
        }
    }
    md.push_str(l(Text::new("---\nFerry ile hazırlandı.\n", "---\nGenerated with Ferry.\n")));
    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::analyze_source;
    use crate::project::Framework;

    fn findings(src: &str) -> Vec<Finding> {
        analyze_source("x.php", src, "8.4".parse().unwrap())
    }

    #[test]
    fn score_is_monotonic() {
        assert_eq!(score(&[]), 100);
        let one = score(&findings("<?php each($a);"));
        let many = score(&findings("<?php each($a); mysql_query($q); mysql_query($q); eval($x);"));
        assert!(one < 100 && many < one, "{one} {many}");
        let lots: Vec<Finding> = (0..500).flat_map(|_| findings("<?php mysql_query($q);")).collect();
        assert_eq!(score(&lots), 0);
    }

    #[test]
    fn summary_counts() {
        let s = summarize(&findings("<?php each($a); utf8_encode($s); eval($x);"));
        assert_eq!((s.total, s.breaking, s.deprecated, s.behavior), (3, 1, 1, 1));
        assert_eq!((s.auto, s.likely, s.manual), (0, 2, 1));
        assert_eq!(s.files_affected, 1);
    }

    #[test]
    fn markdown_report() {
        let project = ProjectInfo {
            root: "/p".into(),
            name: "demo".into(),
            framework: Framework::Generic,
            php_files: 1,
            php_constraint: None,
            current_version: None,
            has_composer_lock: false,
            has_vendor: false,
            is_git_repo: false,
            excluded_dirs: vec![],
        };
        let scan = ScanResult {
            target: "8.4".parse().unwrap(),
            files_scanned: 1,
            lines_scanned: 1,
            findings: findings("<?php each($a);"),
            errors: vec![],
            duration_ms: 1,
        };
        let md = to_markdown(&project, &scan, None, Lang::Tr);
        assert!(md.starts_with("# Ferry raporu: demo"));
        assert!(md.contains("| `x.php` | 1 | each() (PHP 8.0) | Kritik | Gözden geçir |"));
        let md = to_markdown(&project, &scan, None, Lang::En);
        assert!(md.starts_with("# Ferry report: demo"));
        assert!(md.contains("| Breaking | Review | foreach or key()/current()/next() |"));
    }
}
