use serde::Serialize;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub enum ToolError {
    PhpMissing,
    PhpTooOld(String),
    ComposerMissing,
    RectorMissing,
    Spawn { program: String, source: std::io::Error },
    Failed(String),
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            ToolError::PhpMissing => crate::t!(
                "PHP bulunamadı. PHP 7.4 ya da üstünü kurun veya Ayarlar'dan yolunu gösterin.",
                "PHP wasn't found. Install PHP 7.4 or newer, or point to it in Settings."
            ),
            ToolError::PhpTooOld(v) => crate::t!(
                "Bulunan PHP {v}; otomatik düzeltmeler için PHP 7.4 ya da üstü gerekiyor. Ayarlar'dan daha yeni bir PHP gösterin.",
                "Found PHP {v}; automatic fixes need PHP 7.4 or newer. Point to a newer PHP in Settings."
            ),
            ToolError::ComposerMissing => crate::t!(
                "Composer bulunamadı. getcomposer.org adresinden kurabilirsiniz.",
                "Composer wasn't found. You can install it from getcomposer.org."
            ),
            ToolError::RectorMissing => crate::t!("Rector kurulu değil.", "Rector isn't installed."),
            ToolError::Spawn { program, source } => {
                let source = crate::i18n::io_error(source);
                crate::t!("{program} çalıştırılamadı: {source}", "Couldn't run {program}: {source}")
            }
            ToolError::Failed(msg) => msg.clone(),
        };
        f.write_str(&msg)
    }
}

impl std::error::Error for ToolError {}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub path: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolchainStatus {
    pub php: Option<ToolInfo>,
    pub composer: Option<ToolInfo>,
    pub rector: Option<ToolInfo>,
    pub docker: Option<ToolInfo>,
    pub tools_dir: String,
    pub platform: &'static str,
}

pub fn platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

fn home() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}

// GUI apps launched from Finder or the Start menu don't inherit the shell PATH.
fn extra_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let h = home();
    if cfg!(target_os = "macos") {
        dirs.extend(["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin"].map(PathBuf::from));
        if let Some(h) = &h {
            dirs.push(h.join("Library/Application Support/Herd/bin"));
            dirs.push(h.join(".config/herd-lite/bin"));
            dirs.push(h.join(".composer/vendor/bin"));
        }
        dirs.extend(expand_glob(Path::new("/Applications/MAMP/bin/php"), "php", "bin"));
    } else if cfg!(windows) {
        dirs.extend(["C:\\php", "C:\\xampp\\php", "C:\\ProgramData\\ComposerSetup\\bin"].map(PathBuf::from));
        dirs.extend(expand_glob(Path::new("C:\\laragon\\bin\\php"), "php", ""));
        dirs.extend(expand_glob(Path::new("C:\\tools"), "php", ""));
        if let Some(pf) = std::env::var_os("ProgramFiles") {
            dirs.extend(expand_glob(&PathBuf::from(pf), "php", ""));
        }
        if let Some(h) = &h {
            dirs.push(h.join(".config\\herd\\bin"));
            dirs.push(h.join("scoop\\shims"));
            dirs.push(h.join("AppData\\Roaming\\Composer\\vendor\\bin"));
        }
    } else {
        dirs.extend(["/usr/local/bin", "/usr/bin"].map(PathBuf::from));
        if let Some(h) = &h {
            dirs.push(h.join(".config/composer/vendor/bin"));
        }
    }
    dirs
}

fn expand_glob(parent: &Path, prefix: &str, suffix: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().to_lowercase().starts_with(prefix))
        .map(|e| if suffix.is_empty() { e.path() } else { e.path().join(suffix) })
        .collect();
    out.sort();
    out.reverse();
    out
}

pub fn augmented_path() -> std::ffi::OsString {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    for d in extra_dirs() {
        if !dirs.contains(&d) {
            dirs.push(d);
        }
    }
    std::env::join_paths(dirs).unwrap_or_default()
}

pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut cmd = Command::new(program);
    cmd.env("PATH", augmented_path());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn executable_names(name: &str) -> Vec<String> {
    if cfg!(windows) {
        vec![format!("{name}.exe"), format!("{name}.bat"), format!("{name}.cmd"), name.to_string()]
    } else {
        vec![name.to_string()]
    }
}

