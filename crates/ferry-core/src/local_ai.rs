//! A model that runs on this computer with llama.cpp. Ferry downloads both on request, each pinned
//! by size and SHA-256, and starts the server only on 127.0.0.1 with a key made for that run.

use crate::ai::{Kind, Prompt, Provider, Remote};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The nightly build llama.cpp's v0.9.0 release points to.
pub const LLAMA_BUILD: &str = "b12110";
const GIB: u64 = 1 << 30;
const CONTEXT: &str = "8192";

struct Runtime {
    os: &'static str,
    arch: &'static str,
    file: &'static str,
    size: u64,
    sha256: &'static str,
}

const RUNTIMES: [Runtime; 6] = [
    Runtime {
        os: "macos",
        arch: "aarch64",
        file: "llama-b12110-bin-macos-arm64.tar.gz",
        size: 16_650_691,
        sha256: "4f5b8de12e0b9516658ca6a5764c6ffb3d7c659c3dd538556b1413c7b4050662",
    },
    Runtime {
        os: "macos",
        arch: "x86_64",
        file: "llama-b12110-bin-macos-x64.tar.gz",
        size: 12_388_601,
        sha256: "c8094268b6bf62f4998f0e45656fd9f5e2d4afb6b1cd0044d761db64be1dd47d",
    },
    Runtime {
        os: "linux",
        arch: "x86_64",
        file: "llama-b12110-bin-ubuntu-vulkan-x64.tar.gz",
        size: 35_858_360,
        sha256: "a5d741940956ec62e8783aee7e5a4c7c761af867d6f027c3277a51e1a3ac59b0",
    },
    Runtime {
        os: "linux",
        arch: "aarch64",
        file: "llama-b12110-bin-ubuntu-vulkan-arm64.tar.gz",
        size: 29_168_359,
        sha256: "1c68bf23d35230cc607e3d30f8d447d21fef500d9fa8d65e15520dfa3619ad73",
    },
    Runtime {
        os: "windows",
        arch: "x86_64",
        file: "llama-b12110-bin-win-vulkan-x64.zip",
        size: 37_661_146,
        sha256: "9732a5e48887ca54d0bfe4af4324dc85fd56cd8cda19b52df3542fa6ebe6874f",
    },
    Runtime {
        os: "windows",
        arch: "aarch64",
        file: "llama-b12110-bin-win-vulkan-arm64.zip",
        size: 25_728_494,
        sha256: "2be672cf538aabfc9fcb22c21648422b01d623673f15c2742bd43a9bd1a11ce2",
    },
];

fn runtime() -> Option<&'static Runtime> {
    RUNTIMES.iter().find(|r| r.os == std::env::consts::OS && r.arch == std::env::consts::ARCH)
}

/// A model Ferry offers, pinned to one commit of its Hugging Face repository.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: &'static str,
    pub name: &'static str,
    pub license: &'static str,
    pub repo: &'static str,
    pub size: u64,
    #[serde(skip)]
    commit: &'static str,
    #[serde(skip)]
    file: &'static str,
    #[serde(skip)]
    sha256: &'static str,
    #[serde(skip)]
    min_memory: u64,
}

