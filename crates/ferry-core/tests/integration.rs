use ferry_core::{analyzer, project, rector, report, snapshot, toolchain, PhpVersion};
use std::path::{Path, PathBuf};

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
fn a_step_leaves_newer_rules_for_later() {
    let Some(tools) = ferry_core::toolchain::test_tools() else {
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
        PhpVersion::new(8, 0),
        rector::RectorMode::Compat,
        &[],
        work.path(),
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    assert!(preview.errors.is_empty(), "{:?}", preview.errors);
    let names: Vec<_> = preview.changes.iter().flat_map(|c| &c.rules).map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"WhileEachToForeachRector"), "{names:?}");
    assert!(!names.contains(&"ExplicitNullableParamTypeRector"), "{names:?}");
}

#[test]
fn rector_preview_apply_and_restore() {
    let Some(tools) = ferry_core::toolchain::test_tools() else {
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
    let Some(tools) = ferry_core::toolchain::test_tools() else {
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
    let excluded = ["var".to_string()];
    let mut scanned: Vec<String> = project::collect_php_files(&root, &excluded).iter().map(|f| analyzer::relative_path(&root, f)).collect();
    scanned.sort();
    assert!(scanned.contains(&"variables.php".to_string()) && !scanned.iter().any(|f| f.starts_with("var/")), "{scanned:?}");
    let work = tempfile::tempdir().unwrap();
    let preview = rector::preview(
        &php,
        &toolchain::rector_entry(&tools),
        &root,
        PhpVersion::new(8, 4),
        rector::RectorMode::Compat,
        &excluded,
        work.path(),
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    assert!(preview.errors.is_empty(), "{:?}", preview.errors);
    let mut changed: Vec<String> = preview.changes.iter().map(|c| c.file.clone()).collect();
    changed.sort();
    assert_eq!(changed, scanned);
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
    write(
        "vendor/composer/autoload_files.php",
        "<?php\n$vendorDir = dirname(__DIR__);\nreturn array('abc' => $vendorDir . '/shim/mysql.php');\n",
    );
    write("src/Base.php", "<?php\nnamespace App;\nclass Base { protected $name; }\n");
    write(
        "src/Child.php",
        "<?php\nnamespace App;\nclass Child extends Base {\n    function f() { $this->name = 1; $this->extra = 2; }\n}\n",
    );
    write("src/Open.php", "<?php\nclass Open extends \\stdClass { function f() { $this->any = 1; } }\n");
    write(
        "src/Magic.php",
        "<?php\nclass MagicBase { function __set($n, $v) {} }\nclass Kid extends MagicBase { function f() { $this->x = 1; } }\n",
    );
    write("src/Post.php", "<?php\ntrait HasTitle { public $title; }\nclass Post { use HasTitle;\n    function f() { $this->title = 1; $this->body = 2; }\n}\n");
    write(
        "src/Vendor.php",
        "<?php\nclass UsesFramework extends \\Illuminate\\Database\\Eloquent\\Model { function f() { $this->x = 1; } }\n",
    );
    write(
        "src/Allowed.php",
        "<?php\n#[\\AllowDynamicProperties]\nclass Loose {}\nclass Looser extends Loose { function f() { $this->z = 1; } }\n",
    );

    let info = project::inspect(root).unwrap();
    let files = project::collect_php_files(root, &info.excluded_dirs);
    let scan = analyzer::analyze_files(root, &files, PhpVersion::new(8, 4), |_, _, _| {});
    let mut found: Vec<String> = scan.findings.iter().map(|f| format!("{}:{}:{}", f.file, f.line, f.rule_id)).collect();
    found.sort();
    assert_eq!(
        found,
        ["src/Child.php:4:deprecated.dynamic_property", "src/Post.php:4:deprecated.dynamic_property", "src/uses.php:6:removed.ereg",]
    );
    let titles: Vec<&str> =
        scan.findings.iter().filter(|f| f.rule_id == "deprecated.dynamic_property").map(|f| f.snippet.as_str()).collect();
    assert!(titles.iter().all(|s| s.contains("extra") || s.contains("body")), "{titles:?}");
}

#[test]
fn laravel_projects_get_framework_findings() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let write = |rel: &str, body: &str| {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    };
    write("artisan", "#!/usr/bin/env php\n<?php\n");
    write("composer.json", "{\n  \"require\": {\n    \"laravel/framework\": \"^5.8\"\n  }\n}\n");
    write("composer.lock", r#"{"packages":[{"name":"laravel/framework","version":"v5.8.38"}]}"#);
    write("app/Http/Controllers/PostController.php", "<?php\nnamespace App\\Http\\Controllers;\nuse Input;\nclass PostController {\n    public function store() {\n        $slug = str_slug(Input::get('title'));\n        return array_get($this->data, 'x') . str_contains($slug, 'a');\n    }\n}\n");

    let scan = |root: &Path| {
        let info = project::inspect(root).unwrap();
        let files = project::collect_php_files(root, &info.excluded_dirs);
        let mut ids: Vec<String> = analyzer::analyze_files(root, &files, PhpVersion::new(8, 1), |_, _, _| {})
            .findings
            .into_iter()
            .map(|f| format!("{}:{}:{}", f.file, f.line, f.title.en))
            .collect();
        ids.sort();
        ids
    };
    assert_eq!(
        scan(root),
        [
            "app/Http/Controllers/PostController.php:6:Input::get()",
            "app/Http/Controllers/PostController.php:6:str_slug()",
            "app/Http/Controllers/PostController.php:7:array_get()",
            "composer.json:3:Laravel 5",
        ]
    );

    write("vendor/laravel/helpers/src/helpers.php", "<?php\nif (!function_exists('str_slug')) { function str_slug($s) {} }\nif (!function_exists('array_get')) { function array_get($a, $k) {} }\n");
    write(
        "vendor/composer/autoload_files.php",
        "<?php\n$vendorDir = dirname(__DIR__);\nreturn array('h' => $vendorDir . '/laravel/helpers/src/helpers.php');\n",
    );
    assert_eq!(scan(root), ["app/Http/Controllers/PostController.php:6:Input::get()", "composer.json:3:Laravel 5"]);
}

#[test]
fn a_polyfill_behind_a_dead_version_check_does_not_hide_a_removed_function() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("poly.php"), "<?php function each(&$a) { return null; }\n").unwrap();
    std::fs::write(root.join("a.php"), "<?php\nif (PHP_VERSION_ID < 80000) { require 'poly.php'; }\neach($x);\n").unwrap();
    let files = vec![root.join("a.php"), root.join("poly.php")];
    let scan = ferry_core::analyzer::analyze_files(root, &files, PhpVersion::new(8, 5), |_, _, _| {});
    assert!(
        scan.findings.iter().any(|f| f.file == "a.php" && f.line == 3),
        "{:?}",
        scan.findings.iter().map(|f| &f.rule_id).collect::<Vec<_>>()
    );
}

#[test]
fn rector_leaves_alone_what_it_would_break() {
    let Some(tools) = ferry_core::toolchain::test_tools() else {
        eprintln!("FERRY_TEST_TOOLS ayarlı değil; Rector testi atlandı");
        return;
    };
    let php = toolchain::find_php(None).expect("php gerekli");
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let files = [
        (
            "parse.php",
            "<?php\nclass Report\n{\n    function page($query)\n    {\n        parse_str($query);\n        return isset($page) ? (int) $page : 1;\n    }\n}\n",
            "parse_str($query);",
        ),
        ("export.php", "<?php\nfunction dump() {\n    ReflectionFunction::export('strlen');\n}\n", "echo new ReflectionFunction('strlen'), \"\\n\";"),
        (
            "quotes.php",
            "<?php\nfunction quote($a) {\n    return filter_var($a, FILTER_SANITIZE_MAGIC_QUOTES, FILTER_REQUIRE_ARRAY);\n}\n",
            "filter_var($a, FILTER_SANITIZE_ADD_SLASHES, FILTER_REQUIRE_ARRAY)",
        ),
        ("money.php", "<?php\nfunction price($v) {\n    return money_format('%i', $v);\n}\n", "money_format('%i', $v)"),
        ("stop.php", "<?php\necho 'a';\nbreak;\necho 'b';\n", "break;"),
        (
            "wbmp.php",
            "<?php\nfunction thumb($png) {\n    png2wbmp($png, 'png.wbmp', 32, 32, 7);\n    return read_exif_data($png);\n}\n",
            "png2wbmp($png, 'png.wbmp', 32, 32, 7);",
        ),
        ("exif.php", "<?php\nfunction meta($f) {\n    return read_exif_data($f);\n}\n", "exif_read_data($f)"),
    ];
    for (name, src, _) in &files {
        std::fs::write(root.join(name), src).unwrap();
    }
    let work = tempfile::tempdir().unwrap();
    let preview = rector::preview(
        &php,
        &toolchain::rector_entry(&tools),
        root,
        PhpVersion::new(8, 4),
        rector::RectorMode::Compat,
        &[],
        work.path(),
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    assert!(preview.errors.is_empty(), "{:?}", preview.errors);
    let broken: Vec<String> = files
        .iter()
        .filter_map(|(name, src, kept)| {
            let after = preview.changes.iter().find(|c| c.file == *name).map_or(*src, |c| c.new_content.as_str());
            (!after.contains(kept)).then(|| format!("{name}:\n{after}"))
        })
        .collect();
    assert!(broken.is_empty(), "{}", broken.join("\n"));
    let wbmp = preview.changes.iter().find(|c| c.file == "wbmp.php").expect("wbmp.php keeps its other renames");
    assert!(wbmp.new_content.contains("exif_read_data($png)"), "{}", wbmp.new_content);
}
