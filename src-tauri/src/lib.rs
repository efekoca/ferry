use ferry_core::analyzer::{self, ScanResult};
use ferry_core::deps::{self, DependencyReport};
use ferry_core::history;
use ferry_core::i18n::{self, Lang};
use ferry_core::ignore::{self, Ignore, IgnoredFinding};
use ferry_core::project::{self, ProjectInfo};
use ferry_core::rector::{self, FileChange, RectorError, RectorMode, RectorPreview};
use ferry_core::rules::FixKind;
use ferry_core::runtime;
use ferry_core::snapshot::{fnv1a, Snapshot, SnapshotStore};
use ferry_core::toolchain::{self, ToolchainStatus};
use ferry_core::{diff, report, t, PhpVersion};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

const CHANGES_FILE: &str = "changes.json";
const WRITTEN_FILE: &str = "written.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub php_path: Option<String>,
    pub snapshot_limit: Option<u32>,
    pub editor: Option<String>,
}

pub struct AppState {
    data_dir: PathBuf,
    tools_dir: PathBuf,
    settings: Mutex<Settings>,
    scans: Mutex<HashMap<String, ScanResult>>,
    runtime: Mutex<HashMap<String, RuntimeLog>>,
    latest_scan: Mutex<HashMap<String, u64>>,
    previews: Mutex<HashMap<String, (u64, RectorPreview)>>,
    writes: Arc<Mutex<()>>,
    rector_cancel: Mutex<HashMap<String, Arc<AtomicBool>>>,
    rector_running: Arc<AtomicUsize>,
    shutdown: Arc<AtomicBool>,
    next_preview: AtomicU64,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        let settings = std::fs::read(data_dir.join("settings.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Self {
            tools_dir: data_dir.join("tools"),
            data_dir,
            settings: Mutex::new(settings),
            scans: Mutex::new(HashMap::new()),
            runtime: Mutex::new(HashMap::new()),
            latest_scan: Mutex::new(HashMap::new()),
            previews: Mutex::new(HashMap::new()),
            writes: Arc::new(Mutex::new(())),
            rector_cancel: Mutex::new(HashMap::new()),
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
        self.settings.lock().unwrap().php_path.as_deref().filter(|p| !p.trim().is_empty()).map(PathBuf::from)
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
    let path = if cfg!(windows) { path.replace('/', "\\") } else { path };
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
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| t!("İş parçacığı hatası: {e}", "Worker thread failed: {e}"))?
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
    ignored: Vec<IgnoredFinding>,
    stale_ignores: Vec<Ignore>,
    progress: Option<history::Progress>,
    runtime: Option<RuntimeSummary>,
}

#[derive(Clone)]
struct RuntimeLog {
    file: String,
    events: Vec<runtime::Event>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeSummary {
    file: String,
    events: usize,
    confirmed: usize,
    added: usize,
    outside: usize,
}

fn respond(root: &Path, mut scan: ScanResult, log: Option<&RuntimeLog>) -> ScanResponse {
    let runtime = log.map(|log| {
        let merged = runtime::merge(root, &mut scan.findings, &log.events, scan.target);
        RuntimeSummary {
            file: log.file.clone(),
            events: log.events.len(),
            confirmed: merged.confirmed,
            added: merged.added,
            outside: merged.outside,
        }
    });
    let applied = ignore::apply(root, std::mem::take(&mut scan.findings), &ignore::load(root));
    scan.findings = applied.kept;
    let summary = report::summarize(&scan.findings);
    ScanResponse { scan, summary, ignored: applied.ignored, stale_ignores: applied.stale, progress: None, runtime }
}

impl AppState {
    fn runtime_log(&self, path: &str) -> Option<RuntimeLog> {
        self.runtime.lock().unwrap().get(path).cloned()
    }
}

#[tauri::command]
async fn import_runtime_log(state: State<'_, AppState>, path: String, log: String) -> CmdResult<ScanResponse> {
    let path = normalize_path(path);
    let scan = state.scans.lock().unwrap().get(&path).cloned().ok_or_else(|| t!("Önce projeyi tarayın.", "Scan the project first."))?;
    let (root, entry) = blocking(move || {
        let root = project_root(&path)?;
        let failed = |e: &dyn std::fmt::Display| t!("Log okunamadı: {e}", "Couldn't read the log: {e}");
        let meta = std::fs::metadata(&log).map_err(|e| failed(&ferry_core::i18n::io_error(&e)))?;
        if meta.len() > 200 * 1024 * 1024 {
            return Err(t!("Log 200 MB'tan büyük.", "The log is larger than 200 MB."));
        }
        let bytes = std::fs::read(&log).map_err(|e| failed(&ferry_core::i18n::io_error(&e)))?;
        let events = runtime::parse(&String::from_utf8_lossy(&bytes));
        if events.is_empty() {
            return Err(t!(
                "Log'da PHP uyarısı, kullanımdan kalkma ya da hata satırı bulunamadı.",
                "No PHP warning, deprecation or error lines were found in the log."
            ));
        }
        let name = Path::new(&log).file_name().map_or(log.clone(), |n| n.to_string_lossy().to_string());
        Ok((root, RuntimeLog { file: name, events }))
    })
    .await?;
    let key = root.to_string_lossy().to_string();
    state.runtime.lock().unwrap().insert(key, entry.clone());
    Ok(respond(&root, scan, Some(&entry)))
}

#[tauri::command]
async fn clear_runtime_log(state: State<'_, AppState>, path: String) -> CmdResult<ScanResponse> {
    let path = normalize_path(path);
    let scan = state.scans.lock().unwrap().get(&path).cloned().ok_or_else(|| t!("Önce projeyi tarayın.", "Scan the project first."))?;
    state.runtime.lock().unwrap().remove(&path);
    Ok(respond(&project_root(&path)?, scan, None))
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
                let _ = handle.emit("scan://progress", ScanProgress { run, done, total, file: file.to_string() });
            }
        });
        Ok((path, scan))
    })
    .await?;
    let (path, scan) = result;
    let state = app.state::<AppState>();
    let latest = state.latest_scan.lock().unwrap().get(&path) == Some(&run);
    if latest {
        state.scans.lock().unwrap().insert(path.clone(), scan.clone());
    }
    let mut response = respond(Path::new(&path), scan, state.runtime_log(&path).as_ref());
    if latest {
        let file = state.data_dir.join("history").join(format!("{:016x}.json", fnv1a(path.as_bytes())));
        let target = response.scan.target.to_string();
        response.progress = history::record(&file, now_secs(), &target, response.summary.score, &response.scan.findings).ok();
    }
    Ok(response)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[tauri::command]
