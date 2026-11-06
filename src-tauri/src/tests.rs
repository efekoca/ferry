use super::*;
use serde_json::{json, Value};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;

const APP_ORIGIN: &str = if cfg!(windows) { "http://tauri.localhost" } else { "tauri://localhost" };

static LANG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn turkish() -> std::sync::MutexGuard<'static, ()> {
    let guard = LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    i18n::set_lang(Lang::Tr);
    guard
}

struct Harness {
    _app: tauri::App<MockRuntime>,
    webview: WebviewWindow<MockRuntime>,
    _dirs: Vec<tempfile::TempDir>,
    root: String,
}

fn fixture() -> (tempfile::TempDir, PathBuf) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/ferry-core/tests/fixtures/legacy");
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

fn harness() -> Harness {
    let data = tempfile::tempdir().unwrap();
    let (project_dir, root) = fixture();
    let mut state = AppState::new(data.path().to_path_buf());
    if let Some(tools) = std::env::var_os("FERRY_TEST_TOOLS") {
        state.tools_dir = PathBuf::from(tools);
    }
    let app = with_commands(mock_builder()).manage(state).build(mock_context(noop_assets())).unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build().unwrap();
    Harness { _app: app, webview, _dirs: vec![data, project_dir], root: root.to_string_lossy().to_string() }
}

impl Harness {
    fn invoke(&self, cmd: &str, args: Value) -> Result<Value, Value> {
        get_ipc_response(
            &self.webview,
            InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: APP_ORIGIN.parse().unwrap(),
                body: InvokeBody::Json(args),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )
        .map(|body| body.deserialize::<Value>().unwrap())
    }

    fn ok(&self, cmd: &str, args: Value) -> Value {
        self.invoke(cmd, args.clone()).unwrap_or_else(|e| panic!("{cmd}({args}) failed: {e}"))
    }
}

#[test]
fn scan_flow_matches_ui_contract() {
    let _lang = turkish();
    let h = harness();
    let root = h.root.clone();

    let info = h.ok("inspect_project", json!({ "path": root }));
    assert_eq!(info["phpFiles"], 2);
    assert_eq!(info["framework"], "generic");
    assert_eq!(info["currentVersion"], "7.1");

    let scan = h.ok("scan_project", json!({ "path": root, "target": "8.4", "run": 1 }));
    assert_eq!(scan["target"], "8.4");
    assert_eq!(scan["filesScanned"], 2);
    let findings = scan["findings"].as_array().unwrap();
    assert!(findings.iter().any(|f| f["ruleId"] == "removed.mysql" && f["fix"] == "manual"));
    assert!(findings.iter().any(|f| f["severity"] == "breaking"));
    assert_eq!(scan["summary"]["total"], findings.len());
    assert!(scan["summary"]["score"].as_u64().unwrap() < 100);

    let deps = h.ok("check_dependencies", json!({ "path": root, "target": "8.4" }));
    assert_eq!(deps["rootStatus"], "incompatible");
    assert_eq!(deps["hasLockFile"], false);

    let dest = Path::new(&root).join("rapor.md");
    h.ok("export_report", json!({ "path": root, "destination": dest }));
    let md = std::fs::read_to_string(&dest).unwrap();
    assert!(md.contains("# Ferry raporu: legacy"));

    assert_eq!(h.ok("list_snapshots", json!({ "path": root })), json!([]));
    let store = h._app.state::<AppState>().snapshots();
    let canonical = PathBuf::from(normalize_path(root.clone()));
    store.create(&canonical, &["composer.json".into()], "x").unwrap();
    let spelled = format!("{root}/./");
    assert_eq!(
        h.ok("list_snapshots", json!({ "path": spelled })).as_array().unwrap().len(),
        1,
        "same history for any spelling of the path"
    );
    let tools = h.ok("toolchain_status", json!({}));
    assert!(tools["platform"].is_string());
}

#[test]
fn rejects_bad_input_with_messages() {
    let _lang = turkish();
    let h = harness();
    let err = h.invoke("scan_project", json!({ "path": h.root, "target": "7.4", "run": 1 })).unwrap_err();
    assert!(err.as_str().unwrap().contains("Desteklenmeyen hedef sürüm"));
    let err = h.invoke("inspect_project", json!({ "path": "/yok/boyle/bir/klasor" })).unwrap_err();
    assert!(err.as_str().unwrap().contains("Klasör bulunamadı"));
    let file = Path::new(&h.root).join("composer.json");
    let err = h.invoke("inspect_project", json!({ "path": file })).unwrap_err();
    assert!(err.as_str().unwrap().contains("dosya, proje klasörü değil"), "{err}");
    let err = h.invoke("export_report", json!({ "path": h.root, "destination": "/tmp/x.md" })).unwrap_err();
    assert!(err.as_str().unwrap().contains("Önce projeyi tarayın"));
    let err = h.invoke("apply_changes", json!({ "path": h.root, "previewId": 1, "files": ["a.php"], "label": "x" })).unwrap_err();
    assert!(err.as_str().unwrap().contains("Önce değişiklikleri hesaplayın"));
}

#[test]
fn messages_follow_the_ui_language() {
    let _lang = LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let h = harness();
    h.ok("set_language", json!({ "language": "en" }));
    let err = h.invoke("scan_project", json!({ "path": h.root, "target": "7.4", "run": 1 })).unwrap_err();
    h.ok("set_language", json!({ "language": "tr" }));
    assert!(err.as_str().unwrap().contains("Unsupported target version"));
}

#[test]
fn writes_a_probe_with_its_token() {
    let h = harness();
    let dest = Path::new(&h.root).join("ferry-probe.php");
    let token = h.ok("write_probe", json!({ "destination": dest }));
    let text = std::fs::read_to_string(&dest).unwrap();
    assert!(text.contains(&format!("'{}'", token.as_str().unwrap())));
}