pub fn find_executable(name: &str) -> Option<PathBuf> {
    let path = augmented_path();
    for dir in std::env::split_paths(&path) {
        for candidate in executable_names(name) {
            let p = dir.join(&candidate);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

pub fn find_php(override_path: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = override_path.filter(|p| p.is_file()) {
        return Some(p.to_path_buf());
    }
    find_executable("php")
}

pub fn require_supported_php(php: &Path) -> Result<(), ToolError> {
    let Some(version) = php_version(php) else { return Ok(()) };
    let mut parts = version.split(|c: char| !c.is_ascii_digit()).filter_map(|p| p.parse::<u32>().ok());
    match (parts.next(), parts.next()) {
        (Some(major), Some(minor)) if (major, minor) < (7, 4) => Err(ToolError::PhpTooOld(version)),
        _ => Ok(()),
    }
}

pub fn php_version(php: &Path) -> Option<String> {
    let out =
        command(php).args(["-d", "display_startup_errors=0", "-d", "display_errors=stderr", "-r", "echo PHP_VERSION;"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout).lines().map(str::trim).rfind(|l| !l.is_empty()).map(str::to_string)
}

pub fn lint_source(php: &Path, src: &str) -> Option<String> {
    use std::io::Write;
    let mut child = command(php)
        .args(["-n", "-d", "display_errors=1", "-l"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;
    let mut stdin = child.stdin.take()?;
    let input = src.to_string();
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let out = child.wait_with_output().ok()?;
    let _ = writer.join();
    if out.status.success() {
        return None;
    }
    let text = format!("{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| l.to_ascii_lowercase().contains("error") && !l.starts_with("Errors parsing"))
        .or_else(|| text.lines().map(str::trim).find(|l| !l.is_empty()))?;
    Some(line.trim_start_matches("PHP ").to_string())
}

pub fn new_syntax_errors<'a>(php: &Path, files: impl IntoIterator<Item = (&'a str, Option<&'a str>, &'a str)>) -> Vec<String> {
    let files: Vec<_> = files.into_iter().collect();
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, 8);
    let chunk = files.len().div_ceil(workers).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = files
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .filter_map(|&(rel, before, after)| {
                            let error = lint_source(php, after)?;
                            if before.is_some_and(|b| lint_source(php, b).is_some()) {
                                return None;
                            }
                            Some(format!("{rel} — {}", error.replace("Standard input code", rel)))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
    })
}

pub fn find_composer() -> Option<PathBuf> {
    let found = find_executable("composer.phar").or_else(|| find_executable("composer"))?;
    let is_script_wrapper =
        found.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("bat") || e.eq_ignore_ascii_case("cmd"));
    if is_script_wrapper {
        let phar = found.with_file_name("composer.phar");
        return phar.is_file().then_some(phar).or(Some(found));
    }
    Some(found)
}

pub fn php_tool_command(php: &Path, tool: &Path) -> Command {
    let first_line = std::fs::read(tool)
        .ok()
        .map(|b| {
            let end = b.iter().position(|&c| c == b'\n').unwrap_or(b.len()).min(200);
            String::from_utf8_lossy(&b[..end]).to_string()
        })
        .unwrap_or_default();
    let ext = tool.extension().and_then(|e| e.to_str()).unwrap_or("");
    let runs_with_php = matches!(ext, "phar" | "php") || first_line.contains("php") || ext.is_empty() && !first_line.starts_with("#!");
    if runs_with_php {
        let mut cmd = command(php);
        cmd.arg(tool);
        cmd
    } else {
        command(tool)
    }
}

pub fn rector_entry(tools_dir: &Path) -> PathBuf {
    tools_dir.join("vendor").join("rector").join("rector").join("bin").join("rector")
}

fn first_line(bytes: &[u8]) -> Option<String> {
    let s = String::from_utf8_lossy(bytes);
    s.lines().map(str::trim).find(|l| !l.is_empty()).map(str::to_string)
}

pub fn detect(tools_dir: &Path, php_override: Option<&Path>) -> ToolchainStatus {
    let php_path = find_php(php_override);
    let php = php_path.as_ref().map(|p| ToolInfo { path: p.display().to_string(), version: php_version(p) });
    let composer = find_composer().map(|c| {
        let version = php_path.as_ref().and_then(|php| {
            let out = php_tool_command(php, &c).args(["--version", "--no-ansi"]).output().ok()?;
            first_line(&out.stdout).and_then(|l| l.split_whitespace().nth(2).map(str::to_string))
        });
        ToolInfo { path: c.display().to_string(), version }
    });
    let entry = rector_entry(tools_dir);
    let rector = entry.is_file().then(|| {
        let version = php_path.as_ref().and_then(|php| {
            let out = command(php).arg(&entry).arg("--version").output().ok()?;
            first_line(&out.stdout).and_then(|l| l.split_whitespace().nth(1).map(str::to_string))
        });
        ToolInfo { path: entry.display().to_string(), version }
    });
    let docker = find_executable("docker").map(|d| {
        let version = command(&d).args(["--version"]).output().ok().and_then(|o| first_line(&o.stdout));
        ToolInfo { path: d.display().to_string(), version }
    });
    ToolchainStatus { php, composer, rector, docker, tools_dir: tools_dir.display().to_string(), platform: platform() }
}

const INSTALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10 * 60);

pub fn install_rector(tools_dir: &Path, php_override: Option<&Path>, cancel: &std::sync::atomic::AtomicBool) -> Result<(), ToolError> {
    let php = find_php(php_override).ok_or(ToolError::PhpMissing)?;
    require_supported_php(&php)?;
    let composer = find_composer().ok_or(ToolError::ComposerMissing)?;
    std::fs::create_dir_all(tools_dir).map_err(|e| ToolError::Failed(e.to_string()))?;
    let manifest = tools_dir.join("composer.json");
    let json = r#"{
    "name": "ferry/tools",
    "description": "Managed by Ferry.",
    "require": { "rector/rector": "^2.0" },
    "config": { "platform-check": false }
}
"#;
    std::fs::write(&manifest, json).map_err(|e| ToolError::Failed(e.to_string()))?;
    let mut cmd = php_tool_command(&php, &composer);
    cmd.args(["update", "--no-interaction", "--no-progress", "--no-ansi", "--working-dir"]).arg(tools_dir);
    let out = crate::rector::run_cancellable(cmd, Path::new("composer"), cancel, INSTALL_TIMEOUT).map_err(|e| match e {
        crate::rector::RectorError::Tool(t) => t,
        crate::rector::RectorError::TimedOut(minutes) => ToolError::Failed(crate::t!(
            "Composer {minutes} dakikada bitmediği için durduruldu. İnternet bağlantınızı kontrol edip yeniden deneyin.",
            "Composer was stopped because it didn't finish in {minutes} minutes. Check your internet connection and try again."
        )),
        crate::rector::RectorError::Cancelled => ToolError::Failed(crate::t!("Kurulum durduruldu.", "The installation was stopped.")),
        crate::rector::RectorError::Io(_, e) => ToolError::Failed(crate::i18n::io_error(&e)),
        other => ToolError::Failed(other.to_string()),
    })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let tail: Vec<&str> = stderr.lines().rev().take(12).collect();
        return Err(ToolError::Failed(format!(
            "{}\n{}",
            crate::t!("Composer Rector'ı kuramadı:", "Composer couldn't install Rector:"),
            tail.into_iter().rev().collect::<Vec<_>>().join("\n")
        )));
    }
    if !rector_entry(tools_dir).is_file() {
        return Err(ToolError::RectorMissing);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lint_reports_only_newly_broken_files() {
        let Some(php) = find_php(None) else {
            eprintln!("php not found; lint test skipped");
            return;
        };
        let ok = "<?php\necho 1;\n";
        let broken = "<?php\nfunction f( {\n";
        let errors = new_syntax_errors(
            &php,
            [("src/ok.php", Some(ok), ok), ("src/new.php", Some(ok), broken), ("src/old.php", Some(broken), broken)],
        );
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("src/new.php — "), "{errors:?}");
        assert!(errors[0].contains("in src/new.php on line 2"), "{errors:?}");
        let many: Vec<(String, &str)> = (0..20).map(|i| (format!("src/f{i:02}.php"), if i % 7 == 0 { broken } else { ok })).collect();
        let errors = new_syntax_errors(&php, many.iter().map(|(rel, after)| (rel.as_str(), Some(ok), *after)));
        let files: Vec<&str> = errors.iter().map(|e| e.split(" — ").next().unwrap()).collect();
        assert_eq!(files, ["src/f00.php", "src/f07.php", "src/f14.php"], "results keep the input order");
    }

    #[cfg(unix)]
    #[test]
    fn old_php_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let php = dir.path().join("php");
        std::fs::write(&php, "#!/bin/sh\necho 7.2.34\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&php, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(require_supported_php(&php), Err(ToolError::PhpTooOld(v)) if v == "7.2.34"));
        std::fs::write(&php, "#!/bin/sh\necho 8.1.0\n").unwrap();
        assert!(require_supported_php(&php).is_ok());
    }

    #[test]
    fn rector_entry_layout() {
        let p = rector_entry(Path::new("tools"));
        assert!(p.ends_with(Path::new("vendor/rector/rector/bin/rector")));
    }

    #[test]
    fn augmented_path_keeps_original_entries() {
        let joined = augmented_path();
        let parts: Vec<_> = std::env::split_paths(&joined).collect();
        if let Some(orig) = std::env::var_os("PATH") {
            for p in std::env::split_paths(&orig) {
                assert!(parts.contains(&p));
            }
        }
    }

    #[test]
    fn php_tool_command_runs_phars_through_php() {
        let dir = tempfile::tempdir().unwrap();
        let phar = dir.path().join("composer");
        std::fs::write(&phar, "#!/usr/bin/env php\n<?php").unwrap();
        let cmd = php_tool_command(Path::new("php"), &phar);
        assert_eq!(cmd.get_program(), "php");
        let script = dir.path().join("wrapper");
        std::fs::write(&script, "#!/bin/sh\nexec php x").unwrap();
        let cmd = php_tool_command(Path::new("php"), &script);
        assert_eq!(cmd.get_program(), script.as_os_str());
    }
}