async fn ignore_finding(state: State<'_, AppState>, path: String, id: String, reason: String) -> CmdResult<ScanResponse> {
    let path = normalize_path(path);
    let scan = state.scans.lock().unwrap().get(&path).cloned().ok_or_else(|| t!("Önce projeyi tarayın.", "Scan the project first."))?;
    let log = state.runtime_log(&path);
    if reason.trim().is_empty() {
        return Err(t!("Yok sayma için bir gerekçe yazın.", "Write a reason for ignoring it."));
    }
    blocking(move || {
        let root = project_root(&path)?;
        let finding = scan
            .findings
            .iter()
            .find(|f| f.id == id)
            .ok_or_else(|| t!("Bulgu bulunamadı; projeyi yeniden tarayın.", "Finding not found; scan the project again."))?;
        let mut ignores = ignore::load(&root);
        if !ignores.iter().any(|i| i.matches(finding)) {
            ignores.push(Ignore::for_finding(finding, &reason, now_secs()));
        }
        ignore::save(&root, &ignores).map_err(|e| {
            let e = ferry_core::i18n::io_error(&e);
            t!("Yok sayma listesi kaydedilemedi: {e}", "Couldn't save the ignore list: {e}")
        })?;
        Ok(respond(&root, scan, log.as_ref()))
    })
    .await
}

#[tauri::command]
async fn unignore_finding(state: State<'_, AppState>, path: String, id: String) -> CmdResult<ScanResponse> {
    let path = normalize_path(path);
    let scan = state.scans.lock().unwrap().get(&path).cloned().ok_or_else(|| t!("Önce projeyi tarayın.", "Scan the project first."))?;
    let log = state.runtime_log(&path);
    blocking(move || {
        let root = project_root(&path)?;
        let finding = scan
            .findings
            .iter()
            .find(|f| f.id == id)
            .ok_or_else(|| t!("Bulgu bulunamadı; projeyi yeniden tarayın.", "Finding not found; scan the project again."))?;
        let mut ignores = ignore::load(&root);
        ignores.retain(|i| !i.matches(finding));
        ignore::save(&root, &ignores).map_err(|e| {
            let e = ferry_core::i18n::io_error(&e);
            t!("Yok sayma listesi kaydedilemedi: {e}", "Couldn't save the ignore list: {e}")
        })?;
        Ok(respond(&root, scan, log.as_ref()))
    })
    .await
}

