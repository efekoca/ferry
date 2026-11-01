use std::path::{Path, PathBuf};

pub fn lint(php: &Path, files: &[PathBuf]) -> String {
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, 8);
    let chunk = files.len().div_ceil(workers).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = files
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    let mut out = String::new();
                    for file in part {
                        let Ok(result) = crate::toolchain::command(php)
                            .args(["-n", "-d", "error_reporting=-1", "-d", "display_errors=stderr", "-d", "log_errors=0", "-l"])
                            .arg(file)
                            .output()
                        else {
                            continue;
                        };
                        for line in String::from_utf8_lossy(&result.stderr).lines().filter(|l| l.contains(" on line ")) {
                            out.push_str(&format!("PHP {}\n", line.trim().trim_start_matches("PHP ")));
                        }
                    }
                    out
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or_default()).collect()
    })
}

pub fn phpunit(root: &Path) -> Option<PathBuf> {
    let p = root.join("vendor").join("bin").join("phpunit");
    p.is_file().then_some(p)
}

pub fn run_tests(php: &Path, root: &Path, timeout: std::time::Duration) -> Result<String, String> {
    let phpunit = phpunit(root).ok_or_else(|| crate::t!("Projede vendor/bin/phpunit yok.", "The project has no vendor/bin/phpunit."))?;
    let version = crate::toolchain::command(php).arg(&phpunit).arg("--version").current_dir(root).output().ok();
    let modern = version.is_some_and(|v| {
        !String::from_utf8_lossy(&v.stdout).contains("PHPUnit 9") && !String::from_utf8_lossy(&v.stdout).contains("PHPUnit 8")
    });
    let mut cmd = crate::toolchain::command(php);
    cmd.args(["-d", "error_reporting=-1", "-d", "display_errors=1", "-d", "memory_limit=1G"]).arg(&phpunit).current_dir(root);
    if modern {
        cmd.args(["--display-deprecations", "--display-warnings", "--no-progress"]);
    }
    let mut child = cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().map_err(|e| e.to_string())?;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let reader = std::thread::spawn(move || {
        use std::io::Read;
        let (mut a, mut b) = (String::new(), String::new());
        let _ = stdout.read_to_string(&mut a);
        let _ = stderr.read_to_string(&mut b);
        a + &b
    });
    let started = std::time::Instant::now();
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            break;
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    Ok(reader.join().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lint_output_feeds_the_runtime_parser() {
        let Some(php) = crate::toolchain::find_php(None) else { return };
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.php");
        std::fs::write(&file, "<?php\nfunction f(int $a = null) {}\n$x = (real) 1;\n").unwrap();
        let out = lint(&php, &[file]);
        let events = crate::runtime::parse(&out);
        assert!(!events.is_empty(), "{out}");
        assert!(events.iter().all(|e| e.path.ends_with("a.php")));
    }
}
