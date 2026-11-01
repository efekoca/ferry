pub fn token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    (0..2)
        .map(|i| {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u128(nanos ^ i);
            format!("{:016x}", h.finish())
        })
        .collect()
}

pub fn script(token: &str) -> String {
    format!(
        r#"<?php
// Ferry server probe. Delete this file after use.
if (!isset($_GET['token']) || (string) $_GET['token'] !== '{token}') {{
    header('HTTP/1.1 404 Not Found');
    exit;
}}
header('Content-Type: application/json');
header('Cache-Control: no-store');
$keys = array('memory_limit', 'max_execution_time', 'upload_max_filesize', 'post_max_size', 'display_errors', 'error_reporting', 'disable_functions', 'disable_classes', 'opcache.enable', 'date.timezone', 'allow_url_fopen', 'session.save_handler');
$ini = array();
foreach ($keys as $key) {{
    $ini[$key] = ini_get($key);
}}
$data = array(
    'ferryProbe' => 1,
    'php' => PHP_VERSION,
    'sapi' => PHP_SAPI,
    'os' => PHP_OS,
    'server' => isset($_SERVER['SERVER_SOFTWARE']) ? $_SERVER['SERVER_SOFTWARE'] : null,
    'extensions' => array_map('strtolower', get_loaded_extensions()),
    'ini' => $ini,
);
echo json_encode($data, defined('JSON_PRETTY_PRINT') ? JSON_PRETTY_PRINT : 0);
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_answers_only_with_the_token() {
        let t = token();
        assert_eq!(t.len(), 32);
        assert_ne!(t, token());
        let Some(php) = crate::toolchain::find_php(None) else { return };
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("ferry-probe.php");
        std::fs::write(&file, script(&t)).unwrap();
        let run = |query: &str| {
            let code = format!("parse_str('{query}', $_GET); include '{}';", file.display());
            String::from_utf8_lossy(&crate::toolchain::command(&php).args(["-r", &code]).output().unwrap().stdout).to_string()
        };
        assert_eq!(run("token=wrong"), "");
        let json: serde_json::Value = serde_json::from_str(&run(&format!("token={t}"))).unwrap();
        assert_eq!(json["ferryProbe"], 1);
        assert!(json["extensions"].as_array().unwrap().iter().any(|e| e == "core"));
        assert!(json["ini"]["memory_limit"].is_string());
    }
}
