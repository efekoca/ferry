use ferry_core::analyzer::{self, FileError, Finding};
use ferry_core::history;
use ferry_core::i18n::{Lang, Localized};
use ferry_core::ignore::{self, IgnoredFinding};
use ferry_core::rules::{FixKind, Severity};
use ferry_core::{project, report, PhpVersion};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "Usage: ferry-cli scan <project> --target <8.2|8.3|8.4|8.5> [options]
       ferry-cli compare <project> --target <version> [--source <version>] [--format <text|json>]

Options:
  --format <text|json|sarif|github|junit|checkstyle|csv>
                                     Output format (default: text)
  --blame                            With --format csv, add the git blame author of each line
  --output <file>                    Write the report to a file instead of stdout
  --fail-on <breaking|deprecated|any|new|none>
                                     Exit with code 1 when findings at this level remain (default: breaking);
                                     with --baseline only findings missing from the baseline count
  --baseline <file>                  Compare with a baseline written by --write-baseline
  --write-baseline <file>            Save the current findings as a baseline
  --runtime-log <file>               Merge a PHP error log or PHPUnit deprecation output into the results
  --runtime-root <path>              Project root in the log's paths; only paths under it count (repeatable)
  --allow-incomplete                 Don't fail when some files couldn't be scanned
  --lang <en|tr>                     Language of messages (default: en)
  -h, --help                         Show this help

compare lints the project and runs its PHPUnit tests in php:X.Y-cli containers on both
versions, without network access. It needs Docker. --source defaults to the project's version.

Exit codes: 0 passed, 1 findings at the --fail-on level, 2 usage or run error,
3 some files couldn't be scanned (unless --allow-incomplete).
compare exits with 1 when the target adds fatal errors or failing tests";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Text,
    Json,
    Sarif,
    Github,
    Junit,
    Checkstyle,
    Csv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailOn {
    Breaking,
    Deprecated,
    Any,
    New,
    None,
}

#[derive(Debug)]
struct Options {
    root: PathBuf,
    target: PhpVersion,
    format: Format,
    output: Option<PathBuf>,
    fail_on: FailOn,
    lang: Lang,
    baseline: Option<PathBuf>,
    write_baseline: Option<PathBuf>,
    runtime_log: Option<PathBuf>,
    runtime_roots: Vec<String>,
    blame: bool,
    allow_incomplete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Passed,
    Failed,
    Incomplete,
}

fn parse(args: &[String]) -> Result<Options, String> {
    let mut it = args.iter();
    match it.next().map(String::as_str) {
        Some("scan") => {}
        Some("-h" | "--help") | None => return Err(String::new()),
        Some(other) => return Err(format!("unknown command: {other}")),
    }
    let (mut root, mut target, mut format, mut output, mut fail_on, mut lang) =
        (None, None, Format::Text, None, FailOn::Breaking, Lang::En);
    let (mut baseline, mut write_baseline, mut runtime_log, mut blame, mut allow_incomplete) = (None, None, None, false, false);
    let mut runtime_roots = Vec::new();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().cloned().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "-h" | "--help" => return Err(String::new()),
            "--target" => target = Some(value("--target")?),
            "--format" => {
                format = match value("--format")?.as_str() {
                    "text" => Format::Text,
                    "json" => Format::Json,
                    "sarif" => Format::Sarif,
                    "github" => Format::Github,
                    "junit" => Format::Junit,
                    "checkstyle" => Format::Checkstyle,
                    "csv" => Format::Csv,
                    other => return Err(format!("unknown format: {other}")),
                }
            }
            "--output" => output = Some(PathBuf::from(value("--output")?)),
            "--fail-on" => {
                fail_on = match value("--fail-on")?.as_str() {
                    "breaking" => FailOn::Breaking,
                    "deprecated" => FailOn::Deprecated,
                    "any" => FailOn::Any,
                    "new" => FailOn::New,
                    "none" => FailOn::None,
                    other => return Err(format!("unknown --fail-on level: {other}")),
                }
            }
            "--baseline" => baseline = Some(PathBuf::from(value("--baseline")?)),
            "--write-baseline" => write_baseline = Some(PathBuf::from(value("--write-baseline")?)),
            "--runtime-log" => runtime_log = Some(PathBuf::from(value("--runtime-log")?)),
            "--runtime-root" => runtime_roots.extend(ferry_core::runtime::parse_roots(&value("--runtime-root")?)),
            "--blame" => blame = true,
            "--allow-incomplete" => allow_incomplete = true,
            "--lang" => {
                lang = match value("--lang")?.as_str() {
                    "en" => Lang::En,
                    "tr" => Lang::Tr,
                    other => return Err(format!("unknown language: {other}")),
                }
            }
            flag if flag.starts_with('-') => return Err(format!("unknown option: {flag}")),
            path if root.is_none() => root = Some(PathBuf::from(path)),
            extra => return Err(format!("unexpected argument: {extra}")),
        }
    }
    let root = root.ok_or("missing project folder")?;
    let target: PhpVersion = target.ok_or("missing --target")?.parse()?;
    if target == PhpVersion::new(8, 1) {
        return Err("PHP 8.1 is end of life and won't be patched for CVE-2026-109618 and CVE-2026-109619; pick 8.2 or newer".into());
    }
    if !PhpVersion::TARGETS.contains(&target) {
        let list: Vec<String> = PhpVersion::TARGETS.iter().filter(|v| **v != PhpVersion::new(8, 1)).map(ToString::to_string).collect();
        return Err(format!("unsupported target {target}; use one of {}", list.join(", ")));
    }
    if fail_on == FailOn::New && baseline.is_none() {
        return Err("--fail-on new needs --baseline".into());
    }
    Ok(Options {
        root,
        target,
        format,
        output,
        fail_on,
        lang,
        baseline,
        write_baseline,
        runtime_log,
        runtime_roots,
        blame,
        allow_incomplete,
    })
}