/// Biggest first: a computer gets the first one its memory holds.
pub const MODELS: [Model; 3] = [
    Model {
        id: "qwen3-coder-30b-a3b",
        name: "Qwen3 Coder 30B-A3B",
        license: "Apache-2.0",
        repo: "ggml-org/Qwen3-Coder-30B-A3B-Instruct-GGUF",
        size: 17_988_419_552,
        commit: "0ca51122012fe4a20dc82988ee58780b008ce4e3",
        file: "Qwen3-Coder-30B-A3B-Instruct-Q4_K_M.gguf",
        sha256: "f6050b4da22869aa3fb9d94bb3314e9fc5a027d4741c1f97761c387d6b2b2335",
        min_memory: 28 * GIB,
    },
    Model {
        id: "qwen3.5-9b",
        name: "Qwen3.5 9B",
        license: "Apache-2.0",
        repo: "unsloth/Qwen3.5-9B-GGUF",
        size: 5_680_522_464,
        commit: "3885219b6810b007914f3a7950a8d1b469d598a5",
        file: "Qwen3.5-9B-Q4_K_M.gguf",
        sha256: "03b74727a860a56338e042c4420bb3f04b2fec5734175f4cb9fa853daf52b7e8",
        min_memory: 12 * GIB,
    },
    Model {
        id: "qwen3.5-4b",
        name: "Qwen3.5 4B",
        license: "Apache-2.0",
        repo: "unsloth/Qwen3.5-4B-GGUF",
        size: 2_740_937_888,
        commit: "e87f176479d0855a907a41277aca2f8ee7a09523",
        file: "Qwen3.5-4B-Q4_K_M.gguf",
        sha256: "00fe7986ff5f6b463e62455821146049db6f9313603938a70800d1fb69ef11a4",
        min_memory: 6 * GIB,
    },
];

pub fn model(id: &str) -> Option<&'static Model> {
    MODELS.iter().find(|m| m.id == id)
}

/// The model for a computer with this much memory, if it has enough for any and llama.cpp has a
/// build for it.
pub fn recommended(memory: u64) -> Option<&'static Model> {
    runtime()?;
    MODELS.iter().find(|m| memory >= m.min_memory)
}

#[cfg(unix)]
pub fn total_memory() -> u64 {
    let (pages, size) = unsafe { (libc::sysconf(libc::_SC_PHYS_PAGES), libc::sysconf(libc::_SC_PAGESIZE)) };
    if pages > 0 && size > 0 {
        pages as u64 * size as u64
    } else {
        0
    }
}

#[cfg(windows)]
pub fn total_memory() -> u64 {
    #[repr(C)]
    struct MemoryStatus {
        length: u32,
        load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page: u64,
        avail_page: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended: u64,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalMemoryStatusEx(buffer: *mut MemoryStatus) -> i32;
    }
    let mut status = MemoryStatus {
        length: std::mem::size_of::<MemoryStatus>() as u32,
        load: 0,
        total_phys: 0,
        avail_phys: 0,
        total_page: 0,
        avail_page: 0,
        total_virtual: 0,
        avail_virtual: 0,
        avail_extended: 0,
    };
    if unsafe { GlobalMemoryStatusEx(&mut status) } != 0 {
        status.total_phys
    } else {
        0
    }
}

#[cfg(unix)]
pub fn free_space(path: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    (unsafe { libc::statvfs(path.as_ptr(), &mut stat) } == 0).then(|| stat.f_bavail as u64 * stat.f_frsize as u64)
}

#[cfg(windows)]
pub fn free_space(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(dir: *const u16, free: *mut u64, total: *mut u64, total_free: *mut u64) -> i32;
    }
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut free = 0u64;
    (unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, std::ptr::null_mut(), std::ptr::null_mut()) } != 0).then_some(free)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Runtime,
    Model,
    Verify,
    Check,
}

fn stopped() -> String {
    crate::t!("Yerel model kurulumu durduruldu.", "The local model setup was stopped.")
}

fn sha256_of(path: &Path, cancel: &AtomicBool, progress: &dyn Fn(u64)) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    let mut done = 0u64;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(stopped());
        }
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        done += n as u64;
        progress(done);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Downloads `url` to `dest` through a `.part` file that a later run resumes, and moves it into