#[test]
fn reports_uncommitted_files_in_git_projects() {
    let h = harness();
    let none = h.ok("git_status", json!({ "path": h.root, "files": ["src/db.php"] }));
    assert_eq!(none["repo"], false);
    if toolchain::find_executable("git").is_none() {
        return;
    }
    let git = |args: &[&str]| std::process::Command::new("git").arg("-C").arg(&h.root).args(args).output().unwrap();
    git(&["init", "-q"]);
    git(&["-c", "user.email=t@example.com", "-c", "user.name=t", "add", "."]);
    git(&["-c", "user.email=t@example.com", "-c", "user.name=t", "commit", "-qm", "init"]);
    std::fs::write(Path::new(&h.root).join("src/db.php"), "<?php\n// edited\n").unwrap();
    let state = h.ok("git_status", json!({ "path": h.root, "files": ["src/db.php", "src/Greeter.php"] }));
    assert_eq!(state, json!({ "repo": true, "dirty": ["src/db.php"] }));

    std::fs::write(Path::new(&h.root).join("src/new file ü.php"), "<?php\n").unwrap();
    let state = h.ok("git_status", json!({ "path": h.root, "files": ["src/new file ü.php", "src/Greeter.php"] }));
    assert_eq!(state["dirty"], json!(["src/new file ü.php"]), "untracked files and unusual names count");

    let outer = Path::new(&h.root).parent().unwrap();
    let git_outer = |args: &[&str]| std::process::Command::new("git").arg("-C").arg(outer).args(args).output().unwrap();
    std::fs::remove_dir_all(Path::new(&h.root).join(".git")).unwrap();
    git_outer(&["init", "-q"]);
    git_outer(&["-c", "user.email=t@example.com", "-c", "user.name=t", "add", "."]);
    git_outer(&["-c", "user.email=t@example.com", "-c", "user.name=t", "commit", "-qm", "init"]);
    std::fs::write(Path::new(&h.root).join("src/Greeter.php"), "<?php\n// edited\n").unwrap();
    let state = h.ok("git_status", json!({ "path": h.root, "files": ["src/Greeter.php"] }));
    assert_eq!(state["dirty"], json!(["src/Greeter.php"]), "paths stay relative to a project inside a bigger repository");
}

#[test]
fn porcelain_output_keeps_renames_and_spaces() {
    let out = b"R  sub/new name.php\0sub/old.php\0?? sub/x -> y.php\0 M other.php\0";
    assert_eq!(git::porcelain_paths(out, "sub/"), ["new name.php", "x -> y.php", "other.php"]);
}

#[test]
fn compares_every_selectable_target() {
    let h = harness();
    let rows = h.ok("compare_targets", json!({ "path": h.root }));
    let targets: Vec<&str> = rows.as_array().unwrap().iter().map(|r| r["target"].as_str().unwrap()).collect();
    assert_eq!(targets, ["8.2", "8.3", "8.4", "8.5"]);
    assert!(rows[0]["summary"]["breaking"].as_u64().unwrap() > 0);
    assert_eq!(rows[3]["rootBlocks"], true);
}

#[test]
fn settings_round_trip() {
    let h = harness();
    assert_eq!(h.ok("get_settings", json!({})), json!({ "phpPath": null, "snapshotLimit": null, "editor": null, "editorCommand": null }));
    h.ok("save_settings", json!({ "settings": { "phpPath": "/opt/php/bin/php", "editor": "vscode" } }));
    assert_eq!(
        h.ok("get_settings", json!({})),
        json!({ "phpPath": "/opt/php/bin/php", "snapshotLimit": null, "editor": "vscode", "editorCommand": null })
    );
    h.ok("save_settings", json!({ "settings": { "phpPath": null, "snapshotLimit": 10 } }));
    assert_eq!(h.ok("get_settings", json!({}))["snapshotLimit"], 10);
}

#[test]
fn snapshot_changes_fall_back_to_a_fresh_diff() {
    let h = harness();
    let root = PathBuf::from(normalize_path(h.root.clone()));
    let store = h._app.state::<AppState>().snapshots();
    let snap = store.create(&root, &["src/Greeter.php".into()], "eski").unwrap();
    let file = root.join("src/Greeter.php");
    let edited = std::fs::read_to_string(&file).unwrap().replacen("<?php", "<?php\n// edited", 1);
    std::fs::write(&file, edited).unwrap();

    let changes = h.ok("snapshot_changes", json!({ "path": h.root, "id": snap.id }));
    assert_eq!(changes[0]["file"], "src/Greeter.php");
    assert_eq!(changes[0]["rules"], json!([]));
    assert_eq!(changes[0]["diff"]["additions"], 1);
    assert!(h.invoke("snapshot_changes", json!({ "path": h.root, "id": "nope" })).is_err());
    let conflicts = h.ok("snapshot_conflicts", json!({ "path": h.root, "id": snap.id }));
    assert_eq!(conflicts[0]["file"], "src/Greeter.php", "without an apply record every edited file needs a confirmation");
}

#[test]
fn fixes_can_stop_at_a_version_on_the_way() {
    assert_eq!(parse_step("7.4").unwrap(), PhpVersion::new(7, 4));
    assert_eq!(parse_step("8.5").unwrap(), PhpVersion::new(8, 5));
    assert!(parse_step("5.6").is_err());
    assert!(parse_step("9.0").is_err());
}

#[test]
fn a_check_only_counts_for_its_target_and_the_files_it_ran_on() {
    let log = RuntimeLog {
        file: "php -l · PHP 8.4".into(),
        events: Vec::new(),
        check: None,
        roots: Vec::new(),
        target: Some(PhpVersion::new(8, 4)),
        php: None,
        stamp: Some("a".into()),
    };
    let scan = |target, stamp: &str| ScanResult {
        target,
        files_found: 0,
        files_scanned: 0,
        lines_scanned: 0,
        findings: Vec::new(),
        errors: Vec::new(),
        duration_ms: 0,
        stamp: stamp.into(),
    };
    assert!(log.applies_to(&scan(PhpVersion::new(8, 4), "a")));
    assert!(!log.applies_to(&scan(PhpVersion::new(8, 5), "a")));
    assert!(!log.applies_to(&scan(PhpVersion::new(8, 4), "b")), "the files changed");
    assert!(RuntimeLog { target: None, stamp: None, ..log }.applies_to(&scan(PhpVersion::new(8, 5), "b")));
}

#[test]
fn version_files_move_to_the_target_and_can_be_undone_over_ipc() {
    let h = harness();
    let root = h.root.clone();
    let dockerfile = Path::new(&root).join("Dockerfile");
    std::fs::write(&dockerfile, "FROM php:7.4-cli\n").unwrap();
    let preview = h.ok("rector_preview", json!({ "path": root, "target": "8.4", "mode": "pins" }));
    assert_eq!(preview["mode"], "pins");
    assert_eq!(preview["changes"][0]["file"], "Dockerfile");
    let id = preview["id"].as_u64().unwrap();
    let result = h.ok("apply_changes", json!({ "path": root, "previewId": id, "files": ["Dockerfile"], "label": "pins" }));
    assert_eq!(std::fs::read_to_string(&dockerfile).unwrap(), "FROM php:8.4-cli\n");
    h.ok("restore_snapshot", json!({ "path": root, "id": result["snapshot"]["id"] }));
    assert_eq!(std::fs::read_to_string(&dockerfile).unwrap(), "FROM php:7.4-cli\n");
}

