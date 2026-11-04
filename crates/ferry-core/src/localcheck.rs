use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const LOG_LIMIT: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CheckKind {
    #[default]
    Lint,
    Tests,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CheckRun {
    pub kind: CheckKind,
    pub command: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub passed: bool,
    pub duration_ms: u64,
    pub files: usize,
    pub failed: usize,
    pub not_run: usize,
    pub log: String,
    #[serde(skip)]
    pub output: String,
}

fn tail(text: &str) -> String {
    if text.len() <= LOG_LIMIT {
        return text.to_string();
    }
    let mut start = text.len() - LOG_LIMIT;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    format!("…\n{}", &text[start..])
}

const STEP_TIMEOUT: Duration = Duration::from_secs(60);

pub fn lint(php: &Path, files: &[PathBuf]) -> CheckRun {
    let started = Instant::now();
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, 8);
    let chunk = files.len().div_ceil(workers).max(1);
    let never = std::sync::atomic::AtomicBool::new(false);
    let never = &never;
    let parts: Vec<(String, usize, usize)> = std::thread::scope(|scope| {
        let handles: Vec<_> = files
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    let (mut out, mut failed, mut not_run) = (String::new(), 0, 0);
                    for file in part {
                        let mut cmd = crate::toolchain::command(php);
                        cmd.args(["-n", "-d", "error_reporting=-1", "-d", "display_errors=stderr", "-d", "log_errors=0", "-l"]).arg(file);
                        let Ok(result) = crate::toolchain::run_process(cmd, php, never, STEP_TIMEOUT) else {
                            not_run += 1;
                            continue;
                        };
                        let Some(status) = result.status.filter(|_| result.stopped.is_none()) else {
                            not_run += 1;
                            continue;
                        };
                        if !status.success() {
                            failed += 1;
                        }
                        for line in String::from_utf8_lossy(&result.stderr).lines().filter(|l| l.contains(" on line ")) {
                            out.push_str(&format!("PHP {}\n", line.trim().trim_start_matches("PHP ")));
                        }
                    }
                    (out, failed, not_run)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or_default()).collect()
    });
    let output: String = parts.iter().map(|p| p.0.as_str()).collect();
    let failed = parts.iter().map(|p| p.1).sum();
    let not_run = parts.iter().map(|p| p.2).sum();
    CheckRun {
        kind: CheckKind::Lint,
        command: format!("{} -l", php.display()),
        exit_code: None,
        timed_out: false,
        passed: failed == 0 && not_run == 0,
        duration_ms: started.elapsed().as_millis() as u64,
        files: files.len(),
        failed,
        not_run,
        log: tail(&output),
        output,
    }
}

pub fn phpunit(root: &Path) -> Option<PathBuf> {
    let p = root.join("vendor").join("bin").join("phpunit");
    p.is_file().then_some(p)
}

pub fn run_tests(php: &Path, root: &Path, timeout: Duration, cancel: &std::sync::atomic::AtomicBool) -> Result<CheckRun, String> {
    let phpunit = phpunit(root).ok_or_else(|| crate::t!("Projede vendor/bin/phpunit yok.", "The project has no vendor/bin/phpunit."))?;
    let mut probe = crate::toolchain::command(php);
    probe.arg(&phpunit).arg("--version").current_dir(root);
    let version = crate::toolchain::run_process(probe, php, cancel, STEP_TIMEOUT).ok();
    let modern = version.is_some_and(|v| {
        !String::from_utf8_lossy(&v.stdout).contains("PHPUnit 9") && !String::from_utf8_lossy(&v.stdout).contains("PHPUnit 8")
    });
    let extra: &[&str] = if modern { &["--display-deprecations", "--display-warnings", "--no-progress"] } else { &[] };
    let mut cmd = crate::toolchain::command(php);
    cmd.args(["-d", "error_reporting=-1", "-d", "display_errors=1", "-d", "memory_limit=1G"]).arg(&phpunit).args(extra).current_dir(root);
    let done = crate::toolchain::run_process(cmd, php, cancel, timeout).map_err(|e| e.to_string())?;
    let output = format!("{}{}", String::from_utf8_lossy(&done.stdout), String::from_utf8_lossy(&done.stderr));
    let exit_code = done.status.and_then(|s| s.code());
    let timed_out = done.stopped == Some(crate::toolchain::Stop::TimedOut);
    Ok(CheckRun {
        kind: CheckKind::Tests,
        command: format!("{} vendor/bin/phpunit {}", php.display(), extra.join(" ")).trim_end().to_string(),
        exit_code,
        timed_out,
        passed: exit_code == Some(0) && done.stopped.is_none() && ran_tests(&output),
        duration_ms: done.duration.as_millis() as u64,
        log: tail(&output),
        output,
        ..CheckRun::default()
    })
}