fn text(l: &Localized, lang: Lang) -> &str {
    match lang {
        Lang::Tr => &l.tr,
        Lang::En => &l.en,
    }
}

fn severity_name(s: Severity) -> &'static str {
    match s {
        Severity::Breaking => "breaking",
        Severity::Deprecated => "deprecated",
        Severity::Behavior => "behavior",
    }
}

fn fails(f: &Finding, fail_on: FailOn) -> bool {
    match fail_on {
        FailOn::Breaking => f.severity == Severity::Breaking,
        FailOn::Deprecated => f.severity != Severity::Behavior,
        FailOn::Any | FailOn::New => true,
        FailOn::None => false,
    }
}

fn render_errors_text(errors: &[FileError], o: &Options) -> String {
    if errors.is_empty() {
        return String::new();
    }
    let mut out = match (o.lang, errors.len()) {
        (Lang::Tr, n) => format!("{n} dosya taranamadı; sonuçlar onları kapsamıyor:\n"),
        (Lang::En, 1) => "1 file couldn't be scanned, so the results don't cover it:\n".to_string(),
        (Lang::En, n) => format!("{n} files couldn't be scanned, so the results don't cover them:\n"),
    };
    for e in errors {
        out.push_str(&format!("  {}: {}\n", e.file, e.message));
    }
    out
}

fn render_text(findings: &[Finding], ignored: &[IgnoredFinding], summary: &report::Summary, o: &Options) -> String {
    let mut out = String::new();
    for f in findings {
        let fix = match f.fix {
            FixKind::Auto => "auto",
            FixKind::Likely => "review",
            FixKind::Manual => "manual",
        };
        out.push_str(&format!(
            "{}:{}:{}  {} [{}, PHP {}, {}] {}\n    {}\n",
            f.file,
            f.line,
            f.column,
            text(&f.title, o.lang),
            severity_name(f.severity),
            f.since,
            fix,
            f.rule_id,
            text(&f.message, o.lang)
        ));
    }
    let summary_line = match o.lang {
        Lang::Tr => format!(
            "PHP {}: {} bulgu ({} kritik, {} kullanımdan kalkmış, {} riskli), {} yok sayılan, hazırlık puanı {}/100\n",
            o.target,
            summary.total,
            summary.breaking,
            summary.deprecated,
            summary.behavior,
            ignored.len(),
            summary.score
        ),
        Lang::En => format!(
            "PHP {}: {} findings ({} breaking, {} deprecated, {} risky), {} ignored, readiness score {}/100\n",
            o.target,
            summary.total,
            summary.breaking,
            summary.deprecated,
            summary.behavior,
            ignored.len(),
            summary.score
        ),
    };
    if !findings.is_empty() {
        out.push('\n');
    }
    out.push_str(&summary_line);
    out
}

