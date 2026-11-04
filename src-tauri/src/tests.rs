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
    assert_eq!(porcelain_paths(out, "sub/"), ["new name.php", "x -> y.php", "other.php"]);
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
    assert_eq!(h.ok("get_settings", json!({})), json!({ "phpPath": null, "snapshotLimit": null, "editor": null }));
    h.ok("save_settings", json!({ "settings": { "phpPath": "/opt/php/bin/php", "editor": "vscode" } }));
    assert_eq!(h.ok("get_settings", json!({})), json!({ "phpPath": "/opt/php/bin/php", "snapshotLimit": null, "editor": "vscode" }));
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
fn rector_preview_apply_restore_over_ipc() {
    if std::env::var_os("FERRY_TEST_TOOLS").is_none() {
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

#[cfg(unix)]
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