#[tauri::command]
async fn prune_ignores(state: State<'_, AppState>, path: String) -> CmdResult<ScanResponse> {
    let path = normalize_path(path);
    let scan = state.scans.lock().unwrap().get(&path).cloned().ok_or_else(|| t!("Önce projeyi tarayın.", "Scan the project first."))?;
    let log = state.runtime_log(&path);
    blocking(move || {
        let root = project_root(&path)?;
        let ignores = ignore::load(&root);
        let stale = ignore::apply(&root, Vec::new(), &ignores).stale;
        let kept: Vec<Ignore> = ignores.into_iter().filter(|i| !stale.contains(i)).collect();
        ignore::save(&root, &kept).map_err(|e| {
            let e = ferry_core::i18n::io_error(&e);
            t!("Yok sayma listesi kaydedilemedi: {e}", "Couldn't save the ignore list: {e}")
        })?;
        Ok(respond(&root, scan, log.as_ref()))
    })
    .await
}

fn encode_path(path: &str) -> String {
    let mut out = String::new();
    for b in path.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' | b':' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn editor_url(editor: &str, file: &Path, line: usize, column: usize) -> Option<String> {
    let path = file.to_string_lossy().replace('\\', "/");
    let path = if path.starts_with('/') { path } else { format!("/{path}") };
    let encoded = encode_path(&path);
    match editor {
        "vscode" => Some(format!("vscode://file{encoded}:{line}:{column}")),
        "cursor" => Some(format!("cursor://file{encoded}:{line}:{column}")),
        "zed" => Some(format!("zed://file{encoded}:{line}:{column}")),
        "phpstorm" => Some(format!("phpstorm://open?file={}&line={line}&column={column}", encode_path(&file.to_string_lossy()))),
        "sublime" => Some(format!("subl://open?url=file://{encoded}&line={line}&column={column}")),
        _ => None,
    }
}

#[tauri::command]
async fn open_in_editor<R: Runtime>(app: AppHandle<R>, path: String, file: String, line: usize, column: usize) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let path = normalize_path(path);
    let root = std::fs::canonicalize(project_root(&path)?).map_err(|e| e.to_string())?;
    let full = std::fs::canonicalize(root.join(&file)).map_err(|_| t!("Dosya bulunamadı: {file}", "File not found: {file}"))?;
    if !full.starts_with(&root) {
        return Err(t!("Dosya proje klasörünün dışında: {file}", "The file is outside the project folder: {file}"));
    }
    let editor = app.state::<AppState>().settings.lock().unwrap().editor.clone().unwrap_or_default();
    let failed = |e: &dyn std::fmt::Display| t!("Dosya editörde açılamadı: {e}", "Couldn't open the file in the editor: {e}");
    match editor_url(&editor, &full, line, column) {
        Some(url) => app.opener().open_url(url, None::<&str>).map_err(|e| failed(&e)),
        None => app.opener().open_path(full.to_string_lossy(), None::<&str>).map_err(|e| failed(&e)),
    }
}

#[tauri::command]
async fn check_dependencies(state: State<'_, AppState>, path: String, target: String) -> CmdResult<DependencyReport> {
    let path = normalize_path(path);
    let target = parse_target(&target)?;
    let php = state.php_override();
    blocking(move || {
        let mut report = deps::check(&project_root(&path)?, target);
        let modules = toolchain::find_php(php.as_deref())
            .and_then(|php| toolchain::command(php).arg("-m").output().ok())
            .map(|out| ferry_core::extensions::parse_modules(&String::from_utf8_lossy(&out.stdout)));
        if let Some(modules) = modules.filter(|m| !m.is_empty()) {
            for e in &mut report.extensions {
                e.available = Some(modules.contains(&e.name));
            }
        }
        Ok(report)
    })
    .await
}

#[derive(Deserialize)]
struct LockedPackage {
    name: String,
    version: String,
}

