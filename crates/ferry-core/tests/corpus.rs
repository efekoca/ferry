use ferry_core::{analyzer, project, PhpVersion};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;

fn php_and_target() -> Option<(PathBuf, String)> {
    let php = PathBuf::from(std::env::var_os("CORPUS_PHP")?);
    let out = Command::new(&php).args(["-r", "echo PHP_MAJOR_VERSION, '.', PHP_MINOR_VERSION;"]).output().ok()?;
    Some((php, String::from_utf8_lossy(&out.stdout).trim().to_string()))
}

#[test]
#[ignore = "needs CORPUS_PHP"]
fn every_deprecated_function_is_reported() {
    let (php, target) = php_and_target().expect("set CORPUS_PHP to a PHP binary");
    {
        let target = target.as_str();
        let out = Command::new(&php)
            .args([
                "-r",
                r#"foreach(get_defined_functions()["internal"] as $f){ if((new ReflectionFunction($f))->isDeprecated()) echo "$f\n";}"#,
            ])
            .output()
            .unwrap();
        let names = String::from_utf8_lossy(&out.stdout).to_string();
        let mut missing = Vec::new();
        for name in names.lines() {
            let src = format!("<?php\n{name}($x);\n");
            let found = analyzer::analyze_source("t.php", &src, target.parse().unwrap());
            if found.is_empty() {
                missing.push(name.to_string());
            }
        }
        assert!(missing.is_empty(), "PHP {target} deprecates functions Ferry doesn't report: {}", missing.join(" "));
    }
}

#[test]
#[ignore = "needs CORPUS_PHP"]
fn every_deprecated_method_and_constant_is_reported() {
    let (php, target) = php_and_target().expect("set CORPUS_PHP to a PHP binary");
    let script = r#"
        $cur = "";
        set_error_handler(function ($no, $msg) use (&$cur) {
            if ($no === E_DEPRECATED && preg_match('/deprecated since (\d+\.\d+)/', $msg, $m)) echo $cur, " ", $m[1], "\n";
            return true;
        });
        foreach (array_keys(get_defined_constants()) as $k) { $cur = "const $k"; constant($k); }
        foreach (array_merge(get_declared_classes(), get_declared_interfaces()) as $c) {
            $r = new ReflectionClass($c);
            if (!$r->isInternal()) continue;
            foreach ($r->getReflectionConstants() as $k) {
                if ($k->getDeclaringClass()->getName() !== $c) continue;
                $cur = "classconst $c " . $k->getName();
                constant("$c::" . $k->getName());
            }
            foreach ($r->getMethods() as $m) {
                if (!$m->isDeprecated() || $m->getDeclaringClass()->getName() !== $c) continue;
                $attr = $m->getAttributes("Deprecated")[0] ?? null;
                $since = $attr ? ($attr->getArguments()["since"] ?? "?") : "?";
                echo "method $c ", $m->getName(), " ", $m->isStatic() ? 1 : 0, " ", $since, "\n";
            }
        }
    "#;
    let out = Command::new(&php).args(["-r", script]).output().unwrap();
    let mut missing = Vec::new();
    let mut versions = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let parts: Vec<&str> = line.split(' ').collect();
        let (src, since) = match parts.as_slice() {
            ["method", class, method, "1", since] => (format!("<?php\n{class}::{method}();\n"), *since),
            ["method", class, method, _, since] => (format!("<?php\n$o = new {class}();\n$o->{method}();\n"), *since),
            ["classconst", class, name, since] => (format!("<?php\necho {class}::{name};\n"), *since),
            ["const", name, since] => (format!("<?php\necho {name};\n"), *since),
            _ => continue,
        };
        let found = analyzer::analyze_source("t.php", &src, target.parse().unwrap());
        match found.first() {
            None => missing.push(line.to_string()),
            Some(f) if since != "?" && f.since.to_string() != since => versions.push(format!("{line} (Ferry says {})", f.since)),
            _ => {}
        }
    }
    assert!(
        missing.is_empty() && versions.is_empty(),
        "PHP {target} deprecates symbols Ferry doesn't report:\n{}\nwith a different version:\n{}",
        missing.join("\n"),
        versions.join("\n")
    );
}