#[test]
fn a_batch_goes_on_its_own_branch_and_a_dirty_tree_can_be_refused() {
    let _lang = turkish();
    let h = harness();
    let root = h.root.clone();
    let dir = Path::new(&root);
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git").arg("-C").arg(dir).args(args).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    std::fs::write(dir.join("Dockerfile"), "FROM php:7.4-cli\n").unwrap();
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.email", "t@example.com"]);
    git(&["config", "user.name", "T"]);
    let config = ferry_core::config::Config {
        git: ferry_core::config::GitConfig { require_clean: true, branch_per_batch: false },
        ..Default::default()
    };
    ferry_core::config::save(dir, &config).unwrap();
    git(&["add", "."]);
    git(&["commit", "-qm", "first"]);

    let id = h.ok("rector_preview", json!({ "path": root, "target": "8.4", "mode": "pins" }))["id"].as_u64().unwrap();
    std::fs::write(dir.join("notes.txt"), "open work\n").unwrap();
    let args = json!({ "path": root, "previewId": id, "files": ["Dockerfile"], "label": "pins", "commitMessage": "Update PHP" });
    let err = h.invoke("apply_changes", args.clone()).unwrap_err();
    assert!(err.as_str().unwrap().contains("temiz çalışma ağacı"), "{err}");
    assert_eq!(git(&["symbolic-ref", "--short", "HEAD"]), "main");

    std::fs::remove_file(dir.join("notes.txt")).unwrap();
    let result = h.ok("apply_changes", args);
    assert_eq!(result["branch"], "ferry/php-8.4-versions");
    assert_eq!(result["commit"].as_str().unwrap(), git(&["rev-parse", "--short", "HEAD"]));
    assert_eq!(git(&["log", "-1", "--format=%s"]), "Update PHP");
    assert_eq!(git(&["status", "--porcelain"]), "");
}

#[test]
fn rector_preview_apply_restore_over_ipc() {
    if toolchain::test_tools().is_none() {
        eprintln!("FERRY_TEST_TOOLS ayarlı değil; Rector IPC testi atlandı");
        return;
    }
    let _lang = turkish();
    let h = harness();
    let root = h.root.clone();
    let preview = h.ok("rector_preview", json!({ "path": root, "target": "8.4", "mode": "compat" }));
    let changes = preview["changes"].as_array().unwrap();
    let greeter = changes.iter().find(|c| c["file"] == "src/Greeter.php").expect("Greeter change");
    assert!(greeter["diff"]["hunks"][0]["lines"][0]["kind"].is_string());
    assert!(greeter.get("newContent").is_none(), "file contents must not be sent to the UI");

    let original = std::fs::read_to_string(Path::new(&root).join("src/Greeter.php")).unwrap();
    let greeter_path = Path::new(&root).join("src/Greeter.php");

    let older = preview["id"].as_u64().unwrap();
    let preview = h.ok("rector_preview", json!({ "path": root, "target": "8.4", "mode": "compat" }));
    let id = preview["id"].as_u64().unwrap();
    assert_ne!(id, older);
    let err =
        h.invoke("apply_changes", json!({ "path": root, "previewId": older, "files": ["src/Greeter.php"], "label": "x" })).unwrap_err();
    assert!(err.as_str().unwrap().contains("güncel değil"), "{err}");

    std::fs::write(&greeter_path, format!("{original}// edited after preview\n")).unwrap();
    assert!(h.invoke("apply_changes", json!({ "path": root, "previewId": id, "files": ["src/Greeter.php"], "label": "x" })).is_err());
    assert_eq!(h.ok("list_snapshots", json!({ "path": root })), json!([]), "no backup for a refused apply");
    assert!(std::fs::read_to_string(&greeter_path).unwrap().ends_with("// edited after preview\n"));
    std::fs::write(&greeter_path, &original).unwrap();

    let applied = h.ok(
        "apply_changes",
        json!({ "path": root, "previewId": id, "files": ["src/Greeter.php"], "label": "Uyumluluk düzeltmeleri · PHP 8.4" }),
    );
    assert_eq!(applied["written"], json!(["src/Greeter.php"]));
    assert_eq!(applied["lintErrors"], json!([]), "Rector's output must still parse");
    let changed = std::fs::read_to_string(Path::new(&root).join("src/Greeter.php")).unwrap();
    assert_ne!(changed, original);

    let snapshots = h.ok("list_snapshots", json!({ "path": root }));
    let snap_id = snapshots[0]["id"].as_str().unwrap().to_string();
    assert_eq!(snapshots[0]["label"], "Uyumluluk düzeltmeleri · PHP 8.4");
    let history = h.ok("snapshot_changes", json!({ "path": root, "id": snap_id }));
    assert_eq!(history[0]["file"], "src/Greeter.php");
    assert_eq!(history[0]["diff"], greeter["diff"]);
    assert!(!history[0]["rules"].as_array().unwrap().is_empty());
    assert_eq!(h.ok("snapshot_conflicts", json!({ "path": root, "id": snap_id })), json!([]));
    std::fs::write(&greeter_path, format!("{changed}// edited after apply\n")).unwrap();
    assert_eq!(h.ok("snapshot_conflicts", json!({ "path": root, "id": snap_id }))[0]["file"], "src/Greeter.php");
    std::fs::write(&greeter_path, &changed).unwrap();

    assert!(h.invoke("apply_changes", json!({ "path": root, "previewId": id, "files": ["src/Greeter.php"], "label": "x" })).is_err());

    h.ok("restore_snapshot", json!({ "path": root, "id": snap_id }));
    assert_eq!(std::fs::read_to_string(Path::new(&root).join("src/Greeter.php")).unwrap(), original);
    h.ok("delete_snapshot", json!({ "path": root, "id": snap_id }));
    assert_eq!(h.ok("list_snapshots", json!({ "path": root })), json!([]));
}

