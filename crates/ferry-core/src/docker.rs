use crate::runtime::{self, Event};
use crate::toolchain::{self, Stop};
use crate::version::PhpVersion;
use serde::Serialize;
use std::cell::RefCell;
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
const BUILD_TIMEOUT: Duration = Duration::from_secs(20 * 60);
const LOG_LIMIT: usize = 64 * 1024;
const IMAGE_PREFIX: &str = "ferry-php:";

const INSTALLER_URL: &str = "https://github.com/mlocati/docker-php-extension-installer/releases/download/2.12.2/install-php-extensions";
const INSTALLER_SHA256: &str = "ed854df3ed4366fa498c5a784e9d2b4c601fa29f5c04c7f21658a427dc151e9a";

/// Extensions the installer builds without vendor downloads such as Oracle's client.
const INSTALLABLE: &[&str] = &[
    "amqp",
    "apcu",
    "bcmath",
    "bz2",
    "calendar",
    "dba",
    "enchant",
    "exif",
    "gd",
    "gettext",
    "gmp",
    "gnupg",
    "igbinary",
    "imagick",
    "imap",
    "intl",
    "ldap",
    "mcrypt",
    "memcached",
    "mongodb",
    "msgpack",
    "mysql",
    "mysqli",
    "odbc",
    "opcache",
    "pcntl",
    "pdo_mysql",
    "pdo_odbc",
    "pdo_pgsql",
    "pgsql",
    "pspell",
    "recode",
    "redis",
    "shmop",
    "soap",
    "sockets",
    "sysvmsg",
    "sysvsem",
    "sysvshm",
    "tidy",
    "wddx",
    "xmlrpc",
    "xsl",
    "zip",
];

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
    pub installed_extensions: Vec<String>,
    pub image_failed: bool,
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

#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUsage {
    pub images: usize,
    pub bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub install_extensions: bool,
    pub clean_images: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Installing,
    Extensions(PhpVersion),
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

fn built_images(docker: &Path) -> Result<Vec<String>, String> {
    let listed = toolchain::command(docker)
        .args(["image", "ls", "ferry-php", "--format", "{{.Repository}}:{{.Tag}}"])
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&listed.stdout).lines().filter(|l| l.starts_with(IMAGE_PREFIX)).map(str::to_string).collect())
}

/// Only the images Ferry built. The official `php` images stay, since they may be in use elsewhere.
pub fn image_usage() -> Result<ImageUsage, String> {
    let docker = available()?;
    let images = built_images(&docker)?;
    if images.is_empty() {
        return Ok(ImageUsage::default());
    }
    let inspected = toolchain::command(&docker)
        .args(["image", "inspect", "--format", "{{.Size}}"])
        .args(&images)
        .output()
        .map_err(|e| e.to_string())?;
    let bytes = String::from_utf8_lossy(&inspected.stdout).lines().filter_map(|l| l.trim().parse::<u64>().ok()).sum();
    Ok(ImageUsage { images: images.len(), bytes })
}

fn remove(docker: &Path, images: &[String]) -> Result<usize, String> {
    if images.is_empty() {
        return Ok(0);
    }
    let done = toolchain::command(docker).args(["image", "rm"]).args(images).output().map_err(|e| e.to_string())?;
    if done.status.success() {
        Ok(images.len())
    } else {
        let reason = String::from_utf8_lossy(&done.stderr).trim().to_string();
        Err(crate::t!("Kalıplar silinemedi: {reason}", "Couldn't remove the images: {reason}"))
    }
}

pub fn remove_images() -> Result<usize, String> {
    let docker = available()?;
    remove(&docker, &built_images(&docker)?)
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

fn dockerfile(base: &str, extensions: &[String]) -> String {
    format!(
        "FROM {base}\nADD --checksum=sha256:{INSTALLER_SHA256} {INSTALLER_URL} /usr/local/bin/install-php-extensions\n\
         RUN chmod +x /usr/local/bin/install-php-extensions; for e in {}; do install-php-extensions \"$e\" || true; done\n",
        extensions.join(" ")
    )
}

fn installable(missing: &[String]) -> Vec<String> {
    let mut list: Vec<String> = missing.iter().filter(|e| INSTALLABLE.contains(&e.as_str())).cloned().collect();
    list.sort();
    list.dedup();
    list
}

fn loaded(container: &Container, version: PhpVersion, project: &Path, image: &str) -> Result<HashSet<String>, String> {
    let done = container.run(&format!("modules-{}", version.compact()), project, image, false, "php -m", Duration::from_secs(300))?;
    if done.status.and_then(|s| s.code()) == Some(125) {
        let reason = tail(output(&done).trim());
        return Err(crate::t!("{image} kalıbı çalıştırılamadı: {reason}", "Couldn't run the {image} image: {reason}"));
    }
    Ok(output(&done).lines().map(|l| l.trim().to_ascii_lowercase()).collect())
}

/// An image with the extensions added, reused between runs. Project code never runs while it's built.
fn extended_image(
    container: &Container,
    base: &str,
    version: PhpVersion,
    extensions: &[String],
    dir: &Path,
) -> Result<Option<String>, String> {
    let tag = format!("{IMAGE_PREFIX}{version}-{:016x}", crate::snapshot::fnv1a(dockerfile(base, extensions).as_bytes()));
    let cached = toolchain::command(container.docker).args(["image", "inspect", &tag]).output();
    if cached.is_ok_and(|o| o.status.success()) {
        return Ok(Some(tag));
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("Dockerfile"), dockerfile(base, extensions)).map_err(|e| e.to_string())?;
    let mut cmd = toolchain::command(container.docker);
    cmd.args(["build", "--quiet", "-t", &tag]).arg(dir);
    let done = toolchain::run_process(cmd, container.docker, container.cancel, BUILD_TIMEOUT).map_err(|e| e.to_string())?;
    if done.stopped == Some(Stop::Cancelled) {
        return Err(crate::t!("Karşılaştırma durduruldu.", "The comparison was stopped."));
    }
    Ok(done.status.is_some_and(|s| s.success()).then_some(tag))
}

fn lint_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter(|l| l.contains(" on line ")).filter(|l| !l.contains("Deprecated: ") && !l.contains("Warning: "))
}