/// place only once its size and SHA-256 match.
fn fetch(
    url: &str,
    dest: &Path,
    size: u64,
    sha256: &str,
    cancel: &AtomicBool,
    progress: &(dyn Fn(Phase, u64, u64) + Sync),
    phase: Phase,
) -> Result<(), String> {
    let mut part = dest.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    if std::fs::metadata(&part).is_ok_and(|m| m.len() > size) {
        let _ = std::fs::remove_file(&part);
    }
    if std::fs::metadata(&part).map_or(true, |m| m.len() < size) {
        let curl = crate::toolchain::find_executable("curl").ok_or_else(|| crate::t!("curl bulunamadı.", "curl wasn't found."))?;
        let mut cmd = crate::toolchain::command(&curl);
        cmd.args(["-fsSL", "--retry", "3", "--connect-timeout", "30", "--speed-time", "60", "--speed-limit", "1024", "-C", "-", "-o"])
            .arg(&part)
            .arg(url);
        let finished = AtomicBool::new(false);
        let result = std::thread::scope(|scope| {
            scope.spawn(|| {
                while !finished.load(Ordering::Relaxed) {
                    progress(phase, std::fs::metadata(&part).map_or(0, |m| m.len()), size);
                    std::thread::sleep(Duration::from_millis(300));
                }
            });
            let result = crate::toolchain::run_process(cmd, &curl, cancel, Duration::from_secs(24 * 3600));
            finished.store(true, Ordering::Relaxed);
            result
        });
        let done = result.map_err(|e| e.to_string())?;
        if done.stopped.is_some() {
            return Err(stopped());
        }
        if !done.status.is_some_and(|s| s.success()) {
            let reason = String::from_utf8_lossy(&done.stderr).trim().to_string();
            return Err(crate::t!("İndirilemedi: {reason}", "Couldn't download it: {reason}"));
        }
    }
    let got = std::fs::metadata(&part).map_or(0, |m| m.len());
    if got != size {
        return Err(crate::t!("İndirilen dosyanın boyutu beklenenden farklı.", "The downloaded file isn't the expected size."));
    }
    let hash = sha256_of(&part, cancel, &|done| progress(Phase::Verify, done, size))?;
    if hash != sha256 {
        let _ = std::fs::remove_file(&part);
        return Err(crate::t!(
            "İndirilen dosya doğrulanamadı (SHA-256 farklı); silindi.",
            "The downloaded file didn't match its SHA-256, so it was deleted."
        ));
    }
    std::fs::rename(&part, dest).map_err(|e| e.to_string())
}

fn tar_program() -> Option<PathBuf> {
    #[cfg(windows)]
    if let Some(root) = std::env::var_os("SystemRoot") {
        let tar = PathBuf::from(root).join("System32").join("tar.exe");
        if tar.is_file() {
            return Some(tar);
        }
    }
    crate::toolchain::find_executable("tar")
}

/// Where llama.cpp and the models live, under Ferry's tools folder.
#[derive(Debug, Clone)]
pub struct Install {
    dir: PathBuf,
}

impl Install {
    pub fn new(dir: PathBuf) -> Self {
        Install { dir }
    }

    fn runtime_dir(&self) -> PathBuf {
        self.dir.join(LLAMA_BUILD)
    }

    fn server(&self) -> Option<PathBuf> {
        let name = if cfg!(windows) { "llama-server.exe" } else { "llama-server" };
        walkdir::WalkDir::new(self.runtime_dir())
            .max_depth(3)
            .into_iter()
            .flatten()
            .find(|e| e.file_type().is_file() && e.file_name() == name)
            .map(|e| e.into_path())
    }

    fn model_path(&self, model: &Model) -> PathBuf {
        self.dir.join("models").join(model.file)
    }

    pub fn installed(&self, model: &Model) -> bool {
        self.server().is_some() && std::fs::metadata(self.model_path(model)).is_ok_and(|m| m.len() == model.size)
    }

    /// Bytes still to download for `model`.
    pub fn remaining(&self, model: &Model) -> u64 {
        let runtime = if self.server().is_some() { 0 } else { runtime().map_or(0, |r| r.size) };
        let file = if self.model_path(model).is_file() { 0 } else { model.size };
        runtime + file
    }