#[tauri::command]
async fn find_compatible_versions(target: String, packages: Vec<LockedPackage>) -> CmdResult<Vec<ferry_core::packagist::Suggestion>> {
    let target = parse_target(&target)?;
    let curl = toolchain::find_executable("curl").ok_or_else(|| t!("curl bulunamadı.", "curl was not found."))?;
    blocking(move || {
        let fetch = |p: &LockedPackage| {
            let url = ferry_core::packagist::url(&p.name)?;
            let out = toolchain::command(&curl).args(["-fsSL", "--max-time", "20", &url]).output().ok().filter(|o| o.status.success())?;
            ferry_core::packagist::lowest_compatible(&String::from_utf8_lossy(&out.stdout), &p.name, &p.version, target)
        };
        let mut out = Vec::new();
        for chunk in packages.chunks(8) {
            let results: Vec<Option<ferry_core::packagist::Suggestion>> = std::thread::scope(|s| {
                chunk.iter().map(|p| s.spawn(move || fetch(p))).collect::<Vec<_>>().into_iter().map(|h| h.join().ok().flatten()).collect()
            });
            out.extend(results.into_iter().flatten());
        }
        if !packages.is_empty() && out.is_empty() {
            let probe = toolchain::command(&curl).args(["-fsSI", "--max-time", "10", "https://repo.packagist.org/"]).output();
            if !probe.is_ok_and(|o| o.status.success()) {
                return Err(t!("packagist.org'a ulaşılamadı.", "Couldn't reach packagist.org."));
            }
        }
        Ok(out)
    })
    .await
}