#[test]
fn restore_refuses_files_edited_after_the_confirmation() {
    let _lang = turkish();
    let h = harness();
    let root = PathBuf::from(normalize_path(h.root.clone()));
    let store = h._app.state::<AppState>().snapshots();
    let file = root.join("src/Greeter.php");
    let original = std::fs::read_to_string(&file).unwrap();
    let snap = store.create(&root, &["src/Greeter.php".into()], "eski").unwrap();
    let applied = format!("{original}// applied\n");
    std::fs::write(&file, &applied).unwrap();
    let written = json!({ "src/Greeter.php": fnv1a(applied.as_bytes()) }).to_string();
    store.write_attachment(&root, &snap.id, WRITTEN_FILE, written.as_bytes()).unwrap();

    let first = format!("{applied}// edited later\n");
    std::fs::write(&file, &first).unwrap();
    let err = h.invoke("restore_snapshot", json!({ "path": h.root, "id": snap.id })).unwrap_err();
    assert!(err.as_str().unwrap().contains("src/Greeter.php"), "{err}");
    assert!(std::fs::read_to_string(&file).unwrap().ends_with("// edited later\n"));

    let reviewed = h.ok("snapshot_conflicts", json!({ "path": h.root, "id": snap.id }));
    assert_eq!(reviewed, json!([{ "file": "src/Greeter.php", "hash": content_hash(first.as_bytes()) }]));
    let second = format!("{first}// edited after the review\n");
    std::fs::write(&file, &second).unwrap();
    let err = h.invoke("restore_snapshot", json!({ "path": h.root, "id": snap.id, "acknowledged": reviewed })).unwrap_err();
    assert!(err.as_str().unwrap().contains("onaydan sonra"), "{err}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), second);

    let reviewed = h.ok("snapshot_conflicts", json!({ "path": h.root, "id": snap.id }));
    h.ok("restore_snapshot", json!({ "path": h.root, "id": snap.id, "acknowledged": reviewed }));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), original);

    let kept = h.ok("list_snapshots", json!({ "path": h.root }));
    assert_eq!(kept[0]["label"], "Geri yükleme öncesi · eski", "the overwritten edits stay recoverable");
    let kept_id = kept[0]["id"].as_str().unwrap();
    assert_eq!(h.ok("snapshot_conflicts", json!({ "path": h.root, "id": kept_id })), json!([]));
    h.ok("restore_snapshot", json!({ "path": h.root, "id": kept_id }));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), second);
}

fn stage_preview(h: &Harness, files: &[(&str, &str)]) -> u64 {
    let root = PathBuf::from(normalize_path(h.root.clone()));
    let changes = files
        .iter()
        .map(|(file, after)| {
            let before = std::fs::read_to_string(root.join(file)).unwrap();
            FileChange {
                file: file.to_string(),
                rules: Vec::new(),
                fix: FixKind::Auto,
                diff: diff::compute(&before, after, 3),
                original_hash: fnv1a(before.as_bytes()),
                new_content: after.to_string(),
            }
        })
        .collect();
    let preview = RectorPreview { target: "8.4".parse().unwrap(), mode: RectorMode::Compat, changes, errors: Vec::new(), duration_ms: 0 };
    h._app.state::<AppState>().previews.lock().unwrap().insert(normalize_path(h.root.clone()), (7, preview));
    7
}

#[cfg(unix)]
#[test]
fn a_failed_apply_puts_every_touched_file_back() {
    use std::os::unix::fs::PermissionsExt;
    let h = harness();
    let root = PathBuf::from(normalize_path(h.root.clone()));
    let greeter = std::fs::read_to_string(root.join("src/Greeter.php")).unwrap();
    std::fs::create_dir(root.join("locked")).unwrap();
    std::fs::write(root.join("locked/b.php"), "<?php // b\n").unwrap();
    let id = stage_preview(&h, &[("src/Greeter.php", "<?php // changed\n"), ("locked/b.php", "<?php // B\n")]);
    std::fs::set_permissions(root.join("locked"), std::fs::Permissions::from_mode(0o555)).unwrap();
    if std::fs::write(root.join("locked/.probe"), "").is_ok() {
        return;
    }
    let result =
        h.invoke("apply_changes", json!({ "path": h.root, "previewId": id, "files": ["src/Greeter.php", "locked/b.php"], "label": "x" }));
    std::fs::set_permissions(root.join("locked"), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(result.is_err());
    assert_eq!(std::fs::read_to_string(root.join("src/Greeter.php")).unwrap(), greeter);
    assert_eq!(std::fs::read_to_string(root.join("locked/b.php")).unwrap(), "<?php // b\n");
    assert_eq!(h.ok("list_snapshots", json!({ "path": h.root })), json!([]));
}

#[cfg(unix)]
#[test]
fn apply_refuses_hard_links_and_linked_folders() {
    let _lang = turkish();
    let h = harness();
    let root = PathBuf::from(normalize_path(h.root.clone()));
    let outside = tempfile::tempdir().unwrap();
    std::fs::hard_link(root.join("src/Greeter.php"), outside.path().join("alias.php")).unwrap();
    let before = std::fs::read_to_string(root.join("src/Greeter.php")).unwrap();
    let id = stage_preview(&h, &[("src/Greeter.php", "<?php // changed\n")]);
    let err =
        h.invoke("apply_changes", json!({ "path": h.root, "previewId": id, "files": ["src/Greeter.php"], "label": "x" })).unwrap_err();
    assert!(err.as_str().unwrap().contains("hard link"), "{err}");
    assert_eq!(std::fs::read_to_string(outside.path().join("alias.php")).unwrap(), before);
    assert_eq!(h.ok("list_snapshots", json!({ "path": h.root })), json!([]));

    std::fs::remove_file(outside.path().join("alias.php")).unwrap();
    let id = stage_preview(&h, &[("src/Greeter.php", "<?php // changed\n")]);
    std::fs::rename(root.join("src"), outside.path().join("src")).unwrap();
    std::os::unix::fs::symlink(outside.path().join("src"), root.join("src")).unwrap();
    let err =
        h.invoke("apply_changes", json!({ "path": h.root, "previewId": id, "files": ["src/Greeter.php"], "label": "x" })).unwrap_err();
    assert!(err.as_str().unwrap().contains("sembolik"), "{err}");
    assert_eq!(std::fs::read_to_string(outside.path().join("src/Greeter.php")).unwrap(), before);
}

#[test]
fn restore_stops_on_an_unreadable_apply_record() {
    let _lang = turkish();
    let h = harness();
    let root = PathBuf::from(normalize_path(h.root.clone()));
    let store = h._app.state::<AppState>().snapshots();
    let file = root.join("src/Greeter.php");
    let original = std::fs::read_to_string(&file).unwrap();
    let snap = store.create(&root, &["src/Greeter.php".into()], "eski").unwrap();
    std::fs::write(&file, "changed").unwrap();
    store.write_attachment(&root, &snap.id, WRITTEN_FILE, b"{broken").unwrap();
    let err = h.invoke("snapshot_conflicts", json!({ "path": h.root, "id": snap.id })).unwrap_err();
    assert!(err.as_str().unwrap().contains("okunamadı"), "{err}");
    assert!(h.invoke("restore_snapshot", json!({ "path": h.root, "id": snap.id })).is_err());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "changed");
    std::fs::write(&file, original).unwrap();
}

