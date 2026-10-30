use ferry_core::{analyzer, project, PhpVersion};
use std::collections::{BTreeMap, HashMap, HashSet};
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

#[derive(Default)]
struct Tally {
    php: usize,
    matched: usize,
    missed: BTreeMap<String, Vec<String>>,
    extra: BTreeMap<String, Vec<String>>,
}

const COMPILE_TIME: &[&str] = &[
    "removed.curly_offset",
    "removed.real_cast",
    "removed.unset_cast",
    "removed.nested_ternary",
    "removed.__autoload",
    "removed.reserved_name",
    "removed.case_insensitive_constant_decl",
    "deprecated.dollar_brace_interpolation",
    "deprecated.implicit_nullable",
    "deprecated.optional_before_required",
    "deprecated.case_semicolon",
    "deprecated.non_canonical_cast",
    "deprecated.backtick",
    "deprecated.serializable",
];

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
                let mut used = HashSet::new();
                for (line, msg) in &diags {
                    tally.php += 1;
                    let hit = findings.iter().position(|f| f.line.abs_diff(*line) <= 2 && !used.contains(&f.id));
                    match hit {
                        Some(i) => {
                            used.insert(findings[i].id.clone());
                            tally.matched += 1;
                        }
                        None => tally.missed.entry(classify(msg)).or_default().push(format!("{name}/{rel}:{line} {msg}")),
                    }
                }
                let fatal = diags.iter().any(|(_, m)| m.contains("error"));
                for f in &findings {
                    if !COMPILE_TIME.contains(&f.rule_id.as_str()) || used.contains(&f.id) {
                        continue;
                    }
                    if fatal && diags.iter().any(|(l, m)| m.contains("error") && *l < f.line) {
                        continue;
                    }
                    if !diags.iter().any(|(l, _)| l.abs_diff(f.line) <= 2) {
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