fn finding_json(f: &Finding, lang: Lang) -> Value {
    json!({
        "ruleId": f.rule_id,
        "file": f.file,
        "line": f.line,
        "column": f.column,
        "severity": severity_name(f.severity),
        "fix": f.fix,
        "since": f.since.to_string(),
        "title": text(&f.title, lang),
        "message": text(&f.message, lang),
        "replacement": f.replacement.as_ref().map(|r| text(r, lang)),
        "snippet": f.snippet,
        "runtimeHits": f.runtime_hits,
    })
}

fn render_json(
    findings: &[Finding],
    ignored: &[IgnoredFinding],
    summary: &report::Summary,
    scan: &analyzer::ScanResult,
    o: &Options,
) -> String {
    let value = json!({
        "target": o.target.to_string(),
        "complete": scan.errors.is_empty(),
        "filesFound": scan.files_found,
        "filesScanned": scan.files_scanned,
        "errors": scan.errors.iter().map(|e| json!({ "file": e.file, "message": e.message })).collect::<Vec<_>>(),
        "summary": summary,
        "findings": findings.iter().map(|f| finding_json(f, o.lang)).collect::<Vec<_>>(),
        "ignored": ignored.iter().map(|i| { let mut v = finding_json(&i.finding, o.lang); v["reason"] = json!(i.reason); v }).collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&value).unwrap() + "\n"
}

fn sarif_level(s: Severity) -> &'static str {
    match s {
        Severity::Breaking => "error",
        Severity::Deprecated => "warning",
        Severity::Behavior => "note",
    }
}

fn uri(path: &str) -> String {
    path.replace('\\', "/")
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn render_sarif(findings: &[Finding], errors: &[FileError], o: &Options) -> String {
    let mut rules: Vec<Value> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for f in findings {
        if seen.insert(f.rule_id.clone()) {
            rules.push(json!({
                "id": f.rule_id,
                "shortDescription": { "text": text(&f.title, o.lang) },
                "fullDescription": { "text": text(&f.message, o.lang) },
                "defaultConfiguration": { "level": sarif_level(f.severity) },
            }));
        }
    }
    let results: Vec<Value> = findings
        .iter()
        .map(|f| {
            let mut message = text(&f.message, o.lang).to_string();
            if let Some(r) = &f.replacement {
                message.push_str(&format!(" → {}", text(r, o.lang)));
            }
            json!({
                "ruleId": f.rule_id,
                "level": sarif_level(f.severity),
                "message": { "text": message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": uri(&f.file) },
                        "region": { "startLine": f.line, "startColumn": f.column },
                    }
                }],
                "properties": { "since": f.since.to_string(), "fix": f.fix },
            })
        })
        .collect();
    let value = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": {
                "name": "Ferry",
                "version": env!("CARGO_PKG_VERSION"),
                "informationUri": "https://github.com/efekoca/ferry",
                "rules": rules,
            }},
            "results": results,
            "invocations": [{
                "executionSuccessful": errors.is_empty(),
                "toolExecutionNotifications": errors.iter().map(|e| json!({
                    "level": "error",
                    "message": { "text": e.message },
                    "locations": [{ "physicalLocation": { "artifactLocation": { "uri": uri(&e.file) } } }],
                })).collect::<Vec<_>>(),
            }],
        }],
    });
    serde_json::to_string_pretty(&value).unwrap() + "\n"
}

fn escape_data(s: &str) -> String {
    s.replace('%', "%25").replace('\r', "%0D").replace('\n', "%0A")
}