    fn install_runtime(&self, cancel: &AtomicBool, progress: &(dyn Fn(Phase, u64, u64) + Sync)) -> Result<(), String> {
        let runtime =
            runtime().ok_or_else(|| crate::t!("Bu sistem için llama.cpp derlemesi yok.", "There's no llama.cpp build for this system."))?;
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let archive = self.dir.join(runtime.file);
        let url = format!("https://github.com/ggml-org/llama.cpp/releases/download/{LLAMA_BUILD}/{}", runtime.file);
        fetch(&url, &archive, runtime.size, runtime.sha256, cancel, progress, Phase::Runtime)?;
        let tar = tar_program().ok_or_else(|| crate::t!("tar bulunamadı.", "tar wasn't found."))?;
        let staging = self.dir.join(format!(".{LLAMA_BUILD}"));
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
        let mut cmd = crate::toolchain::command(&tar);
        cmd.arg("-xf").arg(&archive).arg("-C").arg(&staging);
        let done = crate::toolchain::run_process(cmd, &tar, cancel, Duration::from_secs(600)).map_err(|e| e.to_string())?;
        if done.stopped.is_some() {
            return Err(stopped());
        }
        if !done.status.is_some_and(|s| s.success()) {
            let reason = String::from_utf8_lossy(&done.stderr).trim().to_string();
            return Err(crate::t!("llama.cpp açılamadı: {reason}", "Couldn't unpack llama.cpp: {reason}"));
        }
        let _ = std::fs::remove_dir_all(self.runtime_dir());
        std::fs::rename(&staging, self.runtime_dir()).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&archive);
        for old in std::fs::read_dir(&self.dir).into_iter().flatten().flatten() {
            let name = old.file_name().to_string_lossy().to_string();
            if name != LLAMA_BUILD && name.starts_with('b') && name[1..].chars().all(|c| c.is_ascii_digit()) {
                let _ = std::fs::remove_dir_all(old.path());
            }
        }
        if self.server().is_none() {
            return Err(crate::t!("llama.cpp arşivinde llama-server bulunamadı.", "The llama.cpp archive had no llama-server."));
        }
        Ok(())
    }

    /// Downloads whatever is missing, then runs the model once to see that it answers.
    pub fn install(&self, model: &'static Model, cancel: &AtomicBool, progress: &(dyn Fn(Phase, u64, u64) + Sync)) -> Result<(), String> {
        std::fs::create_dir_all(self.dir.join("models")).map_err(|e| e.to_string())?;
        let need = self.remaining(model);
        if let Some(free) = free_space(&self.dir) {
            if free < need + GIB {
                let need = format!("{:.1}", (need + GIB) as f64 / GIB as f64);
                return Err(crate::t!(
                    "Yeterli boş alan yok: yaklaşık {need} GB gerekiyor.",
                    "There isn't enough free space: about {need} GB is needed."
                ));
            }
        }
        if self.server().is_none() {
            self.install_runtime(cancel, progress)?;
        }
        let path = self.model_path(model);
        if !path.is_file() {
            let url = format!("https://huggingface.co/{}/resolve/{}/{}", model.repo, model.commit, model.file);
            fetch(&url, &path, model.size, model.sha256, cancel, progress, Phase::Model)?;
        }
        progress(Phase::Check, 0, 1);
        self.check(model, cancel)
    }

    /// Asks the model to give back a few lines unchanged: the server starts, the template works,
    /// no thinking comes back, and the code survives the trip.
    pub fn check(&self, model: &'static Model, cancel: &AtomicBool) -> Result<(), String> {
        let server = Server::start(self, model, cancel)?;
        let code = "<?php\n$total = 0;\nforeach ($items as $item) {\n    $total += $item;\n}";
        let prompt = Prompt::plain(
            "Return the PHP code you are given exactly as it is, in one ```php block, with nothing else.",
            &format!("```php\n{code}\n```"),
        );
        let reply = server.remote(model).complete(&prompt, cancel)?;
        match crate::ai::code_block(&reply) {
            Some(back) if back.trim_end() == code => Ok(()),
            _ => Err(crate::t!(
                "Model denendi ama beklenen yanıtı vermedi; bu bilgisayarda doğru çalışmıyor olabilir.",
                "The model was tried but didn't answer as expected, so it may not work correctly on this computer."
            )),
        }
    }

    pub fn remove(&self) -> Result<(), String> {
        match std::fs::remove_dir_all(&self.dir) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    }

    fn record(&self) -> PathBuf {
        self.dir.join("server.json")
    }

    /// Stops a server a previous run of Ferry left behind, once it proves it's ours: it answers on
    /// the recorded port to the recorded key with one of Ferry's model names.
    pub fn reap(&self) {
        let Some(record) = std::fs::read(self.record()).ok().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok()) else {
            return;
        };
        let _ = std::fs::remove_file(self.record());
        let (Some(pid), Some(port), Some(key)) = (record["pid"].as_u64(), record["port"].as_u64(), record["key"].as_str()) else { return };
        let (Ok(pid), Ok(port)) = (u32::try_from(pid), u16::try_from(port)) else { return };
        if !crate::toolchain::process_alive(pid) {
            return;
        }
        let ours =
            get(port, "/v1/models", Some(key)).is_some_and(|(status, body)| status == 200 && MODELS.iter().any(|m| body.contains(m.id)));
        if ours {
            kill_pid(pid);
        }
    }
}