#[test]
fn an_incomplete_apply_record_still_catches_later_edits() {
    let h = harness();
    let root = PathBuf::from(normalize_path(h.root.clone()));
    let store = h._app.state::<AppState>().snapshots();
    let file = root.join("src/Greeter.php");
    let snap = store.create(&root, &["src/Greeter.php".into()], "eski").unwrap();
    std::fs::write(&file, "edited later").unwrap();
    store.write_attachment(&root, &snap.id, WRITTEN_FILE, b"{}").unwrap();
    let found = h.ok("snapshot_conflicts", json!({ "path": h.root, "id": snap.id }));
    assert_eq!(found[0]["file"], "src/Greeter.php");
    assert!(h.invoke("restore_snapshot", json!({ "path": h.root, "id": snap.id })).is_err());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "edited later");
}

#[cfg(unix)]
#[test]
fn an_unreadable_file_stops_restore_instead_of_counting_as_deleted() {
    use std::os::unix::fs::PermissionsExt;
    let h = harness();
    let root = PathBuf::from(normalize_path(h.root.clone()));
    let store = h._app.state::<AppState>().snapshots();
    let file = root.join("src/Greeter.php");
    let snap = store.create(&root, &["src/Greeter.php".into()], "eski").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
    let unreadable = std::fs::read(&file).is_err();
    let result = h.invoke("snapshot_conflicts", json!({ "path": h.root, "id": snap.id }));
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    if unreadable {
        assert!(result.is_err());
    }
}

#[cfg(unix)]
#[test]
fn an_interrupted_write_stays_flagged_until_it_is_dealt_with() {
    use std::os::unix::fs::PermissionsExt;
    let h = harness();
    let root = PathBuf::from(normalize_path(h.root.clone()));
    let store = h._app.state::<AppState>().snapshots();
    let snap = store.create(&root, &["src/Greeter.php".into()], "eski").unwrap();
    store.mark_writing(&root, &snap.id).unwrap();
    for _ in 0..2 {
        assert_eq!(h.ok("interrupted_writes", json!({ "path": h.root })).as_array().unwrap().len(), 1);
    }
    std::fs::set_permissions(root.join("src"), std::fs::Permissions::from_mode(0o555)).unwrap();
    let probe = std::fs::write(root.join("src/.probe"), "").is_ok();
    std::fs::write(root.join("src/Greeter.php"), "<?php // edited\n").unwrap();
    let found = h.ok("snapshot_conflicts", json!({ "path": h.root, "id": snap.id }));
    let failed = h.invoke("restore_snapshot", json!({ "path": h.root, "id": snap.id, "acknowledged": found }));
    std::fs::set_permissions(root.join("src"), std::fs::Permissions::from_mode(0o755)).unwrap();
    if !probe {
        assert!(failed.is_err());
        let left = h.ok("interrupted_writes", json!({ "path": h.root }));
        assert!(left.as_array().unwrap().iter().any(|s| s["id"] == json!(snap.id)));
    }
    h.ok("dismiss_interrupted", json!({ "path": h.root, "id": snap.id }));
    assert_eq!(h.ok("interrupted_writes", json!({ "path": h.root })), json!([]));
}

#[test]
fn a_trial_is_kept_only_for_the_files_it_ran_on() {
    let h = harness();
    let root = PathBuf::from(normalize_path(h.root.clone()));
    let scanned = project::scanned_files(&root);
    let files = vec!["src/Greeter.php".to_string()];
    let others = project::stamp_without(&root, &scanned, &files);
    let fixed = vec![(files[0].clone(), content_hash(b"<?php // tried\n"))];
    std::fs::write(root.join("src/Greeter.php"), "<?php // not what was tried\n").unwrap();
    assert!(stamp_after_trial(&root, &fixed, &others).is_err());
    std::fs::write(root.join("src/Greeter.php"), "<?php // tried\n").unwrap();
    assert_eq!(stamp_after_trial(&root, &fixed, &others).unwrap(), project::stamp(&root, &scanned));
    std::fs::write(root.join("composer.json"), "{\"name\": \"changed/too\"}").unwrap();
    assert!(stamp_after_trial(&root, &fixed, &others).is_err());
}

#[test]
fn skipped_hunks_are_left_out_of_what_is_tried_and_written() {
    let dir = tempfile::tempdir().unwrap();
    let lines: Vec<String> = (0..30).map(|n| format!("line {n}")).collect();
    let before = lines.join("\n");
    std::fs::write(dir.path().join("a.php"), &before).unwrap();
    let after = before.replace("line 1\n", "first\n").replace("line 28\n", "second\n");
    let change = FileChange {
        file: "a.php".into(),
        rules: Vec::new(),
        fix: FixKind::Auto,
        diff: diff::compute(&before, &after, 3),
        original_hash: fnv1a(before.as_bytes()),
        new_content: after,
    };
    let kept = without_skipped(dir.path(), &change, &HashMap::from([("a.php".to_string(), vec![0])])).unwrap();
    assert!(!kept.new_content.contains("first") && kept.new_content.contains("second"));
    assert_eq!(without_skipped(dir.path(), &change, &HashMap::new()).unwrap().new_content, change.new_content);
}

fn git_repo(root: &Path) -> impl Fn(&[&str]) -> String + '_ {
    let run = move |args: &[&str]| {
        let out = std::process::Command::new("git").arg("-C").arg(root).args(args).output().unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    run(&["init", "-q", "-b", "main"]);
    run(&["config", "user.email", "t@example.com"]);
    run(&["config", "user.name", "T"]);
    run(&["add", "."]);
    run(&["commit", "-qm", "first"]);
    run
}

#[test]
fn a_commit_that_fails_leaves_the_files_on_the_branch_it_started_from() {
    let h = harness();
    let root = PathBuf::from(normalize_path(h.root.clone()));
    let git = git_repo(&root);
    git(&["config", "commit.cleanup", "nonsense"]);
    let id = stage_preview(&h, &[("src/Greeter.php", "<?php // changed\n")]);
    let result = h.ok(
        "apply_changes",
        json!({ "path": h.root, "previewId": id, "files": ["src/Greeter.php"], "label": "x", "commitMessage": "Batch" }),
    );
    assert!(result["gitError"].is_string(), "{result}");
    assert_eq!(result["branch"], Value::Null);
    assert_eq!(git(&["symbolic-ref", "--short", "HEAD"]), "main");
    assert_eq!(git(&["branch", "--list", "ferry/*"]), "");
    assert_eq!(git(&["diff", "--cached", "--name-only"]), "");
    assert_eq!(std::fs::read_to_string(root.join("src/Greeter.php")).unwrap(), "<?php // changed\n");
    assert_eq!(h.ok("interrupted_writes", json!({ "path": h.root })), json!([]));
}