struct Setup<'a> {
    used: &'a [String],
    install_extensions: bool,
    images: &'a Path,
    built: RefCell<Vec<String>>,
}

fn run_version(
    container: &Container,
    version: PhpVersion,
    project: &Path,
    setup: &Setup,
    step: &dyn Fn(Step),
) -> Result<VersionRun, String> {
    let base = format!("php:{version}-cli");
    let used = setup.used;
    let mut image = base.clone();
    let mut modules = loaded(container, version, project, &image)?;
    let missing: Vec<String> = used.iter().filter(|e| !modules.contains(*e)).cloned().collect();
    let wanted = if setup.install_extensions { installable(&missing) } else { Vec::new() };
    let mut image_failed = false;
    if !wanted.is_empty() {
        step(Step::Extensions(version));
        match extended_image(container, &base, version, &wanted, &setup.images.join(version.compact()))? {
            Some(tag) => {
                setup.built.borrow_mut().push(tag.clone());
                modules = loaded(container, version, project, &tag)?;
                image = tag;
            }
            None => image_failed = true,
        }
    }
    let installed_extensions = wanted.iter().filter(|e| modules.contains(*e)).cloned().collect();
    step(Step::Lint(version));
    let lint = container.run(&format!("lint-{}", version.compact()), project, &image, false, LINT_SCRIPT, LINT_TIMEOUT)?;
    let lint_output = output(&lint);
    let lint_errors = lint_lines(&lint_output).count();
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
    Ok(VersionRun { version, image, lint_errors, tests, missing_extensions, installed_extensions, image_failed, events, log: tail(&log) })
}

fn differs(events: &[Event], other: &[Event]) -> Vec<Event> {
    let known: HashSet<(&str, usize, &str)> = other.iter().map(|e| (e.path.as_str(), e.line, e.message.as_str())).collect();
    events.iter().filter(|e| !known.contains(&(e.path.as_str(), e.line, e.message.as_str()))).cloned().collect()
}

/// Lints the project and runs its tests on both versions, each in a fresh copy with no network.
/// Only Composer and image builds use the network, and neither runs the project's code, scripts or plugins.
/// With `clean_images`, the images used are removed afterwards, also when the run fails or is stopped.
pub fn compare(
    root: &Path,
    source: PhpVersion,
    target: PhpVersion,
    options: Options,
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
    let images = work.path().join("images");
    let setup = Setup { used: &used, install_extensions: options.install_extensions, images: &images, built: RefCell::new(Vec::new()) };
    let runs = run_version(&container, source, &base, &setup, &step)
        .and_then(|first| run_version(&container, target, &target_dir, &setup, &step).map(|second| (first, second)));
    if options.clean_images {
        let _ = remove(&docker, &setup.built.borrow());
    }
    let (source_run, target_run) = runs?;
    let new_on_target = differs(&target_run.events, &source_run.events);
    let gone_on_target = differs(&source_run.events, &target_run.events);
    Ok(Comparison { source: source_run, target: target_run, dependencies_installed, new_on_target, gone_on_target })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Level;

    const PLAIN: Options = Options { install_extensions: false, clean_images: false };

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
    fn images_add_only_extensions_the_installer_can_build() {
        let missing = ["zip", "oci8", "gd", "zip", "interbase"].map(String::from);
        assert_eq!(installable(&missing), ["gd", "zip"]);
        let file = dockerfile("php:8.4-cli", &installable(&missing));
        assert!(file.starts_with("FROM php:8.4-cli\nADD --checksum=sha256:ed854df3"), "{file}");
        assert!(file.contains("for e in gd zip;"), "{file}");
    }

    #[test]
    fn extensions_are_built_into_the_image() {
        if std::env::var_os("FERRY_TEST_DOCKER").is_none() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.php"), "<?php\n$b = new mysqli('h');\necho bcadd('1', '2');\n").unwrap();
        let options = Options { install_extensions: true, clean_images: true };
        let result = compare(dir.path(), PhpVersion::new(7, 4), PhpVersion::new(8, 4), options, &AtomicBool::new(false), |_| {}).unwrap();
        let docker = available().unwrap();
        let left = built_images(&docker).unwrap();
        for run in [&result.source, &result.target] {
            assert_eq!(run.installed_extensions, ["bcmath", "mysqli"], "{run:?}");
            assert!(run.missing_extensions.is_empty() && run.image.starts_with(IMAGE_PREFIX), "{run:?}");
            assert!(!left.contains(&run.image), "{left:?}");
        }
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
        let result = compare(dir.path(), PhpVersion::new(7, 4), PhpVersion::new(8, 4), PLAIN, &AtomicBool::new(false), |_| {}).unwrap();
        assert_eq!((result.source.lint_errors, result.target.lint_errors), (0, 1), "{}", result.target.log);
        assert!(result.target.events.iter().all(|e| e.path == "/app/a.php"), "{:?}", result.target.events);
        assert!(result.new_on_target.iter().any(|e| e.level == Level::Fatal), "{:?}", result.new_on_target);
    }
}