#[derive(Default)]
struct Tally {
    php: usize,
    matched: usize,
    missed: BTreeMap<String, Vec<String>>,
    extra: BTreeMap<String, Vec<String>>,
}

fn classify(msg: &str) -> String {
    let m = msg.to_ascii_lowercase();
    let key = [
        ("array and string offset access syntax with curly braces", "curly"),
        ("real", "real_cast"),
        ("unset", "unset_cast"),
        ("nested ternary", "nested_ternary"),
        ("__autoload", "autoload"),
        ("reserved", "reserved"),
        ("${", "dollar_brace"),
        ("implicitly marking parameter", "implicit_nullable"),
        ("declared before required parameter", "optional_before_required"),
        ("case separator", "case_semicolon"),
        ("non-canonical cast", "non_canonical_cast"),
        ("backticks", "backtick"),
        ("serializable", "serializable"),
        ("#[", "attribute"),
    ];
    key.iter().find(|(k, _)| m.contains(k)).map(|(_, v)| v.to_string()).unwrap_or_else(|| msg.chars().take(70).collect())
}

fn lint(php: &Path, file: &Path) -> Vec<(usize, String)> {
    let Ok(out) = Command::new(php).args(["-n", "-d", "error_reporting=-1", "-d", "display_errors=1", "-l"]).arg(file).output() else {
        return Vec::new();
    };
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    text.lines()
        .filter(|l| l.contains(" on line ") && (l.contains("Deprecated") || l.contains("error")))
        .filter_map(|l| {
            let line = l.rsplit(" on line ").next()?.trim().parse().ok()?;
            Some((line, l.trim().trim_start_matches("PHP ").to_string()))
        })
        .collect()
}

#[test]
#[ignore = "needs CORPUS and CORPUS_PHP"]
fn compile_time_findings_match_php() {
    let corpus = PathBuf::from(std::env::var_os("CORPUS").expect("set CORPUS to a folder of PHP projects"));
    let (php, target) = php_and_target().expect("set CORPUS_PHP to a PHP binary");
    let mut projects: Vec<PathBuf> = std::fs::read_dir(&corpus).unwrap().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    projects.sort();
    {
        let target = target.as_str();
        let mut tally = Tally::default();
        let mut rule_counts: BTreeMap<String, usize> = BTreeMap::new();
        for root in &projects {
            let files = project::collect_php_files(root, &[]);
            let result = analyzer::analyze_files(root, &files, target.parse::<PhpVersion>().unwrap(), |_, _, _| {});
            for f in &result.findings {
                *rule_counts.entry(f.rule_id.clone()).or_default() += 1;
            }
            let mut by_file: HashMap<String, Vec<&analyzer::Finding>> = HashMap::new();
            for f in &result.findings {
                by_file.entry(f.file.clone()).or_default().push(f);
            }
            let name = root.file_name().unwrap().to_string_lossy().to_string();
            let chunks: Vec<Vec<PathBuf>> = files.chunks(files.len().div_ceil(8).max(1)).map(|c| c.to_vec()).collect();
            let php2 = php.clone();
            let results: Vec<(PathBuf, Vec<(usize, String)>)> = std::thread::scope(|s| {
                let handles: Vec<_> = chunks
                    .iter()
                    .map(|chunk| {
                        let php = php2.clone();
                        s.spawn(move || chunk.iter().map(|f| (f.clone(), lint(&php, f))).collect::<Vec<_>>())
                    })
                    .collect();
                handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
            });
            for (file, diags) in results {
                let rel = analyzer::relative_path(root, &file);
                let findings = by_file.get(&rel).cloned().unwrap_or_default();
                let near = |a: usize, b: usize| a.abs_diff(b) <= 10;
                for (line, msg) in &diags {
                    tally.php += 1;
                    if findings.iter().any(|f| near(f.line, *line)) {
                        tally.matched += 1;
                    } else {
                        tally.missed.entry(classify(msg)).or_default().push(format!("{name}/{rel}:{line} {msg}"));
                    }
                }
                if diags.iter().any(|(_, m)| m.contains("Parse error")) {
                    continue;
                }
                for f in &findings {
                    let compiled =
                        ferry_core::rules::COMPILE_TIME.contains(&f.rule_id.as_str()) || f.rule_id == "removed.incompatible_override";
                    if !compiled || diags.iter().any(|(l, m)| m.contains("error") && *l < f.line) {
                        continue;
                    }
                    if !diags.iter().any(|(l, _)| near(*l, f.line)) {
                        tally.extra.entry(f.rule_id.clone()).or_default().push(format!("{name}/{rel}:{} {}", f.line, f.snippet));
                    }
                }
            }
        }
        println!("\n===== target {target}: php diagnostics {} matched {}", tally.php, tally.matched);
        println!("finding counts: {rule_counts:?}");
        for (k, v) in &tally.missed {
            println!("MISSED [{k}] x{}", v.len());
            for e in v.iter().take(4) {
                println!("    {e}");
            }
        }
        for (k, v) in &tally.extra {
            println!("EXTRA [{k}] x{}", v.len());
            for e in v.iter().take(4) {
                println!("    {e}");
            }
        }
    }
}