fn ran_tests(output: &str) -> bool {
    !output.contains("No tests executed")
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_empty_test_suite_is_not_a_pass() {
        assert!(!super::ran_tests("PHPUnit 11.0\n\nNo tests executed!\n"));
        assert!(super::ran_tests("OK (3 tests, 5 assertions)\n"));
    }

    use super::*;

    #[test]
    fn lint_output_feeds_the_runtime_parser() {
        let Some(php) = crate::toolchain::find_php(None) else { return };
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.php");
        std::fs::write(&file, "<?php\nfunction f(int $a = null) {}\n$x = (real) 1;\n").unwrap();
        let run = lint(&php, &[file]);
        let events = crate::runtime::parse(&run.output);
        assert!(!events.is_empty(), "{}", run.output);
        assert_eq!((run.files, run.not_run), (1, 0));
        assert!(events.iter().all(|e| e.path.ends_with("a.php")));
    }

    fn fake_phpunit(body: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("vendor/bin")).unwrap();
        std::fs::write(
            dir.path().join("vendor/bin/phpunit"),
            format!("<?php\nif (in_array('--version', $argv)) {{ echo \"PHPUnit 10.5.0\\n\"; exit(0); }}\n{body}"),
        )
        .unwrap();
        dir
    }

    #[test]
    fn test_results_keep_the_exit_status() {
        let Some(php) = crate::toolchain::find_php(None) else { return };
        let never = std::sync::atomic::AtomicBool::new(false);
        let failing = fake_phpunit("echo \"There was 1 failure:\\n1) assertion failed\\n\"; exit(1);");
        let run = run_tests(&php, failing.path(), Duration::from_secs(60), &never).unwrap();
        assert_eq!((run.exit_code, run.passed, run.timed_out), (Some(1), false, false));
        assert!(run.log.contains("assertion failed"), "{}", run.log);

        let passing = fake_phpunit("echo \"OK (3 tests)\\n\";");
        let run = run_tests(&php, passing.path(), Duration::from_secs(60), &never).unwrap();
        assert_eq!((run.exit_code, run.passed), (Some(0), true));
        assert!(run.command.ends_with("vendor/bin/phpunit --display-deprecations --display-warnings --no-progress"), "{}", run.command);
    }

    #[test]
    fn heavy_stderr_does_not_block_and_timeouts_are_reported() {
        let Some(php) = crate::toolchain::find_php(None) else { return };
        let never = std::sync::atomic::AtomicBool::new(false);
        let noisy = fake_phpunit("fwrite(STDERR, str_repeat('x', 1 << 20)); echo \"COMPLETED\\n\";");
        let run = run_tests(&php, noisy.path(), Duration::from_secs(30), &never).unwrap();
        assert!(run.passed && run.output.contains("COMPLETED"), "exit {:?}", run.exit_code);
        assert!(run.log.len() <= LOG_LIMIT + 8);

        let slow = fake_phpunit("echo \"started\\n\"; sleep(30);");
        let started = Instant::now();
        let run = run_tests(&php, slow.path(), Duration::from_millis(800), &never).unwrap();
        assert!(run.timed_out && !run.passed);
        assert_eq!(run.exit_code, None);
        assert!(started.elapsed() < Duration::from_secs(10));
    }
}