fn escape_property(s: &str) -> String {
    escape_data(s).replace(':', "%3A").replace(',', "%2C")
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

fn render_junit(findings: &[Finding], errors: &[FileError], o: &Options) -> String {
    let mut by_file: std::collections::BTreeMap<&str, Vec<&Finding>> = std::collections::BTreeMap::new();
    for f in findings {
        by_file.entry(f.file.as_str()).or_default().push(f);
    }
    let failures = findings.iter().filter(|f| f.severity != Severity::Behavior).count();
    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuites name=\"Ferry PHP {}\" tests=\"{}\" failures=\"{failures}\" errors=\"{}\">\n",
        o.target,
        findings.len() + errors.len(),
        errors.len()
    );
    if !errors.is_empty() {
        out.push_str(&format!("  <testsuite name=\"scan\" tests=\"{0}\" failures=\"0\" errors=\"{0}\">\n", errors.len()));
        for e in errors {
            out.push_str(&format!(
                "    <testcase classname=\"scan\" name=\"{}\">\n      <error message=\"{}\"/>\n    </testcase>\n",
                xml(&e.file),
                xml(&e.message)
            ));
        }
        out.push_str("  </testsuite>\n");
    }
    for (file, list) in by_file {
        let failing = list.iter().filter(|f| f.severity != Severity::Behavior).count();
        out.push_str(&format!("  <testsuite name=\"{}\" tests=\"{}\" failures=\"{failing}\">\n", xml(file), list.len()));
        for f in list {
            out.push_str(&format!(
                "    <testcase classname=\"{}\" name=\"{}:{} {}\">\n",
                xml(&f.rule_id),
                xml(file),
                f.line,
                xml(text(&f.title, o.lang))
            ));
            let tag = if f.severity == Severity::Behavior { "system-out" } else { "failure" };
            let body = format!("{} (PHP {}, {})", text(&f.message, o.lang), f.since, severity_name(f.severity));
            if tag == "failure" {
                out.push_str(&format!(
                    "      <failure type=\"{}\" message=\"{}\">{}</failure>\n",
                    severity_name(f.severity),
                    xml(&body),
                    xml(&f.snippet)
                ));
            } else {
                out.push_str(&format!("      <system-out>{}</system-out>\n", xml(&body)));
            }
            out.push_str("    </testcase>\n");
        }
        out.push_str("  </testsuite>\n");
    }
    out + "</testsuites>\n"
}

fn render_checkstyle(findings: &[Finding], errors: &[FileError], o: &Options) -> String {
    let mut by_file: std::collections::BTreeMap<&str, Vec<&Finding>> = std::collections::BTreeMap::new();
    for f in findings {
        by_file.entry(f.file.as_str()).or_default().push(f);
    }
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<checkstyle version=\"4.3\">\n");
    for e in errors {
        out.push_str(&format!(
            "  <file name=\"{}\">\n    <error severity=\"error\" message=\"{}\" source=\"ferry.scan\"/>\n  </file>\n",
            xml(&e.file.replace('\\', "/")),
            xml(&e.message)
        ));
    }
    for (file, list) in by_file {
        out.push_str(&format!("  <file name=\"{}\">\n", xml(&file.replace('\\', "/"))));
        for f in list {
            let severity = match f.severity {
                Severity::Breaking => "error",
                Severity::Deprecated => "warning",
                Severity::Behavior => "info",
            };
            out.push_str(&format!(
                "    <error line=\"{}\" column=\"{}\" severity=\"{severity}\" message=\"{}\" source=\"ferry.{}\"/>\n",
                f.line,
                f.column,
                xml(&format!("{}: {}", text(&f.title, o.lang), text(&f.message, o.lang))),
                xml(&f.rule_id)
            ));
        }
        out.push_str("  </file>\n");
    }
    out + "</checkstyle>\n"
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn blame_authors(root: &std::path::Path, file: &str) -> std::collections::HashMap<usize, String> {
    let mut authors = std::collections::HashMap::new();
    let Some(git) = ferry_core::toolchain::find_executable("git") else { return authors };
    let Ok(out) = ferry_core::toolchain::command(git).arg("-C").arg(root).args(["blame", "--line-porcelain", "--", file]).output() else {
        return authors;
    };
    let mut line = 0;
    for l in String::from_utf8_lossy(&out.stdout).lines() {
        let mut parts = l.split(' ');
        if let (Some(hash), Some(_), Some(final_line)) = (parts.next(), parts.next(), parts.next()) {
            if hash.len() == 40 && hash.bytes().all(|b| b.is_ascii_hexdigit()) {
                line = final_line.parse().unwrap_or(0);
                continue;
            }
        }
        if let Some(author) = l.strip_prefix("author ") {
            authors.insert(line, author.to_string());
        }
    }
    authors
}

