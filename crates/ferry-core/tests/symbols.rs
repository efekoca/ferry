use ferry_core::analyzer::analyze_source;
use ferry_core::symbols::*;
use ferry_core::PhpVersion;

fn hit(src: &str) -> bool {
    !analyze_source("a.php", src, PhpVersion::new(8, 5)).is_empty()
}

#[test]
fn every_generated_symbol_is_reported() {
    let mut missing: Vec<String> = Vec::new();
    for (name, since, _) in REMOVED_FUNCTIONS.iter().chain(DEPRECATED_FUNCTIONS) {
        if !hit(&format!("<?php\n{name}($a);\n")) {
            missing.push(format!("fn {name} {}.{}", since.0, since.1));
        }
    }
    for (name, since, _) in REMOVED_CONSTANTS.iter().chain(DEPRECATED_CONSTANTS) {
        if !hit(&format!("<?php\necho {name};\n")) {
            missing.push(format!("const {name} {}.{}", since.0, since.1));
        }
    }
    for (_, since, _, class) in REMOVED_CLASSES {
        if !hit(&format!("<?php\n$x = new {class}();\n")) {
            missing.push(format!("class {class} {}.{}", since.0, since.1));
        }
    }
    for (key, since, class) in REMOVED_METHODS.iter().chain(DEPRECATED_METHODS) {
        let method = key.split("::").nth(1).unwrap();
        if !hit(&format!("<?php\nfunction f({class} $o) {{ return $o->{method}(); }}\n")) && !hit(&format!("<?php\n{class}::{method}();\n"))
        {
            missing.push(format!("method {class}::{method} {}.{}", since.0, since.1));
        }
    }
    assert!(missing.is_empty(), "{} symbols are not reported:\n{}", missing.len(), missing.join("\n"));
}
