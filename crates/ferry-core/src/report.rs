use crate::analyzer::{Finding, ScanResult};
use crate::deps::{DependencyReport, DependencyStatus};
use crate::i18n::{Lang, Text};
use crate::origin::OriginKind;
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
    let mut s = Summary { total: findings.len(), score: score(findings), ..Summary::default() };
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

fn docker_section(result: &crate::docker::Comparison, lang: Lang) -> String {
    let l = |t: Text| t.get(lang).to_string();
    let summary = result.summary(0);
    let mut md = format!("## {}\n\n", l(Text::new("Docker karşılaştırması", "Docker comparison")));
    let line = match lang {
        Lang::Tr => format!("PHP {} ile PHP {} karşılaştırıldı.", summary.source, summary.target),
        Lang::En => format!("PHP {} was compared with PHP {}.", summary.source, summary.target),
    };
    let verdict = if summary.regressed {
        l(Text::new("Hedef sürümde yeni sorunlar var.", "The target version has new problems."))
    } else if summary.tests_compared {
        l(Text::new(
            "Testler iki sürümde de çalıştı ve hedef yeni bir hata eklemedi.",
            "Tests ran on both versions and the target added no new errors.",
        ))
    } else {
        l(Text::new(
            "Testler karşılaştırılamadı; bu sonuç sözdizimi ve sayfa kontrolüne dayanıyor.",
            "Tests couldn't be compared, so this result rests on the syntax and page checks.",
        ))
    };
    md.push_str(&format!("{line} {verdict}\n\n"));
    let (pages, failures, messages, tables) = match lang {
        Lang::Tr => ("Farklı sayfa", "Yalnızca hedefte kalan test", "Hedefte yeni PHP mesajı", "Farklı tablo"),
        Lang::En => ("Differing pages", "Tests failing only on the target", "New PHP messages on the target", "Differing tables"),
    };
    md.push_str(&format!("- {pages}: {} / {}\n", summary.differing_pages, summary.pages));
    if summary.unverified_pages > 0 {
        let label = l(Text::new("Karşılaştırılamayan sayfa", "Pages that couldn't be compared"));
        md.push_str(&format!("- {label}: {}\n", summary.unverified_pages));
    }
    md.push_str(&format!("- {failures}: {}\n- {messages}: {}\n", summary.new_failures, summary.new_messages));
    if result.database_used {
        md.push_str(&format!("- {tables}: {}\n", summary.data_changes));
    }
    md.push('\n');
    for name in result.new_failures.iter().take(20) {
        md.push_str(&format!("- `{}`\n", md_escape(name)));
    }
    for page in result.pages.iter().filter(|p| p.broke()).take(20) {
        md.push_str(&format!("- `{}` ({} → {})\n", md_escape(&page.path), page.source_status, page.target_status));
    }
    if !result.new_failures.is_empty() || result.pages.iter().any(|p| p.broke()) {
        md.push('\n');
    }
    if result.origins.iter().any(|o| o.kind != OriginKind::Project) {
        md.push_str(&format!("{}\n\n", l(Text::new("Yeni mesajların kaynağı:", "Where the new messages come from:"))));
        for o in &result.origins {
            let kind = match o.kind {
                OriginKind::Project => Text::new("Proje kodu", "Your code"),
                OriginKind::Core => Text::new("Çekirdek", "Core"),
                OriginKind::Package => Text::new("Paket", "Package"),
                OriginKind::Extension => Text::new("Eklenti ya da tema", "Plugin or theme"),
            };
            let name = [o.name.as_str(), o.version.as_deref().unwrap_or_default()].join(" ");
            md.push_str(&format!("- {} {}: {}\n", l(kind), md_escape(name.trim()), o.messages));
            if let Some(advice) = &o.advice {
                md.push_str(&format!("  {}\n", md_escape(advice.get(lang))));
            }
        }
        md.push('\n');
    }
    md
}