fn render_csv(root: &std::path::Path, findings: &[Finding], o: &Options) -> String {
    let mut out = String::from("file,line,column,rule,severity,fix,since,title,message,replacement");
    if o.blame {
        out.push_str(",author");
    }
    out.push('\n');
    let mut blames: std::collections::HashMap<&str, std::collections::HashMap<usize, String>> = std::collections::HashMap::new();
    for f in findings {
        let fix = match f.fix {
            FixKind::Auto => "auto",
            FixKind::Likely => "review",
            FixKind::Manual => "manual",
        };
        let fields = [
            f.file.clone(),
            f.line.to_string(),
            f.column.to_string(),
            f.rule_id.clone(),
            severity_name(f.severity).to_string(),
            fix.to_string(),
            f.since.to_string(),
            text(&f.title, o.lang).to_string(),
            text(&f.message, o.lang).to_string(),
            f.replacement.as_ref().map(|r| text(r, o.lang).to_string()).unwrap_or_default(),
        ];
        out.push_str(&fields.iter().map(|v| csv_field(v)).collect::<Vec<_>>().join(","));
        if o.blame {
            let authors = blames.entry(f.file.as_str()).or_insert_with(|| blame_authors(root, &f.file));
            out.push(',');
            out.push_str(&csv_field(authors.get(&f.line).map(String::as_str).unwrap_or("")));
        }
        out.push('\n');
    }
    out
}

fn render_github(findings: &[Finding], errors: &[FileError], o: &Options) -> String {
    let unscanned: String = errors
        .iter()
        .map(|e| format!("::error file={},title=Ferry::{}\n", escape_property(&e.file.replace('\\', "/")), escape_data(&e.message)))
        .collect();
    unscanned
        + &findings
            .iter()
            .map(|f| {
                let level = match f.severity {
                    Severity::Breaking => "error",
                    Severity::Deprecated => "warning",
                    Severity::Behavior => "notice",
                };
                format!(
                    "::{level} file={},line={},col={},title={}::{}\n",
                    escape_property(&f.file.replace('\\', "/")),
                    f.line,
                    f.column,
                    escape_property(&format!("{} (PHP {})", text(&f.title, o.lang), f.since)),
                    escape_data(text(&f.message, o.lang))
                )
            })
            .collect::<String>()
}

