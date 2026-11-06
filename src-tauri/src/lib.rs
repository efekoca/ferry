use ferry_core::analyzer::{self, ScanResult};
use ferry_core::deps::{self, DependencyReport};
use ferry_core::git;
use ferry_core::history;
use ferry_core::i18n::{self, Lang};
use ferry_core::ignore::{self, Ignore, IgnoredFinding};
use ferry_core::project::{self, ProjectInfo};
use ferry_core::rector::{self, FileChange, RectorError, RectorMode, RectorPreview};
use ferry_core::rules::FixKind;
use ferry_core::runtime;
use ferry_core::snapshot::{content_hash, fnv1a, Snapshot, SnapshotStore};
use ferry_core::toolchain::{self, ToolchainStatus};
use ferry_core::{database, diff, docker, report, t, PhpVersion};
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
    pub editor_command: Option<String>,
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
    docker_cancel: Mutex<HashMap<String, Arc<AtomicBool>>>,
    docker_results: Mutex<HashMap<String, docker::Comparison>>,
    docker_trials: Mutex<HashMap<String, docker::Comparison>>,
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
            docker_cancel: Mutex::new(HashMap::new()),
            docker_results: Mutex::new(HashMap::new()),
            docker_trials: Mutex::new(HashMap::new()),
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

