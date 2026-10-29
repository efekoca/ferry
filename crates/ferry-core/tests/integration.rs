use std::path::{Path, PathBuf};
use ferry_core::{analyzer, project, rector, report, snapshot, toolchain, PhpVersion};

fn copy_fixture() -> (tempfile::TempDir, PathBuf) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/legacy");
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("legacy");
    for entry in walkdir::WalkDir::new(&src) {
        let entry = entry.unwrap();
        let dest = root.join(entry.path().strip_prefix(&src).unwrap());
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&dest).unwrap();
        } else {
            std::fs::copy(entry.path(), &dest).unwrap();
        }
    }
    (dir, root)
}

#[test]
fn scan_fixture_project() {
    let (_dir, root) = copy_fixture();
    let info = project::inspect(&root).unwrap();
    assert_eq!(info.php_files, 2, "vendor must be skipped");
    assert_eq!(info.current_version, Some(PhpVersion::new(7, 1)));

    let files = project::collect_php_files(&root, &info.excluded_dirs);
    let mut calls = 0;
    let scan = analyzer::analyze_files(&root, &files, PhpVersion::new(8, 4), |_, _, _| calls += 1);
    assert_eq!(calls, 3, "progress for each file plus completion");
    let rules: Vec<_> = scan.findings.iter().map(|f| f.rule_id.as_str()).collect();
    for expected in [
        "removed.mysql",
        "removed.each",
        "deprecated.dollar_brace_interpolation",
        "deprecated.utf8_encode",
        "deprecated.get_class_no_args",
        "deprecated.implicit_nullable",
        "review.eval",
    ] {
        assert!(rules.contains(&expected), "missing {expected} in {rules:?}");
    }
    let summary = report::summarize(&scan.findings);
    assert!(summary.breaking >= 3 && summary.score < 100);
}

#[test]
fn rector_preview_apply_and_restore() {
    let Some(tools) = std::env::var_os("FERRY_TEST_TOOLS").map(PathBuf::from) else {
        eprintln!("FERRY_TEST_TOOLS ayarlı değil; Rector testi atlandı");
        return;
    };
    let php = toolchain::find_php(None).expect("php gerekli");
    let (_dir, root) = copy_fixture();
    let work = tempfile::tempdir().unwrap();
    let preview = rector::preview(
        &php,
        &toolchain::rector_entry(&tools),
        &root,
        PhpVersion::new(8, 4),
        rector::RectorMode::Compat,
        &[],
        work.path(),
    )
    .unwrap();
    assert!(preview.errors.is_empty(), "{:?}", preview.errors);
    let files: Vec<&str> = preview.changes.iter().map(|c| c.file.as_str()).collect();
    let greeter = preview
        .changes
        .iter()
        .find(|c| c.file == "src/Greeter.php")
        .unwrap_or_else(|| panic!("Greeter changes missing; root {} changed {files:?}", root.display()));
    let names: Vec<_> = greeter.rules.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"ExplicitNullableParamTypeRector"), "{names:?}");
    assert!(names.contains(&"WhileEachToForeachRector"), "{names:?}");
    assert!(!names.contains(&"StrContainsRector"), "compat mode must not modernize: {names:?}");
    assert!(preview.changes.iter().all(|c| !c.file.starts_with("vendor/")));

    let store = snapshot::SnapshotStore::new(work.path().join("snapshots"));
    let original = std::fs::read_to_string(root.join("src/Greeter.php")).unwrap();
    let snap = store.create(&root, std::slice::from_ref(&greeter.file), "test").unwrap();
    rector::apply(&root, &[greeter]).unwrap();

    let updated = std::fs::read_to_string(root.join("src/Greeter.php")).unwrap();
    assert!(updated.contains("?Foo $foo = null"));
    assert!(updated.contains("foreach ($this->items as $k => $v)"));
    let lint = toolchain::command(&php).arg("-l").arg(root.join("src/Greeter.php")).output().unwrap();
    assert!(lint.status.success(), "{}", String::from_utf8_lossy(&lint.stdout));

    store.restore(&root, &snap.id).unwrap();
    assert_eq!(std::fs::read_to_string(root.join("src/Greeter.php")).unwrap(), original);
}
