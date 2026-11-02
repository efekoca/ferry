use crate::ext_map::{CLASS_EXTENSIONS, FUNCTION_EXTENSIONS, REMOVED_PREFIXES};
use crate::lexer::{lex, TokenKind};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionUse {
    pub name: String,
    pub uses: usize,
    pub file: String,
    pub line: usize,
    pub symbol: String,
    pub declared: bool,
    pub removed: bool,
    pub available: Option<bool>,
    pub required_by: Vec<String>,
    pub dev: bool,
}

pub fn normalize(name: &str) -> String {
    let n = name.trim().to_ascii_lowercase().replace(['-', ' '], "_");
    n.strip_prefix("zend_").map(str::to_string).unwrap_or(n)
}

fn requirements(root: &Path) -> Vec<(String, String, bool)> {
    let read = |file: &str| std::fs::read_to_string(root.join(file)).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
    let exts = |v: &serde_json::Value, key: &str| -> Vec<String> {
        v.get(key)
            .and_then(|o| o.as_object())
            .map(|o| o.keys().filter_map(|k| k.strip_prefix("ext-").map(normalize)).collect())
            .unwrap_or_default()
    };
    let mut out = Vec::new();
    if let Some(manifest) = read("composer.json") {
        out.extend(exts(&manifest, "require").into_iter().map(|e| (e, "composer.json".to_string(), false)));
        out.extend(exts(&manifest, "require-dev").into_iter().map(|e| (e, "composer.json (require-dev)".to_string(), true)));
    }
    if let Some(lock) = read("composer.lock") {
        for (key, dev) in [("packages", false), ("packages-dev", true)] {
            for pkg in lock.get(key).and_then(|v| v.as_array()).into_iter().flatten() {
                let name = pkg.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                out.extend(exts(pkg, "require").into_iter().map(|e| (e, name.to_string(), dev)));
            }
        }
    }
    out
}

fn lookup<'a>(table: &'a [(&'a str, &'a str)], name: &str) -> Option<&'a str> {
    table.binary_search_by(|(n, _)| (*n).cmp(name)).ok().map(|i| table[i].1)
}

fn extension_of_function(name: &str) -> Option<(&'static str, bool)> {
    if let Some(e) = lookup(FUNCTION_EXTENSIONS, name) {
        return Some((e, false));
    }
    REMOVED_PREFIXES.iter().find(|(p, _)| name.starts_with(p)).map(|&(_, e)| (e, true))
}

pub fn used(root: &Path, files: &[PathBuf]) -> Vec<ExtensionUse> {
    let required = requirements(root);
    let declared: Vec<String> = required.iter().filter(|(_, by, _)| by.starts_with("composer.json")).map(|(e, ..)| e.clone()).collect();
    let mut found: BTreeMap<String, ExtensionUse> = BTreeMap::new();
    for path in files {
        if std::fs::metadata(path).is_ok_and(|m| m.len() > crate::project::MAX_FILE_BYTES) {
            continue;
        }
        let Ok(bytes) = std::fs::read(path) else { continue };
        let tokens = lex(&String::from_utf8_lossy(&bytes)).tokens;
        let rel = crate::analyzer::relative_path(root, path);
        for (k, t) in tokens.iter().enumerate() {
            if t.kind != TokenKind::Ident {
                continue;
            }
            let name = t.text.trim_start_matches('\\');
            if name.contains('\\') {
                continue;
            }
            let lower = name.to_ascii_lowercase();
            let prev = k.checked_sub(1).map(|p| &tokens[p]);
            let next = tokens.get(k + 1);
            if prev.is_some_and(|p| p.is_punct("->") || p.is_punct("?->") || p.is_punct("::") || p.is_ident_ci("const")) {
                continue;
            }
            let hit = if next.is_some_and(|n| n.is_punct("(")) && !prev.is_some_and(|p| p.is_ident_ci("function") || p.is_ident_ci("new")) {
                extension_of_function(&lower)
            } else if prev.is_some_and(|p| p.is_ident_ci("new") || p.is_ident_ci("instanceof") || p.is_ident_ci("extends"))
                || next.is_some_and(|n| n.is_punct("::"))
            {
                lookup(CLASS_EXTENSIONS, &lower).map(|e| (e, false))
            } else {
                None
            };
            let Some((ext, removed)) = hit else { continue };
            found.entry(ext.to_string()).and_modify(|u| u.uses += 1).or_insert_with(|| ExtensionUse {
                name: ext.to_string(),
                uses: 1,
                file: rel.clone(),
                line: t.line,
                symbol: if next.is_some_and(|n| n.is_punct("(")) { format!("{name}()") } else { name.to_string() },
                declared: declared.iter().any(|d| d == ext),
                removed,
                available: None,
                required_by: Vec::new(),
                dev: false,
            });
        }
    }
    for (ext, by, dev) in required {
        let entry = found.entry(ext.clone()).or_insert_with(|| ExtensionUse {
            name: ext.clone(),
            uses: 0,
            file: if by.starts_with("composer.json") { "composer.json".into() } else { "composer.lock".into() },
            line: 0,
            symbol: format!("ext-{ext}"),
            declared: declared.contains(&ext),
            removed: false,
            available: None,
            required_by: Vec::new(),
            dev: true,
        });
        entry.dev &= dev;
        if !entry.required_by.contains(&by) {
            entry.required_by.push(by);
        }
    }
    let mut out: Vec<ExtensionUse> = found.into_values().collect();
    out.sort_by(|a, b| b.uses.cmp(&a.uses).then_with(|| a.name.cmp(&b.name)));
    out
}

