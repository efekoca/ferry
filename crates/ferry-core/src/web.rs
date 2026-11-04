use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

const PORT: u16 = 8080;
const PAGE_LIMIT: usize = 60;
pub const LOG_MARKER: &str = "FERRY-SERVER-LOG";

const CRAWLER: &str = r#"<?php
$queue = array_slice($argv, 2);
$limit = (int) $argv[1];
$seen = array();
$context = stream_context_create(array('http' => array('ignore_errors' => true, 'follow_location' => 0, 'timeout' => 20)));
$fetch = function ($path) use ($context) {
    $body = @file_get_contents('http://127.0.0.1:8080' . $path, false, $context);
    $head = isset($http_response_header) ? $http_response_header : array();
    $status = 0;
    $location = '';
    foreach ($head as $line) {
        if (preg_match('#^HTTP/\S+ (\d+)#', $line, $m)) { $status = (int) $m[1]; }
        if (stripos($line, 'Location:') === 0) { $location = trim(substr($line, 9)); }
    }
    return array($status, $location, (string) $body);
};
while ($queue && count($seen) < $limit) {
    $path = array_shift($queue);
    if (isset($seen[$path])) { continue; }
    $seen[$path] = true;
    list($status, $location, $body) = $fetch($path);
    list(, , $again) = $fetch($path);
    preg_match_all('/(?:Fatal error|Parse error|Uncaught|Warning|Deprecated|Notice):? [^<\n]{0,200}/', $body, $errors);
    $text = trim(preg_replace('/\s+/', ' ', strip_tags($body)));
    echo 'FERRYPAGE ' . json_encode(array(
        'path' => $path,
        'status' => $status,
        'location' => $location,
        'stable' => $body === $again,
        'hash' => md5($body),
        'errors' => array_values(array_unique(array_map('utf8_clean', $errors[0]))),
        'text' => utf8_clean(substr($text, 0, 600)),
    )) . "\n";
    preg_match_all('/href=["\']([^"\'#?]+)/i', $body, $links);
    foreach ($links[1] as $link) {
        $link = preg_replace('#^https?://127\.0\.0\.1:8080#', '', $link);
        if ($link === '') { continue; }
        $unsafe = preg_match('#logout|logoff|signout|delete|remove|destroy|_profiler|_wdt|_debugbar|telescope|horizon|\.(css|js|png|jpe?g|gif|svg|ico|woff2?|pdf|zip)$#i', $link);
        if ($link[0] === '/' && substr($link, 0, 2) !== '//' && !$unsafe) { $queue[] = $link; }
    }
}
function utf8_clean($text) {
    return (string) @iconv('UTF-8', 'UTF-8//IGNORE', $text);
}
"#;

