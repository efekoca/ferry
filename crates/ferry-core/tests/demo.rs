use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use ferry_core::deps::{self, DependencyStatus};
use ferry_core::project::{self, Framework};
use ferry_core::rules::{patterns, CONSTANTS, FUNCTIONS};
use ferry_core::{analyzer, report, PhpVersion};

fn demo(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../demo").join(name)
}

fn scan(name: &str, target: PhpVersion) -> (project::ProjectInfo, analyzer::ScanResult) {
    let root = demo(name);
    let info = project::inspect(&root).unwrap();
    let files = project::collect_php_files(&root, &info.excluded_dirs);
    let result = analyzer::analyze_files(&root, &files, target, |_, _, _| {});
    (info, result)
}

#[test]
fn kuzey_magaza_triggers_every_rule() {
    let (info, result) = scan("kuzey-magaza", PhpVersion::new(8, 5));
    assert_eq!(info.framework, Framework::Generic);
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert!(result.findings.iter().all(|f| !f.file.starts_with("vendor/")));

    let found: BTreeSet<&str> = result.findings.iter().map(|f| f.rule_id.as_str()).collect();
    let pattern_ids = [
        patterns::AUTOLOAD.id,
        patterns::CURLY_OFFSET.id,
        patterns::REAL_CAST.id,
        patterns::UNSET_CAST.id,
        patterns::DOLLAR_BRACE_INTERPOLATION.id,
        patterns::GET_CLASS_NO_ARGS.id,
        patterns::IMPLICIT_NULLABLE.id,
        patterns::NON_CANONICAL_CAST.id,
        patterns::BACKTICK.id,
        patterns::EVAL.id,
        patterns::VARIABLE_VARIABLE.id,
        patterns::EXTRACT.id,
    ];
    let missing: Vec<&str> = FUNCTIONS
        .iter()
        .chain(CONSTANTS)
        .map(|r| r.id)
        .chain(pattern_ids)
        .filter(|id| !found.contains(id))
        .collect();
    assert!(missing.is_empty(), "demo never triggers: {missing:?}");

    let summary = report::summarize(&result.findings);
    assert!(summary.score < 50, "score {}", summary.score);

    let deps = deps::check(&demo("kuzey-magaza"), PhpVersion::new(8, 5));
    assert_eq!(deps.root_status, DependencyStatus::Incompatible);
    let statuses: BTreeSet<_> = deps.packages.iter().map(|p| format!("{:?}", p.status)).collect();
    assert_eq!(statuses.len(), 3, "compatible, incompatible and unknown packages: {statuses:?}");
    assert!(deps.packages.iter().any(|p| p.dev));
}

#[test]
fn wp_blog_skips_core_and_finds_theme_issues() {
    let (info, result) = scan("wp-blog", PhpVersion::new(8, 4));
    assert_eq!(info.framework, Framework::WordPress);
    assert!(result.findings.iter().all(|f| f.file.starts_with("wp-content/")));
    let summary = report::summarize(&result.findings);
    assert!((50..90).contains(&summary.score), "score {}", summary.score);
}

#[test]
fn laravel_api_is_nearly_ready() {
    let (info, result) = scan("laravel-api", PhpVersion::new(8, 5));
    assert_eq!(info.framework, Framework::Laravel);
    assert!(result.findings.iter().all(|f| !f.file.starts_with("storage/")));
    assert!(!result.findings.is_empty());
    assert!(report::summarize(&result.findings).score >= 80);
    let deps = deps::check(&demo("laravel-api"), PhpVersion::new(8, 5));
    assert_eq!(deps.root_status, DependencyStatus::Compatible);
    assert!(deps.packages.iter().all(|p| p.status == DependencyStatus::Compatible));
}

#[test]
fn rector_handles_every_demo_project() {
    let Some(tools) = std::env::var_os("FERRY_TEST_TOOLS").map(PathBuf::from) else {
        eprintln!("FERRY_TEST_TOOLS ayarlı değil; demo Rector testi atlandı");
        return;
    };
    let php = ferry_core::toolchain::find_php(None).expect("php gerekli");
    for name in ["kuzey-magaza", "wp-blog", "laravel-api"] {
        let root = demo(name);
        let info = project::inspect(&root).unwrap();
        let work = tempfile::tempdir().unwrap();
        let preview = ferry_core::rector::preview(
            &php,
            &ferry_core::toolchain::rector_entry(&tools),
            &root,
            PhpVersion::new(8, 5),
            ferry_core::rector::RectorMode::Compat,
            &info.excluded_dirs,
            work.path(),
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
        assert!(preview.errors.is_empty(), "{name}: {:?}", preview.errors);
        let curly_files: BTreeSet<String> = analyzer::analyze_files(
            &root,
            &project::collect_php_files(&root, &info.excluded_dirs),
            PhpVersion::new(8, 5),
            |_, _, _| {},
        )
        .findings
        .into_iter()
        .filter(|f| f.rule_id == patterns::CURLY_OFFSET.id)
        .map(|f| f.file)
        .collect();
        for file in &curly_files {
            let change = preview.changes.iter().find(|c| &c.file == file).unwrap_or_else(|| panic!("{name}: {file} not fixed"));
            assert!(change.rules.iter().any(|r| r.name == "CurlyToSquareBracketArrayStringRector"), "{name}: {file}");
            assert!(change.rules.len() > 1 || name != "kuzey-magaza", "{name}: Rector skipped {file}");
        }
        assert!(!preview.changes.is_empty(), "{name}: Rector found nothing");
        assert!(preview.changes.iter().all(|c| !c.file.starts_with("vendor/")), "{name}");
    }
}
