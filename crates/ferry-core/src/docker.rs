use crate::runtime::{self, Event};
use crate::toolchain::{self, Stop};
use crate::version::PhpVersion;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use walkdir::WalkDir;

/// Where the project sits inside the containers, and so in their log paths.
pub const CONTAINER_ROOT: &str = "/app";

const INSTALL_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const LINT_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const TEST_TIMEOUT: Duration = Duration::from_secs(20 * 60);
const LOG_LIMIT: usize = 64 * 1024;

const LINT_SCRIPT: &str = "find /app -type f -name '*.php' -not -path '/app/vendor/*' -print0 \
    | xargs -0 -P 4 -n 1 php -n -d error_reporting=-1 -d display_errors=stderr -d log_errors=0 -l 2>&1 | grep -v '^No syntax errors'";
const TEST_SCRIPT: &str = "flags='--display-deprecations --display-warnings --no-progress'; \
    php vendor/bin/phpunit --version | grep -qE 'PHPUnit [0-9]\\.' && flags=''; \
    exec php -d error_reporting=-1 -d display_errors=1 -d memory_limit=1G vendor/bin/phpunit $flags";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestRun {
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionRun {
    pub version: PhpVersion,
    pub image: String,
    pub lint_errors: usize,
    pub tests: Option<TestRun>,
    pub missing_extensions: Vec<String>,
    pub events: Vec<Event>,
    pub log: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub source: VersionRun,
    pub target: VersionRun,
    pub dependencies_installed: bool,
    pub new_on_target: Vec<Event>,
    pub gone_on_target: Vec<Event>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Installing,
    Lint(PhpVersion),
    Tests(PhpVersion),
}

pub fn available() -> Result<PathBuf, String> {
    let docker = toolchain::find_executable("docker").ok_or_else(|| crate::t!("Docker bulunamadı.", "Docker wasn't found."))?;
    let running = toolchain::command(&docker).args(["version", "--format", "{{.Server.Version}}"]).output();
    if running.is_ok_and(|o| o.status.success()) {
        Ok(docker)
    } else {
        Err(crate::t!("Docker çalışmıyor; önce Docker'ı başlatın.", "Docker isn't running. Start it first."))
    }
}

/// Copies the project without following links: inside a container a link can only reach what was copied.
fn copy_project(root: &Path, dest: &Path) -> std::io::Result<()> {
    let skipped = |name: &str| name != "vendor" && crate::project::ALWAYS_SKIP.contains(&name);
    let entries =
        WalkDir::new(root).follow_links(false).into_iter().filter_entry(|e| e.depth() == 0 || !skipped(&e.file_name().to_string_lossy()));
    for entry in entries.flatten() {
        let Ok(rel) = entry.path().strip_prefix(root) else { continue };
        let to = dest.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&to)?;
        } else if entry.file_type().is_file() {
            std::fs::copy(entry.path(), &to)?;
        } else {
            #[cfg(unix)]
            if let Ok(link) = std::fs::read_link(entry.path()) {
                std::os::unix::fs::symlink(link, &to)?;
            }
        }
    }
    Ok(())
}

fn owner_flag(path: &Path) -> Vec<String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let Ok(meta) = std::fs::metadata(path) {
            return vec!["--user".into(), format!("{}:{}", meta.uid(), meta.gid())];
        }
    }
    let _ = path;
    Vec::new()
}

struct Container<'a> {
    docker: &'a Path,
    cancel: &'a AtomicBool,
    prefix: String,
}

