use ferry_core::deps::{self, DependencyStatus};
use ferry_core::project::{self, Framework};
use ferry_core::rules::{patterns, FixKind, CONSTANTS, FUNCTIONS, MEMBERS, TYPE_CURRENCY_CALL};
use ferry_core::{analyzer, report, PhpVersion};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

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
        patterns::PHP4_CONSTRUCTOR.id,
        patterns::NESTED_TERNARY.id,
        patterns::CASE_INSENSITIVE_CONSTANT.id,
        patterns::PARSE_STR_ONE_ARG.id,
        patterns::ASSERT_STRING.id,
        patterns::BAREWORD_KEY.id,
        patterns::CSV_ESCAPE.id,
        patterns::MB_ENCODING.id,
        patterns::RESERVED_NAME.id,
        patterns::RETURN_TYPE_WILL_CHANGE.id,
        patterns::HTTP_RESPONSE_HEADER.id,
        patterns::IMPLODE_ORDER.id,
        patterns::EMPTY_CALL.id,
        patterns::SESSION_HANDLER_ARGS.id,
        patterns::INI_SETTING.id,
        patterns::NULL_OFFSET.id,
        patterns::CASE_SEMICOLON.id,
        patterns::CONCAT_PRECEDENCE.id,
        patterns::SERIALIZABLE.id,
        patterns::REMOVED_VARIABLE.id,
        patterns::DYNAMIC_PROPERTY.id,
        patterns::OPTIONAL_BEFORE_REQUIRED.id,
        patterns::NULL_TO_INTERNAL.id,
        patterns::PARTIAL_CALLABLE.id,
        patterns::INCOMPATIBLE_OVERRIDE.id,
        patterns::INCOMPATIBLE_USER_OVERRIDE.id,
        patterns::BUILTIN_RESULT_BY_REF.id,
        patterns::ASSIGN_NEW_BY_REF.id,
        patterns::UNSET_THIS.id,
        patterns::BREAK_OUTSIDE_LOOP.id,
    ];
    let missing: Vec<&str> = FUNCTIONS
        .iter()
        .chain(CONSTANTS)
        .map(|r| r.id)
        .chain(MEMBERS.iter().map(|m| m.rule.id))
        .chain([TYPE_CURRENCY_CALL.id])
        .chain(pattern_ids)
        .filter(|id| !found.contains(id))
        .collect();
    assert!(missing.is_empty(), "demo never triggers: {missing:?}");
    let own_fixes = [
        "deprecated.mysqli_no_data",
        "deprecated.mysqli_is_mariadb",
        "deprecated.mysqli_update_max_length",
        "deprecated.dom_php_err",
        "deprecated.pgsql_transaction_constant",
        "deprecated.pgsql_libpq_version_str",
        "deprecated.u_multiple_decimal_seperators",
        "deprecated.oci8_alias",
        "removed.mb_overload",
        "removed.filter_sanitize_magic_quotes",
        "deprecated.openssl_x509_free",
        "deprecated.openssl_pkey_free",
        "deprecated.openssl_free_key",
        "deprecated.shmop_close",
        "removed.oci_internal_debug",
        "removed.set_magic_quotes_runtime",
        "removed.magic_quotes_runtime",
        "deprecated.curlopt_binarytransfer",
        "deprecated.file_binary",
        "deprecated.file_text",
        "removed.filter_flag_scheme_required",
        "removed.filter_flag_host_required",
        "deprecated.curlpipe_http1",
        "deprecated.e_strict",
        "removed.unset_this",
        "deprecated.mhash",
        "deprecated.mhash_constant",
        "deprecated.mysqli_kill",
        "deprecated.mysqli_refresh",
        "deprecated.mysqli_refresh_constant",
        "deprecated.intlcal_set",
        "deprecated.intlgregcal_create_instance",
        "deprecated.lcg_value",
        "removed.get_magic_quotes_gpc",
        "removed.get_magic_quotes_runtime",
        "deprecated.pdo_fetch_serialize",
        "deprecated.mysqli_store_result_mode",
        "deprecated.mysqli_store_result_copy_data",
        "removed.datefmt_set_timezone_id",
        "removed.reflection_export",
        "removed.ldap_control_paged_result",
        "removed.ldap_control_paged_result_response",
        "deprecated.assert_options",
        "deprecated.assert_constants",
        "deprecated.zip_fl_recompress",
        "removed.fgetss",
        "removed.gzgetss",
        "removed.recode",
        "removed.ezmlm_hash",
        "removed.sql_regcase",
    ];
    for id in own_fixes {
        assert!(result.findings.iter().any(|f| f.rule_id == id && f.fix != FixKind::Manual), "{id} isn't fixed in the demo");
    }

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
    assert!(result.findings.iter().all(|f| f.file.starts_with("wp-content/") || f.rule_id == "framework.wordpress_version"));
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
    let framework = deps.packages.iter().find(|p| p.name == "laravel/framework").unwrap();
    assert_eq!(framework.status, DependencyStatus::Incompatible, "Laravel 10 stops at PHP 8.3");
    assert!(framework.note.is_some());
    assert!(deps.packages.iter().filter(|p| p.name != "laravel/framework").all(|p| p.status == DependencyStatus::Compatible));
}