#[derive(Debug, Clone, Deserialize)]
struct Fetched {
    path: String,
    status: u16,
    location: String,
    stable: bool,
    hash: String,
    errors: Vec<String>,
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PageChange {
    Same,
    Status,
    Redirect,
    Errors,
    Body,
    /// The page changes between two requests on the source, so its body can't be compared.
    Unstable,
    /// The page fails on both versions, so nothing can be said about the target.
    Unverified,
    Missing,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDiff {
    pub path: String,
    pub change: PageChange,
    pub source_status: u16,
    pub target_status: u16,
    pub new_errors: Vec<String>,
    pub source_text: String,
    pub target_text: String,
}

impl PageDiff {
    /// Whether a page that worked on the source breaks on the target.
    pub fn broke(&self) -> bool {
        !failed(self.source_status) && (failed(self.target_status) || fatal(&self.new_errors))
    }
}

#[derive(Debug, Clone, Default)]
pub struct Crawl {
    pages: Vec<Fetched>,
}

pub fn document_root(project: &Path) -> &'static str {
    ["public", "web", "webroot", "htdocs", "pub"].into_iter().find(|d| project.join(d).join("index.php").is_file()).unwrap_or(".")
}

fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

/// Serves the project inside the container and requests its pages with GET only.
pub fn script(project: &Path, paths: &[String]) -> String {
    let root = document_root(project);
    let list: Vec<String> = paths.iter().map(|p| quoted(p)).collect();
    format!(
        "cat > /tmp/crawl.php <<'FERRYEOF'\n{CRAWLER}\nFERRYEOF\n\
         php -d display_errors=1 -d error_reporting=-1 -S 127.0.0.1:{PORT} -t {root} > /tmp/server.log 2>&1 &\n\
         sleep 1\nphp /tmp/crawl.php {PAGE_LIMIT} {}\necho {LOG_MARKER}\ncat /tmp/server.log\n",
        list.join(" ")
    )
}

pub fn parse(output: &str) -> Crawl {
    let pages = output.lines().filter_map(|l| l.strip_prefix("FERRYPAGE ")).filter_map(|l| serde_json::from_str(l).ok()).collect();
    Crawl { pages }
}

pub fn server_log(output: &str) -> &str {
    output.split_once(LOG_MARKER).map_or("", |(_, log)| log)
}

fn failed(status: u16) -> bool {
    status == 0 || status >= 500
}

fn fatal(errors: &[String]) -> bool {
    errors.iter().any(|e| e.contains("Fatal error") || e.contains("Uncaught") || e.contains("Parse error"))
}

fn classify(source: &Fetched, target: &Fetched, new_errors: &[String]) -> PageChange {
    let broken = |page: &Fetched| failed(page.status) || fatal(&page.errors);
    if broken(source) && broken(target) {
        PageChange::Unverified
    } else if source.status != target.status {
        PageChange::Status
    } else if source.location != target.location {
        PageChange::Redirect
    } else if !new_errors.is_empty() {
        PageChange::Errors
    } else if !source.stable {
        PageChange::Unstable
    } else if source.hash != target.hash {
        PageChange::Body
    } else {
        PageChange::Same
    }
}

pub fn compare(source: &Crawl, target: &Crawl) -> Vec<PageDiff> {
    let by_path: HashMap<&str, &Fetched> = target.pages.iter().map(|p| (p.path.as_str(), p)).collect();
    let mut diffs: Vec<PageDiff> = source
        .pages
        .iter()
        .map(|s| match by_path.get(s.path.as_str()) {
            Some(t) => {
                let new_errors: Vec<String> = t.errors.iter().filter(|e| !s.errors.contains(e)).cloned().collect();
                PageDiff {
                    path: s.path.clone(),
                    change: classify(s, t, &new_errors),
                    source_status: s.status,
                    target_status: t.status,
                    new_errors,
                    source_text: s.text.clone(),
                    target_text: t.text.clone(),
                }
            }
            None => PageDiff {
                path: s.path.clone(),
                change: PageChange::Missing,
                source_status: s.status,
                target_status: 0,
                new_errors: Vec::new(),
                source_text: s.text.clone(),
                target_text: String::new(),
            },
        })
        .collect();
    diffs.sort_by_key(|d| d.change == PageChange::Same);
    diffs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(path: &str, status: u16, hash: &str, errors: &[&str]) -> Fetched {
        Fetched {
            path: path.into(),
            status,
            location: String::new(),
            stable: true,
            hash: hash.into(),
            errors: errors.iter().map(|e| e.to_string()).collect(),
            text: String::new(),
        }
    }

    fn change(source: Fetched, target: Fetched) -> PageChange {
        compare(&Crawl { pages: vec![source] }, &Crawl { pages: vec![target] })[0].change
    }

    #[test]
    fn pages_are_classified_without_false_passes() {
        assert_eq!(change(page("/", 200, "a", &[]), page("/", 200, "a", &[])), PageChange::Same);
        assert_eq!(change(page("/", 200, "a", &[]), page("/", 500, "b", &[])), PageChange::Status);
        assert_eq!(change(page("/", 200, "a", &[]), page("/", 200, "a", &["Deprecated: x"])), PageChange::Errors);
        assert_eq!(change(page("/", 200, "a", &[]), page("/", 200, "b", &[])), PageChange::Body);
        assert_eq!(change(page("/", 500, "a", &[]), page("/", 500, "a", &[])), PageChange::Unverified);
        let fatal = ["Uncaught PDOException: could not find driver"];
        assert_eq!(change(page("/", 200, "a", &fatal), page("/", 200, "a", &fatal)), PageChange::Unverified);
        let moving = Fetched { stable: false, ..page("/", 200, "a", &[]) };
        assert_eq!(change(moving.clone(), page("/", 200, "b", &[])), PageChange::Unstable);
    }

    #[test]
    fn a_page_missing_on_the_target_is_reported() {
        let diffs = compare(&Crawl { pages: vec![page("/a", 200, "a", &[])] }, &Crawl::default());
        assert_eq!(diffs[0].change, PageChange::Missing);
    }

    #[test]
    fn crawl_output_is_separated_from_the_server_log() {
        let out = "noise\nFERRYPAGE {\"path\":\"/\",\"status\":200,\"location\":\"\",\"stable\":true,\"hash\":\"h\",\"errors\":[],\"text\":\"t\"}\nFERRY-SERVER-LOG\nPHP Warning: x in /app/a.php on line 1\n";
        assert_eq!(parse(out).pages.len(), 1);
        assert!(server_log(out).contains("PHP Warning"));
    }

    #[test]
    fn the_document_root_follows_the_framework_layout() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(document_root(dir.path()), ".");
        std::fs::create_dir(dir.path().join("public")).unwrap();
        std::fs::write(dir.path().join("public/index.php"), "<?php").unwrap();
        assert_eq!(document_root(dir.path()), "public");
    }
}