impl Container<'_> {
    fn run(
        &self,
        name: &str,
        dir: &Path,
        image: &str,
        network: bool,
        script: &str,
        timeout: Duration,
    ) -> Result<toolchain::Finished, String> {
        let name = format!("{}-{name}", self.prefix);
        let mut cmd = toolchain::command(self.docker);
        cmd.args(["run", "--rm", "--name", &name, "--cap-drop", "ALL", "--security-opt", "no-new-privileges"])
            .args(["--pids-limit", "512", "--memory", "2g", "-e", "HOME=/tmp", "-w", CONTAINER_ROOT])
            .args(owner_flag(dir))
            .arg("-v")
            .arg(format!("{}:{CONTAINER_ROOT}", dir.display()));
        if !network {
            cmd.args(["--network", "none"]);
        }
        cmd.args([image, "sh", "-c", script]);
        let done = toolchain::run_process(cmd, self.docker, self.cancel, timeout).map_err(|e| e.to_string())?;
        if done.stopped.is_some() {
            let _ = toolchain::command(self.docker).args(["rm", "-f", &name]).output();
        }
        if done.stopped == Some(Stop::Cancelled) {
            return Err(crate::t!("Karşılaştırma durduruldu.", "The comparison was stopped."));
        }
        Ok(done)
    }
}

fn output(done: &toolchain::Finished) -> String {
    format!("{}{}", String::from_utf8_lossy(&done.stdout), String::from_utf8_lossy(&done.stderr))
}

fn tail(text: &str) -> String {
    let mut start = text.len().saturating_sub(LOG_LIMIT);
    while !text.is_char_boundary(start) {
        start += 1;
    }
    text[start..].to_string()
}

fn lint_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter(|l| l.contains(" on line ")).filter(|l| !l.contains("Deprecated: ") && !l.contains("Warning: "))
}

fn run_version(
    container: &Container,
    version: PhpVersion,
    project: &Path,
    used: &[String],
    step: &dyn Fn(Step),
) -> Result<VersionRun, String> {
    let image = format!("php:{version}-cli");
    step(Step::Lint(version));
    let lint = container.run(&format!("lint-{}", version.compact()), project, &image, false, LINT_SCRIPT, LINT_TIMEOUT)?;
    let lint_output = output(&lint);
    if lint.status.and_then(|s| s.code()) == Some(125) {
        let reason = tail(lint_output.trim());
        return Err(crate::t!("{image} kalıbı çalıştırılamadı: {reason}", "Couldn't run the {image} image: {reason}"));
    }
    let lint_errors = lint_lines(&lint_output).count();
    let modules = container.run(&format!("modules-{}", version.compact()), project, &image, false, "php -m", Duration::from_secs(60))?;
    let modules: HashSet<String> = output(&modules).lines().map(|l| l.trim().to_ascii_lowercase()).collect();
    let missing_extensions = used.iter().filter(|e| !modules.contains(*e)).cloned().collect();
    let mut log = lint_output;
    let has_tests = project.join("vendor/bin/phpunit").exists();
    let tests = if has_tests {
        step(Step::Tests(version));
        let run = container.run(&format!("tests-{}", version.compact()), project, &image, false, TEST_SCRIPT, TEST_TIMEOUT)?;
        log.push_str(&output(&run));
        let exit_code = run.status.and_then(|s| s.code());
        Some(TestRun { exit_code, timed_out: run.stopped == Some(Stop::TimedOut), passed: exit_code == Some(0) && run.stopped.is_none() })
    } else {
        None
    };
    let events = runtime::parse(&log);
    Ok(VersionRun { version, image, lint_errors, tests, missing_extensions, events, log: tail(&log) })
}

fn differs(events: &[Event], other: &[Event]) -> Vec<Event> {
    let known: HashSet<(&str, usize, &str)> = other.iter().map(|e| (e.path.as_str(), e.line, e.message.as_str())).collect();
    events.iter().filter(|e| !known.contains(&(e.path.as_str(), e.line, e.message.as_str()))).cloned().collect()
}

