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
        &std::sync::atomic::AtomicBool::new(false),
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

#[test]
fn rector_sees_the_same_files_as_the_scanner() {
    let Some(tools) = std::env::var_os("FERRY_TEST_TOOLS").map(PathBuf::from) else {
        return;
    };
    let php = toolchain::find_php(None).expect("php gerekli");
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("vendor/[acme] app");
    let nullable = "<?php\nfunction f(Foo $a = null) {}\n";
    for file in ["variables.php", "src/x.inc", "src/m.module", "src/t.phtml", "web/core.install", "var/cache/c.php", "lib/vendor/v.php"] {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, nullable).unwrap();
    }
    let work = tempfile::tempdir().unwrap();
    let preview = rector::preview(
        &php,
        &toolchain::rector_entry(&tools),
        &root,
        PhpVersion::new(8, 4),
        rector::RectorMode::Compat,
        &["var".to_string()],
        work.path(),
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    assert!(preview.errors.is_empty(), "{:?}", preview.errors);
    let mut changed: Vec<&str> = preview.changes.iter().map(|c| c.file.as_str()).collect();
    changed.sort();
    assert_eq!(changed, ["src/m.module", "src/t.phtml", "src/x.inc", "variables.php", "web/core.install"]);
}

#[test]
fn project_index_sees_polyfills_and_class_hierarchies() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let write = |rel: &str, body: &str| {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    };
    write("lib/polyfill.php", "<?php\nif (!function_exists('each')) {\n    function each(&$a) { return []; }\n}\n");
    write("src/uses.php", "<?php\nnamespace App;\n$x = each($arr);\n$y = \\each($arr);\nmysql_query('x');\nereg('a', 'b');\n");
    write("vendor/shim/mysql.php", "<?php\nfunction mysql_query($q) {}\n");
    write("vendor/composer/autoload_files.php", "<?php\n$vendorDir = dirname(__DIR__);\nreturn array('abc' => $vendorDir . '/shim/mysql.php');\n");
    write("src/Base.php", "<?php\nnamespace App;\nclass Base { protected $name; }\n");
    write("src/Child.php", "<?php\nnamespace App;\nclass Child extends Base {\n    function f() { $this->name = 1; $this->extra = 2; }\n}\n");
    write("src/Open.php", "<?php\nclass Open extends \\stdClass { function f() { $this->any = 1; } }\n");
    write("src/Magic.php", "<?php\nclass MagicBase { function __set($n, $v) {} }\nclass Kid extends MagicBase { function f() { $this->x = 1; } }\n");
    write("src/Post.php", "<?php\ntrait HasTitle { public $title; }\nclass Post { use HasTitle;\n    function f() { $this->title = 1; $this->body = 2; }\n}\n");
    write("src/Vendor.php", "<?php\nclass UsesFramework extends \\Illuminate\\Database\\Eloquent\\Model { function f() { $this->x = 1; } }\n");
    write("src/Allowed.php", "<?php\n#[\\AllowDynamicProperties]\nclass Loose {}\nclass Looser extends Loose { function f() { $this->z = 1; } }\n");

    let info = project::inspect(root).unwrap();
    let files = project::collect_php_files(root, &info.excluded_dirs);
    let scan = analyzer::analyze_files(root, &files, PhpVersion::new(8, 4), |_, _, _| {});
    let mut found: Vec<String> = scan.findings.iter().map(|f| format!("{}:{}:{}", f.file, f.line, f.rule_id)).collect();
    found.sort();
    assert_eq!(
        found,
        [
            "src/Child.php:4:deprecated.dynamic_property",
            "src/Post.php:4:deprecated.dynamic_property",
            "src/uses.php:6:removed.ereg",
        ]
    );
    let titles: Vec<&str> = scan.findings.iter().filter(|f| f.rule_id == "deprecated.dynamic_property").map(|f| f.snippet.as_str()).collect();
    assert!(titles.iter().all(|s| s.contains("extra") || s.contains("body")), "{titles:?}");
}