#[test]
fn new_syntax_errors_are_not_committed() {
    let Some(php) = toolchain::find_php(None) else { return };
    let Some(version) = toolchain::php_version(&php).and_then(|v| v.parse::<PhpVersion>().ok()) else { return };
    let h = harness();
    let root = PathBuf::from(normalize_path(h.root.clone()));
    let git = git_repo(&root);
    let head = git(&["rev-parse", "HEAD"]);
    let id = stage_preview(&h, &[("src/Greeter.php", "<?php function {\n")]);
    h._app.state::<AppState>().previews.lock().unwrap().get_mut(&root.to_string_lossy().to_string()).unwrap().1.target = version;
    let result = h.ok(
        "apply_changes",
        json!({ "path": h.root, "previewId": id, "files": ["src/Greeter.php"], "label": "x", "commitMessage": "Batch" }),
    );
    assert_eq!(result["lintErrors"].as_array().unwrap().len(), 1, "{result}");
    assert_eq!(result["commit"], Value::Null);
    assert!(result["gitError"].is_string());
    assert_eq!(git(&["rev-parse", "HEAD"]), head);
    assert_eq!(git(&["symbolic-ref", "--short", "HEAD"]), "main");
}

#[test]
fn an_ignored_file_stops_a_separate_commit_before_anything_is_written() {
    let h = harness();
    let root = PathBuf::from(normalize_path(h.root.clone()));
    let git = git_repo(&root);
    std::fs::write(root.join(".gitignore"), "local.php\n").unwrap();
    std::fs::write(root.join("local.php"), "<?php // local\n").unwrap();
    git(&["add", ".gitignore"]);
    git(&["commit", "-qm", "ignore"]);
    let id = stage_preview(&h, &[("local.php", "<?php // changed\n")]);
    let result = h.invoke(
        "apply_changes",
        json!({ "path": h.root, "previewId": id, "files": ["local.php"], "label": "x", "commitMessage": "Batch" }),
    );
    assert!(result.is_err());
    assert_eq!(std::fs::read_to_string(root.join("local.php")).unwrap(), "<?php // local\n");
    assert_eq!(h.ok("list_snapshots", json!({ "path": h.root })), json!([]));
    assert_eq!(git(&["symbolic-ref", "--short", "HEAD"]), "main");
}

#[test]
fn a_new_scan_stops_the_old_one_and_closing_stops_the_last() {
    let h = harness();
    let state = h._app.state::<AppState>();
    let old = Arc::new(AtomicBool::new(false));
    state.scan_cancel.lock().unwrap().insert(normalize_path(h.root.clone()), old.clone());
    h.ok("scan_project", json!({ "path": h.root, "target": "8.4", "run": 1 }));
    assert!(old.load(Ordering::Relaxed));
    assert!(state.scan_cancel.lock().unwrap().is_empty());

    let running = Arc::new(AtomicBool::new(false));
    state.scan_cancel.lock().unwrap().insert(normalize_path(h.root.clone()), running.clone());
    h.ok("close_project", json!({ "path": h.root }));
    assert!(running.load(Ordering::Relaxed));
}

#[test]
fn cancelling_one_project_leaves_other_previews_running() {
    let h = harness();
    let state = h._app.state::<AppState>();
    let a = Arc::new(AtomicBool::new(false));
    let b = Arc::new(AtomicBool::new(false));
    let path_a = normalize_path(h.root.clone());
    state.rector_cancel.lock().unwrap().insert(path_a, a.clone());
    state.rector_cancel.lock().unwrap().insert("/elsewhere".into(), b.clone());

    h.ok("cancel_rector_preview", json!({ "path": h.root }));
    assert!(a.load(Ordering::Relaxed));
    assert!(!b.load(Ordering::Relaxed));

    h.ok("cancel_rector_preview", json!({}));
    assert!(b.load(Ordering::Relaxed));
}

#[test]
fn missing_folders_are_shown_with_native_separators() {
    let missing = std::env::temp_dir().join("ferry-missing").to_string_lossy().into_owned() + "/sub/dir";
    let shown = normalize_path(missing.clone());
    if cfg!(windows) {
        assert!(!shown.contains('/'), "{shown}");
        assert!(shown.ends_with(r"\sub\dir"), "{shown}");
    } else {
        assert_eq!(shown, missing);
    }
}

#[test]
fn backup_usage_and_delete_all_over_ipc() {
    let h = harness();
    assert_eq!(h.ok("snapshot_usage", json!({})), json!({ "snapshots": 0, "projects": 0, "bytes": 0 }));
    let root = PathBuf::from(normalize_path(h.root.clone()));
    h._app.state::<AppState>().snapshots().create(&root, &["src/Greeter.php".into()], "x").unwrap();
    let usage = h.ok("snapshot_usage", json!({}));
    assert_eq!((usage["snapshots"].as_u64(), usage["projects"].as_u64()), (Some(1), Some(1)));
    assert!(usage["bytes"].as_u64().unwrap() > 0);
    h.ok("delete_all_snapshots", json!({}));
    assert_eq!(h.ok("snapshot_usage", json!({}))["snapshots"], 0);
    assert_eq!(h.ok("list_snapshots", json!({ "path": h.root })), json!([]));
}

#[test]
fn ignored_findings_move_out_of_the_summary() {
    let _lang = turkish();
    let h = harness();
    let root = h.root.clone();
    let scan = h.ok("scan_project", json!({ "path": root, "target": "8.4", "run": 1 }));
    let total = scan["summary"]["total"].as_u64().unwrap();
    let first = scan["findings"][0].clone();
    let err = h.invoke("ignore_finding", json!({ "path": root, "id": first["id"], "reason": "  " })).unwrap_err();
    assert_eq!(err, json!("Yok sayma için bir gerekçe yazın."));

    let after = h.ok("ignore_finding", json!({ "path": root, "id": first["id"], "reason": "Eski kopya" }));
    assert_eq!(after["summary"]["total"].as_u64().unwrap(), total - 1);
    assert_eq!(after["ignored"][0]["reason"], "Eski kopya");
    assert_eq!(after["ignored"][0]["finding"]["id"], first["id"]);
    assert!(Path::new(&root).join(".ferry/ignore.json").exists());

    let rescan = h.ok("scan_project", json!({ "path": root, "target": "8.4", "run": 2 }));
    assert_eq!(rescan["ignored"].as_array().unwrap().len(), 1);
    assert_eq!(rescan["progress"]["history"].as_array().unwrap().len(), 2);
    assert_eq!(rescan["progress"]["newSinceLast"], 0);
    assert_eq!(rescan["progress"]["fixedSinceLast"], 1);
    assert_eq!(scan["progress"]["newSinceLast"], Value::Null);

    let restored = h.ok("unignore_finding", json!({ "path": root, "id": first["id"] }));
    assert_eq!(restored["summary"]["total"].as_u64().unwrap(), total);
    assert_eq!(restored["ignored"], json!([]));
    assert_eq!(restored["staleIgnores"], json!([]));

    let stale = json!({ "ignores": [{ "ruleId": "removed.each", "file": "gone.php", "snippet": "each($x);", "reason": "old" }] });
    std::fs::write(Path::new(&root).join(".ferry/ignore.json"), stale.to_string()).unwrap();
    let rescan = h.ok("scan_project", json!({ "path": root, "target": "8.4", "run": 3 }));
    assert_eq!(rescan["staleIgnores"][0]["file"], "gone.php");
    let pruned = h.ok("prune_ignores", json!({ "path": root }));
    assert_eq!(pruned["staleIgnores"], json!([]));
    assert!(!std::fs::read_to_string(Path::new(&root).join(".ferry/ignore.json")).unwrap().contains("gone.php"));
}

