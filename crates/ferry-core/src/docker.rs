use crate::runtime::{self, Event};
use crate::toolchain::{self, Stop};
use crate::version::PhpVersion;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use walkdir::WalkDir;

/// Where the project sits inside the containers, and so in their log paths.
pub const CONTAINER_ROOT: &str = "/app";

const INSTALL_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const LINT_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const TEST_TIMEOUT: Duration = Duration::from_secs(20 * 60);
const PAGE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
/// Symfony's entry points need the file this one plugin generates, so it alone may run.
const SYMFONY_RUNTIME_ONLY: &str = r#"'$j = json_decode(file_get_contents("composer.json"), true); $j["config"]["allow-plugins"] = ["symfony/runtime" => true, "*" => false]; file_put_contents("composer.json", json_encode($j));'"#;
/// Keeps container names apart when two projects run at once.
static RUNS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
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
/// For suites that stop the whole process, such as tests that end it on purpose.
const TEST_BY_FILE_SCRIPT: &str = "flags='--display-deprecations --display-warnings --no-progress'; \
    php vendor/bin/phpunit --version | grep -qE 'PHPUnit [0-9]\\.' && flags=''; \
    bad=0; for f in $(find . -name '*Test.php' -not -path './vendor/*' | sort); do \
    timeout 120 php -d error_reporting=-1 -d display_errors=1 -d memory_limit=1G vendor/bin/phpunit $flags \"$f\" 2>&1 || bad=1; done; exit $bad";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestRun {
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub passed: bool,
    pub by_file: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionRun {
    pub version: PhpVersion,
    pub image: String,
    pub lint_errors: usize,
    pub tests: Option<TestRun>,
    pub failed_tests: Vec<String>,
    #[serde(skip)]
    pub crawl: crate::web::Crawl,
    #[serde(skip)]
    pub data: crate::database::Snapshot,
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
    pub install_log: String,
    pub new_failures: Vec<String>,
    pub pages: Vec<crate::web::PageDiff>,
    pub data_changes: Vec<crate::database::DataChange>,
    pub database_used: bool,
    pub new_on_target: Vec<Event>,
    pub gone_on_target: Vec<Event>,
    pub origins: Vec<crate::origin::Origin>,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUsage {
    pub images: usize,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    pub install_extensions: bool,
    pub clean_images: bool,
    /// Pages to start crawling from. Empty skips the page comparison.
    pub pages: Vec<String>,
    pub database: Option<crate::database::Source>,
    /// Runs once on the source version with network access, before anything is compared.
    pub prepare: Option<String>,
    /// Replaces the PHPUnit run. Only its exit code decides whether the tests passed.
    pub test: Option<String>,
    /// Files replaced only in the target's copy, as (relative path, content), so the old code
    /// on the old version is compared with the fixed code on the new one.
    pub fixes: Vec<(String, String)>,
}

const HISTORY_LIMIT: usize = 20;

/// What a past comparison found, kept after its full result is gone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub at: u64,
    pub source: String,
    pub target: String,
    pub regressed: bool,
    pub tests_compared: bool,
    pub pages: usize,
    pub broken_pages: usize,
    pub differing_pages: usize,
    pub unverified_pages: usize,
    pub new_failures: usize,
    pub new_messages: usize,
    pub data_changes: usize,
}

impl Comparison {
    pub fn regressed(&self) -> bool {
        let fatal = self.new_on_target.iter().any(|e| e.level == runtime::Level::Fatal);
        let tests = self.source.tests.as_ref().is_some_and(|t| t.passed) && self.target.tests.as_ref().is_some_and(|t| !t.passed);
        fatal
            || tests
            || !self.new_failures.is_empty()
            || self.pages.iter().any(crate::web::PageDiff::broke)
            || self.target.lint_errors > self.source.lint_errors
    }

    pub fn summary(&self, at: u64) -> Summary {
        use crate::web::PageChange;
        let count = |wanted: &[PageChange]| self.pages.iter().filter(|p| wanted.contains(&p.change)).count();
        Summary {
            at,
            source: self.source.version.to_string(),
            target: self.target.version.to_string(),
            regressed: self.regressed(),
            tests_compared: self.source.tests.as_ref().is_some_and(|t| t.passed) && self.target.tests.is_some(),
            pages: self.pages.len(),
            broken_pages: self.pages.iter().filter(|p| p.broke()).count(),
            differing_pages: count(&[PageChange::Status, PageChange::Redirect, PageChange::Errors, PageChange::Body, PageChange::Missing]),
            unverified_pages: count(&[PageChange::Unverified, PageChange::Unstable]),
            new_failures: self.new_failures.len(),
            new_messages: self.new_on_target.len(),
            data_changes: self.data_changes.len(),
        }
    }
}

/// How one kind of problem moved between a comparison and its rerun with fixes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Delta<T> {
    pub fixed: Vec<T>,
    pub remaining: Vec<T>,
    pub introduced: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub messages: Delta<Event>,
    pub tests: Delta<String>,
    pub pages: Delta<String>,
    pub tables: Delta<String>,
    pub lint_before: usize,
    pub lint_after: usize,
    pub regressed_before: bool,
    pub regressed_after: bool,
    pub origins: Vec<crate::origin::Origin>,
}