pub fn to_markdown(
    project: &ProjectInfo,
    scan: &ScanResult,
    deps: Option<&DependencyReport>,
    docker: Option<&crate::docker::Comparison>,
    lang: Lang,
) -> String {
    let l = |t: Text| t.get(lang);
    let s = summarize(&scan.findings);
    let current = project.current_version.map_or_else(|| l(Text::new("bilinmiyor", "unknown")).to_string(), |v| format!("PHP {v}"));
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
    md.push_str(&format!("## {}\n\n| | {} |\n|---|---:|\n", l(Text::new("Özet", "Summary")), l(Text::new("Adet", "Count"))));
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

    if !scan.errors.is_empty() {
        md.push_str(&format!(
            "## {}\n\n{}\n\n",
            l(Text::new("Taranamayan dosyalar", "Files not scanned")),
            l(Text::new(
                "Bu dosyalar analiz edilemedi; sonuçlar onları kapsamıyor.",
                "These files couldn't be analysed, so the results don't cover them."
            ))
        ));
        for e in &scan.errors {
            md.push_str(&format!("- `{}`: {}\n", md_escape(&e.file), md_escape(&e.message)));
        }
        md.push('\n');
    }

    if scan.findings.is_empty() && scan.errors.is_empty() {
        md.push_str(l(Text::new(
            "Hedef sürümle uyumsuz bir şey bulunmadı.\n\n",
            "Nothing incompatible with the target version was found.\n\n",
        )));
    } else if scan.findings.is_empty() {
        md.push_str(l(Text::new(
            "Taranabilen dosyalarda hedef sürümle uyumsuz bir şey bulunmadı.\n\n",
            "Nothing incompatible with the target version was found in the files that could be scanned.\n\n",
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
        let bad: Vec<_> = deps.packages.iter().filter(|p| p.status == DependencyStatus::Incompatible).collect();
        let unknown = deps.packages.iter().filter(|p| p.status == DependencyStatus::Unknown).count();
        md.push_str(&format!("## {}\n\n", l(Text::new("Bağımlılıklar", "Dependencies"))));
        if deps.root_status == DependencyStatus::Incompatible {
            let constraint = match (&deps.root_constraint, &deps.root_conflict) {
                (Some(r), Some(c)) => format!("require `{r}`, conflict `{c}`"),
                (None, Some(c)) => format!("conflict `{c}`"),
                (r, None) => format!("`{}`", r.as_deref().unwrap_or("")),
            };
            md.push_str(&match lang {
                Lang::Tr => format!("composer.json'daki PHP kısıtı ({constraint}) PHP {} ile uyumlu değil.\n\n", deps.target),
                Lang::En => format!("The PHP constraint in composer.json ({constraint}) doesn't allow PHP {}.\n\n", deps.target),
            });
        } else if deps.root_status == DependencyStatus::Unknown && deps.root_constraint.is_some() {
            md.push_str(&match lang {
                Lang::Tr => format!("composer.json'daki PHP kısıtı okunamadığı için PHP {} ile uyumu doğrulanamadı.\n\n", deps.target),
                Lang::En => {
                    format!("The PHP constraint in composer.json couldn't be read, so PHP {} support isn't confirmed.\n\n", deps.target)
                }
            });
        }
        if deps.lock_unreadable {
            md.push_str(l(Text::new(
                "composer.lock okunamadı; paketler kontrol edilmedi.\n\n",
                "composer.lock couldn't be read, so packages weren't checked.\n\n",
            )));
        } else if !deps.has_lock_file {
            md.push_str(l(Text::new("composer.lock bulunamadı.\n\n", "No composer.lock found.\n\n")));
        } else if bad.is_empty() && unknown > 0 {
            md.push_str(&match lang {
                Lang::Tr => format!("Engel bulunmadı, ancak {unknown} paketin PHP {} ile uyumu doğrulanamadı.\n\n", deps.target),
                Lang::En => format!(
                    "No blockers found, but {} couldn't be confirmed for PHP {}.\n\n",
                    if unknown == 1 { "1 package".to_string() } else { format!("{unknown} packages") },
                    deps.target
                ),
            });
        } else if bad.is_empty() {
            md.push_str(&match lang {
                Lang::Tr => format!("{} paketin hepsi PHP {} ile uyumlu görünüyor.\n\n", deps.packages.len(), deps.target),
                Lang::En => format!("All {} packages look compatible with PHP {}.\n\n", deps.packages.len(), deps.target),
            });
        } else {
            if unknown > 0 {
                md.push_str(&match lang {
                    Lang::Tr => format!("Ayrıca {unknown} paketin PHP {} ile uyumu doğrulanamadı.\n\n", deps.target),
                    Lang::En => format!(
                        "{} also couldn't be confirmed for PHP {}.\n\n",
                        if unknown == 1 { "1 package".to_string() } else { format!("{unknown} packages") },
                        deps.target
                    ),
                });
            }
            md.push_str(l(Text::new("| Paket | Sürüm | PHP gereksinimi |\n", "| Package | Version | PHP requirement |\n")));
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
    if let Some(result) = docker {
        md.push_str(&docker_section(result, lang));
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
    fn blocked_dependencies_still_report_unconfirmed_packages() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("composer.lock"),
            r#"{"packages": [
                {"name": "a/old", "version": "1.0.0", "require": {"php": "<7.0"}},
                {"name": "b/free", "version": "1.0.0"}
            ]}"#,
        )
        .unwrap();
        let deps = crate::deps::check_packages(dir.path(), "8.4".parse().unwrap());
        let project = ProjectInfo {
            root: "/p".into(),
            name: "demo".into(),
            framework: Framework::Generic,
            php_files: 0,
            php_constraint: None,
            current_version: None,
            has_composer_lock: true,
            has_vendor: false,
            is_git_repo: false,
            excluded_dirs: vec![],
            config: Default::default(),
            config_error: None,
        };
        let scan = ScanResult {
            target: deps.target,
            files_found: 0,
            files_scanned: 0,
            lines_scanned: 0,
            findings: vec![],
            errors: vec![],
            duration_ms: 1,
        };
        let md = to_markdown(&project, &scan, Some(&deps), None, Lang::En);
        assert!(md.contains("1 package also couldn't be confirmed for PHP 8.4."), "{md}");
        assert!(md.contains("a/old"), "{md}");
    }

    #[test]
    fn markdown_includes_the_docker_comparison() {
        let run = |version: &str| crate::docker::VersionRun {
            version: version.parse().unwrap(),
            image: String::new(),
            lint_errors: 0,
            tests: None,
            failed_tests: Vec::new(),
            crawl: Default::default(),
            data: Default::default(),
            missing_extensions: Vec::new(),
            installed_extensions: Vec::new(),
            image_failed: false,
            events: Vec::new(),
            log: String::new(),
        };
        let comparison = crate::docker::Comparison {
            source: run("7.4"),
            target: run("8.4"),
            dependencies_installed: true,
            install_log: String::new(),
            new_failures: vec!["CartTest::testTotal".into()],
            pages: Vec::new(),
            data_changes: Vec::new(),
            database_used: false,
            new_on_target: Vec::new(),
            gone_on_target: Vec::new(),
            origins: vec![crate::origin::Origin {
                kind: OriginKind::Core,
                name: "WordPress".into(),
                version: None,
                messages: 3,
                fatal: 1,
                advice: Some(crate::i18n::Localized::same("Update it.".to_string())),
            }],
        };
        assert!(comparison.regressed());
        let text = docker_section(&comparison, Lang::En);
        assert!(text.contains("PHP 7.4 was compared with PHP 8.4. The target version has new problems."), "{text}");
        assert!(text.contains("`CartTest::testTotal`"), "{text}");
        assert!(text.contains("- Core WordPress: 3\n  Update it."), "{text}");
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
            config: Default::default(),
            config_error: None,
        };
        let scan = ScanResult {
            target: "8.4".parse().unwrap(),
            files_found: 1,
            files_scanned: 1,
            lines_scanned: 1,
            findings: findings("<?php each($a);"),
            errors: vec![],
            duration_ms: 1,
        };
        let md = to_markdown(&project, &scan, None, None, Lang::Tr);
        assert!(md.starts_with("# Ferry raporu: demo"));
        assert!(md.contains("| `x.php` | 1 | each() (PHP 8.0) | Kritik | Gözden geçir |"));
        let md = to_markdown(&project, &scan, None, None, Lang::En);
        assert!(md.starts_with("# Ferry report: demo"));
        assert!(md.contains("| Breaking | Review | foreach or key()/current()/next() |"));

        let partial = ScanResult {
            files_scanned: 0,
            findings: vec![],
            errors: vec![crate::analyzer::FileError { file: "big.php".into(), message: "too large".into() }],
            ..scan
        };
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("composer.lock"), r#"{"packages":[{"name":"a/b","version":"1.0.0"}]}"#).unwrap();
        let deps = crate::deps::check_packages(dir.path(), "8.4".parse().unwrap());
        let md = to_markdown(&project, &scan, Some(&deps), None, Lang::En);
        assert!(md.contains("No blockers found, but 1 package couldn't be confirmed") && !md.contains("look compatible"), "{md}");
        std::fs::write(dir.path().join("composer.lock"), "{broken").unwrap();
        let deps = crate::deps::check_packages(dir.path(), "8.4".parse().unwrap());
        assert!(to_markdown(&project, &scan, Some(&deps), None, Lang::En).contains("composer.lock couldn't be read"));

        let md = to_markdown(&project, &partial, None, None, Lang::En);
        assert!(md.contains("## Files not scanned") && md.contains("- `big.php`: too large"), "{md}");
        assert!(
            md.contains("in the files that could be scanned") && !md.contains("Nothing incompatible with the target version was found.\n")
        );
    }
}
