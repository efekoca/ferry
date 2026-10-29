use super::*;
use serde_json::{json, Value};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;

const APP_ORIGIN: &str = if cfg!(windows) { "http://tauri.localhost" } else { "tauri://localhost" };

static LANG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    let app = with_commands(mock_builder())
        .manage(state)
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    Harness {
        _app: app,
        webview,
        _dirs: vec![data, project_dir],
        root: root.to_string_lossy().to_string(),
    }
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
        self.invoke(cmd, args.clone())
            .unwrap_or_else(|e| panic!("{cmd}({args}) failed: {e}"))
    }
}

#[test]
fn scan_flow_matches_ui_contract() {
    let h = harness();
    let root = h.root.clone();

    let info = h.ok("inspect_project", json!({ "path": root }));
    assert_eq!(info["phpFiles"], 2);
    assert_eq!(info["framework"], "generic");
    assert_eq!(info["currentVersion"], "7.1");

    let scan = h.ok("scan_project", json!({ "path": root, "target": "8.4" }));
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
    let tools = h.ok("toolchain_status", json!({}));
    assert!(tools["platform"].is_string());
}

#[test]
fn rejects_bad_input_with_messages() {
    let _lang = LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let h = harness();
    let err = h.invoke("scan_project", json!({ "path": h.root, "target": "7.4" })).unwrap_err();
    assert!(err.as_str().unwrap().contains("Desteklenmeyen hedef sürüm"));
    let err = h.invoke("inspect_project", json!({ "path": "/yok/boyle/bir/klasor" })).unwrap_err();
    assert!(err.as_str().unwrap().contains("Klasör bulunamadı"));
    let err = h.invoke("export_report", json!({ "path": h.root, "destination": "/tmp/x.md" })).unwrap_err();
    assert!(err.as_str().unwrap().contains("Önce projeyi tarayın"));
    let err = h
        .invoke("apply_changes", json!({ "path": h.root, "files": ["a.php"], "label": "x" }))
        .unwrap_err();
    assert!(err.as_str().unwrap().contains("Önce değişiklikleri hesaplayın"));
}

#[test]
fn messages_follow_the_ui_language() {
    let _lang = LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let h = harness();
    h.ok("set_language", json!({ "language": "en" }));
    let err = h.invoke("scan_project", json!({ "path": h.root, "target": "7.4" })).unwrap_err();
    h.ok("set_language", json!({ "language": "tr" }));
    assert!(err.as_str().unwrap().contains("Unsupported target version"));
}

#[test]
fn settings_round_trip() {
    let h = harness();
    assert_eq!(h.ok("get_settings", json!({})), json!({ "phpPath": null }));
    h.ok("save_settings", json!({ "settings": { "phpPath": "/opt/php/bin/php" } }));
    assert_eq!(h.ok("get_settings", json!({}))["phpPath"], "/opt/php/bin/php");
}

#[test]
fn snapshot_changes_fall_back_to_a_fresh_diff() {
    let h = harness();
    let root = PathBuf::from(&h.root);
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
}

#[test]
fn rector_preview_apply_restore_over_ipc() {
    if std::env::var_os("FERRY_TEST_TOOLS").is_none() {
        eprintln!("FERRY_TEST_TOOLS ayarlı değil; Rector IPC testi atlandı");
        return;
    }
    let h = harness();
    let root = h.root.clone();
    let preview = h.ok("rector_preview", json!({ "path": root, "target": "8.4", "mode": "compat" }));
    let changes = preview["changes"].as_array().unwrap();
    let greeter = changes.iter().find(|c| c["file"] == "src/Greeter.php").expect("Greeter change");
    assert!(greeter["diff"]["hunks"][0]["lines"][0]["kind"].is_string());
    assert!(greeter.get("newContent").is_none(), "file contents must not be sent to the UI");

    let original = std::fs::read_to_string(Path::new(&root).join("src/Greeter.php")).unwrap();
    let applied = h.ok(
        "apply_changes",
        json!({ "path": root, "files": ["src/Greeter.php"], "label": "Uyumluluk düzeltmeleri · PHP 8.4" }),
    );
    assert_eq!(applied["written"], json!(["src/Greeter.php"]));
    let changed = std::fs::read_to_string(Path::new(&root).join("src/Greeter.php")).unwrap();
    assert_ne!(changed, original);

    let snapshots = h.ok("list_snapshots", json!({ "path": root }));
    let id = snapshots[0]["id"].as_str().unwrap().to_string();
    assert_eq!(snapshots[0]["label"], "Uyumluluk düzeltmeleri · PHP 8.4");
    let history = h.ok("snapshot_changes", json!({ "path": root, "id": id }));
    assert_eq!(history[0]["file"], "src/Greeter.php");
    assert_eq!(history[0]["diff"], greeter["diff"]);
    assert!(!history[0]["rules"].as_array().unwrap().is_empty());

    assert!(h
        .invoke("apply_changes", json!({ "path": root, "files": ["src/Greeter.php"], "label": "x" }))
        .is_err());

    h.ok("restore_snapshot", json!({ "path": root, "id": id }));
    assert_eq!(std::fs::read_to_string(Path::new(&root).join("src/Greeter.php")).unwrap(), original);
    h.ok("delete_snapshot", json!({ "path": root, "id": id }));
    assert_eq!(h.ok("list_snapshots", json!({ "path": root })), json!([]));
}
