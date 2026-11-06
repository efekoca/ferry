use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

const PORT: u16 = 8080;
const PAGE_LIMIT: usize = 60;
const EXCERPT: usize = 200;
pub const LOG_MARKER: &str = "FERRY-SERVER-LOG";

/// PHP's built-in server treats paths with a dot differently between versions, so one script decides.
const ROUTER: &str = r#"<?php
$path = parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH);
$file = $_SERVER['DOCUMENT_ROOT'] . $path;
if ($path !== '/' && (is_file($file) || is_file(rtrim($file, '/') . '/index.php'))) {
    return false;
}
if (!is_file($_SERVER['DOCUMENT_ROOT'] . '/index.php')) {
    http_response_code(404);
    return true;
}
$_SERVER['SCRIPT_NAME'] = '/index.php';
$_SERVER['SCRIPT_FILENAME'] = $_SERVER['DOCUMENT_ROOT'] . '/index.php';
chdir($_SERVER['DOCUMENT_ROOT']);
require 'index.php';
"#;

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
    $plain = function ($html) { return preg_replace('/\b(?=[a-f]*\d)(?=\d*[a-f])[a-f0-9]{6,}\b|\d+(?:\.\d+)? ?(?:ms|MiB)|\d{1,2}[:.]\d{2}[:.]\d{2}(?: ?[AP]M)?/i', '', $html); };
    preg_match_all('/(?:Fatal error|Parse error|Uncaught|Warning|Deprecated|Notice):? [^<\n]{0,200}/', $body, $errors);
    $raw = trim(preg_replace('/\s+/', ' ', $plain($body)));
    echo 'FERRYPAGE ' . json_encode(array(
        'path' => $path,
        'status' => $status,
        'location' => $location,
        'stable' => $plain($body) === $plain($again),
        'hash' => md5($plain($body)),
        'errors' => array_values(array_unique(array_map('utf8_clean', $errors[0]))),
        'text' => utf8_clean(substr($raw, 0, 60000)),
    )) . "\n";
    preg_match_all('/href=["\']([^"\'#]+)/i', $body, $links);
    foreach (array_merge($location === '' ? array() : array($location), $links[1]) as $link) {
        $next = same_site($path, $link);
        if ($next === null) { continue; }
        $unsafe = preg_match('#logout|logoff|signout|delete|remove|destroy|_profiler|_wdt|_debugbar|telescope|horizon|\.(css|js|png|jpe?g|gif|svg|ico|woff2?|pdf|zip)$#i', (string) strtok($next, '?'));
        if (!$unsafe) { $queue[] = $next; }
    }
}
if ($queue) { echo "FERRYLIMIT\n"; }
function same_site($base, $link) {
    $link = preg_replace('#^https?://127\.0\.0\.1:8080#', '', trim($link));
    if ($link === '' || preg_match('#^(//|[a-z][a-z0-9+.-]*:)#i', $link)) { return null; }
    list($query) = array(strstr($link, '?') ?: '');
    $path = $query === '' ? $link : substr($link, 0, -strlen($query));
    $from = (string) strtok($base, '?');
    if ($path === '') { return $from . $query; }
    if ($path[0] !== '/') { $path = rtrim(dirname($from . 'x'), '/') . '/' . $path; }
    $parts = array();
    foreach (explode('/', $path) as $part) {
        if ($part === '..') { array_pop($parts); } elseif ($part !== '.' && $part !== '') { $parts[] = $part; }
    }
    return '/' . implode('/', $parts) . (substr($path, -1) === '/' && $parts ? '/' : '') . $query;
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
        let lost_success = self.source_status < 300 && self.target_status >= 300;
        let lost_redirect = self.source_status < 400 && self.target_status >= 400;
        !failed(self.source_status) && (failed(self.target_status) || fatal(&self.new_errors) || lost_success || lost_redirect)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Crawl {
    pages: Vec<Fetched>,
    /// The page limit stopped the crawl with links left to visit.
    pub limited: bool,
}

impl Crawl {
    pub fn paths(&self) -> Vec<String> {
        self.pages.iter().map(|p| p.path.clone()).collect()
    }
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
         cat > /tmp/router.php <<'FERRYEOF'\n{ROUTER}\nFERRYEOF\n\
         export APP_ENV=prod APP_DEBUG=0\n\
         php -d display_errors=0 -d log_errors=1 -d error_reporting=-1 -S 127.0.0.1:{PORT} -t {root} /tmp/router.php > /tmp/server.log 2>&1 &\n\
         sleep 1\nphp /tmp/crawl.php {PAGE_LIMIT} {}\necho {LOG_MARKER}\ncat /tmp/server.log\n",
        list.join(" ")
    )
}

pub fn parse(output: &str) -> Crawl {
    let pages = output.lines().filter_map(|l| l.strip_prefix("FERRYPAGE ")).filter_map(|l| serde_json::from_str(l).ok()).collect();
    Crawl { pages, limited: output.lines().any(|l| l == "FERRYLIMIT") }
}

pub fn server_log(output: &str) -> &str {
    output.split_once(LOG_MARKER).map_or("", |(_, log)| log)
}

fn failed(status: u16) -> bool {
    status == 0 || status >= 500
}

/// The first place two texts differ, with a little context on each side.
fn first_difference(source: &str, target: &str) -> (String, String) {
    let at = source.chars().zip(target.chars()).take_while(|(a, b)| a == b).count();
    let excerpt = |text: &str| text.chars().skip(at.saturating_sub(EXCERPT)).take(EXCERPT * 2).collect::<String>();
    (excerpt(source), excerpt(target))
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
    } else if !source.stable || !target.stable {
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
                let (source_text, target_text) = first_difference(&s.text, &t.text);
                PageDiff {
                    path: s.path.clone(),
                    change: classify(s, t, &new_errors),
                    source_status: s.status,
                    target_status: t.status,
                    new_errors,
                    source_text,
                    target_text,
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
        compare(&Crawl { pages: vec![source], limited: false }, &Crawl { pages: vec![target], limited: false })[0].change
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
    fn excerpts_show_where_pages_differ() {
        let long = "a".repeat(1000);
        let (source, target) = first_difference(&format!("{long}old tail"), &format!("{long}new tail"));
        assert!(source.contains("old tail") && target.contains("new tail"), "{source}");
        assert!(source.len() < 500);
    }

    #[test]
    fn a_page_missing_on_the_target_is_reported() {
        let diffs = compare(&Crawl { pages: vec![page("/a", 200, "a", &[])], limited: false }, &Crawl::default());
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
