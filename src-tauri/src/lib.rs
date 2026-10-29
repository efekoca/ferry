use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use ferry_core::analyzer::{self, ScanResult};
use ferry_core::deps::{self, DependencyReport};
use ferry_core::project::{self, ProjectInfo};
use ferry_core::rector::{self, FileChange, RectorMode, RectorPreview};
use ferry_core::snapshot::{Snapshot, SnapshotStore};
use ferry_core::toolchain::{self, ToolchainStatus};
use ferry_core::i18n::{self, Lang};
use ferry_core::rules::FixKind;
use ferry_core::{diff, report, t, PhpVersion};

const CHANGES_FILE: &str = "changes.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub php_path: Option<String>,
}

pub struct AppState {
    data_dir: PathBuf,
    tools_dir: PathBuf,
    settings: Mutex<Settings>,
    scans: Mutex<HashMap<String, ScanResult>>,
    previews: Mutex<HashMap<String, RectorPreview>>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        let settings = std::fs::read(data_dir.join("settings.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self {
            tools_dir: data_dir.join("tools"),
            data_dir,
            settings: Mutex::new(settings),
            scans: Mutex::new(HashMap::new()),
            previews: Mutex::new(HashMap::new()),
        }
    }

    fn tools_dir(&self) -> PathBuf {
        self.tools_dir.clone()
    }

    fn snapshots(&self) -> SnapshotStore {
        SnapshotStore::new(self.data_dir.join("snapshots"))
    }

    fn php_override(&self) -> Option<PathBuf> {
        self.settings
            .lock()
            .unwrap()
            .php_path
            .as_deref()
            .filter(|p| !p.trim().is_empty())
            .map(PathBuf::from)
    }

    fn settings_path(&self) -> PathBuf {
        self.data_dir.join("settings.json")
    }
}

type CmdResult<T> = Result<T, String>;

fn parse_target(target: &str) -> CmdResult<PhpVersion> {
    let v: PhpVersion = target.parse()?;
    if !PhpVersion::TARGETS.contains(&v) {
        return Err(t!("Desteklenmeyen hedef sürüm: {v}", "Unsupported target version: {v}"));
    }
    Ok(v)
}