#[test]
#[ignore = "needs CORPUS"]
fn scan_speed() {
    let corpus = PathBuf::from(std::env::var_os("CORPUS").expect("set CORPUS to a folder of PHP projects"));
    let mut projects: Vec<PathBuf> = std::fs::read_dir(&corpus).unwrap().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    projects.sort();
    for root in projects {
        let started = std::time::Instant::now();
        let info = project::inspect(&root).unwrap();
        let files = project::collect_php_files(&root, &info.excluded_dirs);
        let walked = started.elapsed();
        let started = std::time::Instant::now();
        let result = analyzer::analyze_files(&root, &files, "8.4".parse().unwrap(), |_, _, _| {});
        let scanned = started.elapsed();
        let lines: usize = result.lines_scanned;
        println!(
            "SPEED {:<12} files={:>5} lines={:>8} findings={:>6} walk={:>5}ms scan={:>6}ms ({:.0} files/s)",
            root.file_name().unwrap().to_string_lossy(),
            files.len(),
            lines,
            result.findings.len(),
            walked.as_millis(),
            scanned.as_millis(),
            files.len() as f64 / scanned.as_secs_f64()
        );
    }
}

#[test]
#[ignore = "needs CORPUS and FERRY_TEST_TOOLS"]
fn preview_speed() {
    let corpus = PathBuf::from(std::env::var_os("CORPUS").expect("set CORPUS"));
    let tools = PathBuf::from(std::env::var_os("FERRY_TEST_TOOLS").expect("set FERRY_TEST_TOOLS"));
    let php = ferry_core::toolchain::find_php(None).expect("php");
    for name in std::env::var("PREVIEW_PROJECTS").unwrap_or_default().split(',').filter(|s| !s.is_empty()) {
        let root = corpus.join(name);
        let info = project::inspect(&root).unwrap();
        let work = tempfile::tempdir().unwrap();
        let started = std::time::Instant::now();
        let preview = ferry_core::rector::preview(
            &php,
            &ferry_core::toolchain::rector_entry(&tools),
            &root,
            "8.4".parse().unwrap(),
            ferry_core::rector::RectorMode::Compat,
            &info.excluded_dirs,
            work.path(),
            &std::sync::atomic::AtomicBool::new(false),
        );
        match preview {
            Ok(p) => println!(
                "PREVIEW {name}: {} changed files, {} errors, {:.1}s",
                p.changes.len(),
                p.errors.len(),
                started.elapsed().as_secs_f64()
            ),
            Err(e) => println!("PREVIEW {name}: failed after {:.1}s: {e}", started.elapsed().as_secs_f64()),
        }
    }
}