fn run(o: &Options) -> Result<Outcome, String> {
    ferry_core::i18n::set_lang(o.lang);
    let root = std::fs::canonicalize(&o.root).map_err(|e| format!("{}: {e}", o.root.display()))?;
    let info = project::inspect(&root).map_err(|e| e.to_string())?;
    let files = project::collect_php_files(&root, &info.excluded_dirs);
    let mut scan = analyzer::analyze_files(&root, &files, o.target, |_, _, _| {});
    let mut outside_fatals = 0;
    if let Some(path) = &o.runtime_log {
        let text = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let events = ferry_core::runtime::parse(&String::from_utf8_lossy(&text));
        let merged = ferry_core::runtime::merge(&root, &mut scan.findings, &events, o.target, &o.runtime_roots);
        eprintln!(
            "ferry-cli: runtime log: {} events, {} matched findings, {} new, {} outside the project",
            events.len(),
            merged.confirmed,
            merged.added,
            merged.outside
        );
        outside_fatals = merged.outside_fatals;
        if outside_fatals > 0 {
            eprintln!("ferry-cli: runtime log: {outside_fatals} fatal errors could not be tied to a project file");
        }
    }
    let applied = ignore::apply(&root, std::mem::take(&mut scan.findings), &ignore::load(&root));
    let (findings, ignored) = (applied.kept, applied.ignored);
    for stale in &applied.stale {
        eprintln!("ferry-cli: .ferry/ignore.json entry for {} in {} no longer matches any code", stale.rule_id, stale.file);
    }
    let summary = report::summarize(&findings);
    let baseline: Option<Vec<String>> = match &o.baseline {
        Some(path) => {
            let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let value: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            let list = value["fingerprints"].as_array().ok_or_else(|| format!("{}: not a ferry-cli baseline", path.display()))?;
            Some(list.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        }
        None => None,
    };
    let fresh: Vec<Finding> = match &baseline {
        Some(b) => history::new_findings(b, &findings).into_iter().cloned().collect(),
        None => findings.clone(),
    };
    if let Some(path) = &o.write_baseline {
        let value =
            json!({ "target": o.target.to_string(), "fingerprints": findings.iter().map(history::fingerprint).collect::<Vec<_>>() });
        std::fs::write(path, serde_json::to_string_pretty(&value).unwrap() + "\n").map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let mut rendered = match o.format {
        Format::Text => render_text(&findings, &ignored, &summary, o) + &render_errors_text(&scan.errors, o),
        Format::Json => render_json(&findings, &ignored, &summary, &scan, o),
        Format::Sarif => render_sarif(&findings, &scan.errors, o),
        Format::Github => render_github(&fresh, &scan.errors, o) + &render_text(&[], &ignored, &summary, o),
        Format::Junit => render_junit(&findings, &scan.errors, o),
        Format::Checkstyle => render_checkstyle(&findings, &scan.errors, o),
        Format::Csv => render_csv(&root, &findings, o),
    };
    if baseline.is_some() && matches!(o.format, Format::Text | Format::Github) {
        rendered.push_str(&match o.lang {
            Lang::Tr => format!("Baseline'a göre {} yeni bulgu\n", fresh.len()),
            Lang::En => format!("{} new since the baseline\n", fresh.len()),
        });
    }
    match &o.output {
        Some(path) => std::fs::write(path, rendered).map_err(|e| format!("{}: {e}", path.display()))?,
        None => print!("{rendered}"),
    }
    if !scan.errors.is_empty() && !matches!(o.format, Format::Text) {
        eprint!("ferry-cli: {}", render_errors_text(&scan.errors, o));
    }
    Ok(if fresh.iter().any(|f| fails(f, o.fail_on)) {
        Outcome::Failed
    } else if (!scan.errors.is_empty() || outside_fatals > 0) && !o.allow_incomplete {
        Outcome::Incomplete
    } else {
        Outcome::Passed
    })
}

fn run_compare(args: &[String]) -> Result<bool, String> {
    let (mut root, mut source, mut target, mut json_output) = (None, None, None, false);
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().cloned().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--target" => target = Some(value("--target")?.parse::<PhpVersion>().map_err(|e| e.to_string())?),
            "--source" => source = Some(value("--source")?.parse::<PhpVersion>().map_err(|e| e.to_string())?),
            "--format" => {
                json_output = match value("--format")?.as_str() {
                    "json" => true,
                    "text" => false,
                    other => return Err(format!("unknown format: {other}")),
                }
            }
            other if !other.starts_with('-') && root.is_none() => root = Some(PathBuf::from(other)),
            other => return Err(format!("unknown option: {other}")),
        }
    }
    let root = std::fs::canonicalize(root.ok_or("compare needs a project folder")?).map_err(|e| e.to_string())?;
    let target = target.ok_or("--target is required")?;
    let current = project::inspect(&root).map_err(|e| e.to_string())?.current_version;
    let source = source.or(current).ok_or("--source is required: the project's version couldn't be detected")?;
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let result = ferry_core::docker::compare(&root, source, target, &cancel, |step| match step {
        ferry_core::docker::Step::Installing => eprintln!("ferry-cli: installing dependencies"),
        ferry_core::docker::Step::Lint(v) => eprintln!("ferry-cli: linting on PHP {v}"),
        ferry_core::docker::Step::Tests(v) => eprintln!("ferry-cli: running tests on PHP {v}"),
    })?;
    let failed = |run: &ferry_core::docker::VersionRun| run.tests.as_ref().is_some_and(|t| !t.passed);
    let fatal = result.new_on_target.iter().any(|e| e.level == ferry_core::runtime::Level::Fatal);
    if json_output {
        println!("{}", serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?);
    } else {
        for run in [&result.source, &result.target] {
            let tests = run.tests.as_ref().map_or("no tests".to_string(), |t| {
                if t.passed {
                    "tests passed".into()
                } else {
                    format!("tests failed ({:?})", t.exit_code)
                }
            });
            println!("PHP {}: {} syntax errors, {tests}, {} messages", run.version, run.lint_errors, run.events.len());
            if !run.missing_extensions.is_empty() {
                println!("  missing in {}: {}", run.image, run.missing_extensions.join(", "));
            }
        }
        if !result.dependencies_installed {
            println!("Dependencies couldn't be installed, so tests may not have run.");
        }
        println!("{} new on PHP {target}, {} gone", result.new_on_target.len(), result.gone_on_target.len());
        for e in &result.new_on_target {
            println!("  {}:{} {}", e.path.trim_start_matches("/app/"), e.line, e.message);
        }
    }
    Ok(fatal || failed(&result.target) && !failed(&result.source))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "compare") {
        return match run_compare(&args[1..]) {
            Ok(false) => ExitCode::SUCCESS,
            Ok(true) => ExitCode::from(1),
            Err(e) => {
                eprintln!("ferry-cli: {e}");
                ExitCode::from(2)
            }
        };
    }
    let options = match parse(&args) {
        Ok(o) => o,
        Err(e) if e.is_empty() => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            eprintln!("ferry-cli: {e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(&options) {
        Ok(Outcome::Passed) => ExitCode::SUCCESS,
        Ok(Outcome::Failed) => ExitCode::from(1),
        Ok(Outcome::Incomplete) => ExitCode::from(3),
        Err(e) => {
            eprintln!("ferry-cli: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn parses_options() {
        let o = parse(&args("scan ./site --target 8.4 --format sarif --fail-on deprecated --lang tr")).unwrap();
        assert_eq!((o.format, o.fail_on, o.lang, o.target.to_string()), (Format::Sarif, FailOn::Deprecated, Lang::Tr, "8.4".to_string()));
        assert_eq!(parse(&args("scan ./site")).unwrap_err(), "missing --target");
        assert!(parse(&args("scan ./site --target 8.1")).unwrap_err().contains("PHP 8.1 is end of life"));
        assert!(parse(&args("scan ./site --target 7.4")).unwrap_err().contains("unsupported target 7.4"));
        assert_eq!(parse(&args("scan ./site --target 8.4 --format xml")).unwrap_err(), "unknown format: xml");
        assert_eq!(parse(&args("--help")).unwrap_err(), "");
        assert_eq!(parse(&args("scan . --target 8.4 --fail-on new")).unwrap_err(), "--fail-on new needs --baseline");
    }

    #[test]
    fn scans_respect_the_ignore_file_and_fail_level() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.php"), "<?php\neach($a);\n$x = \"${y}\";\n").unwrap();
        let out = dir.path().join("out.json");
        let options = |fail_on| Options {
            root: dir.path().to_path_buf(),
            target: PhpVersion::new(8, 2),
            format: Format::Json,
            output: Some(out.clone()),
            fail_on,
            lang: Lang::En,
            baseline: None,
            write_baseline: None,
            runtime_log: None,
            runtime_roots: Vec::new(),
            blame: false,
            allow_incomplete: false,
        };
        assert_eq!(run(&options(FailOn::Breaking)).unwrap(), Outcome::Failed);
        let report: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        let each = report["findings"].as_array().unwrap().iter().find(|f| f["ruleId"] == "removed.each").unwrap().clone();
        assert_eq!((each["file"].as_str(), each["line"].as_u64()), (Some("a.php"), Some(2)));

        std::fs::create_dir(dir.path().join(".ferry")).unwrap();
        let ignore = json!({ "ignores": [{ "ruleId": "removed.each", "file": "a.php", "snippet": "each($a);", "reason": "dead code" }] });
        std::fs::write(dir.path().join(".ferry/ignore.json"), ignore.to_string()).unwrap();
        assert_eq!(run(&options(FailOn::Breaking)).unwrap(), Outcome::Passed);
        assert_eq!(run(&options(FailOn::Deprecated)).unwrap(), Outcome::Failed);
        let report: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(report["ignored"][0]["reason"], "dead code");
        assert_eq!(report["summary"]["breaking"], 0);
    }

    #[test]
    fn baselines_only_fail_on_new_findings() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.php"), "<?php\neach($a);\n").unwrap();
        let base = dir.path().join("base.json");
        let options = |baseline: Option<PathBuf>, write: Option<PathBuf>| Options {
            root: dir.path().to_path_buf(),
            target: PhpVersion::new(8, 2),
            format: Format::Json,
            output: Some(dir.path().join("out.json")),
            fail_on: if baseline.is_some() { FailOn::New } else { FailOn::None },
            lang: Lang::En,
            baseline,
            write_baseline: write,
            runtime_log: None,
            runtime_roots: Vec::new(),
            blame: false,
            allow_incomplete: false,
        };
        assert_eq!(run(&options(None, Some(base.clone()))).unwrap(), Outcome::Passed);
        assert_eq!(run(&options(Some(base.clone()), None)).unwrap(), Outcome::Passed);
        std::fs::write(dir.path().join("a.php"), "<?php\neach($a);\nsplit($b);\n").unwrap();
        assert_eq!(run(&options(Some(base), None)).unwrap(), Outcome::Failed);
    }

    #[test]
    fn junit_checkstyle_and_csv_outputs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.php"), "<?php\neach($a);\n$x = \"${y}\";\n").unwrap();
        let git = |args: &[&str]| std::process::Command::new("git").arg("-C").arg(dir.path()).args(args).output();
        let has_git = git(&["init", "-q"]).is_ok_and(|o| o.status.success());
        if has_git {
            git(&["-c", "user.email=ada@example.com", "-c", "user.name=Ada Lovelace", "add", "."]).unwrap();
            git(&["-c", "user.email=ada@example.com", "-c", "user.name=Ada Lovelace", "commit", "-qm", "init"]).unwrap();
        }
        let out = dir.path().join("out.txt");
        let render = |format, blame| {
            let o = Options {
                root: dir.path().to_path_buf(),
                target: PhpVersion::new(8, 2),
                format,
                output: Some(out.clone()),
                fail_on: FailOn::None,
                lang: Lang::En,
                baseline: None,
                write_baseline: None,
                runtime_log: None,
                runtime_roots: Vec::new(),
                blame,
                allow_incomplete: false,
            };
            run(&o).unwrap();
            std::fs::read_to_string(&out).unwrap()
        };
        let junit = render(Format::Junit, false);
        assert!(junit.contains(r#"<testsuites name="Ferry PHP 8.2" tests="2" failures="2" errors="0">"#), "{junit}");
        assert!(junit.contains("<failure type=\"breaking\""));
        let checkstyle = render(Format::Checkstyle, false);
        assert!(checkstyle.contains(r#"<error line="2" column="1" severity="error""#), "{checkstyle}");
        assert!(checkstyle.contains(r#"source="ferry.removed.each""#));
        let csv = render(Format::Csv, true);
        let mut lines = csv.lines();
        assert_eq!(lines.next().unwrap(), "file,line,column,rule,severity,fix,since,title,message,replacement,author");
        let first = lines.next().unwrap();
        assert!(first.starts_with("a.php,2,1,removed.each,breaking,"), "{first}");
        if has_git {
            assert!(first.ends_with(",Ada Lovelace"), "{first}");
        }
        assert_eq!(csv_field("a,\"b\""), "\"a,\"\"b\"\"\"");
    }

    #[test]
    fn github_annotations_escape_their_fields() {
        assert_eq!(escape_property("a,b:c%"), "a%2Cb%3Ac%25");
        assert_eq!(escape_data("x\ny"), "x%0Ay");
    }

    #[test]
    fn an_incomplete_scan_is_not_a_pass() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.php"), format!("<?php each($a);\n//{}\n", "x".repeat(3 * 1024 * 1024))).unwrap();
        std::fs::write(dir.path().join("b.php"), "<?php echo 1;\n").unwrap();
        let out = dir.path().join("out.json");
        let base = format!("scan {} --target 8.4 --format json --output {}", dir.path().display(), out.display());
        assert_eq!(run(&parse(&args(&base)).unwrap()).unwrap(), Outcome::Incomplete);
        let report: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(
            (report["complete"].as_bool(), report["filesFound"].as_u64(), report["filesScanned"].as_u64()),
            (Some(false), Some(2), Some(1))
        );
        assert_eq!(report["errors"][0]["file"], "a.php");
        assert_eq!(run(&parse(&args(&format!("{base} --allow-incomplete"))).unwrap()).unwrap(), Outcome::Passed);

        let sarif = format!("scan {} --target 8.4 --format sarif --allow-incomplete --output {}", dir.path().display(), out.display());
        run(&parse(&args(&sarif)).unwrap()).unwrap();
        let report: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(report["runs"][0]["invocations"][0]["executionSuccessful"], false);
    }

    #[test]
    fn sarif_locations_are_valid_uris() {
        assert_eq!(uri("src\\my file#1.php"), "src/my%20file%231.php");
        assert_eq!(uri("şablon/100%.php"), "%C5%9Fablon/100%25.php");
    }
}