/// Fixes can stop at an older version on the way to the target, one step at a time.
fn parse_step(target: &str) -> CmdResult<PhpVersion> {
    let v: PhpVersion = target.parse()?;
    let newest = PhpVersion::TARGETS.iter().max().copied().unwrap_or(v);
    if v < PhpVersion::new(7, 0) || v > newest {
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

#[tauri::command]
fn interrupted_writes(state: State<'_, AppState>, path: String) -> Vec<Snapshot> {
    state.snapshots().take_interrupted(Path::new(&normalize_path(path)))
}

#[tauri::command]
async fn save_project_config(path: String, config: ferry_core::config::Config) -> CmdResult<ProjectInfo> {
    let path = normalize_path(path);
    blocking(move || {
        let root = project_root(&path)?;
        ferry_core::config::save(&root, &config)?;
        project::inspect(&root).map_err(|e| e.to_string())
    })
    .await
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
    check: Option<ferry_core::localcheck::CheckRun>,
    roots: Vec<String>,
    /// Set for checks run here: their result only says something about this target.
    target: Option<PhpVersion>,
    php: Option<PhpVersion>,
}

impl RuntimeLog {
    fn applies_to(&self, target: PhpVersion) -> bool {
        self.target.is_none_or(|t| t == target)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeSummary {
    file: String,
    events: usize,
    confirmed: usize,
    added: usize,
    outside: usize,
    outside_fatals: usize,
    check: Option<ferry_core::localcheck::CheckRun>,
    roots: Vec<String>,
}

fn presented(root: &Path, scan: &ScanResult, log: Option<&RuntimeLog>) -> Vec<ferry_core::analyzer::Finding> {
    let mut findings = scan.findings.clone();
    if let Some(log) = log.filter(|l| l.applies_to(scan.target)) {
        runtime::merge_from(root, &mut findings, &log.events, scan.target, &log.roots, log.php);
    }
    findings
}

fn respond(root: &Path, mut scan: ScanResult, log: Option<&RuntimeLog>) -> ScanResponse {
    let runtime = log.filter(|l| l.applies_to(scan.target)).map(|log| {
        let merged = runtime::merge_from(root, &mut scan.findings, &log.events, scan.target, &log.roots, log.php);
        RuntimeSummary {
            file: log.file.clone(),
            events: log.events.len(),
            confirmed: merged.confirmed,
            added: merged.added,
            outside: merged.outside,
            outside_fatals: merged.outside_fatals,
            check: log.check.clone(),
            roots: log.roots.clone(),
        }
    });
    let applied = ignore::apply(root, std::mem::take(&mut scan.findings), &ignore::load(root));
    scan.findings = applied.kept;
    let summary = report::summarize(&scan.findings);
    ScanResponse { scan, summary, ignored: applied.ignored, stale_ignores: applied.stale, progress: None, runtime }
}

impl AppState {
    fn check_history_path(&self, path: &str) -> PathBuf {
        self.data_dir.join("checks").join(format!("{:016x}.json", fnv1a(path.as_bytes())))
    }

    fn docker_history_path(&self, path: &str) -> PathBuf {
        self.data_dir.join("docker").join(format!("{:016x}.json", fnv1a(path.as_bytes())))
    }

    /// A check ran on the old files, so it says nothing about the new ones.
    fn forget_check(&self, path: &str) {
        let mut logs = self.runtime.lock().unwrap();
        if logs.get(path).is_some_and(|l| l.check.is_some()) {
            logs.remove(path);
        }
    }

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
        let text = String::from_utf8_lossy(&bytes);
        let events = runtime::parse(&text);
        if events.is_empty() {
            return Err(t!(
                "Log'da PHP uyarısı, kullanımdan kalkma ya da hata satırı bulunamadı.",
                "No PHP warning, deprecation or error lines were found in the log."
            ));
        }
        let name = Path::new(&log).file_name().map_or(log.clone(), |n| n.to_string_lossy().to_string());
        Ok((root, RuntimeLog { file: name, php: runtime::php_version(&text), events, check: None, roots: Vec::new(), target: None }))
    })
    .await?;
    let mut entry = entry;
    let key = root.to_string_lossy().to_string();
    let mut logs = state.runtime.lock().unwrap();
    entry.roots = logs.get(&key).map(|l| l.roots.clone()).unwrap_or_default();
    logs.insert(key, entry.clone());
    drop(logs);
    Ok(respond(&root, scan, Some(&entry)))
}

#[tauri::command]
async fn set_runtime_roots(state: State<'_, AppState>, path: String, roots: String) -> CmdResult<ScanResponse> {
    let path = normalize_path(path);
    let scan = state.scans.lock().unwrap().get(&path).cloned().ok_or_else(|| t!("Önce projeyi tarayın.", "Scan the project first."))?;
    let root = project_root(&path)?;
    let mut logs = state.runtime.lock().unwrap();
    let log = logs.get_mut(&path).ok_or_else(|| t!("Önce bir log içe aktarın.", "Import a log first."))?;
    log.roots = runtime::parse_roots(&roots);
    let entry = log.clone();
    drop(logs);
    Ok(respond(&root, scan, Some(&entry)))
}

fn selectable_lines() -> Vec<String> {
    PhpVersion::TARGETS.iter().filter(|v| **v != PhpVersion::new(8, 1)).map(ToString::to_string).collect()
}

#[tauri::command]
async fn local_php_versions(state: State<'_, AppState>) -> CmdResult<Vec<toolchain::LocalPhp>> {
    let chosen = state.php_override();
    blocking(move || Ok(toolchain::local_php_with(&selectable_lines(), chosen.as_deref()))).await
}

#[tauri::command]
async fn run_local_check(state: State<'_, AppState>, path: String, target: String, kind: String) -> CmdResult<ScanResponse> {
    let path = normalize_path(path);
    let target = parse_target(&target)?;
    let scan = state.scans.lock().unwrap().get(&path).cloned().ok_or_else(|| t!("Önce projeyi tarayın.", "Scan the project first."))?;
    let shutdown = state.shutdown.clone();
    let chosen = state.php_override();
    let (root, entry) = blocking(move || {
        let root = project_root(&path)?;
        let local = toolchain::local_php_with(&[target.to_string()], chosen.as_deref()).into_iter().next().ok_or_else(|| {
            t!(
                "Bu bilgisayarda PHP {target} bulunamadı. Homebrew, Herd ya da paket yöneticinizle kurun.",
                "PHP {target} wasn't found on this computer. Install it with Homebrew, Herd or your package manager."
            )
        })?;
        let (label, run) = match kind.as_str() {
            "lint" => {
                let info = project::inspect(&root).map_err(|e| e.to_string())?;
                let files = project::collect_php_files(&root, &info.excluded_dirs);
                (format!("php -l · PHP {}", local.version), ferry_core::localcheck::lint(&local.path, &files))
            }
            "tests" => (
                format!("PHPUnit · PHP {}", local.version),
                ferry_core::localcheck::run_tests(&local.path, &root, Duration::from_secs(20 * 60), &shutdown)?,
            ),
            _ => return Err(t!("Bilinmeyen kontrol.", "Unknown check.")),
        };
        let events = runtime::parse(&run.output);
        Ok((root, RuntimeLog { file: label, events, check: Some(run), roots: Vec::new(), target: Some(target), php: None }))
    })
    .await?;
    if let Some(run) = &entry.check {
        let key = root.to_string_lossy();
        let _ = ferry_core::localcheck::record(
            &state.check_history_path(&key),
            run.summary(now_secs(), target.to_string(), entry.file.clone()),
        );
    }
    state.runtime.lock().unwrap().insert(root.to_string_lossy().to_string(), entry.clone());
    Ok(respond(&root, scan, Some(&entry)))
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum DatabaseChoice {
    Dump {
        path: String,
    },
    Live,
    Seed,
    Ssh {
        host: String,
        engine: database::Engine,
        name: String,
        user: String,
        #[serde(default, rename = "sendPassword")]
        send_password: bool,
    },
}

impl DatabaseChoice {
    fn into_source(self) -> database::Source {
        match self {
            DatabaseChoice::Dump { path } => database::Source::Dump(PathBuf::from(path)),
            DatabaseChoice::Live => database::Source::Live,
            DatabaseChoice::Seed => database::Source::Seed,
            DatabaseChoice::Ssh { host, engine, name, user, send_password } => {
                database::Source::Ssh(database::Remote { host, engine, name, user, send_password })
            }
        }
    }
}

#[tauri::command]
fn ssh_hosts() -> Vec<String> {
    database::ssh_hosts()
}

#[tauri::command]
async fn database_plan(path: String) -> CmdResult<database::Plan> {
    let root = project_root(&normalize_path(path))?;
    blocking(move || Ok(database::plan(&root))).await
}

#[derive(Clone, Serialize)]
struct DockerStep {
    step: &'static str,
    version: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DockerResponse {
    comparison: docker::Comparison,
    scan: Option<ScanResponse>,
    outcome: Option<docker::Outcome>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixChoice {
    preview_id: u64,
    files: Vec<String>,
}

impl AppState {
    fn chosen_fixes(&self, path: &str, target: PhpVersion, choice: &FixChoice) -> CmdResult<Vec<FileChange>> {
        let previews = self.previews.lock().unwrap();
        let (id, preview) = previews.get(path).ok_or_else(|| t!("Önce değişiklikleri hesaplayın.", "Calculate the changes first."))?;
        if *id != choice.preview_id || preview.target != target {
            return Err(t!(
                "Önizleme güncel değil. Değişiklikleri yeniden hesaplayın.",
                "The preview is out of date. Calculate the changes again."
            ));
        }
        let chosen: Vec<FileChange> = preview.changes.iter().filter(|c| choice.files.contains(&c.file)).cloned().collect();
        if chosen.is_empty() {
            return Err(t!("Denenecek dosya seçilmedi.", "No files selected to try."));
        }
        Ok(chosen)
    }

    fn keep_docker_result(&self, path: String, root: &Path, comparison: &docker::Comparison) -> Option<ScanResponse> {
        let _ = docker::record(&self.docker_history_path(&path), comparison.summary(now_secs()));
        self.docker_results.lock().unwrap().insert(path.clone(), comparison.clone());
        let entry = RuntimeLog {
            file: format!("Docker · PHP {} → {}", comparison.source.version, comparison.target.version),
            events: comparison.new_on_target.clone(),
            check: None,
            roots: vec![docker::CONTAINER_ROOT.to_string()],
            target: Some(comparison.target.version),
            php: None,
        };
        let scan = self.scans.lock().unwrap().get(&path).cloned();
        if scan.is_some() {
            self.runtime.lock().unwrap().insert(path, entry.clone());
        }
        scan.map(|scan| respond(root, scan, Some(&entry)))
    }
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
async fn docker_compare<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    source: String,
    target: String,
    extensions: bool,
    clean_images: bool,
    pages: Vec<String>,
    database: Option<DatabaseChoice>,
    prepare: Option<String>,
    test: Option<String>,
    fix: Option<FixChoice>,
) -> CmdResult<DockerResponse> {
    let path = normalize_path(path);
    let source: PhpVersion = source.parse().map_err(|_| t!("Geçersiz sürüm.", "Invalid version."))?;
    let target = parse_target(&target)?;
    let state = app.state::<AppState>();
    let before = match &fix {
        Some(_) => Some(
            state
                .docker_results
                .lock()
                .unwrap()
                .get(&path)
                .filter(|c| c.source.version == source && c.target.version == target)
                .cloned()
                .ok_or_else(|| t!("Önce projeyi karşılaştırın.", "Compare the project first."))?,
        ),
        None => None,
    };
    let fixes = fix.as_ref().map(|choice| state.chosen_fixes(&path, target, choice)).transpose()?.unwrap_or_default();
    let cancel = Arc::new(AtomicBool::new(false));
    if let Some(previous) = state.docker_cancel.lock().unwrap().insert(path.clone(), cancel.clone()) {
        previous.store(true, Ordering::Relaxed);
    }
    let running = state.rector_running.clone();
    running.fetch_add(1, Ordering::SeqCst);
    let (handle, key, mine) = (app.clone(), path.clone(), cancel.clone());
    let result = blocking(move || {
        let _running = RunningGuard(running);
        let root = project_root(&key)?;
        rector::check_fresh(&root, &fixes.iter().collect::<Vec<_>>()).map_err(|e| e.to_string())?;
        let options = docker::Options {
            install_extensions: extensions,
            clean_images,
            pages,
            database: database.map(DatabaseChoice::into_source),
            prepare,
            test,
            fixes: fixes.into_iter().map(|c| (c.file, c.new_content)).collect(),
        };
        let comparison = docker::compare(&root, source, target, options, &mine, |step| {
            let (step, version) = match step {
                docker::Step::Installing => ("install", None),
                docker::Step::Extensions(v) => ("extensions", Some(v.to_string())),
                docker::Step::Lint(v) => ("lint", Some(v.to_string())),
                docker::Step::Database => ("database", None),
                docker::Step::Prepare => ("prepare", None),
                docker::Step::Pages(v) => ("pages", Some(v.to_string())),
                docker::Step::Tests(v) => ("tests", Some(v.to_string())),
            };
            let _ = handle.emit("docker://step", DockerStep { step, version });
        })?;
        Ok((root, comparison))
    })
    .await;
    {
        let mut tokens = state.docker_cancel.lock().unwrap();
        if tokens.get(&path).is_some_and(|t| Arc::ptr_eq(t, &cancel)) {
            tokens.remove(&path);
        }
    }
    let (root, comparison) = result?;
    if let Some(before) = before {
        let outcome = Some(docker::outcome(&before, &comparison));
        state.docker_trials.lock().unwrap().insert(path, comparison.clone());
        return Ok(DockerResponse { comparison, scan: None, outcome });
    }
    let scan = state.keep_docker_result(path, &root, &comparison);
    Ok(DockerResponse { comparison, scan, outcome: None })
}

#[tauri::command]
fn keep_docker_trial(state: State<'_, AppState>, path: String) -> CmdResult<DockerResponse> {
    let path = normalize_path(path);
    let root = project_root(&path)?;
    let comparison =
        state.docker_trials.lock().unwrap().remove(&path).ok_or_else(|| t!("Önce düzeltmeleri deneyin.", "Try the fixes first."))?;
    let scan = state.keep_docker_result(path, &root, &comparison);
    Ok(DockerResponse { comparison, scan, outcome: None })
}

#[tauri::command]
fn check_history(state: State<'_, AppState>, path: String) -> Vec<ferry_core::localcheck::Summary> {
    ferry_core::localcheck::history(&state.check_history_path(&normalize_path(path)))
}

#[tauri::command]
fn docker_history(state: State<'_, AppState>, path: String) -> Vec<docker::Summary> {
    docker::history(&state.docker_history_path(&normalize_path(path)))
}

#[tauri::command]
async fn docker_image_usage() -> CmdResult<docker::ImageUsage> {
    blocking(docker::image_usage).await
}

#[tauri::command]
async fn remove_docker_images() -> CmdResult<usize> {
    blocking(docker::remove_images).await
}

#[tauri::command]
fn cancel_docker_compare(state: State<'_, AppState>, path: String) {
    if let Some(token) = state.docker_cancel.lock().unwrap().get(&normalize_path(path)) {
        token.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
fn close_project(state: State<'_, AppState>, path: String) {
    let path = normalize_path(path);
    state.runtime.lock().unwrap().remove(&path);
    state.previews.lock().unwrap().remove(&path);
    state.docker_results.lock().unwrap().remove(&path);
    state.docker_trials.lock().unwrap().remove(&path);
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
    let writes = state.writes.clone();
    blocking(move || {
        let _guard = writes.lock().unwrap_or_else(|e| e.into_inner());
        let root = project_root(&path)?;
        let shown = presented(&root, &scan, log.as_ref());
        let finding = shown
            .iter()
            .find(|f| f.id == id)
            .ok_or_else(|| t!("Bulgu bulunamadı; projeyi yeniden tarayın.", "Finding not found; scan the project again."))?;
        let mut ignores = ignore::load_for_update(&root)?;
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
    let writes = state.writes.clone();
    blocking(move || {
        let _guard = writes.lock().unwrap_or_else(|e| e.into_inner());
        let root = project_root(&path)?;
        let shown = presented(&root, &scan, log.as_ref());
        let finding = shown
            .iter()
            .find(|f| f.id == id)
            .ok_or_else(|| t!("Bulgu bulunamadı; projeyi yeniden tarayın.", "Finding not found; scan the project again."))?;
        let mut ignores = ignore::load_for_update(&root)?;
        if !ignores.iter().any(|i| i.matches(finding)) {
            return Err(t!(
                "Bu bulgu kaynak koddaki bir ferry-ignore yorumuyla yok sayılıyor; yeniden görmek için yorumu kaldırın.",
                "This finding is ignored by a ferry-ignore comment in the code; remove the comment to see it again."
            ));
        }
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
    let writes = state.writes.clone();
    blocking(move || {
        let _guard = writes.lock().unwrap_or_else(|e| e.into_inner());
        let root = project_root(&path)?;
        let ignores = ignore::load_for_update(&root)?;
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

#[derive(Debug, PartialEq, Eq)]
enum EditorCall {
    Url(String),
    Command(PathBuf, Vec<String>),
}

fn split_args(template: &str) -> Vec<String> {
    let (mut args, mut current, mut quote, mut started) = (Vec::new(), String::new(), None, false);
    for c in template.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => current.push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                started = true;
            }
            (None, c) if c.is_whitespace() => {
                if started {
                    args.push(std::mem::take(&mut current));
                }
                started = false;
            }
            (None, c) => {
                current.push(c);
                started = true;
            }
        }
    }
    if started {
        args.push(current);
    }
    args
}

/// Paths are filled in after the command is split, and nothing goes through a shell, so a file
/// name can't turn into an argument or a command of its own.
fn custom_editor(template: &str, file: &Path, line: usize, column: usize) -> Option<EditorCall> {
    let template = template.trim();
    if !template.contains("{file}") {
        return None;
    }
    let fill =
        |text: &str, path: &str| text.replace("{file}", path).replace("{line}", &line.to_string()).replace("{column}", &column.to_string());
    if template
        .split_once("://")
        .is_some_and(|(scheme, _)| !scheme.is_empty() && scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c)))
    {
        let path = file.to_string_lossy().replace('\\', "/");
        return Some(EditorCall::Url(fill(template, &encode_path(&path))));
    }
    let mut args = split_args(template).into_iter().map(|arg| fill(&arg, &file.to_string_lossy()));
    let program = args.next()?;
    let found = toolchain::find_executable(&program).unwrap_or_else(|| PathBuf::from(&program));
    Some(EditorCall::Command(found, args.collect()))
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
    let (editor, command) = {
        let state = app.state::<AppState>();
        let settings = state.settings.lock().unwrap();
        (settings.editor.clone().unwrap_or_default(), settings.editor_command.clone().unwrap_or_default())
    };
    let failed = |e: &dyn std::fmt::Display| t!("Dosya editörde açılamadı: {e}", "Couldn't open the file in the editor: {e}");
    if editor == "custom" {
        let reason = t!("Özel komutta {{file}} olmalı.", "The custom command needs {{file}}.");
        return match custom_editor(&command, &full, line, column).ok_or_else(|| failed(&reason))? {
            EditorCall::Url(url) => app.opener().open_url(url, None::<&str>).map_err(|e| failed(&e)),
            EditorCall::Command(program, args) => {
                let mut child = toolchain::command(&program).args(args).spawn().map_err(|e| failed(&e))?;
                std::thread::spawn(move || child.wait());
                Ok(())
            }
        };
    }
    match editor_url(&editor, &full, line, column) {
        Some(url) => app.opener().open_url(url, None::<&str>).map_err(|e| failed(&e)),
        None if project::is_php_file(&full) => app.opener().open_path(full.to_string_lossy(), None::<&str>).map_err(|e| failed(&e)),
        None => app.opener().reveal_item_in_dir(&full).map_err(|e| failed(&e)),
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
struct WpItem {
    kind: String,
    slug: String,
}

#[tauri::command]
async fn wordpress_latest(items: Vec<WpItem>) -> CmdResult<Vec<ferry_core::wordpress::Latest>> {
    let curl = toolchain::find_executable("curl").ok_or_else(|| t!("curl bulunamadı.", "curl was not found."))?;
    blocking(move || {
        let fetch = |item: &WpItem| {
            let url = ferry_core::wordpress::api_url(&item.kind, &item.slug)?;
            let out = toolchain::command(&curl).args(["-sSL", "--max-time", "20", &url]).output().ok()?;
            ferry_core::wordpress::parse_latest(&item.kind, &item.slug, &String::from_utf8_lossy(&out.stdout))
        };
        let mut out = Vec::new();
        for chunk in items.chunks(8) {
            let results: Vec<_> = std::thread::scope(|s| {
                chunk.iter().map(|i| s.spawn(move || fetch(i))).collect::<Vec<_>>().into_iter().map(|h| h.join().ok().flatten()).collect()
            });
            out.extend(results.into_iter().flatten());
        }
        if !items.is_empty() && out.is_empty() {
            return Err(t!(
                "api.wordpress.org'a ulaşılamadı ya da hiçbir eklenti orada yayımlanmıyor.",
                "Couldn't reach api.wordpress.org, or none of these are published there."
            ));
        }
        Ok(out)
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GitState {
    repo: bool,
    dirty: Vec<String>,
}

#[tauri::command]
async fn git_status(path: String, files: Vec<String>) -> CmdResult<GitState> {
    let path = normalize_path(path);
    blocking(move || {
        let root = project_root(&path)?;
        let status = ferry_core::git::status(&root, &files)?;
        Ok(GitState { repo: status.repo, dirty: status.dirty })
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
async fn write_probe(destination: String) -> CmdResult<String> {
    blocking(move || {
        let token = ferry_core::probe::token();
        std::fs::write(&destination, ferry_core::probe::script(&token)).map_err(|e| {
            let e = ferry_core::i18n::io_error(&e);
            t!("Yoklama dosyası kaydedilemedi: {e}", "Couldn't save the probe file: {e}")
        })?;
        Ok(token)
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
    let target = parse_step(&target)?;
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
        if mode == RectorMode::Pins {
            return Ok(ferry_core::server::bump(&root, target));
        }
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
    lint_checked: bool,
    branch: Option<String>,
    commit: Option<String>,
    git_error: Option<String>,
}

fn check_git(root: &Path, files: &[String], own_commit: bool) -> Result<(), String> {
    if ferry_core::config::load(root)?.git.require_clean {
        let status = git::status(root, &[])?;
        if !status.dirty.is_empty() {
            let count = status.dirty.len();
            return Err(t!(
                "Projede commit edilmemiş {count} dosya var. Proje ayarları temiz çalışma ağacı istiyor; önce bunları commit edin.",
                "The project has {count} uncommitted files, and its settings require a clean working tree. Commit them first."
            ));
        }
    }
    if own_commit {
        let status = git::status(root, files)?;
        if let Some(file) = status.dirty.first() {
            return Err(t!(
                "{file} dosyasında commit edilmemiş değişiklik var, ayrı commit'e karışırdı. Önce commit edin ya da ayrı dal seçeneğini kapatın.",
                "{file} has uncommitted changes that would end up in the separate commit. Commit them first or turn the separate branch off."
            ));
        }
    }
    Ok(())
}

fn batch_branch(target: PhpVersion, mode: RectorMode) -> String {
    let mode = match mode {
        RectorMode::Compat => "compat",
        RectorMode::Modernize => "modernize",
        RectorMode::Pins => "versions",
    };
    format!("ferry/php-{target}-{mode}")
}

#[tauri::command]
async fn apply_changes(
    state: State<'_, AppState>,
    path: String,
    preview_id: u64,
    files: Vec<String>,
    skipped: Option<HashMap<String, Vec<usize>>>,
    label: String,
    commit_message: Option<String>,
) -> CmdResult<ApplyResult> {
    let path = normalize_path(path);
    let root = project_root(&path)?;
    let (selected, target, mode): (Vec<FileChange>, PhpVersion, RectorMode) = {
        let previews = state.previews.lock().unwrap();
        let (id, preview) = previews.get(&path).ok_or_else(|| t!("Önce değişiklikleri hesaplayın.", "Calculate the changes first."))?;
        if *id != preview_id {
            return Err(t!(
                "Önizleme güncel değil. Değişiklikleri yeniden hesaplayın.",
                "The preview is out of date. Calculate the changes again."
            ));
        }
        let selected = preview.changes.iter().filter(|c| files.contains(&c.file)).cloned().collect();
        (selected, preview.target, preview.mode)
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
        check_git(&root, &applied_files, commit_message.is_some())?;
        let selected: Vec<FileChange> = selected
            .iter()
            .map(|change| {
                let mut change = change.clone();
                if let Some(hunks) = skipped.as_ref().and_then(|s| s.get(&change.file)).filter(|h| !h.is_empty()) {
                    let original = std::fs::read_to_string(root.join(&change.file)).map_err(|e| e.to_string())?;
                    change.new_content = diff::without_hunks(&original, &change.new_content, hunks, 3);
                    change.diff = diff::compute(&original, &change.new_content, 3);
                }
                Ok(change)
            })
            .collect::<Result<_, String>>()?;
        let refs: Vec<&FileChange> = selected.iter().collect();
        let snapshot = store.create(&root, &applied_files, &label).map_err(|e| e.to_string())?;
        let written_hashes: HashMap<&str, String> =
            selected.iter().map(|c| (c.file.as_str(), content_hash(c.new_content.as_bytes()))).collect();
        let metadata = serde_json::to_vec(&selected)
            .map_err(|e| e.to_string())
            .and_then(|json| store.write_attachment(&root, &snapshot.id, CHANGES_FILE, &json).map_err(|e| e.to_string()))
            .and_then(|_| serde_json::to_vec(&written_hashes).map_err(|e| e.to_string()))
            .and_then(|json| store.write_attachment(&root, &snapshot.id, WRITTEN_FILE, &json).map_err(|e| e.to_string()));
        if let Err(e) = metadata {
            let _ = store.delete(&root, &snapshot.id);
            return Err(e);
        }
        let originals: HashMap<String, String> =
            refs.iter().filter_map(|c| Some((c.file.clone(), std::fs::read_to_string(root.join(&c.file)).ok()?))).collect();
        let batch = match commit_message {
            Some(_) => match git::start_batch(&root, &batch_branch(target, mode)) {
                Ok(batch) => Some(batch),
                Err(e) => {
                    let _ = store.delete(&root, &snapshot.id);
                    return Err(e);
                }
            },
            None => None,
        };
        let mut written = Vec::new();
        store.mark_writing(&root, &snapshot.id).map_err(|e| e.to_string())?;
        let outcome = rector::write_changes(&root, &refs, &mut written);
        match outcome {
            Ok(()) => {
                store.clear_writing(&root, &snapshot.id);
                if let Some(limit) = snapshot_limit {
                    let _ = store.prune(&root, limit.max(1) as usize);
                }
                let lint = toolchain::find_php(php_override.as_deref())
                    .filter(|php| toolchain::php_version(php).and_then(|v| v.parse::<PhpVersion>().ok()).is_some_and(|v| v >= target))
                    .map(|php| {
                        let files =
                            selected.iter().map(|c| (c.file.as_str(), originals.get(&c.file).map(String::as_str), c.new_content.as_str()));
                        toolchain::new_syntax_errors(&php, files)
                    });
                let lint_checked = lint.as_ref().is_some_and(|l| l.not_run.is_empty());
                let committed = commit_message.as_deref().map(|message| git::commit(&root, &written, message));
                let (commit, git_error) = match committed {
                    Some(Ok(hash)) => (Some(hash), None),
                    Some(Err(e)) => (None, Some(e)),
                    None => (None, None),
                };
                Ok(ApplyResult {
                    snapshot,
                    written,
                    lint_errors: lint.map(|l| l.errors).unwrap_or_default(),
                    lint_checked,
                    branch: batch.map(|b| b.branch),
                    commit,
                    git_error,
                })
            }
            Err(e) => match store.restore_files(&root, &snapshot.id, Some(&written)) {
                Ok(_) => {
                    store.clear_writing(&root, &snapshot.id);
                    let _ = store.delete(&root, &snapshot.id);
                    if let Some(batch) = &batch {
                        git::abort_batch(&root, batch);
                    }
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
    state.forget_check(&path);
    Ok(result)
}

#[tauri::command]
async fn list_snapshots(state: State<'_, AppState>, path: String) -> CmdResult<Vec<Snapshot>> {
    let path = normalize_path(path);
    let store = state.snapshots();
    blocking(move || store.list(Path::new(&path)).map_err(|e| e.to_string())).await
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Conflict {
    file: String,
    hash: Option<String>,
}

#[tauri::command]
async fn restore_snapshot(
    state: State<'_, AppState>,
    path: String,
    id: String,
    acknowledged: Option<Vec<Conflict>>,
) -> CmdResult<Vec<String>> {
    let path = normalize_path(path);
    let store = state.snapshots();
    let writes = state.writes.clone();
    let restored = blocking(move || {
        let _guard = writes.lock().unwrap_or_else(|e| e.into_inner());
        let root = project_root(&path)?;
        let acknowledged = acknowledged.unwrap_or_default();
        let found = conflicts(&store, &root, &id)?;
        let unseen: Vec<&str> = found.iter().filter(|c| !acknowledged.contains(c)).map(|c| c.file.as_str()).collect();
        if !unseen.is_empty() {
            let files = unseen.join(", ");
            return Err(t!(
                "Şu dosyalar onaydan sonra değişti: {files}. Geri yüklemeden önce yeniden gözden geçirin.",
                "These files changed after you confirmed: {files}. Review them again before restoring."
            ));
        }
        if !found.is_empty() {
            keep_overwritten(&store, &root, &id, &found)?;
        }
        store.mark_writing(&root, &id).map_err(|e| e.to_string())?;
        let restored = store.restore(&root, &id).map_err(|e| e.to_string());
        store.clear_writing(&root, &id);
        restored.map(|files| (path, files))
    })
    .await?;
    state.forget_check(&restored.0);
    state.previews.lock().unwrap().remove(&restored.0);
    Ok(restored.1)
}

fn keep_overwritten(store: &SnapshotStore, root: &Path, id: &str, found: &[Conflict]) -> CmdResult<()> {
    let label = store.get(root, id).map_err(|e| e.to_string())?.label;
    let files: Vec<String> = found.iter().filter(|c| c.hash.is_some()).map(|c| c.file.clone()).collect();
    if files.is_empty() {
        return Ok(());
    }
    let backup = store.create(root, &files, &t!("Geri yükleme öncesi · {label}", "Before restore · {label}")).map_err(|e| e.to_string())?;
    let restored: HashMap<String, Option<String>> = store
        .get(root, id)
        .map_err(|e| e.to_string())?
        .files
        .into_iter()
        .filter(|f| files.contains(&f.path))
        .map(|f| (f.path, f.hash))
        .collect();
    let json = serde_json::to_vec(&restored).map_err(|e| e.to_string())?;
    store.write_attachment(root, &backup.id, WRITTEN_FILE, &json).map_err(|e| {
        let _ = store.delete(root, &backup.id);
        e.to_string()
    })
}

fn conflicts(store: &SnapshotStore, root: &Path, id: &str) -> CmdResult<Vec<Conflict>> {
    let current = |file: &str| std::fs::read(root.join(file)).ok().map(|b| content_hash(&b));
    let unreadable = || {
        t!(
            "Yedeğin uygulama kaydı okunamadı; değişiklik kontrolü yapılamadığı için geri yükleme durduruldu.",
            "The backup's apply record couldn't be read; restore stopped because edits can't be checked."
        )
    };
    let expected: HashMap<String, Option<String>> = match store.read_attachment(root, id, WRITTEN_FILE).map_err(|e| e.to_string())? {
        Some(bytes) => serde_json::from_slice::<HashMap<String, serde_json::Value>>(&bytes)
            .map_err(|_| unreadable())?
            .into_iter()
            .map(|(file, hash)| match hash {
                serde_json::Value::String(h) => Ok((file, Some(h))),
                serde_json::Value::Number(n) => n.as_u64().map(|n| (file, Some(format!("{n:016x}")))).ok_or_else(unreadable),
                serde_json::Value::Null => Ok((file, None)),
                _ => Err(unreadable()),
            })
            .collect::<CmdResult<_>>()?,
        None => store.get(root, id).map_err(|e| e.to_string())?.files.into_iter().map(|f| (f.path, f.hash)).collect(),
    };
    let mut changed: Vec<Conflict> = expected
        .into_iter()
        .filter_map(|(file, hash)| {
            let now = current(&file);
            (now != hash).then_some(Conflict { file, hash: now })
        })
        .collect();
    changed.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(changed)
}

#[tauri::command]
async fn snapshot_conflicts(state: State<'_, AppState>, path: String, id: String) -> CmdResult<Vec<Conflict>> {
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
    let writes = state.writes.clone();
    blocking(move || {
        let _guard = writes.lock().unwrap_or_else(|e| e.into_inner());
        store.delete(Path::new(&path), &id).map_err(|e| e.to_string())
    })
    .await
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
    let comparison = state.docker_results.lock().unwrap().get(&path).filter(|c| c.target.version == scan.target).cloned();
    blocking(move || {
        let root = project_root(&path)?;
        let info = project::inspect(&root).map_err(|e| e.to_string())?;
        let scan = respond(&root, scan, log.as_ref()).scan;
        let deps = deps::check(&root, scan.target);
        let md = report::to_markdown(&info, &scan, Some(&deps), comparison.as_ref(), i18n::lang());
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
    let path = state.settings_path();
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, &path)).map_err(|e| failed(&e))?;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

pub fn with_commands<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        inspect_project,
        save_project_config,
        interrupted_writes,
        scan_project,
        check_dependencies,
        write_probe,
        git_status,
        compare_targets,
        find_compatible_versions,
        wordpress_latest,
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
        set_runtime_roots,
        docker_compare,
        keep_docker_trial,
        database_plan,
        ssh_hosts,
        cancel_docker_compare,
        docker_image_usage,
        docker_history,
        check_history,
        remove_docker_images,
        clear_runtime_log,
        local_php_versions,
        run_local_check,
        close_project,
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
            state.docker_cancel.lock().unwrap().values().for_each(|t| t.store(true, Ordering::Relaxed));
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