#[test]
fn custom_editor_commands() {
    let file = Path::new("/home/efe/My Site/a; rm.php");
    let call = custom_editor("my-editor --wait '+{line}:{column}' \"{file}\" --flag=\"a b\"", file, 12, 3).unwrap();
    let EditorCall::Command(program, args) = call else { panic!("expected a command") };
    assert_eq!(program, PathBuf::from("my-editor"));
    assert_eq!(args, ["--wait", "+12:3", "/home/efe/My Site/a; rm.php", "--flag=a b"]);
    let EditorCall::Command(_, args) = custom_editor("e {file}", file, 1, 1).unwrap() else { panic!("expected a command") };
    assert_eq!(args.len(), 1, "a file name with spaces stays one argument");
    assert_eq!(
        custom_editor("mvim://open?url=file://{file}&line={line}", Path::new("/a b.php"), 7, 1),
        Some(EditorCall::Url("mvim://open?url=file:///a%20b.php&line=7".into()))
    );
    assert_eq!(custom_editor("nvim +{line}", file, 1, 1), None);
    assert_eq!(custom_editor("  ", file, 1, 1), None);
    assert_eq!(split_args("a '' \"b c\""), ["a", "", "b c"]);
}

#[test]
fn editor_links() {
    let file = Path::new("/home/efe/My Site/index.php");
    assert_eq!(editor_url("vscode", file, 12, 3).unwrap(), "vscode://file/home/efe/My%20Site/index.php:12:3");
    assert_eq!(editor_url("phpstorm", file, 12, 3).unwrap(), "phpstorm://open?file=/home/efe/My%20Site/index.php&line=12&column=3");
    assert_eq!(editor_url("vscode", Path::new("C:\\www\\a.php"), 1, 1).unwrap(), "vscode://file/C:/www/a.php:1:1");
    assert!(editor_url("", file, 1, 1).is_none());
    let h = harness();
    let err = h.invoke("open_in_editor", json!({ "path": h.root, "file": "../outside.php", "line": 1, "column": 1 })).unwrap_err();
    assert!(err.as_str().unwrap().contains("outside.php"));
}

#[test]
fn runtime_and_inline_ignores_behave() {
    let _lang = turkish();
    let h = harness();
    let root = h.root.clone();
    h.ok("scan_project", json!({ "path": root, "target": "8.4", "run": 1 }));
    let log = Path::new(&root).join("php_errors.log");
    std::fs::write(&log, "PHP Warning:  Undefined variable $x in /srv/app/src/Greeter.php on line 3\n").unwrap();
    let merged = h.ok("import_runtime_log", json!({ "path": root, "log": log }));
    let id = merged["findings"].as_array().unwrap().iter().find(|f| f["ruleId"] == "runtime.warning").unwrap()["id"].clone();
    let ignored = h.ok("ignore_finding", json!({ "path": root, "id": id, "reason": "known" }));
    assert!(ignored["ignored"].as_array().unwrap().iter().any(|i| i["finding"]["id"] == id && i["inline"] == false));
    assert!(!ignored["findings"].as_array().unwrap().iter().any(|f| f["id"] == id));
    let shown = h.ok("unignore_finding", json!({ "path": root, "id": id }));
    assert!(shown["findings"].as_array().unwrap().iter().any(|f| f["id"] == id));

    let file = Path::new(&root).join("src/Greeter.php");
    let src = std::fs::read_to_string(&file).unwrap();
    std::fs::write(
        &file,
        src.replacen(
            "        while (list($k, $v) = each($this->items)) {",
            "        // ferry-ignore removed.each legacy loop\n        while (list($k, $v) = each($this->items)) {",
            1,
        ),
    )
    .unwrap();
    let rescan = h.ok("scan_project", json!({ "path": root, "target": "8.4", "run": 2 }));
    let inline = rescan["ignored"].as_array().unwrap().iter().find(|i| i["finding"]["ruleId"] == "removed.each").unwrap();
    assert_eq!((inline["inline"].as_bool(), inline["reason"].as_str()), (Some(true), Some("legacy loop")));
    let err = h.invoke("unignore_finding", json!({ "path": root, "id": inline["finding"]["id"] })).unwrap_err();
    assert!(err.as_str().unwrap().contains("ferry-ignore yorumuyla"), "{err}");
}

#[test]
fn docker_comparison_feeds_the_findings() {
    if std::env::var_os("FERRY_TEST_DOCKER").is_none() {
        return;
    }
    let h = harness();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.php"), "<?php\nfunction f(int $a = null) {}\necho 1;\n").unwrap();
    h.ok("scan_project", json!({ "path": dir.path(), "target": "8.4", "run": 1 }));
    let result = h.ok(
        "docker_compare",
        json!({ "path": dir.path(), "source": "7.4", "target": "8.4", "extensions": false, "cleanImages": false, "pages": [], "database": null, "prepare": null }),
    );
    assert!(!result["comparison"]["newOnTarget"].as_array().unwrap().is_empty(), "{result}");
    assert_eq!(result["scan"]["runtime"]["outside"], 0, "{result}");
    assert!(result["scan"]["findings"].as_array().unwrap().iter().any(|f| f["runtimeHits"].as_u64() > Some(0)), "{result}");
}