fn project_root(path: &str) -> CmdResult<PathBuf> {
    let p = PathBuf::from(path);
    if !p.is_dir() {
        return Err(t!("Klasör bulunamadı: {path}", "Folder not found: {path}"));
    }
    Ok(p)
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> CmdResult<T> + Send + 'static) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| t!("İş parçacığı hatası: {e}", "Worker thread failed: {e}"))?
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanProgress {
    done: usize,
    total: usize,
    file: String,
}

#[tauri::command]
async fn inspect_project(path: String) -> CmdResult<ProjectInfo> {
    blocking(move || project::inspect(&project_root(&path)?).map_err(|e| e.to_string())).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanResponse {
    #[serde(flatten)]
    scan: ScanResult,
    summary: report::Summary,
}

#[tauri::command]
async fn scan_project<R: Runtime>(app: AppHandle<R>, path: String, target: String) -> CmdResult<ScanResponse> {
    let target = parse_target(&target)?;
    let handle = app.clone();
    let result = blocking(move || {
        let root = project_root(&path)?;
        let info = project::inspect(&root).map_err(|e| e.to_string())?;
        let files = project::collect_php_files(&root, &info.excluded_dirs);
        let mut last = Instant::now() - Duration::from_secs(1);
        let scan = analyzer::analyze_files(&root, &files, target, |done, total, file| {
            if done == total || last.elapsed() >= Duration::from_millis(50) {
                last = Instant::now();
                let _ = handle.emit(
                    "scan://progress",
                    ScanProgress {
                        done,
                        total,
                        file: file.to_string(),
                    },
                );
            }
        });
        Ok((path, scan))
    })
    .await?;
    let (path, scan) = result;
    let state = app.state::<AppState>();
    let summary = report::summarize(&scan.findings);
    state.scans.lock().unwrap().insert(path, scan.clone());
    Ok(ScanResponse { scan, summary })
}

#[tauri::command]
async fn check_dependencies(path: String, target: String) -> CmdResult<DependencyReport> {
    let target = parse_target(&target)?;
    blocking(move || Ok(deps::check(&project_root(&path)?, target))).await
}

#[tauri::command]
async fn toolchain_status(state: State<'_, AppState>) -> CmdResult<ToolchainStatus> {
    let tools = state.tools_dir();
    let php = state.php_override();
    blocking(move || Ok(toolchain::detect(&tools, php.as_deref()))).await
}

#[tauri::command]
async fn install_rector(state: State<'_, AppState>) -> CmdResult<ToolchainStatus> {
    let tools = state.tools_dir();
    let php = state.php_override();
    blocking(move || {
        toolchain::install_rector(&tools, php.as_deref()).map_err(|e| e.to_string())?;
        Ok(toolchain::detect(&tools, php.as_deref()))
    })
    .await
}

#[tauri::command]
async fn rector_preview(
    state: State<'_, AppState>,
    path: String,
    target: String,
    mode: RectorMode,
) -> CmdResult<RectorPreview> {
    let target = parse_target(&target)?;
    let tools = state.tools_dir();
    let work = state.data_dir.join("rector");
    let php_override = state.php_override();
    let key = path.clone();
    let preview = blocking(move || {
        let root = project_root(&path)?;
        let php = toolchain::find_php(php_override.as_deref()).ok_or_else(|| toolchain::ToolError::PhpMissing.to_string())?;
        let info = project::inspect(&root).map_err(|e| e.to_string())?;
        rector::preview(
            &php,
            &toolchain::rector_entry(&tools),
            &root,
            target,
            mode,
            &info.excluded_dirs,
            &work,
        )
        .map_err(|e| e.to_string())
    })
    .await?;
    state.previews.lock().unwrap().insert(key, preview.clone());
    Ok(preview)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplyResult {
    snapshot: Snapshot,
    written: Vec<String>,
}

#[tauri::command]
async fn apply_changes(
    state: State<'_, AppState>,
    path: String,
    files: Vec<String>,
    label: String,
) -> CmdResult<ApplyResult> {
    let root = project_root(&path)?;
    let selected: Vec<FileChange> = {
        let previews = state.previews.lock().unwrap();
        let preview = previews
            .get(&path)
            .ok_or_else(|| t!("Önce değişiklikleri hesaplayın.", "Calculate the changes first."))?;
        preview
            .changes
            .iter()
            .filter(|c| files.contains(&c.file))
            .cloned()
            .collect()
    };
    if selected.is_empty() {
        return Err(t!("Uygulanacak dosya seçilmedi.", "No files selected to apply."));
    }
    let store = state.snapshots();
    let applied_files: Vec<String> = selected.iter().map(|c| c.file.clone()).collect();
    let result = blocking(move || {
        let snapshot = store
            .create(&root, &applied_files, &label)
            .map_err(|e| e.to_string())?;
        let refs: Vec<&FileChange> = selected.iter().collect();
        match rector::apply(&root, &refs) {
            Ok(written) => {
                if let Ok(json) = serde_json::to_vec(&selected) {
                    let _ = store.write_attachment(&root, &snapshot.id, CHANGES_FILE, &json);
                }
                Ok(ApplyResult { snapshot, written })
            }
            Err(e) => {
                let _ = store.restore(&root, &snapshot.id);
                let _ = store.delete(&root, &snapshot.id);
                Err(e.to_string())
            }
        }
    })
    .await?;
    if let Some(preview) = state.previews.lock().unwrap().get_mut(&path) {
        preview.changes.retain(|c| !result.written.contains(&c.file));
    }
    Ok(result)
}

#[tauri::command]
async fn list_snapshots(state: State<'_, AppState>, path: String) -> CmdResult<Vec<Snapshot>> {
    let store = state.snapshots();
    blocking(move || store.list(Path::new(&path)).map_err(|e| e.to_string())).await
}

#[tauri::command]
async fn restore_snapshot(state: State<'_, AppState>, path: String, id: String) -> CmdResult<Vec<String>> {
    let store = state.snapshots();
    state.previews.lock().unwrap().remove(&path);
    blocking(move || store.restore(&project_root(&path)?, &id).map_err(|e| e.to_string())).await
}

#[tauri::command]
async fn snapshot_changes(state: State<'_, AppState>, path: String, id: String) -> CmdResult<Vec<FileChange>> {
    let store = state.snapshots();
    blocking(move || {
        let root = project_root(&path)?;
        if let Some(bytes) = store.read_attachment(&root, &id, CHANGES_FILE).map_err(|e| e.to_string())? {
            if let Ok(changes) = serde_json::from_slice::<Vec<FileChange>>(&bytes) {
                return Ok(changes);
            }
        }
        let originals = store.originals(&root, &id).map_err(|e| e.to_string())?;
        Ok(originals
            .into_iter()
            .map(|(file, before)| {
                let after = std::fs::read(root.join(&file))
                    .map(|b| String::from_utf8_lossy(&b).into_owned())
                    .unwrap_or_default();
                FileChange {
                    diff: diff::compute(before.as_deref().unwrap_or(""), &after, 3),
                    file,
                    rules: Vec::new(),
                    fix: FixKind::Auto,
                    original_hash: 0,
                    new_content: String::new(),
                }
            })
            .collect())
    })
    .await
}

#[tauri::command]
async fn delete_snapshot(state: State<'_, AppState>, path: String, id: String) -> CmdResult<()> {
    let store = state.snapshots();
    blocking(move || store.delete(Path::new(&path), &id).map_err(|e| e.to_string())).await
}

#[tauri::command]
async fn export_report(state: State<'_, AppState>, path: String, destination: String) -> CmdResult<()> {
    let scan = state
        .scans
        .lock()
        .unwrap()
        .get(&path)
        .cloned()
        .ok_or_else(|| t!("Önce projeyi tarayın.", "Scan the project first."))?;
    blocking(move || {
        let root = project_root(&path)?;
        let info = project::inspect(&root).map_err(|e| e.to_string())?;
        let deps = deps::check(&root, scan.target);
        let md = report::to_markdown(&info, &scan, Some(&deps), i18n::lang());
        std::fs::write(&destination, md).map_err(|e| t!("Rapor kaydedilemedi: {e}", "Couldn't save the report: {e}"))
    })
    .await
}

#[tauri::command]
fn set_language(language: Lang) {
    i18n::set_lang(language);
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(state: State<'_, AppState>, settings: Settings) -> CmdResult<()> {
    let json = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&state.data_dir).map_err(|e| e.to_string())?;
    std::fs::write(state.settings_path(), json).map_err(|e| e.to_string())?;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

pub fn with_commands<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        inspect_project,
        scan_project,
        check_dependencies,
        toolchain_status,
        install_rector,
        rector_preview,
        apply_changes,
        list_snapshots,
        restore_snapshot,
        delete_snapshot,
        snapshot_changes,
        export_report,
        get_settings,
        save_settings,
        set_language,
    ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            app.manage(AppState::new(data_dir));
            #[cfg(debug_assertions)]
            if std::env::var_os("FERRY_E2E").is_some() {
                use tauri::Listener;
                app.listen_any("e2e://log", |event| println!("[e2e] {}", event.payload()));
            }
            Ok(())
        })
        .on_page_load(|webview, payload| {
            #[cfg(debug_assertions)]
            if let (Some(script), tauri::webview::PageLoadEvent::Finished) =
                (std::env::var_os("FERRY_E2E"), payload.event())
            {
                if let Ok(body) = std::fs::read_to_string(script) {
                    let root = std::env::var("FERRY_E2E_ROOT").unwrap_or_default();
                    let prelude = format!("window.__FERRY_E2E_ROOT__ = {};", serde_json::to_string(&root).unwrap());
                    let _ = webview.eval(format!("{prelude}\n{body}"));
                }
            }
            #[cfg(not(debug_assertions))]
            let _ = (webview, payload);
        });
    with_commands(builder)
        .run(tauri::generate_context!())
        .expect("failed to start Ferry");
}

#[cfg(test)]
mod tests;