fn kill_pid(pid: u32) {
    #[cfg(unix)]
    if let Ok(pid) = i32::try_from(pid) {
        unsafe { libc::kill(pid, libc::SIGKILL) };
    }
    #[cfg(windows)]
    {
        let _ = crate::toolchain::command(Path::new("taskkill")).args(["/F", "/PID", &pid.to_string()]).output();
    }
}

fn get(port: u16, path: &str, key: Option<&str>) -> Option<(u16, String)> {
    let address = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = std::net::TcpStream::connect_timeout(&address, Duration::from_secs(1)).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let auth = key.map(|k| format!("Authorization: Bearer {k}\r\n")).unwrap_or_default();
    let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{auth}Connection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).ok()?;
    let mut bytes = Vec::new();
    let _ = stream.read_to_end(&mut bytes);
    let text = String::from_utf8_lossy(&bytes);
    let status = text.split_whitespace().nth(1)?.parse().ok()?;
    Some((status, text.split_once("\r\n\r\n").map(|(_, body)| body.to_string()).unwrap_or_default()))
}

fn random_key() -> Result<String, String> {
    let mut bytes = [0u8; 24];
    getrandom::getrandom(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn log_tail(path: &Path) -> String {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    lines[lines.len().saturating_sub(4)..].join("\n")
}

/// A running llama-server. Dropping it stops the process.
pub struct Server {
    child: std::process::Child,
    port: u16,
    key: String,
    model: &'static str,
    record: PathBuf,
}

impl Server {
    fn start(install: &Install, model: &'static Model, cancel: &AtomicBool) -> Result<Server, String> {
        let exe = install.server().ok_or_else(|| crate::t!("llama.cpp kurulu değil.", "llama.cpp isn't installed."))?;
        install.reap();
        let port = std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()).map_err(|e| e.to_string())?.port();
        let key = random_key()?;
        let log_path = install.dir.join("server.log");
        let log = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;
        let mut cmd = crate::toolchain::command(&exe);
        cmd.arg("-m").arg(install.model_path(model));
        cmd.args(["--alias", model.id, "--host", "127.0.0.1", "--port", &port.to_string(), "-c", CONTEXT, "-np", "1"]);
        cmd.args(["--jinja", "--reasoning", "off", "--reasoning-budget", "0", "--no-webui", "--offline"]);
        cmd.env("LLAMA_API_KEY", &key);
        if let Some(dir) = exe.parent() {
            cmd.current_dir(dir);
        }
        cmd.stdin(std::process::Stdio::null()).stdout(log.try_clone().map_err(|e| e.to_string())?).stderr(log);
        let child = cmd.spawn().map_err(|e| crate::t!("llama.cpp başlatılamadı: {e}", "Couldn't start llama.cpp: {e}"))?;
        let record = install.record();
        let mut server = Server { child, port, key, model: model.id, record };
        let _ =
            std::fs::write(&server.record, serde_json::json!({ "pid": server.child.id(), "port": port, "key": server.key }).to_string());
        let started = Instant::now();
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(crate::t!("AI önerisi durduruldu.", "The AI suggestion was stopped."));
            }
            if let Ok(Some(_)) = server.child.try_wait() {
                let tail = log_tail(&log_path);
                return Err(crate::t!("llama.cpp kapandı: {tail}", "llama.cpp exited: {tail}"));
            }
            if get(port, "/health", None).is_some_and(|(status, _)| status == 200) {
                return Ok(server);
            }
            if started.elapsed() > Duration::from_secs(300) {
                return Err(crate::t!("Model beş dakikada yüklenemedi.", "The model didn't load within five minutes."));
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    fn running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    fn remote(&self, model: &Model) -> Remote {
        Remote {
            kind: Kind::Openai,
            model: model.id.to_string(),
            base_url: format!("http://127.0.0.1:{}/v1", self.port),
            key: Some(self.key.clone()),
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.record);
    }
}

/// The one server Ferry keeps, shared by every provider that uses it, so quitting or switching
/// providers always has something to stop.
#[derive(Default)]
pub struct Slot {
    server: Mutex<Option<Server>>,
    used: Mutex<Option<Instant>>,
}

impl Slot {
    pub fn stop(&self) {
        self.server.lock().unwrap().take();
    }

    pub fn stop_if_idle(&self, after: Duration) {
        if self.used.lock().unwrap().is_some_and(|t| t.elapsed() > after) {
            self.stop();
            self.used.lock().unwrap().take();
        }
    }

    fn touch(&self) {
        *self.used.lock().unwrap() = Some(Instant::now());
    }
}

/// The installed model as a provider. The server starts on the first request and stops after a
/// while without one.
pub struct LocalModel {
    install: Install,
    model: &'static Model,
    slot: std::sync::Arc<Slot>,
}

impl LocalModel {
    pub fn new(install: Install, model: &'static Model, slot: std::sync::Arc<Slot>) -> Self {
        LocalModel { install, model, slot }
    }

    fn remote(&self, cancel: &AtomicBool) -> Result<Remote, String> {
        let mut server = self.slot.server.lock().unwrap();
        if server.as_mut().is_some_and(|s| s.model != self.model.id || !s.running()) {
            server.take();
        }
        if server.is_none() {
            *server = Some(Server::start(&self.install, self.model, cancel)?);
        }
        self.slot.touch();
        Ok(server.as_ref().map(|s| s.remote(self.model)).expect("started above"))
    }
}

impl Provider for LocalModel {
    fn name(&self) -> String {
        let name = self.model.name;
        crate::t!("Yerel model ({name})", "Local model ({name})")
    }

    fn complete(&self, prompt: &Prompt, cancel: &AtomicBool) -> Result<String, String> {
        let reply = self.remote(cancel)?.complete(prompt, cancel);
        self.slot.touch();
        reply
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_computer_gets_the_biggest_model_its_memory_holds() {
        if runtime().is_none() {
            return;
        }
        assert_eq!(recommended(64 * GIB).map(|m| m.id), Some("qwen3-coder-30b-a3b"));
        assert_eq!(recommended(32 * GIB).map(|m| m.id), Some("qwen3-coder-30b-a3b"));
        assert_eq!(recommended(24 * GIB).map(|m| m.id), Some("qwen3.5-9b"));
        assert_eq!(recommended(16 * GIB).map(|m| m.id), Some("qwen3.5-9b"));
        assert_eq!(recommended(8 * GIB).map(|m| m.id), Some("qwen3.5-4b"));
        assert!(recommended(4 * GIB).is_none());
        assert!(total_memory() > GIB);
        assert!(free_space(&std::env::temp_dir()).is_some_and(|f| f > 0));
    }

    #[test]
    fn every_download_is_pinned_by_commit_size_and_hash() {
        for m in &MODELS {
            assert_eq!(m.commit.len(), 40, "{}", m.id);
            assert_eq!(m.sha256.len(), 64, "{}", m.id);
            assert!(m.sha256.chars().all(|c| c.is_ascii_hexdigit()) && m.size > GIB && m.license == "Apache-2.0", "{}", m.id);
        }
        for r in &RUNTIMES {
            assert!(r.file.contains(LLAMA_BUILD) && r.sha256.len() == 64 && r.size > 1_000_000, "{}", r.file);
        }
    }

    /// Serves `body` for every request until the test ends.
    fn serve(body: &'static [u8]) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut seen = Vec::new();
                let mut chunk = [0u8; 1024];
                while !seen.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => seen.extend_from_slice(&chunk[..n]),
                    }
                }
                let _ = write!(stream, "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n", body.len());
                let _ = stream.write_all(body);
            }
        });
        url
    }

    #[test]
    fn a_download_is_kept_only_when_its_size_and_hash_match() {
        if crate::toolchain::find_executable("curl").is_none() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let url = serve(b"model weights");
        let hash = sha256_of(
            &{
                let p = dir.path().join("sample");
                std::fs::write(&p, b"model weights").unwrap();
                p
            },
            &AtomicBool::new(false),
            &|_| {},
        )
        .unwrap();
        let seen = Mutex::new(Vec::new());
        let progress = |phase: Phase, done: u64, _: u64| seen.lock().unwrap().push((phase, done));
        let dest = dir.path().join("m.gguf");
        fetch(&url, &dest, 13, &hash, &AtomicBool::new(false), &progress, Phase::Model).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"model weights");
        assert!(seen.lock().unwrap().iter().any(|(p, d)| *p == Phase::Verify && *d == 13));

        let wrong = dir.path().join("w.gguf");
        let err = fetch(&url, &wrong, 13, &"0".repeat(64), &AtomicBool::new(false), &progress, Phase::Model).unwrap_err();
        assert!(err.contains("SHA-256"), "{err}");
        assert!(!wrong.exists() && !dir.path().join("w.gguf.part").exists());
        let short = fetch(&url, &dir.path().join("s.gguf"), 99, &hash, &AtomicBool::new(false), &progress, Phase::Model).unwrap_err();
        assert!(!short.is_empty());
    }

    #[test]
    fn a_leftover_server_is_stopped_only_when_it_proves_its_ours() {
        let dir = tempfile::tempdir().unwrap();
        let install = Install::new(dir.path().to_path_buf());
        std::fs::create_dir_all(dir.path()).unwrap();
        let mut sleeper = if cfg!(windows) {
            crate::toolchain::command("ping").args(["-n", "30", "127.0.0.1"]).spawn().unwrap()
        } else {
            std::process::Command::new("sleep").arg("30").spawn().unwrap()
        };
        let stranger = serve(br#"{"data":[{"id":"someone-elses-model"}]}"#);
        let port = stranger.rsplit(':').next().unwrap().parse::<u16>().unwrap();
        std::fs::write(install.record(), serde_json::json!({ "pid": sleeper.id(), "port": port, "key": "k" }).to_string()).unwrap();
        install.reap();
        assert!(matches!(sleeper.try_wait(), Ok(None)), "a process that isn't Ferry's server was stopped");
        assert!(!install.record().exists());

        let ours = serve(br#"{"data":[{"id":"qwen3.5-9b"}]}"#);
        let port = ours.rsplit(':').next().unwrap().parse::<u16>().unwrap();
        std::fs::write(install.record(), serde_json::json!({ "pid": sleeper.id(), "port": port, "key": "k" }).to_string()).unwrap();
        install.reap();
        let stopped = (0..50).any(|_| {
            std::thread::sleep(Duration::from_millis(50));
            matches!(sleeper.try_wait(), Ok(Some(_)))
        });
        assert!(stopped);
    }
}
