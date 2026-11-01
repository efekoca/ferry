use ferry_core::analyzer::{self, Finding};
use ferry_core::i18n::{Lang, Localized};
use ferry_core::ignore::{self, IgnoredFinding};
use ferry_core::rules::{FixKind, Severity};
use ferry_core::{project, report, PhpVersion};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "Usage: ferry-cli scan <project> --target <8.2|8.3|8.4|8.5> [options]

Options:
  --format <text|json|sarif|github>  Output format (default: text)
  --output <file>                    Write the report to a file instead of stdout
  --fail-on <breaking|deprecated|any|none>
                                     Exit with code 1 when findings at this level remain (default: breaking)
  --lang <en|tr>                     Language of messages (default: en)
  -h, --help                         Show this help";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Text,
    Json,
    Sarif,
    Github,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailOn {
    Breaking,
    Deprecated,
    Any,
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
                    other => return Err(format!("unknown format: {other}")),
                }
            }
            "--output" => output = Some(PathBuf::from(value("--output")?)),
            "--fail-on" => {
                fail_on = match value("--fail-on")?.as_str() {
                    "breaking" => FailOn::Breaking,
                    "deprecated" => FailOn::Deprecated,
                    "any" => FailOn::Any,
                    "none" => FailOn::None,
                    other => return Err(format!("unknown --fail-on level: {other}")),
                }
            }
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
    Ok(Options { root, target, format, output, fail_on, lang })
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
        FailOn::Any => true,
        FailOn::None => false,
    }
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
    })
}

fn render_json(findings: &[Finding], ignored: &[IgnoredFinding], summary: &report::Summary, files: usize, o: &Options) -> String {
    let value = json!({
        "target": o.target.to_string(),
        "filesScanned": files,
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

fn render_sarif(findings: &[Finding], o: &Options) -> String {
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
                        "artifactLocation": { "uri": f.file.replace('\\', "/") },
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

fn render_github(findings: &[Finding], o: &Options) -> String {
    findings
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
        .collect()
}

fn run(o: &Options) -> Result<bool, String> {
    ferry_core::i18n::set_lang(o.lang);
    let root = std::fs::canonicalize(&o.root).map_err(|e| format!("{}: {e}", o.root.display()))?;
    let info = project::inspect(&root).map_err(|e| e.to_string())?;
    let files = project::collect_php_files(&root, &info.excluded_dirs);
    let scan = analyzer::analyze_files(&root, &files, o.target, |_, _, _| {});
    let (findings, ignored) = ignore::split(scan.findings, &ignore::load(&root));
    let summary = report::summarize(&findings);
    let rendered = match o.format {
        Format::Text => render_text(&findings, &ignored, &summary, o),
        Format::Json => render_json(&findings, &ignored, &summary, scan.files_scanned, o),
        Format::Sarif => render_sarif(&findings, o),
        Format::Github => render_github(&findings, o) + &render_text(&[], &ignored, &summary, o),
    };
    match &o.output {
        Some(path) => std::fs::write(path, rendered).map_err(|e| format!("{}: {e}", path.display()))?,
        None => print!("{rendered}"),
    }
    Ok(findings.iter().any(|f| fails(f, o.fail_on)))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
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
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::from(1),
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
        };
        assert!(run(&options(FailOn::Breaking)).unwrap());
        let report: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        let each = report["findings"].as_array().unwrap().iter().find(|f| f["ruleId"] == "removed.each").unwrap().clone();
        assert_eq!((each["file"].as_str(), each["line"].as_u64()), (Some("a.php"), Some(2)));

        std::fs::create_dir(dir.path().join(".ferry")).unwrap();
        let ignore = json!({ "ignores": [{ "ruleId": "removed.each", "file": "a.php", "snippet": "each($a);", "reason": "dead code" }] });
        std::fs::write(dir.path().join(".ferry/ignore.json"), ignore.to_string()).unwrap();
        assert!(!run(&options(FailOn::Breaking)).unwrap());
        assert!(run(&options(FailOn::Deprecated)).unwrap());
        let report: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(report["ignored"][0]["reason"], "dead code");
        assert_eq!(report["summary"]["breaking"], 0);
    }

    #[test]
    fn github_annotations_escape_their_fields() {
        assert_eq!(escape_property("a,b:c%"), "a%2Cb%3Ac%25");
        assert_eq!(escape_data("x\ny"), "x%0Ay");
    }
}