#[tauri::command]
async fn compare_targets(path: String) -> CmdResult<Vec<ferry_core::compare::TargetSummary>> {
    let path = normalize_path(path);
    blocking(move || {
        let root = project_root(&path)?;
        let info = project::inspect(&root).map_err(|e| e.to_string())?;
        let files = project::collect_php_files(&root, &info.excluded_dirs);
        let targets: Vec<PhpVersion> = PhpVersion::TARGETS.into_iter().filter(|v| *v != PhpVersion::new(8, 1)).collect();
        Ok(ferry_core::compare::compare(&root, &files, &targets))
    })
    .await
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
async fn rector_preview(state: State<'_, AppState>, path: String, target: String, mode: RectorMode) -> CmdResult<PreviewResponse> {
    let path = normalize_path(path);
    let target = parse_target(&target)?;
    let tools = state.tools_dir();
    let work = state.data_dir.join("rector");
    let php_override = state.php_override();
    let key = path.clone();
    let cancel = Arc::new(AtomicBool::new(false));
    if let Some(previous) = state.rector_cancel.lock().unwrap().insert(key.clone(), cancel.clone()) {
        previous.store(true, Ordering::Relaxed);
    }
    let mine = cancel.clone();
    let running = state.rector_running.clone();
    running.fetch_add(1, Ordering::SeqCst);
    let preview = blocking(move || {
        let _running = RunningGuard(running);
        let root = project_root(&path)?;
        let php = toolchain::find_php(php_override.as_deref()).ok_or_else(|| toolchain::ToolError::PhpMissing.to_string())?;
        toolchain::require_supported_php(&php).map_err(|e| e.to_string())?;
        let info = project::inspect(&root).map_err(|e| e.to_string())?;
        rector::preview(&php, &toolchain::rector_entry(&tools), &root, target, mode, &info.excluded_dirs, &work, &cancel)
            .map_err(|e| e.to_string())
    })
    .await;
    {
        let mut tokens = state.rector_cancel.lock().unwrap();
        let current = tokens.get(&key).is_some_and(|t| Arc::ptr_eq(t, &mine));
        if current {
            tokens.remove(&key);
        }
        if !current || mine.load(Ordering::Relaxed) {
            return Err(RectorError::Cancelled.to_string());
        }
    }
    let preview = preview?;
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
fn cancel_rector_preview(state: State<'_, AppState>, path: Option<String>) {
    let tokens = state.rector_cancel.lock().unwrap();
    match path.map(normalize_path) {
        Some(path) => {
            if let Some(token) = tokens.get(&path) {
                token.store(true, Ordering::Relaxed);
            }
        }
        None => tokens.values().for_each(|t| t.store(true, Ordering::Relaxed)),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplyResult {
    snapshot: Snapshot,
    written: Vec<String>,
    lint_errors: Vec<String>,
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
    let (selected, target): (Vec<FileChange>, PhpVersion) = {
        let previews = state.previews.lock().unwrap();
        let (id, preview) = previews.get(&path).ok_or_else(|| t!("Önce değişiklikleri hesaplayın.", "Calculate the changes first."))?;
        if *id != preview_id {
            return Err(t!(
                "Önizleme güncel değil. Değişiklikleri yeniden hesaplayın.",
                "The preview is out of date. Calculate the changes again."
            ));
        }
        let selected = preview.changes.iter().filter(|c| files.contains(&c.file)).cloned().collect();
        (selected, preview.target)
    };
    if selected.is_empty() {
        return Err(t!("Uygulanacak dosya seçilmedi.", "No files selected to apply."));
    }
    let store = state.snapshots();
    let applied_files: Vec<String> = selected.iter().map(|c| c.file.clone()).collect();
    let writes = state.writes.clone();
    let php_override = state.php_override();
    let snapshot_limit = state.settings.lock().unwrap().snapshot_limit;
    let result = blocking(move || {
        let _guard = writes.lock().unwrap_or_else(|e| e.into_inner());
        let refs: Vec<&FileChange> = selected.iter().collect();
        rector::check_fresh(&root, &refs).map_err(|e| e.to_string())?;
        let snapshot = store.create(&root, &applied_files, &label).map_err(|e| e.to_string())?;
        if let Err(e) = rector::check_fresh(&root, &refs) {
            let _ = store.delete(&root, &snapshot.id);
            return Err(e.to_string());
        }
        let originals: HashMap<String, String> =
            refs.iter().filter_map(|c| Some((c.file.clone(), std::fs::read_to_string(root.join(&c.file)).ok()?))).collect();
        let mut written = Vec::new();
        match rector::write_changes(&root, &refs, &mut written) {
            Ok(()) => {
                if let Ok(json) = serde_json::to_vec(&selected) {
                    let _ = store.write_attachment(&root, &snapshot.id, CHANGES_FILE, &json);
                }
                let written_hashes: HashMap<&str, u64> =
                    selected.iter().map(|c| (c.file.as_str(), fnv1a(c.new_content.as_bytes()))).collect();
                if let Ok(json) = serde_json::to_vec(&written_hashes) {
                    let _ = store.write_attachment(&root, &snapshot.id, WRITTEN_FILE, &json);
                }
                if let Some(limit) = snapshot_limit {
                    let _ = store.prune(&root, limit.max(1) as usize);
                }
                let lint_errors = toolchain::find_php(php_override.as_deref())
                    .filter(|php| toolchain::php_version(php).and_then(|v| v.parse::<PhpVersion>().ok()).is_some_and(|v| v >= target))
                    .map(|php| {
                        let files =
                            selected.iter().map(|c| (c.file.as_str(), originals.get(&c.file).map(String::as_str), c.new_content.as_str()));
                        toolchain::new_syntax_errors(&php, files)
                    })
                    .unwrap_or_default();
                Ok(ApplyResult { snapshot, written, lint_errors })
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
async fn restore_snapshot(
    state: State<'_, AppState>,
    path: String,
    id: String,
    acknowledged: Option<Vec<String>>,
) -> CmdResult<Vec<String>> {
    let path = normalize_path(path);
    let store = state.snapshots();
    let writes = state.writes.clone();
    let restored = blocking(move || {
        let _guard = writes.lock().unwrap_or_else(|e| e.into_inner());
        let root = project_root(&path)?;
        let acknowledged = acknowledged.unwrap_or_default();
        let unseen: Vec<String> = conflicts(&store, &root, &id)?.into_iter().filter(|f| !acknowledged.contains(f)).collect();
        if !unseen.is_empty() {
            let files = unseen.join(", ");
            return Err(t!(
                "Şu dosyalar onaydan sonra değişti: {files}. Geri yüklemeden önce yeniden gözden geçirin.",
                "These files changed after you confirmed: {files}. Review them again before restoring."
            ));
        }
        store.restore(&root, &id).map_err(|e| e.to_string()).map(|files| (path, files))
    })
    .await?;
    state.previews.lock().unwrap().remove(&restored.0);
    Ok(restored.1)
}

fn conflicts(store: &SnapshotStore, root: &Path, id: &str) -> CmdResult<Vec<String>> {
    let Some(bytes) = store.read_attachment(root, id, WRITTEN_FILE).map_err(|e| e.to_string())? else {
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
}

#[tauri::command]
async fn snapshot_conflicts(state: State<'_, AppState>, path: String, id: String) -> CmdResult<Vec<String>> {
    let path = normalize_path(path);
    let store = state.snapshots();
    blocking(move || conflicts(&store, &project_root(&path)?, &id)).await
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
                let after = std::fs::read(root.join(&file)).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
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
async fn snapshot_usage(state: State<'_, AppState>) -> CmdResult<ferry_core::snapshot::Usage> {
    let store = state.snapshots();
    blocking(move || Ok(store.usage())).await
}

#[tauri::command]
async fn delete_all_snapshots(state: State<'_, AppState>) -> CmdResult<()> {
    let store = state.snapshots();
    let writes = state.writes.clone();
    blocking(move || {
        let _guard = writes.lock().unwrap_or_else(|e| e.into_inner());
        store.delete_all().map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn export_report(state: State<'_, AppState>, path: String, destination: String) -> CmdResult<()> {
    let path = normalize_path(path);
    let scan = state.scans.lock().unwrap().get(&path).cloned().ok_or_else(|| t!("Önce projeyi tarayın.", "Scan the project first."))?;
    let log = state.runtime_log(&path);
    blocking(move || {
        let root = project_root(&path)?;
        let info = project::inspect(&root).map_err(|e| e.to_string())?;
        let scan = respond(&root, scan, log.as_ref()).scan;
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
        compare_targets,
        find_compatible_versions,
        toolchain_status,
        install_rector,
        rector_preview,
        apply_changes,
        list_snapshots,
        restore_snapshot,
        delete_snapshot,
        snapshot_changes,
        snapshot_conflicts,
        snapshot_usage,
        delete_all_snapshots,
        cancel_rector_preview,
        export_report,
        ignore_finding,
        unignore_finding,
        prune_ignores,
        import_runtime_log,
        clear_runtime_log,
        open_in_editor,
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
            #[cfg(debug_assertions)]
            let data_dir = std::env::var_os("FERRY_DATA_DIR").map(PathBuf::from).unwrap_or(data_dir);
            std::fs::create_dir_all(&data_dir)?;
            remove_stale_runs(&data_dir.join("rector"));
            #[allow(unused_mut)]
            let mut state = AppState::new(data_dir);
            #[cfg(debug_assertions)]
            if let Some(tools) = std::env::var_os("FERRY_TOOLS_DIR") {
                state.tools_dir = PathBuf::from(tools);
            }
            app.manage(state);
            #[cfg(debug_assertions)]
            if std::env::var_os("FERRY_E2E").is_some() {
                use tauri::Listener;
                app.listen_any("e2e://log", |event| println!("[e2e] {}", event.payload()));
            }
            Ok(())
        })
        .on_page_load(|webview, payload| {
            #[cfg(debug_assertions)]
            if let (Some(script), tauri::webview::PageLoadEvent::Finished) = (std::env::var_os("FERRY_E2E"), payload.event()) {
                if let Ok(body) = std::fs::read_to_string(script) {
                    let root = std::env::var("FERRY_E2E_ROOT").unwrap_or_default();
                    let prelude = format!("window.__FERRY_E2E_ROOT__ = {};", serde_json::to_string(&root).unwrap());
                    let _ = webview.eval(format!("{prelude}\n{body}"));
                }
            }
            #[cfg(not(debug_assertions))]
            let _ = (webview, payload);
        });
    with_commands(builder).build(tauri::generate_context!()).expect("failed to start Ferry").run(|app, event| {
        if let tauri::RunEvent::Exit = event {
            let state = app.state::<AppState>();
            state.shutdown.store(true, Ordering::Relaxed);
            state.rector_cancel.lock().unwrap().values().for_each(|t| t.store(true, Ordering::Relaxed));
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
            let old =
                entry.metadata().and_then(|m| m.modified()).is_ok_and(|t| t.elapsed().is_ok_and(|age| age > Duration::from_secs(60 * 60)));
            if old && entry.file_name().to_string_lossy().starts_with("run-") {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
}

#[cfg(test)]
mod tests;