pub fn parse_modules(php_m: &str) -> Vec<String> {
    php_m
        .lines()
        .map(str::trim)
        .take_while(|l| !l.eq_ignore_ascii_case("[Zend Modules]"))
        .filter(|l| !l.is_empty() && !l.starts_with('['))
        .map(normalize)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_extension_uses_and_declarations() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("composer.json"), r#"{"require":{"php":">=7.4","ext-curl":"*"}}"#).unwrap();
        std::fs::write(
            dir.path().join("a.php"),
            "<?php\n$ch = curl_init($u);\ncurl_exec($ch);\n$f = new NumberFormatter('tr', 1);\n$m = imap_open($a, $b, $c);\n$x->curl_init();\nfunction mb_strlen($s) {}\nstrlen($s);\n",
        )
        .unwrap();
        let files = vec![dir.path().join("a.php")];
        let used = used(dir.path(), &files);
        let summary: Vec<_> = used.iter().map(|u| (u.name.as_str(), u.uses, u.declared, u.removed, u.line)).collect();
        assert_eq!(summary, [("curl", 2, true, false, 2), ("imap", 1, false, true, 5), ("intl", 1, false, false, 4)]);
        assert!(used.iter().all(|u| u.required_by.is_empty() || u.name == "curl"));

        std::fs::write(
            dir.path().join("composer.json"),
            r#"{"require":{"ext-intl":"*","ext-Zend-OPcache":"*"},"require-dev":{"ext-xdebug":"*"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("composer.lock"),
            r#"{"packages":[{"name":"acme/pdf","require":{"ext-gd":"*"}}],"packages-dev":[{"name":"acme/test","require":{"ext-xdebug":"*"}}]}"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("a.php"), "<?php echo 1;\n").unwrap();
        let used = super::used(dir.path(), &files);
        let summary: Vec<_> = used.iter().map(|u| (u.name.as_str(), u.uses, u.dev, u.required_by.join(","))).collect();
        assert_eq!(
            summary,
            [
                ("gd", 0, false, "acme/pdf".to_string()),
                ("intl", 0, false, "composer.json".to_string()),
                ("opcache", 0, false, "composer.json".to_string()),
                ("xdebug", 0, true, "composer.json (require-dev),acme/test".to_string()),
            ]
        );
        assert_eq!(normalize("Zend OPcache"), "opcache");
        assert_eq!(parse_modules("[PHP Modules]\nCore\ncurl\nIntl\n\n[Zend Modules]\nZend OPcache\n"), ["core", "curl", "intl"]);
    }
}
