use std::fs;
use std::path::Path;

fn sources(dir: &Path, out: &mut String) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push_str(&fs::read_to_string(&path).unwrap());
        }
    }
}

#[test]
fn coverage_matrix_names_real_rules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let data = fs::read_to_string(root.join("../../docs/coverage.json")).unwrap();
    let items: Vec<serde_json::Value> = serde_json::from_str(&data).unwrap();
    let mut code = String::new();
    sources(&root.join("src"), &mut code);

    let mut problems = Vec::new();
    for item in &items {
        let status = item["status"].as_str().unwrap_or_default();
        let rules: Vec<&str> = item["rules"].as_array().map(|r| r.iter().filter_map(|r| r.as_str()).collect()).unwrap_or_default();
        let change = item["change"].as_str().unwrap_or_default();
        match status {
            "covered" | "partial" if rules.is_empty() => problems.push(format!("{status} without rules: {change}")),
            "covered" | "partial" | "gap" | "runtime" | "n/a" => {}
            _ => problems.push(format!("unknown status {status:?}: {change}")),
        }
        for rule in rules {
            if !code.contains(&format!("\"{rule}\"")) {
                problems.push(format!("unknown rule {rule}: {change}"));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));

    let page = fs::read_to_string(root.join("../../docs/COVERAGE.md")).unwrap();
    let total = format!("**{}** |", items.len());
    assert!(page.contains(&total), "docs/COVERAGE.md is stale; run node scripts/php-upgrading/render.mjs");
}