impl Outcome {
    pub fn worsened(&self) -> bool {
        self.lint_after > self.lint_before
            || !self.messages.introduced.is_empty()
            || !self.tests.introduced.is_empty()
            || !self.pages.introduced.is_empty()
            || !self.tables.introduced.is_empty()
    }
}

fn delta<T: Clone, K: Eq + std::hash::Hash>(before: &[T], after: &[T], key: impl Fn(&T) -> K) -> Delta<T> {
    let counts = |items: &[T]| {
        let mut map: HashMap<K, usize> = HashMap::new();
        items.iter().for_each(|item| *map.entry(key(item)).or_default() += 1);
        map
    };
    let take = |map: &mut HashMap<K, usize>, item: &T| map.get_mut(&key(item)).filter(|n| **n > 0).map(|n| *n -= 1).is_some();
    let mut unseen = counts(after);
    let fixed = before.iter().filter(|item| !take(&mut unseen, item)).cloned().collect();
    let mut unseen = counts(before);
    let (remaining, introduced) = after.iter().cloned().partition(|item| take(&mut unseen, item));
    Delta { fixed, remaining, introduced }
}

/// Lines shift when code is fixed, so messages are matched without them or any number inside.
fn event_key(event: &Event) -> (String, runtime::Level, String) {
    let message = event.message.chars().filter(|c| !c.is_ascii_digit()).collect();
    (event.path.clone(), event.level, message)
}

/// What a rerun with fixes changed. A test or page the rerun couldn't check stays open.
pub fn outcome(before: &Comparison, after: &Comparison) -> Outcome {
    let differing = |c: &Comparison| -> Vec<String> {
        c.pages.iter().filter(|p| p.change != crate::web::PageChange::Same).map(|p| p.path.clone()).collect()
    };
    let (pages_before, mut pages_after) = (differing(before), differing(after));
    let unchecked = pages_before.iter().filter(|path| !after.pages.iter().any(|p| &p.path == *path)).cloned();
    pages_after.extend(unchecked.collect::<Vec<_>>());
    let tests_checked = after.target.tests.as_ref().is_some_and(|t| !t.timed_out && (t.passed || !after.target.failed_tests.is_empty()));
    let tests = if tests_checked { after.new_failures.as_slice() } else { before.new_failures.as_slice() };
    let tables = |c: &Comparison| -> Vec<String> { c.data_changes.iter().map(|d| d.table.clone()).collect() };
    Outcome {
        messages: delta(&before.new_on_target, &after.new_on_target, event_key),
        tests: delta(&before.new_failures, tests, Clone::clone),
        pages: delta(&pages_before, &pages_after, Clone::clone),
        tables: delta(&tables(before), &tables(after), Clone::clone),
        lint_before: before.target.lint_errors,
        lint_after: after.target.lint_errors,
        regressed_before: before.regressed(),
        regressed_after: after.regressed(),
        origins: after.origins.clone(),
    }
}

