use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use ferry_core::analyzer::{self, ScanResult};
use ferry_core::deps::{self, DependencyReport};
use ferry_core::project::{self, ProjectInfo};
use ferry_core::rector::{self, FileChange, RectorError, RectorMode, RectorPreview};
use ferry_core::snapshot::{fnv1a, Snapshot, SnapshotStore};
use ferry_core::toolchain::{self, ToolchainStatus};
use ferry_core::i18n::{self, Lang};
use ferry_core::rules::FixKind;
use ferry_core::{diff, report, t, PhpVersion};

const CHANGES_FILE: &str = "changes.json";
const WRITTEN_FILE: &str = "written.json";

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
    latest_scan: Mutex<HashMap<String, u64>>,
    previews: Mutex<HashMap<String, (u64, RectorPreview)>>,
    writes: Arc<Mutex<()>>,
    rector_cancel: Mutex<Arc<AtomicBool>>,
    rector_running: Arc<AtomicUsize>,
    shutdown: Arc<AtomicBool>,
    next_preview: AtomicU64,
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
            latest_scan: Mutex::new(HashMap::new()),
            previews: Mutex::new(HashMap::new()),
            writes: Arc::new(Mutex::new(())),
            rector_cancel: Mutex::new(Arc::new(AtomicBool::new(false))),
            rector_running: Arc::new(AtomicUsize::new(0)),
            shutdown: Arc::new(AtomicBool::new(false)),
            next_preview: AtomicU64::new(1),
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