/// Lints the project and runs its tests on both versions, each in a fresh copy with no network.
/// Only Composer runs with network access, and without the project's scripts or plugins.
pub fn compare(
    root: &Path,
    source: PhpVersion,
    target: PhpVersion,
    cancel: &AtomicBool,
    step: impl Fn(Step),
) -> Result<Comparison, String> {
    let docker = available()?;
    let work = tempfile::Builder::new().prefix("ferry-docker-").tempdir().map_err(|e| e.to_string())?;
    let copy = |name: &str| {
        let dest = work.path().join(name);
        copy_project(root, &dest).map_err(|e| crate::t!("Proje kopyalanamadı: {e}", "Couldn't copy the project: {e}"))?;
        Ok::<PathBuf, String>(dest)
    };
    let base = copy("base")?;
    let container = Container { docker: &docker, cancel, prefix: format!("ferry-{}", std::process::id()) };
    let needs_install = base.join("composer.json").is_file() && !base.join("vendor/autoload.php").is_file();
    let mut dependencies_installed = !needs_install;
    if needs_install {
        step(Step::Installing);
        let install = "composer install --no-interaction --no-progress --no-scripts --no-plugins";
        let script = if base.join("composer.lock").is_file() {
            format!("{install} --ignore-platform-reqs")
        } else {
            format!("composer config platform.php {source}.0 && {install} '--ignore-platform-req=ext-*'")
        };
        let done = container.run("install", &base, "composer:2", true, &script, INSTALL_TIMEOUT)?;
        dependencies_installed = done.status.is_some_and(|s| s.success());
    }
    let files = crate::project::collect_php_files(root, &[]);
    let used: Vec<String> = crate::extensions::used(root, &files).into_iter().map(|e| e.name.to_ascii_lowercase()).collect();
    let target_dir = work.path().join("target");
    copy_project(&base, &target_dir).map_err(|e| crate::t!("Proje kopyalanamadı: {e}", "Couldn't copy the project: {e}"))?;
    let source_run = run_version(&container, source, &base, &used, &step)?;
    let target_run = run_version(&container, target, &target_dir, &used, &step)?;
    let new_on_target = differs(&target_run.events, &source_run.events);
    let gone_on_target = differs(&source_run.events, &target_run.events);
    Ok(Comparison { source: source_run, target: target_run, dependencies_installed, new_on_target, gone_on_target })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Level;

    fn event(path: &str, line: usize, message: &str) -> Event {
        Event { path: path.into(), line, level: Level::Warning, message: message.into(), count: 1 }
    }

    #[test]
    fn differences_ignore_how_often_a_message_appeared() {
        let old = [event("/app/a.php", 1, "same"), event("/app/a.php", 2, "gone")];
        let new = [Event { count: 5, ..event("/app/a.php", 1, "same") }, event("/app/a.php", 3, "new")];
        assert_eq!(differs(&new, &old), [event("/app/a.php", 3, "new")]);
        assert_eq!(differs(&old, &new), [event("/app/a.php", 2, "gone")]);
    }

    #[test]
    fn the_copy_keeps_vendor_but_not_version_control() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("p");
        for f in ["a.php", "vendor/x/b.php", ".git/config", "node_modules/m.js"] {
            let path = root.join(f);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "x").unwrap();
        }
        let dest = dir.path().join("copy");
        copy_project(&root, &dest).unwrap();
        assert!(dest.join("a.php").is_file() && dest.join("vendor/x/b.php").is_file());
        assert!(!dest.join(".git").exists() && !dest.join("node_modules").exists());
    }

    #[test]
    fn lint_output_counts_only_errors() {
        let out = "PHP Parse error:  syntax error in /app/a.php on line 2\nPHP Deprecated:  old in /app/b.php on line 1\nErrors parsing /app/a.php\n";
        assert_eq!(lint_lines(out).count(), 1);
    }

    #[test]
    fn compares_two_versions_in_docker() {
        if std::env::var_os("FERRY_TEST_DOCKER").is_none() {
            eprintln!("FERRY_TEST_DOCKER ayarlı değil; Docker testi atlandı");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.php"), "<?php\nfunction f(int $a = null) {}\n$x = (real) 1;\n").unwrap();
        std::fs::write(dir.path().join("b.php"), "<?php\necho 1;\n").unwrap();
        let result = compare(dir.path(), PhpVersion::new(7, 4), PhpVersion::new(8, 4), &AtomicBool::new(false), |_| {}).unwrap();
        assert_eq!((result.source.lint_errors, result.target.lint_errors), (0, 1), "{}", result.target.log);
        assert!(result.target.events.iter().all(|e| e.path == "/app/a.php"), "{:?}", result.target.events);
        assert!(result.new_on_target.iter().any(|e| e.level == Level::Fatal), "{:?}", result.new_on_target);
    }
}