pub fn history(file: &Path) -> Vec<Summary> {
    std::fs::read(file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

pub fn record(file: &Path, summary: Summary) -> std::io::Result<Vec<Summary>> {
    let mut entries = history(file);
    entries.insert(0, summary);
    entries.truncate(HISTORY_LIMIT);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file, serde_json::to_vec(&entries)?)?;
    Ok(entries)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Installing,
    Extensions(PhpVersion),
    Lint(PhpVersion),
    Database,
    Prepare,
    Pages(PhpVersion),
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

type SharedDatabase = (Option<String>, Option<String>, Vec<(String, String)>);

pub(crate) struct Container<'a> {
    pub docker: &'a Path,
    pub cancel: &'a AtomicBool,
    pub prefix: String,
    shared: RefCell<Option<SharedDatabase>>,
}

impl Container<'_> {
    /// Later runs share the network of `database` and receive its settings.
    pub fn attach(&self, database: Option<&str>, socket: Option<String>, env: Vec<(String, String)>) {
        *self.shared.borrow_mut() = Some((database.map(str::to_string), socket, env));
    }

    pub fn detach(&self) {
        *self.shared.borrow_mut() = None;
    }

    pub fn run(
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
        match &*self.shared.borrow() {
            Some((database, socket, env)) if !network => {
                if let Some(socket) = socket {
                    cmd.arg("-v").arg(socket);
                }
                match database {
                    Some(database) => cmd.arg("--network").arg(format!("container:{database}")),
                    None => cmd.args(["--network", "none"]),
                };
                for (key, value) in env {
                    cmd.arg("-e").arg(key).env(key, value);
                }
            }
            _ if !network => {
                cmd.args(["--network", "none"]);
            }
            _ => {}
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

pub(crate) fn output(done: &toolchain::Finished) -> String {
    format!("{}{}", String::from_utf8_lossy(&done.stdout), String::from_utf8_lossy(&done.stderr))
}

pub(crate) fn tail(text: &str) -> String {
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
    pages: &'a [String],
    test: Option<&'a str>,
    drivers: &'a [String],
    images: &'a Path,
    database: Option<&'a crate::database::Prepared>,
    built: RefCell<Vec<String>>,
}

struct PreparedImage {
    image: String,
    modules: HashSet<String>,
    wanted: Vec<String>,
    image_failed: bool,
}

fn prepare_image(
    container: &Container,
    version: PhpVersion,
    project: &Path,
    setup: &Setup,
    step: &dyn Fn(Step),
) -> Result<PreparedImage, String> {
    let base = format!("php:{version}-cli");
    let mut image = base.clone();
    let mut modules = loaded(container, version, project, &image)?;
    let missing: Vec<String> = setup.used.iter().filter(|e| !modules.contains(*e)).cloned().collect();
    let needed: Vec<String> = missing.iter().filter(|e| setup.install_extensions || setup.drivers.contains(e)).cloned().collect();
    let wanted = installable(&needed);
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
    Ok(PreparedImage { image, modules, wanted, image_failed })
}

fn run_version(
    container: &Container,
    version: PhpVersion,
    project: &Path,
    setup: &Setup,
    step: &dyn Fn(Step),
) -> Result<VersionRun, String> {
    let used = setup.used;
    let PreparedImage { image, modules, wanted, image_failed } = prepare_image(container, version, project, setup, step)?;
    let installed_extensions = wanted.iter().filter(|e| modules.contains(*e)).cloned().collect();
    step(Step::Lint(version));
    let lint = container.run(&format!("lint-{}", version.compact()), project, &image, false, LINT_SCRIPT, LINT_TIMEOUT)?;
    let lint_output = output(&lint);
    let lint_errors = lint_lines(&lint_output).count();
    let missing_extensions = used.iter().filter(|e| !modules.contains(*e)).cloned().collect();
    let mut log = lint_output;
    let sqlite = setup.database.filter(|p| p.connection.engine == crate::database::Engine::Sqlite);
    let database = match setup.database {
        Some(prepared) if sqlite.is_some() => {
            container.attach(None, None, crate::database::environment(&prepared.connection));
            None
        }
        Some(prepared) => {
            step(Step::Database);
            let running = crate::database::start(container, &version.compact(), &prepared.connection, prepared.dump.as_deref())?;
            let socket = crate::database::socket_mount(&running, &prepared.connection);
            container.attach(Some(&running.name), socket, crate::database::environment(&prepared.connection));
            Some(running)
        }
        None => None,
    };
    let mut data = crate::database::Snapshot::new();
    let mut crawl = crate::web::Crawl::default();
    if !setup.pages.is_empty() {
        step(Step::Pages(version));
        let script = crate::web::script(project, setup.pages);
        let run = container.run(&format!("pages-{}", version.compact()), project, &image, false, &script, PAGE_TIMEOUT)?;
        let text = output(&run);
        crawl = crate::web::parse(&text);
        log.push_str(crate::web::server_log(&text));
    }
    if let Some(running) = &database {
        data = running.snapshot();
    }
    if let Some(prepared) = sqlite {
        data = crate::database::sqlite_snapshot(container, version, project, &image, &prepared.connection);
    }
    let has_tests = project.join("vendor/bin/phpunit").exists() && project.join("vendor/autoload.php").is_file();
    let mut failed = Vec::new();
    let tests = if let Some(command) = setup.test {
        step(Step::Tests(version));
        let run = container.run(&format!("tests-{}", version.compact()), project, &image, false, command, TEST_TIMEOUT)?;
        log.push_str(&output(&run));
        failed = failed_tests(&output(&run));
        let exit_code = run.status.and_then(|s| s.code());
        Some(TestRun {
            exit_code,
            timed_out: run.stopped == Some(Stop::TimedOut),
            passed: run.stopped.is_none() && exit_code == Some(0),
            by_file: false,
        })
    } else if has_tests {
        step(Step::Tests(version));
        let run = container.run(&format!("tests-{}", version.compact()), project, &image, false, TEST_SCRIPT, TEST_TIMEOUT)?;
        let mut run = run;
        let crashed = run.stopped.is_none() && !matches!(run.status.and_then(|s| s.code()), Some(0..=2));
        if crashed {
            run =
                container.run(&format!("tests-files-{}", version.compact()), project, &image, false, TEST_BY_FILE_SCRIPT, TEST_TIMEOUT)?;
        }
        log.push_str(&output(&run));
        failed = failed_tests(&output(&run));
        let exit_code = run.status.and_then(|s| s.code());
        Some(TestRun {
            exit_code,
            timed_out: run.stopped == Some(Stop::TimedOut),
            passed: run.stopped.is_none() && (exit_code == Some(0) || failed.is_empty() && !crashed && tests_passed(&output(&run))),
            by_file: crashed,
        })
    } else {
        None
    };
    container.detach();
    drop(database);
    let events = runtime::parse(&log);
    Ok(VersionRun {
        version,
        image,
        lint_errors,
        tests,
        failed_tests: failed,
        crawl,
        data,
        missing_extensions,
        installed_extensions,
        image_failed,
        events,
        log: tail(&log),
    })
}

/// Symfony's bridge exits with 1 over deprecations even though every test passed.
fn tests_passed(output: &str) -> bool {
    output.lines().any(|l| l.starts_with("OK (") || l.starts_with("OK, but"))
}

/// The tests PHPUnit lists under its errors and failures, leaving out skipped and risky ones.
fn failed_tests(output: &str) -> Vec<String> {
    let mut failing = false;
    let mut found = Vec::new();
    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("There w") {
            failing = rest.contains("error") || rest.contains("failure");
        } else if line.contains(" triggered ") {
            failing = false;
        } else if let Some((number, name)) = line.split_once(") ") {
            let test = name.contains("::") && !name.starts_with('/');
            if failing && test && !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()) && !found.iter().any(|f| f == name) {
                found.push(name.to_string());
            }
        }
    }
    found
}

/// Symfony's bridge downloads PHPUnit on first use, which the offline test run can't do.
fn phpunit_bridge_version(project: &Path) -> Option<String> {
    let manifest = std::fs::read_to_string(project.join("composer.json")).ok()?;
    if !manifest.contains("symfony/phpunit-bridge") || manifest.contains("\"phpunit/phpunit\"") {
        return None;
    }
    let config = std::fs::read_to_string(project.join("phpunit.xml.dist"))
        .or_else(|_| std::fs::read_to_string(project.join("phpunit.xml")))
        .unwrap_or_default();
    let version = config.split("SYMFONY_PHPUNIT_VERSION\" value=\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("9.6");
    let safe = version.chars().all(|c| c.is_ascii_digit() || c == '.');
    let version = if safe { version } else { "9.6" };
    Some(format!("composer require --dev 'phpunit/phpunit:^{version}' --no-interaction --no-progress --no-scripts --no-plugins --no-security-blocking --ignore-platform-reqs"))
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
    let container = Container {
        docker: &docker,
        cancel,
        prefix: format!("ferry-{}-{}", std::process::id(), RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)),
        shared: RefCell::new(None),
    };
    let needs_install = base.join("composer.json").is_file() && !base.join("vendor/autoload.php").is_file();
    let mut dependencies_installed = !needs_install;
    let mut install_log = String::new();
    if needs_install {
        step(Step::Installing);
        let runtime = std::fs::read_to_string(base.join("composer.json")).is_ok_and(|t| t.contains("symfony/runtime"));
        let install = if runtime {
            format!("php -r {SYMFONY_RUNTIME_ONLY} && composer install --no-interaction --no-progress --no-scripts --no-security-blocking")
        } else {
            "composer install --no-interaction --no-progress --no-scripts --no-plugins --no-security-blocking".to_string()
        };
        let script = if base.join("composer.lock").is_file() {
            format!("{install} --ignore-platform-reqs")
        } else {
            let advisories = "composer remove --no-update --dev roave/security-advisories 2>/dev/null";
            format!("{advisories}; composer config platform.php {source}.0 && {install} '--ignore-platform-req=ext-*'")
        };
        let bridge = phpunit_bridge_version(&base).map_or(String::new(), |v| format!(" && {v}"));
        let script =
            format!("mkdir /tmp/build && cp -a /app/. /tmp/build && cd /tmp/build && {{ {script}{bridge}; }} && cp -a vendor /app/");
        let done = container.run("install", &base, "composer:2", true, &script, INSTALL_TIMEOUT)?;
        dependencies_installed = done.status.is_some_and(|s| s.success());
        if !dependencies_installed {
            install_log = tail(&output(&done));
        }
    }
    let files = crate::project::collect_php_files(root, &[]);
    let mut used: Vec<String> = crate::extensions::used(root, &files).into_iter().map(|e| e.name.to_ascii_lowercase()).collect();
    let drivers: Vec<String> =
        options.database.iter().flat_map(|c| crate::database::engine_for(root, c).extensions().map(String::from)).collect();
    used.extend(drivers.iter().cloned());
    used.sort();
    used.dedup();
    let images = work.path().join("images");
    let mut setup = Setup {
        used: &used,
        install_extensions: options.install_extensions,
        pages: &options.pages,
        test: options.test.as_deref(),
        drivers: &drivers,
        images: &images,
        database: None,
        built: RefCell::new(Vec::new()),
    };
    if options.database == Some(crate::database::Source::Seed) && !dependencies_installed {
        let reason = install_log.trim();
        return Err(crate::t!(
            "Test verisi için bağımlılıklar kurulamadı: {reason}",
            "Dependencies for the test data couldn't be installed: {reason}"
        ));
    }
    let prepared = match &options.database {
        Some(choice) => {
            let seed_image = || prepare_image(&container, source, &base, &setup, &step).map(|p| p.image);
            Some(crate::database::prepare(&container, root, &base, choice, &work.path().join("data"), &seed_image, source, &step)?)
        }
        None => None,
    };
    setup.database = prepared.as_ref();
    if let Some(command) = &options.prepare {
        step(Step::Prepare);
        let image = prepare_image(&container, source, &base, &setup, &step)?.image;
        let run = container.run("prepare", &base, &image, true, command, INSTALL_TIMEOUT)?;
        if !run.status.is_some_and(|s| s.success()) {
            let reason = tail(&output(&run));
            return Err(crate::t!("Hazırlık komutu başarısız oldu: {reason}", "The prepare command failed: {reason}"));
        }
    }
    let target_dir = work.path().join("target");
    copy_project(&base, &target_dir).map_err(|e| crate::t!("Proje kopyalanamadı: {e}", "Couldn't copy the project: {e}"))?;
    for (file, content) in &options.fixes {
        crate::safefs::resolve(&target_dir, file)
            .and_then(|path| crate::safefs::replace(&path, content.as_bytes()))
            .map_err(|e| crate::t!("Düzeltme kopyaya yazılamadı ({file}): {e}", "Couldn't write the fix to the copy ({file}): {e}"))?;
    }
    let runs = run_version(&container, source, &base, &setup, &step)
        .and_then(|first| run_version(&container, target, &target_dir, &setup, &step).map(|second| (first, second)));
    if options.clean_images {
        let _ = remove(&docker, &setup.built.borrow());
    }
    let (source_run, target_run) = runs?;
    let pages = crate::web::compare(&source_run.crawl, &target_run.crawl);
    let data_changes = crate::database::data_changes(&source_run.data, &target_run.data);
    let new_failures = target_run.failed_tests.iter().filter(|t| !source_run.failed_tests.contains(t)).cloned().collect();
    let new_on_target = differs(&target_run.events, &source_run.events);
    let gone_on_target = differs(&source_run.events, &target_run.events);
    let origins = crate::origin::group(root, &new_on_target, target);
    Ok(Comparison {
        source: source_run,
        target: target_run,
        dependencies_installed,
        install_log,
        new_failures,
        pages,
        data_changes,
        database_used: options.database.is_some(),
        new_on_target,
        gone_on_target,
        origins,
    })
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

    fn comparison(new_failures: &[&str], events: Vec<Event>) -> Comparison {
        let run = |version| VersionRun {
            version,
            image: String::new(),
            lint_errors: 0,
            tests: Some(TestRun { exit_code: Some(1), timed_out: false, passed: new_failures.is_empty(), by_file: false }),
            failed_tests: new_failures.iter().map(|t| t.to_string()).collect(),
            crawl: Default::default(),
            data: Default::default(),
            missing_extensions: Vec::new(),
            installed_extensions: Vec::new(),
            image_failed: false,
            events: Vec::new(),
            log: String::new(),
        };
        Comparison {
            source: run(PhpVersion::new(7, 4)),
            target: run(PhpVersion::new(8, 4)),
            dependencies_installed: true,
            install_log: String::new(),
            new_failures: new_failures.iter().map(|t| t.to_string()).collect(),
            pages: Vec::new(),
            data_changes: Vec::new(),
            database_used: false,
            new_on_target: events,
            gone_on_target: Vec::new(),
            origins: Vec::new(),
        }
    }

    fn page(path: &str, change: crate::web::PageChange) -> crate::web::PageDiff {
        crate::web::PageDiff {
            path: path.into(),
            change,
            source_status: 200,
            target_status: 200,
            new_errors: Vec::new(),
            source_text: String::new(),
            target_text: String::new(),
        }
    }

    #[test]
    fn an_outcome_follows_messages_across_shifted_lines() {
        let before = comparison(&["A::x", "A::y"], vec![event("/app/a.php", 10, "moved at 10"), event("/app/a.php", 20, "gone")]);
        let after = comparison(&["A::y", "A::z"], vec![event("/app/a.php", 14, "moved at 14"), event("/app/b.php", 3, "new")]);
        let result = outcome(&before, &after);
        assert_eq!(result.messages.fixed, [event("/app/a.php", 20, "gone")]);
        assert_eq!(result.messages.remaining, [event("/app/a.php", 14, "moved at 14")]);
        assert_eq!(result.messages.introduced, [event("/app/b.php", 3, "new")]);
        assert_eq!(result.tests.fixed, ["A::x"]);
        assert_eq!(result.tests.remaining, ["A::y"]);
        assert_eq!(result.tests.introduced, ["A::z"]);
        assert!(result.worsened());
    }

    #[test]
    fn repeated_messages_are_counted_one_by_one() {
        let same = |line| event("/app/a.php", line, "same");
        let before = comparison(&[], vec![same(1), same(2)]);
        let result = outcome(&before, &comparison(&[], vec![same(1)]));
        assert_eq!((result.messages.fixed.len(), result.messages.remaining.len()), (1, 1));
        assert!(!result.worsened());
    }

    #[test]
    fn what_the_rerun_could_not_check_is_not_called_fixed() {
        use crate::web::PageChange::{Errors, Same, Unverified};
        let mut before = comparison(&["A::x"], Vec::new());
        before.pages = vec![page("/a", Errors), page("/b", Errors), page("/c", Errors)];
        let mut after = comparison(&[], Vec::new());
        after.target.tests.as_mut().unwrap().passed = false;
        after.pages = vec![page("/a", Same), page("/b", Unverified)];
        let result = outcome(&before, &after);
        assert_eq!(result.tests.remaining, ["A::x"]);
        assert!(result.tests.fixed.is_empty());
        assert_eq!(result.pages.fixed, ["/a"]);
        assert_eq!(result.pages.remaining, ["/b", "/c"]);
        assert!(result.regressed_after);
    }

    #[test]
    fn a_test_command_replaces_phpunit_on_each_version() {
        if std::env::var_os("FERRY_TEST_DOCKER").is_none() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("check.php"), "<?php exit(PHP_MAJOR_VERSION === 8 ? 3 : 0);\n").unwrap();
        let options = Options { test: Some("php check.php".to_string()), ..Options::default() };
        let result = compare(dir.path(), PhpVersion::new(7, 4), PhpVersion::new(8, 4), options, &AtomicBool::new(false), |_| {}).unwrap();
        assert!(result.source.tests.as_ref().is_some_and(|t| t.passed), "{:?}", result.source);
        assert!(result.target.tests.as_ref().is_some_and(|t| !t.passed && t.exit_code == Some(3)), "{:?}", result.target);
        assert!(result.regressed());
    }

    #[test]
    fn a_fix_runs_on_the_target_copy_only() {
        if std::env::var_os("FERRY_TEST_DOCKER").is_none() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let broken = "<?php\nfunction total($a) { return each($a); }\necho PHP_MAJOR_VERSION, ' ', json_encode(total(['ok']));\n";
        std::fs::write(dir.path().join("index.php"), broken).unwrap();
        let options = Options { pages: vec!["/".to_string()], ..Options::default() };
        let (source, target) = (PhpVersion::new(7, 4), PhpVersion::new(8, 4));
        let before = compare(dir.path(), source, target, options.clone(), &AtomicBool::new(false), |_| {}).unwrap();
        assert!(before.regressed(), "{before:?}");
        let fixed = "<?php\nfunction total($a) { return [1 => current($a), 'value' => current($a), 0 => key($a), 'key' => key($a)]; }\necho 7, ' ', json_encode(total(['ok']));\n";
        let options = Options { fixes: vec![("index.php".to_string(), fixed.to_string())], ..options };
        let after = compare(dir.path(), source, target, options, &AtomicBool::new(false), |_| {}).unwrap();
        assert_eq!(std::fs::read_to_string(dir.path().join("index.php")).unwrap(), broken);
        let result = outcome(&before, &after);
        assert!(result.regressed_before && !result.regressed_after, "{result:?}");
        assert!(result.messages.remaining.is_empty() && !result.messages.fixed.is_empty(), "{result:?}");
        assert_eq!(result.pages.fixed, ["/"], "{result:?}\n{:?}", after.pages);
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
        let options = Options { install_extensions: true, clean_images: true, ..Options::default() };
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
    fn databases_are_imported_for_each_version() {
        if std::env::var_os("FERRY_TEST_DOCKER").is_none() {
            return;
        }
        for (dump, url, sql) in [
            ("CREATE TABLE items (name TEXT);\nINSERT INTO items VALUES ('elma'), ('armut');\n", "mysql:host=127.0.0.1;dbname=ferry", "my"),
            (
                "-- PostgreSQL database dump\nCREATE TABLE items (name TEXT);\nINSERT INTO items VALUES ('elma'), ('armut');\n",
                "pgsql:host=127.0.0.1;dbname=ferry",
                "pg",
            ),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let page = format!("<?php $d = new PDO('{url}', getenv('DB_USERNAME'), getenv('DB_PASSWORD')); foreach ($d->query('select name from items') as $r) {{ echo $r[0], ' '; }}");
            std::fs::write(dir.path().join("index.php"), page).unwrap();
            let file = dir.path().join(format!("{sql}.sql"));
            std::fs::write(&file, dump).unwrap();
            let options = Options {
                install_extensions: true,
                clean_images: true,
                pages: vec!["/".to_string()],
                database: Some(crate::database::Source::Dump(file)),
                ..Options::default()
            };
            let result =
                compare(dir.path(), PhpVersion::new(8, 3), PhpVersion::new(8, 4), options, &AtomicBool::new(false), |_| {}).unwrap();
            let page = &result.pages[0];
            assert!(page.source_text.contains("elma armut"), "{sql}: {page:?}");
            assert_eq!(page.change, crate::web::PageChange::Same, "{sql}: {page:?}");
            assert!(result.data_changes.is_empty(), "{sql}: {:?}", result.data_changes);
            assert_eq!(result.source.data.get("items").map(|r| r.0), Some(2), "{sql}");
        }
    }

    #[test]
    fn hardcoded_localhost_reaches_mysql_through_its_socket() {
        if std::env::var_os("FERRY_TEST_DOCKER").is_none() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let page = "<?php $d = mysqli_connect('localhost', 'ferry', 'ferry', 'ferry'); $e = new PDO('mysql:host=localhost;dbname=ferry', 'ferry', 'ferry'); $r = mysqli_query($d, 'select name from items'); echo mysqli_fetch_row($r)[0], ' ', $e->query('select count(*) from items')->fetchColumn();";
        std::fs::write(dir.path().join("index.php"), page).unwrap();
        let dump = dir.path().join("seed.sql");
        std::fs::write(&dump, "CREATE TABLE items (name TEXT);\nINSERT INTO items VALUES ('elma');\n").unwrap();
        let options = Options {
            clean_images: true,
            pages: vec!["/".to_string()],
            database: Some(crate::database::Source::Dump(dump)),
            ..Options::default()
        };
        let result = compare(dir.path(), PhpVersion::new(8, 3), PhpVersion::new(8, 4), options, &AtomicBool::new(false), |_| {}).unwrap();
        assert!(result.pages[0].source_text.contains("elma 1"), "{:?}\n{}", result.pages, result.source.log);
        assert_eq!(result.pages[0].change, crate::web::PageChange::Same, "{:?}", result.pages);
    }

    #[test]
    fn sqlite_files_stay_separate_for_each_version() {
        if std::env::var_os("FERRY_TEST_DOCKER").is_none() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".env"), "DB_CONNECTION=sqlite\nDB_DATABASE=data/app.sqlite\n").unwrap();
        let page = "<?php $d = new PDO('sqlite:' . getenv('DB_DATABASE')); foreach ($d->query('select name from items') as $r) { echo $r[0], ' '; }";
        std::fs::write(dir.path().join("index.php"), page).unwrap();
        let dump = dir.path().join("seed.sql");
        std::fs::write(&dump, "PRAGMA foreign_keys=OFF;\nCREATE TABLE items (name TEXT);\nINSERT INTO items VALUES ('elma');\n").unwrap();
        let options = Options { pages: vec!["/".to_string()], database: Some(crate::database::Source::Dump(dump)), ..Options::default() };
        let result = compare(dir.path(), PhpVersion::new(8, 3), PhpVersion::new(8, 4), options, &AtomicBool::new(false), |_| {}).unwrap();
        assert!(result.pages[0].source_text.contains("elma"), "{:?}", result.pages);
        assert_eq!(result.source.data.get("items").map(|r| r.0), Some(1), "{:?}", result.source.data);
        assert_eq!(result.source.data, result.target.data);
        assert!(!dir.path().join("data").exists(), "the project stays untouched");
    }

    #[test]
    fn pages_are_compared_between_versions() {
        if std::env::var_os("FERRY_TEST_DOCKER").is_none() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.php"), "<?php echo '<a href=\"/old.php\">old</a><a href=\"/same.php\">same</a>';").unwrap();
        std::fs::write(dir.path().join("old.php"), "<?php $a = [1]; reset($a); print_r(each($a));").unwrap();
        std::fs::write(dir.path().join("same.php"), "<?php echo 'same';").unwrap();
        let options = Options { pages: vec!["/".to_string()], ..Options::default() };
        let result = compare(dir.path(), PhpVersion::new(7, 4), PhpVersion::new(8, 4), options, &AtomicBool::new(false), |_| {}).unwrap();
        let change = |path: &str| result.pages.iter().find(|p| p.path == path).map(|p| p.change);
        assert_eq!(change("/same.php"), Some(crate::web::PageChange::Same), "{:?}", result.pages);
        assert_eq!(change("/old.php"), Some(crate::web::PageChange::Status), "{:?}", result.pages);
    }

    #[test]
    fn stopping_ends_the_run_quickly() {
        if std::env::var_os("FERRY_TEST_DOCKER").is_none() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.php"), "<?php\necho gmp_add(1, 2);\n").unwrap();
        let cancel = AtomicBool::new(false);
        let options = Options { install_extensions: true, clean_images: true, ..Options::default() };
        let started = std::time::Instant::now();
        let result = std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(Duration::from_secs(6));
                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            });
            compare(dir.path(), PhpVersion::new(7, 3), PhpVersion::new(8, 3), options, &cancel, |s| {
                eprintln!("{s:?} {:?}", started.elapsed())
            })
        });
        eprintln!("{:?} {:?}", started.elapsed(), result.as_ref().map(|_| "done"));
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_secs(30), "{:?}", started.elapsed());
    }

    #[test]
    fn the_phpunit_bridge_gets_a_real_phpunit() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("composer.json"), r#"{"require-dev":{"symfony/phpunit-bridge":"^7.0"}}"#).unwrap();
        std::fs::write(dir.path().join("phpunit.xml.dist"), r#"<server name="SYMFONY_PHPUNIT_VERSION" value="10.5"/>"#).unwrap();
        assert!(phpunit_bridge_version(dir.path()).unwrap().contains("phpunit/phpunit:^10.5"));
        std::fs::write(dir.path().join("phpunit.xml.dist"), r#"<server name="SYMFONY_PHPUNIT_VERSION" value="1; rm -rf /"/>"#).unwrap();
        assert!(phpunit_bridge_version(dir.path()).unwrap().contains("phpunit/phpunit:^9.6"));
        std::fs::write(dir.path().join("composer.json"), r#"{"require-dev":{"symfony/phpunit-bridge":"^7.0","phpunit/phpunit":"^10"}}"#)
            .unwrap();
        assert_eq!(phpunit_bridge_version(dir.path()), None);
    }

    #[test]
    fn history_keeps_the_latest_runs_first() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("deep/history.json");
        let entry = |at| Summary {
            at,
            source: "7.4".into(),
            target: "8.4".into(),
            regressed: false,
            tests_compared: true,
            pages: 1,
            broken_pages: 0,
            differing_pages: 0,
            unverified_pages: 0,
            new_failures: 0,
            new_messages: 0,
            data_changes: 0,
        };
        for at in 0..25 {
            record(&file, entry(at)).unwrap();
        }
        let kept = history(&file);
        assert_eq!((kept.len(), kept[0].at, kept[19].at), (20, 24, 5));
    }

    #[test]
    fn a_bridge_exit_with_ok_output_counts_as_passed() {
        assert!(tests_passed("PHPUnit\n\nOK (51 tests, 115 assertions)\n\nRemaining indirect deprecation notices (3)\n"));
        assert!(tests_passed("OK, but there were issues!\n"));
        assert!(!tests_passed("ERRORS!\nTests: 1, Errors: 1.\n"));
    }

    #[test]
    fn only_real_failures_count_as_failed_tests() {
        let out = "There were 2 errors:\n\n1) A::one\nError: x\n\n/app/a.php:1\n\n2) A::two with data set #1\nboom\n\n--\n\nThere was 1 failure:\n\n1) B::three\nnope\n\nThere were 3 skipped tests:\n\n1) C::four\nskip\n\nThere was 1 error:\n\n1) D::five\nx\n\n2 tests triggered 2 PHP deprecations:\n\n1) /app/src/Logger.php:331\nold\n\n2) E::six\nold\n";
        assert_eq!(failed_tests(out), ["A::one", "A::two with data set #1", "B::three", "D::five"]);
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
        let result =
            compare(dir.path(), PhpVersion::new(7, 4), PhpVersion::new(8, 4), Options::default(), &AtomicBool::new(false), |_| {}).unwrap();
        assert_eq!((result.source.lint_errors, result.target.lint_errors), (0, 1), "{}", result.target.log);
        assert!(result.target.events.iter().all(|e| e.path == "/app/a.php"), "{:?}", result.target.events);
        assert!(result.new_on_target.iter().any(|e| e.level == Level::Fatal), "{:?}", result.new_on_target);
    }
}