fn normalize_path(path: String) -> String {
    let Ok(canonical) = std::fs::canonicalize(&path) else { return path };
    let text = canonical.to_string_lossy();
    let Some(rest) = text.strip_prefix(r"\\?\") else { return text.into_owned() };
    let drive = |s: &str| s.as_bytes().get(1) == Some(&b':') && s.as_bytes()[0].is_ascii_alphabetic();
    match rest.strip_prefix("UNC\\") {
        Some(_) if drive(&path) => path,
        Some(unc) => format!(r"\\{unc}"),
        None if drive(rest) => rest.to_string(),
        None => path,
    }
}

fn project_root(path: &str) -> CmdResult<PathBuf> {
    let p = PathBuf::from(path);
    match std::fs::metadata(&p) {
        Ok(meta) if meta.is_dir() => Ok(p),
        Ok(_) => Err(t!("Bu bir dosya, proje klasörü değil: {path}", "This is a file, not a project folder: {path}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(t!("Klasör bulunamadı: {path}", "Folder not found: {path}")),
        Err(e) => {
            let reason = ferry_core::i18n::io_error(&e);
            Err(t!("Klasör açılamadı ({path}): {reason}", "Couldn't open the folder ({path}): {reason}"))
        }
    }
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> CmdResult<T> + Send + 'static) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| t!("İş parçacığı hatası: {e}", "Worker thread failed: {e}"))?
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanProgress {
    run: u64,
    done: usize,
    total: usize,
    file: String,
}

#[tauri::command]
async fn inspect_project(path: String) -> CmdResult<ProjectInfo> {
    let path = normalize_path(path);
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
async fn scan_project<R: Runtime>(app: AppHandle<R>, path: String, target: String, run: u64) -> CmdResult<ScanResponse> {
    let path = normalize_path(path);
    let target = parse_target(&target)?;
    app.state::<AppState>().latest_scan.lock().unwrap().insert(path.clone(), run);
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
                        run,
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
    if state.latest_scan.lock().unwrap().get(&path) == Some(&run) {
        state.scans.lock().unwrap().insert(path, scan.clone());
    }
    Ok(ScanResponse { scan, summary })
}

#[tauri::command]
async fn check_dependencies(path: String, target: String) -> CmdResult<DependencyReport> {
    let path = normalize_path(path);
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
    let shutdown = state.shutdown.clone();
    let running = state.rector_running.clone();
    running.fetch_add(1, Ordering::SeqCst);
    blocking(move || {
        let _running = RunningGuard(running);
        toolchain::install_rector(&tools, php.as_deref(), &shutdown).map_err(|e| e.to_string())?;
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
) -> CmdResult<PreviewResponse> {
    let path = normalize_path(path);
    let target = parse_target(&target)?;
    let tools = state.tools_dir();
    let work = state.data_dir.join("rector");
    let php_override = state.php_override();
    let key = path.clone();
    let cancel = Arc::new(AtomicBool::new(false));
    std::mem::replace(&mut *state.rector_cancel.lock().unwrap(), cancel.clone()).store(true, Ordering::Relaxed);
    let mine = cancel.clone();
    let running = state.rector_running.clone();
    running.fetch_add(1, Ordering::SeqCst);
    let preview = blocking(move || {
        let _running = RunningGuard(running);
        let root = project_root(&path)?;
        let php = toolchain::find_php(php_override.as_deref()).ok_or_else(|| toolchain::ToolError::PhpMissing.to_string())?;
        toolchain::require_supported_php(&php).map_err(|e| e.to_string())?;
        let info = project::inspect(&root).map_err(|e| e.to_string())?;
        rector::preview(
            &php,
            &toolchain::rector_entry(&tools),
            &root,
            target,
            mode,
            &info.excluded_dirs,
            &work,
            &cancel,
        )
        .map_err(|e| e.to_string())
    })
    .await?;
    if !Arc::ptr_eq(&state.rector_cancel.lock().unwrap(), &mine) {
        return Err(RectorError::Cancelled.to_string());
    }
    let id = state.next_preview.fetch_add(1, Ordering::Relaxed);
    state.previews.lock().unwrap().insert(key, (id, preview.clone()));
    Ok(PreviewResponse { id, preview })
}

struct RunningGuard(Arc<AtomicUsize>);

impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewResponse {
    id: u64,
    #[serde(flatten)]
    preview: RectorPreview,
}

#[tauri::command]
fn cancel_rector_preview(state: State<'_, AppState>) {
    state.rector_cancel.lock().unwrap().store(true, Ordering::Relaxed);
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
    preview_id: u64,
    files: Vec<String>,
    label: String,
) -> CmdResult<ApplyResult> {
    let path = normalize_path(path);
    let root = project_root(&path)?;
    let selected: Vec<FileChange> = {
        let previews = state.previews.lock().unwrap();
        let (id, preview) = previews
            .get(&path)
            .ok_or_else(|| t!("Önce değişiklikleri hesaplayın.", "Calculate the changes first."))?;
        if *id != preview_id {
            return Err(t!(
                "Önizleme güncel değil. Değişiklikleri yeniden hesaplayın.",
                "The preview is out of date. Calculate the changes again."
            ));
        }
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
    let writes = state.writes.clone();
    let result = blocking(move || {
        let _guard = writes.lock().unwrap_or_else(|e| e.into_inner());
        let refs: Vec<&FileChange> = selected.iter().collect();
        rector::check_fresh(&root, &refs).map_err(|e| e.to_string())?;
        let snapshot = store
            .create(&root, &applied_files, &label)
            .map_err(|e| e.to_string())?;
        if let Err(e) = rector::check_fresh(&root, &refs) {
            let _ = store.delete(&root, &snapshot.id);
            return Err(e.to_string());
        }
        let mut written = Vec::new();
        match rector::write_changes(&root, &refs, &mut written) {
            Ok(()) => {
                if let Ok(json) = serde_json::to_vec(&selected) {
                    let _ = store.write_attachment(&root, &snapshot.id, CHANGES_FILE, &json);
                }
                let written_hashes: HashMap<&str, u64> = selected
                    .iter()
                    .map(|c| (c.file.as_str(), fnv1a(c.new_content.as_bytes())))
                    .collect();
                if let Ok(json) = serde_json::to_vec(&written_hashes) {
                    let _ = store.write_attachment(&root, &snapshot.id, WRITTEN_FILE, &json);
                }
                Ok(ApplyResult { snapshot, written })
            }
            Err(e) => match store.restore_files(&root, &snapshot.id, Some(&written)) {
                Ok(_) => {
                    let _ = store.delete(&root, &snapshot.id);
                    Err(e.to_string())
                }
                Err(restore) => Err(t!(
                    "{e}\nDosyalar geri alınamadı ({restore}). Yedek Geçmiş sayfasında duruyor; oradan geri yükleyebilirsiniz.",
                    "{e}\nCouldn't roll the files back ({restore}). The backup is kept in History; you can restore it from there."
                )),
            },
        }
    })
    .await?;
    if let Some((_, preview)) = state.previews.lock().unwrap().get_mut(&path) {
        preview.changes.retain(|c| !result.written.contains(&c.file));
    }
    Ok(result)
}

#[tauri::command]
async fn list_snapshots(state: State<'_, AppState>, path: String) -> CmdResult<Vec<Snapshot>> {
    let path = normalize_path(path);
    let store = state.snapshots();
    blocking(move || store.list(Path::new(&path)).map_err(|e| e.to_string())).await
}

#[tauri::command]
async fn restore_snapshot(state: State<'_, AppState>, path: String, id: String) -> CmdResult<Vec<String>> {
    let path = normalize_path(path);
    let store = state.snapshots();
    state.previews.lock().unwrap().remove(&path);
    let writes = state.writes.clone();
    blocking(move || {
        let _guard = writes.lock().unwrap_or_else(|e| e.into_inner());
        store.restore(&project_root(&path)?, &id).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn snapshot_conflicts(state: State<'_, AppState>, path: String, id: String) -> CmdResult<Vec<String>> {
    let path = normalize_path(path);
    let store = state.snapshots();
    blocking(move || {
        let root = project_root(&path)?;
        let Some(bytes) = store.read_attachment(&root, &id, WRITTEN_FILE).map_err(|e| e.to_string())? else {
            return Ok(Vec::new());
        };
        let written: HashMap<String, u64> = serde_json::from_slice(&bytes).unwrap_or_default();
        let mut changed: Vec<String> = written
            .into_iter()
            .filter(|(file, hash)| std::fs::read(root.join(file)).map(|b| fnv1a(&b)).ok() != Some(*hash))
            .map(|(file, _)| file)
            .collect();
        changed.sort();
        Ok(changed)
    })
    .await
}

#[tauri::command]
async fn snapshot_changes(state: State<'_, AppState>, path: String, id: String) -> CmdResult<Vec<FileChange>> {
    let path = normalize_path(path);
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
    let path = normalize_path(path);
    let store = state.snapshots();
    blocking(move || store.delete(Path::new(&path), &id).map_err(|e| e.to_string())).await
}

#[tauri::command]
async fn export_report(state: State<'_, AppState>, path: String, destination: String) -> CmdResult<()> {
    let path = normalize_path(path);
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
        std::fs::write(&destination, md).map_err(|e| {
            let e = ferry_core::i18n::io_error(&e);
            t!("Rapor kaydedilemedi: {e}", "Couldn't save the report: {e}")
        })
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
    let failed = |e: &dyn std::fmt::Display| t!("Ayarlar kaydedilemedi: {e}", "Couldn't save the settings: {e}");
    let json = serde_json::to_vec_pretty(&settings).map_err(|e| failed(&e))?;
    std::fs::create_dir_all(&state.data_dir).map_err(|e| failed(&e))?;
    std::fs::write(state.settings_path(), json).map_err(|e| failed(&e))?;
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
        snapshot_conflicts,
        cancel_rector_preview,
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
            remove_stale_runs(&data_dir.join("rector"));
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
        .build(tauri::generate_context!())
        .expect("failed to start Ferry")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                let state = app.state::<AppState>();
                state.shutdown.store(true, Ordering::Relaxed);
                state.rector_cancel.lock().unwrap().store(true, Ordering::Relaxed);
                let started = Instant::now();
                while state.rector_running.load(Ordering::SeqCst) > 0 && started.elapsed() < Duration::from_secs(3) {
                    std::thread::sleep(Duration::from_millis(50));
                }
                while state.writes.try_lock().is_err() && started.elapsed() < Duration::from_secs(10) {
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        });
}

fn remove_stale_runs(work: &Path) {
    let Ok(projects) = std::fs::read_dir(work) else { return };
    for project in projects.flatten() {
        let Ok(entries) = std::fs::read_dir(project.path()) else { continue };
        for entry in entries.flatten() {
            let old = entry
                .metadata()
                .and_then(|m| m.modified())
                .is_ok_and(|t| t.elapsed().is_ok_and(|age| age > Duration::from_secs(60 * 60)));
            if old && entry.file_name().to_string_lossy().starts_with("run-") {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
}

#[cfg(test)]
mod tests;
