use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    pub kind: String,
    pub slug: String,
    pub name: String,
    pub version: Option<String>,
    pub requires_php: Option<String>,
    pub dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Latest {
    pub kind: String,
    pub slug: String,
    pub version: Option<String>,
    pub requires_php: Option<String>,
    pub last_updated: Option<String>,
    pub closed: bool,
}

fn header(text: &str, key: &str) -> Option<String> {
    text.lines().take(80).find_map(|l| {
        let l = l.trim_start_matches(|c: char| c == '*' || c == '#' || c == '/' || c.is_whitespace());
        let rest = l.strip_prefix(key)?.trim_start();
        let value = rest.strip_prefix(':')?.trim().trim_end_matches("*/").trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn head(path: &Path) -> Option<String> {
    use std::io::Read;
    let mut buf = vec![0; 8192];
    let n = std::fs::File::open(path).ok()?.read(&mut buf).ok()?;
    Some(String::from_utf8_lossy(&buf[..n]).into_owned())
}

pub fn inventory(root: &Path) -> Vec<Component> {
    let content = root.join("wp-content");
    let mut out = Vec::new();
    let entries = |dir: &Path| -> Vec<std::path::PathBuf> {
        let mut v: Vec<_> = std::fs::read_dir(dir).map(|r| r.flatten().map(|e| e.path()).collect()).unwrap_or_default();
        v.sort();
        v
    };
    for path in entries(&content.join("plugins")) {
        let slug_of = |p: &Path| p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let candidates: Vec<std::path::PathBuf> = if path.is_dir() {
            let mut files = entries(&path);
            files.retain(|f| f.extension().is_some_and(|e| e == "php"));
            files
        } else if path.extension().is_some_and(|e| e == "php") {
            vec![path.clone()]
        } else {
            Vec::new()
        };
        let Some((file, text)) = candidates.iter().find_map(|f| head(f).filter(|t| header(t, "Plugin Name").is_some()).map(|t| (f, t)))
        else {
            continue;
        };
        let slug = if path.is_dir() { path.file_name().unwrap().to_string_lossy().to_string() } else { slug_of(file) };
        out.push(Component {
            kind: "plugin".into(),
            name: header(&text, "Plugin Name").unwrap_or_else(|| slug.clone()),
            version: header(&text, "Version"),
            requires_php: header(&text, "Requires PHP"),
            dir: crate::analyzer::relative_path(root, &path),
            slug,
        });
    }
    for path in entries(&content.join("themes")) {
        let Some(text) = head(&path.join("style.css")).filter(|t| header(t, "Theme Name").is_some()) else { continue };
        let slug = path.file_name().unwrap().to_string_lossy().to_string();
        out.push(Component {
            kind: "theme".into(),
            name: header(&text, "Theme Name").unwrap_or_else(|| slug.clone()),
            version: header(&text, "Version"),
            requires_php: header(&text, "Requires PHP"),
            dir: crate::analyzer::relative_path(root, &path),
            slug,
        });
    }
    out
}

pub fn api_url(kind: &str, slug: &str) -> Option<String> {
    let valid = !slug.is_empty() && slug.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_');
    let (path, action) = match kind {
        "plugin" => ("plugins", "plugin_information"),
        "theme" => ("themes", "theme_information"),
        _ => return None,
    };
    valid.then(|| {
        format!("https://api.wordpress.org/{path}/info/1.2/?action={action}&request%5Bslug%5D={slug}&request%5Bfields%5D%5Bsections%5D=0")
    })
}

pub fn parse_latest(kind: &str, slug: &str, json: &str) -> Option<Latest> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let text = |k: &str| v.get(k).and_then(|x| x.as_str()).filter(|s| !s.is_empty()).map(str::to_string);
    let closed = v.get("error").and_then(|e| e.as_str()) == Some("closed") || v.get("closed").and_then(|c| c.as_bool()) == Some(true);
    if v.get("error").is_some() && !closed {
        return None;
    }
    Some(Latest {
        kind: kind.into(),
        slug: slug.into(),
        version: text("version"),
        requires_php: text("requires_php"),
        last_updated: text("last_updated"),
        closed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_plugin_and_theme_headers() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let write = |p: &str, t: &str| {
            let p = root.join(p);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, t).unwrap();
        };
        write("wp-content/plugins/akismet/class.php", "<?php\nclass A {}\n");
        write(
            "wp-content/plugins/akismet/akismet.php",
            "<?php\n/*\n * Plugin Name: Akismet Anti-spam\n * Version: 4.1.3\n * Requires PHP: 5.6\n */\n",
        );
        write("wp-content/plugins/hello.php", "<?php\n/*\nPlugin Name: Hello Dolly\nVersion: 1.7.2\n*/\n");
        write("wp-content/plugins/index.php", "<?php\n// Silence is golden.\n");
        write("wp-content/themes/twentytwenty/style.css", "/*\nTheme Name: Twenty Twenty\nVersion: 1.2\nRequires PHP: 5.2.4\n*/\n");
        let found: Vec<(String, String, Option<String>, Option<String>)> =
            inventory(root).into_iter().map(|c| (c.kind, c.slug, c.version, c.requires_php)).collect();
        assert_eq!(
            found,
            [
                ("plugin".into(), "akismet".into(), Some("4.1.3".into()), Some("5.6".into())),
                ("plugin".into(), "hello".into(), Some("1.7.2".into()), None),
                ("theme".into(), "twentytwenty".into(), Some("1.2".into()), Some("5.2.4".into())),
            ]
        );
        let latest =
            parse_latest("plugin", "akismet", r#"{"slug":"akismet","version":"5.5","requires_php":"7.2","last_updated":"2026-10-01"}"#)
                .unwrap();
        assert_eq!((latest.version.as_deref(), latest.closed), (Some("5.5"), false));
        assert!(parse_latest("plugin", "x", r#"{"error":"Plugin not found."}"#).is_none());
        assert!(parse_latest("plugin", "x", r#"{"error":"closed","name":"X"}"#).unwrap().closed);
        assert_eq!(api_url("plugin", "../x"), None);
    }
}