#[test]
fn database_plans_never_include_the_password() {
    let h = harness();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".env"), "DB_CONNECTION=pgsql\nDB_DATABASE=shop\nDB_USERNAME=app\nDB_PASSWORD=hunter2\n").unwrap();
    std::fs::create_dir(dir.path().join("database")).unwrap();
    std::fs::write(dir.path().join("database/dump.sql"), "-- dump").unwrap();
    let plan = h.ok("database_plan", json!({ "path": dir.path() }));
    assert_eq!((plan["engine"].as_str(), plan["name"].as_str(), plan["live"].as_bool()), (Some("postgres"), Some("shop"), Some(true)));
    assert_eq!(plan["dumps"], json!(["database/dump.sql"]));
    assert!(!plan.to_string().contains("hunter2"));
}

#[test]
fn skipped_hunks_stay_out_of_the_applied_file() {
    let h = harness();
    let root = PathBuf::from(normalize_path(h.root.clone()));
    let before: String = (1..=30).map(|n| format!("// line {n}\n")).collect();
    std::fs::write(root.join("a.php"), format!("<?php\n{before}")).unwrap();
    let after = format!("<?php\n{}", before.replace("line 3\n", "LINE 3\n").replace("line 25\n", "LINE 25\n"));
    let id = stage_preview(&h, &[("a.php", &after)]);
    let args = json!({ "path": h.root, "previewId": id, "files": ["a.php"], "skipped": { "a.php": [1] }, "label": "x" });
    assert_eq!(h.ok("apply_changes", args)["written"], json!(["a.php"]));
    let written = std::fs::read_to_string(root.join("a.php")).unwrap();
    assert!(written.contains("LINE 3") && !written.contains("LINE 25"), "{written}");
}

#[test]
fn a_folder_without_php_is_an_incomplete_scan() {
    let h = harness();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("README.md"), "No PHP here.\n").unwrap();
    let scan = h.ok("scan_project", json!({ "path": dir.path(), "target": "8.4", "run": 1 }));
    assert_eq!(scan["errors"].as_array().unwrap().len(), 1, "{scan}");
}

#[test]
fn runtime_roots_narrow_an_imported_log() {
    let h = harness();
    let root = h.root.clone();
    h.ok("scan_project", json!({ "path": root, "target": "8.4", "run": 1 }));
    let log = Path::new(&root).join("php_errors.log");
    std::fs::write(&log, "PHP Warning:  Undefined variable $x in /other/src/Greeter.php on line 3\n").unwrap();
    let guessed = h.ok("import_runtime_log", json!({ "path": root, "log": log }));
    assert_eq!(guessed["runtime"]["outside"], 0);
    let narrowed = h.ok("set_runtime_roots", json!({ "path": root, "roots": "/srv/app" }));
    assert_eq!(narrowed["runtime"]["outside"], 1);
    assert_eq!(narrowed["runtime"]["roots"], json!(["/srv/app"]));
    let again = h.ok("import_runtime_log", json!({ "path": root, "log": log }));
    assert_eq!(again["runtime"]["outside"], 1, "a new log keeps the root");
    let widened = h.ok("set_runtime_roots", json!({ "path": root, "roots": "" }));
    assert_eq!(widened["runtime"]["outside"], 0);
}

#[test]
fn runtime_logs_confirm_and_add_findings() {
    let _lang = turkish();
    let h = harness();
    let root = h.root.clone();
    h.ok("scan_project", json!({ "path": root, "target": "8.4", "run": 1 }));
    let log = Path::new(&root).join("php_errors.log");
    std::fs::write(
        &log,
        "PHP Deprecated:  The each() function is deprecated in /srv/app/src/Greeter.php on line 9\nPHP Warning:  Undefined variable $x in /srv/app/src/Greeter.php on line 3\nPHP Notice:  ignored line\n",
    )
    .unwrap();
    let merged = h.ok("import_runtime_log", json!({ "path": root, "log": log }));
    assert_eq!(merged["runtime"]["confirmed"], 1);
    assert_eq!(merged["runtime"]["added"], 1);
    let findings = merged["findings"].as_array().unwrap();
    assert!(findings.iter().any(|f| f["ruleId"] == "removed.each" && f["runtimeHits"] == 1));
    assert!(findings.iter().any(|f| f["ruleId"] == "runtime.warning" && f["file"] == "src/Greeter.php"));
    let rescan = h.ok("scan_project", json!({ "path": root, "target": "8.4", "run": 2 }));
    assert_eq!(rescan["runtime"]["added"], 1);
    h.ok("close_project", json!({ "path": root }));
    let reopened = h.ok("scan_project", json!({ "path": root, "target": "8.4", "run": 3 }));
    assert_eq!(reopened["runtime"], Value::Null, "closing the project drops its log");
    h.ok("import_runtime_log", json!({ "path": root, "log": log }));
    let cleared = h.ok("clear_runtime_log", json!({ "path": root }));
    assert_eq!(cleared["runtime"], Value::Null);
    let empty = Path::new(&root).join("empty.log");
    std::fs::write(&empty, "nothing here\n").unwrap();
    assert!(h.invoke("import_runtime_log", json!({ "path": root, "log": empty })).is_err());
}

#[test]
fn lints_with_a_local_php_of_the_target() {
    let _lang = turkish();
    let h = harness();
    let root = h.root.clone();
    let local = h.ok("local_php_versions", json!({}));
    let Some(line) = local.as_array().unwrap().first().map(|l| l["line"].as_str().unwrap().to_string()) else { return };
    h.ok("scan_project", json!({ "path": root, "target": line, "run": 1 }));
    let merged = h.ok("run_local_check", json!({ "path": root, "target": line, "kind": "lint" }));
    assert!(merged["runtime"]["file"].as_str().unwrap().starts_with("php -l · PHP "));
    assert!(merged["runtime"]["events"].as_u64().unwrap() > 0);
    let check = &merged["runtime"]["check"];
    assert_eq!(check["kind"], "lint");
    assert!(check["files"].as_u64().unwrap() > 0);
    assert_eq!(check["notRun"], 0);
    assert!(check.get("output").is_none(), "only the trimmed log is sent");
    let err = h.invoke("run_local_check", json!({ "path": root, "target": line, "kind": "tests" })).unwrap_err();
    assert_eq!(err, json!("Projede vendor/bin/phpunit yok."));
}

#[test]
fn a_real_test_does_not_pass_its_container_root_to_the_next_log() {
    let log = |stamp: Option<String>| RuntimeLog {
        file: String::new(),
        php: None,
        events: Vec::new(),
        check: None,
        roots: vec!["/app".to_string()],
        target: None,
        stamp,
    };
    assert_eq!(typed_roots(&log(None)), ["/app"]);
    assert!(typed_roots(&log(Some("s".to_string()))).is_empty());
}