#[test]
fn rector_handles_every_demo_project() {
    let Some(tools) = ferry_core::toolchain::test_tools() else {
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
        let curly_files: BTreeSet<String> =
            analyzer::analyze_files(&root, &project::collect_php_files(&root, &info.excluded_dirs), PhpVersion::new(8, 5), |_, _, _| {})
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
        if name == "kuzey-magaza" {
            let report = preview.changes.iter().find(|c| c.file == "src/Legacy/OldReport.php").expect("OldReport fixed");
            assert!(report.new_content.contains("$row['status']"), "bare keys are quoted");
            assert!(!report.new_content.contains("STATUS"), "Rector must not turn bare keys into constants");
            assert!(report.rules.iter().any(|r| r.name == "QuoteBarewordKeys"));
        }
    }
}

#[test]
fn auto_findings_are_fixed_by_rector() {
    let Some(tools) = ferry_core::toolchain::test_tools() else {
        return;
    };
    let php = ferry_core::toolchain::find_php(None).expect("php gerekli");
    let mut unfixed = BTreeSet::new();
    for name in ["kuzey-magaza", "wp-blog", "laravel-api"] {
        let root = demo(name);
        let info = project::inspect(&root).unwrap();
        let work = tempfile::tempdir().unwrap();
        let target = PhpVersion::new(8, 5);
        let preview = ferry_core::rector::preview(
            &php,
            &ferry_core::toolchain::rector_entry(&tools),
            &root,
            target,
            ferry_core::rector::RectorMode::Compat,
            &info.excluded_dirs,
            work.path(),
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
        let findings =
            analyzer::analyze_files(&root, &project::collect_php_files(&root, &info.excluded_dirs), target, |_, _, _| {}).findings;
        for f in findings.iter().filter(|f| f.fix != ferry_core::rules::FixKind::Manual) {
            let Some(change) = preview.changes.iter().find(|c| c.file == f.file) else {
                unfixed.insert(format!("{} ({:?}) {}:{}", f.rule_id, f.fix, f.file, f.line));
                continue;
            };
            let touched = if FUNCTIONS.iter().chain(CONSTANTS).any(|r| r.id == f.rule_id) {
                let source = std::fs::read_to_string(root.join(&f.file)).unwrap();
                let line = source.lines().nth(f.line - 1).unwrap_or_default();
                let word: String = line.chars().skip(f.column - 1).take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                let count = |s: &str| {
                    let lower = s.to_lowercase();
                    let w = word.to_lowercase();
                    lower
                        .match_indices(&w)
                        .filter(|(i, _)| {
                            let before = lower[..*i].chars().last();
                            let after = lower[i + w.len()..].chars().next();
                            !before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '$')
                                && !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
                        })
                        .count()
                };
                !word.is_empty() && count(&change.new_content) < count(&source)
            } else {
                change
                    .diff
                    .hunks
                    .iter()
                    .flat_map(|h| &h.lines)
                    .any(|l| l.kind == ferry_core::diff::LineKind::Del && l.old_no == Some(f.line))
            };
            if !touched {
                unfixed.insert(format!("{} ({:?}) {}:{}", f.rule_id, f.fix, f.file, f.line));
            }
        }
    }
    assert!(unfixed.is_empty(), "classified as fixable but Rector left them:\n{}", unfixed.into_iter().collect::<Vec<_>>().join("\n"));
}

#[test]
fn framework_versions_are_checked_for_demo_projects() {
    let rule_files = |name: &str, target: PhpVersion| -> Vec<(String, String)> {
        let root = demo(name);
        let info = project::inspect(&root).unwrap();
        analyzer::analyze_files(&root, &project::collect_php_files(&root, &info.excluded_dirs), target, |_, _, _| {})
            .findings
            .into_iter()
            .filter(|f| f.rule_id.starts_with("framework."))
            .map(|f| (f.rule_id, f.file))
            .collect()
    };
    assert!(rule_files("laravel-api", PhpVersion::new(8, 3)).is_empty());
    assert_eq!(rule_files("laravel-api", PhpVersion::new(8, 4)), [("framework.laravel_version".to_string(), "composer.json".to_string())]);
    assert!(rule_files("wp-blog", PhpVersion::new(8, 2)).is_empty());
    assert_eq!(
        rule_files("wp-blog", PhpVersion::new(8, 4)),
        [("framework.wordpress_version".to_string(), "wp-includes/version.php".to_string())]
    );
}
