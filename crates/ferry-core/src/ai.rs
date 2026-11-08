//! Suggestions for findings Ferry can't fix on its own. Only the code around a finding leaves the
//! computer, masked first, and a suggestion becomes an ordinary change: previewed, backed up and
//! linted before anything is written.

use crate::analyzer::Finding;
use crate::rector::{AppliedRule, FileChange};
use crate::rules::FixKind;
use crate::version::PhpVersion;
use serde::Serialize;
use std::path::Path;
use std::sync::atomic::AtomicBool;

/// Lines on each side of a finding: enough for the surrounding function in most code.
const AROUND: usize = 20;

const SYSTEM: &str = "You fix PHP code for an upgrade to a newer PHP version. Change only what the problem needs, \
and keep the behavior, style and indentation. A value shown as [redacted] was hidden before sending; leave any \
line that holds one exactly as it is.";

/// Exactly what goes to a provider, so it can be shown before anything is sent.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub system: String,
    pub user: String,
    pub file: String,
    pub first_line: usize,
    pub last_line: usize,
    #[serde(skip)]
    original: String,
    #[serde(skip)]
    masked: Vec<(usize, String)>,
}

/// Anything that turns a prompt into a reply: a cloud model with the user's key, or a local one.
pub trait Provider: Send + Sync {
    fn name(&self) -> String;
    fn complete(&self, prompt: &Prompt, cancel: &AtomicBool) -> Result<String, String>;
}

fn line_break(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

pub fn prompt(root: &Path, finding: &Finding, target: PhpVersion) -> Result<Prompt, String> {
    if finding.rule_id.starts_with("framework.") {
        return Err(crate::t!(
            "Bu bulgu kodla değil, framework ya da CMS güncellenerek çözülür.",
            "This finding is solved by updating the framework or CMS, not the code."
        ));
    }
    if !crate::project::is_php_file(Path::new(&finding.file)) {
        return Err(crate::t!("AI önerileri yalnızca PHP dosyaları için.", "AI suggestions are for PHP files only."));
    }
    let path = crate::safefs::resolve(root, &finding.file).map_err(|e| e.to_string())?;
    let bytes = crate::safefs::read_bytes(&path).map_err(|e| e.to_string())?;
    let original = String::from_utf8(bytes)
        .map_err(|_| crate::t!("Dosya UTF-8 değil, bu yüzden AI'a gönderilmedi.", "The file isn't UTF-8, so it wasn't sent to AI."))?;
    let lines: Vec<&str> = original.lines().collect();
    if finding.line == 0 || finding.line > lines.len() {
        return Err(crate::t!(
            "Bulgunun satırı dosyada yok; projeyi yeniden tarayın.",
            "The finding's line isn't in the file; scan again."
        ));
    }
    let first = finding.line.saturating_sub(AROUND).max(1);
    let last = (finding.line + AROUND).min(lines.len());
    let mut masked = Vec::new();
    let excerpt: Vec<String> = (first..=last)
        .map(|n| {
            let line = lines[n - 1];
            let shown = crate::redact::text(line);
            if shown != line {
                masked.push((n, shown.clone()));
            }
            shown
        })
        .collect();
    let hint = finding.replacement.as_ref().map_or(String::new(), |r| format!("\nSuggested direction: {}", r.en));
    let user = format!(
        "Ferry found this problem in a project moving to PHP {target}:\n{}: {}{hint}\n\nFile {}, lines {first}-{last}. \
         The problem is on line {}.\n```php\n{}\n```\n\nReply with lines {first}-{last} corrected, as one ```php block and \
         nothing else. Keep every line that doesn't need to change exactly as it is.",
        finding.title.en,
        finding.message.en,
        finding.file,
        finding.line,
        excerpt.join("\n"),
    );
    Ok(Prompt { system: SYSTEM.to_string(), user, file: finding.file.clone(), first_line: first, last_line: last, original, masked })
}

fn code_block(reply: &str) -> Option<&str> {
    let open = reply.find("```")?;
    let body = &reply[open + 3..];
    let body = &body[body.find('\n')? + 1..];
    let close = body.rfind("```")?;
    Some(body[..close].strip_suffix('\n').unwrap_or(&body[..close]))
}

/// The reply as a change to the file. A line the model was shown masked has to come back
/// unchanged, and then gets its real value back.
pub fn suggestion(finding: &Finding, prompt: &Prompt, reply: &str) -> Result<FileChange, String> {
    let code =
        code_block(reply).ok_or_else(|| crate::t!("Yanıtta kod bulunamadı.", "The reply had no code in it."))?.trim_end_matches('\r');
    let lines: Vec<&str> = prompt.original.lines().collect();
    let window = prompt.last_line - prompt.first_line + 1;
    let mut replaced = Vec::new();
    for line in code.lines() {
        let line = line.trim_end_matches('\r');
        if line.contains(crate::redact::MASK) {
            let Some(&(n, _)) = prompt.masked.iter().find(|(_, shown)| shown == line) else {
                return Err(crate::t!(
                    "Öneri gizlenmiş bir değeri içeren satırı değiştirdi; uygulanmadı.",
                    "The suggestion changed a line that held a hidden value, so it wasn't used."
                ));
            };
            replaced.push(lines[n - 1].to_string());
        } else {
            replaced.push(line.to_string());
        }
    }
    if replaced.len() > window * 3 + 20 {
        return Err(crate::t!(
            "Öneri beklenenden çok uzun; uygulanmadı.",
            "The suggestion was much longer than expected, so it wasn't used."
        ));
    }
    let mut out: Vec<String> = lines[..prompt.first_line - 1].iter().map(|l| l.to_string()).collect();
    out.extend(replaced);
    out.extend(lines[prompt.last_line..].iter().map(|l| l.to_string()));
    let newline = line_break(&prompt.original);
    let mut new_content = out.join(newline);
    if prompt.original.ends_with('\n') {
        new_content.push_str(newline);
    }
    if new_content == prompt.original {
        return Err(crate::t!("Öneri kodu değiştirmedi.", "The suggestion didn't change the code."));
    }
    let rule = AppliedRule {
        class: "ai".to_string(),
        name: finding.rule_id.clone(),
        title: finding.title.clone(),
        since: Some(finding.since),
        fix: FixKind::Manual,
    };
    Ok(FileChange {
        file: finding.file.clone(),
        rules: vec![rule],
        fix: FixKind::Manual,
        diff: crate::diff::compute(&prompt.original, &new_content, 3),
        original_hash: crate::snapshot::fnv1a(prompt.original.as_bytes()),
        new_content,
    })
}

/// The providers Ferry can call with the user's own key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Anthropic,
    /// Anything that speaks OpenAI's chat completions: OpenAI itself, or a local server such as
    /// Ollama or LM Studio.
    Openai,
}

pub const ANTHROPIC_MODELS: [&str; 3] = ["claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5"];
const ANTHROPIC_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_VERSION: &str = "2026-10-21";
/// A declined request runs again on a model chosen for the reason it was declined.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
const MAX_TOKENS: u32 = 16_000;
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// Servers people often run on their own computer, where Ferry looks before asking for an address.
pub const LOCAL_SERVERS: [(&str, &str); 2] = [("Ollama", "http://127.0.0.1:11434/v1"), ("LM Studio", "http://127.0.0.1:1234/v1")];

/// `scheme://host[:port]` of an address: the part a key belongs to.
pub fn origin(url: &str) -> String {
    let (scheme, rest) = url.trim().split_once("://").unwrap_or(("", url.trim()));
    let scheme = scheme.to_ascii_lowercase();
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let authority = authority.rsplit('@').next().unwrap_or(authority).to_ascii_lowercase();
    let authority = match (scheme.as_str(), authority.rsplit_once(':')) {
        ("http", Some((host, "80"))) | ("https", Some((host, "443"))) => host.to_string(),
        _ => authority,
    };
    format!("{scheme}://{authority}")
}

/// A server running on this computer that speaks OpenAI's API, with the chat models it offers.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalServer {
    pub name: String,
    pub base_url: String,
    pub models: Vec<String>,
}

/// The servers in `candidates` that answer, each asked for its models with a short timeout.
pub fn local_servers(candidates: &[(&str, &str)], cancel: &AtomicBool) -> Vec<LocalServer> {
    candidates
        .iter()
        .filter_map(|(name, url)| {
            let remote = Remote { kind: Kind::Openai, model: String::new(), base_url: url.to_string(), key: None };
            let models = remote.models(cancel, std::time::Duration::from_secs(3)).ok()?;
            let models = models.into_iter().filter(|m| !m.to_ascii_lowercase().contains("embed")).collect();
            Some(LocalServer { name: name.to_string(), base_url: url.to_string(), models })
        })
        .collect()
}

/// A provider reached over HTTPS with the system's curl. The key goes to curl on its standard
/// input, so it never shows on a command line.
#[derive(Debug, Clone)]
pub struct Remote {
    pub kind: Kind,
    pub model: String,
    pub base_url: String,
    pub key: Option<String>,
}

struct Request {
    url: String,
    headers: Vec<String>,
    body: Option<String>,
}

fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\"").replace(['\n', '\r'], ""))
}

fn curl_config(request: &Request, body_file: Option<&Path>) -> String {
    let mut config = format!("url = {}\n", quoted(&request.url));
    for header in &request.headers {
        config.push_str(&format!("header = {}\n", quoted(header)));
    }
    if let Some(file) = body_file {
        config.push_str(&format!("data-binary = {}\n", quoted(&format!("@{}", file.display()))));
    }
    config.push_str("write-out = \"\\n%{http_code}\"\n");
    config
}

fn error_message(body: &str) -> String {
    let value: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    let message = value["error"]["message"].as_str().or_else(|| value["error"].as_str()).or_else(|| value["message"].as_str());
    message.map_or_else(|| body.trim().chars().take(300).collect(), str::to_string)
}

fn failure(status: u16, body: &str) -> String {
    let message = error_message(body);
    match status {
        401 | 403 => crate::t!("API anahtarı kabul edilmedi: {message}", "The API key wasn't accepted: {message}"),
        404 => crate::t!("Model ya da adres bulunamadı: {message}", "The model or address wasn't found: {message}"),
        429 | 500..=599 => crate::t!(
            "Sağlayıcı şu anda yanıt veremiyor ({message}). Biraz sonra yeniden deneyin.",
            "The provider can't answer right now ({message}). Try again a little later."
        ),
        _ => crate::t!("Sağlayıcı isteği reddetti: {message}", "The provider refused the request: {message}"),
    }
}

fn anthropic_reply(body: &str) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    if value["stop_reason"] == "refusal" {
        return Err(crate::t!("Model bu isteği reddetti.", "The model declined this request."));
    }
    let text: String =
        value["content"].as_array().into_iter().flatten().filter(|b| b["type"] == "text").filter_map(|b| b["text"].as_str()).collect();
    if text.trim().is_empty() {
        return Err(crate::t!("Yanıt boş geldi.", "The reply was empty."));
    }
    Ok(text)
}

fn openai_reply(body: &str) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let text = value["choices"][0]["message"]["content"].as_str().unwrap_or_default();
    if text.trim().is_empty() {
        return Err(crate::t!("Yanıt boş geldi.", "The reply was empty."));
    }
    Ok(text.to_string())
}

impl Remote {
    pub fn anthropic(model: &str, key: &str) -> Self {
        Remote { kind: Kind::Anthropic, model: model.to_string(), base_url: ANTHROPIC_URL.to_string(), key: Some(key.to_string()) }
    }

    fn host(&self) -> String {
        let rest = self.base_url.split("://").nth(1).unwrap_or(&self.base_url);
        rest.split('/').next().unwrap_or(rest).to_string()
    }

    fn headers(&self) -> Vec<String> {
        let key = self.key.as_deref().filter(|k| !k.is_empty());
        match self.kind {
            Kind::Anthropic => {
                let mut headers = vec![format!("anthropic-version: {ANTHROPIC_VERSION}")];
                headers.extend(key.map(|k| format!("x-api-key: {k}")));
                headers
            }
            Kind::Openai => key.map(|k| format!("Authorization: Bearer {k}")).into_iter().collect(),
        }
    }

    fn base(&self) -> &str {
        self.base_url.trim_end_matches('/')
    }

    fn completion(&self, prompt: &Prompt) -> Request {
        let mut headers = self.headers();
        headers.push("content-type: application/json".to_string());
        let (url, body) = match self.kind {
            Kind::Anthropic => {
                let mut body = serde_json::json!({
                    "model": self.model,
                    "max_tokens": MAX_TOKENS,
                    "system": prompt.system,
                    "messages": [{ "role": "user", "content": prompt.user }],
                });
                if !self.model.starts_with("claude-haiku") {
                    body["fallbacks"] = "default".into();
                    headers.push(format!("anthropic-beta: {FALLBACK_BETA}"));
                }
                (format!("{}/v1/messages", self.base()), body)
            }
            Kind::Openai => {
                let messages = [("system", &prompt.system), ("user", &prompt.user)];
                let messages: Vec<_> = messages.iter().map(|(role, text)| serde_json::json!({ "role": role, "content": text })).collect();
                (format!("{}/chat/completions", self.base()), serde_json::json!({ "model": self.model, "messages": messages }))
            }
        };
        Request { url, headers, body: Some(body.to_string()) }
    }

    fn send(&self, request: &Request, cancel: &AtomicBool, timeout: std::time::Duration) -> Result<(u16, String), String> {
        let curl = crate::toolchain::find_executable("curl").ok_or_else(|| crate::t!("curl bulunamadı.", "curl wasn't found."))?;
        let body = match &request.body {
            Some(text) => {
                let mut file = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
                std::io::Write::write_all(&mut file, text.as_bytes()).map_err(|e| e.to_string())?;
                Some(file)
            }
            None => None,
        };
        let config = curl_config(request, body.as_ref().map(|f| f.path()));
        let mut cmd = crate::toolchain::command(&curl);
        cmd.args(["-sS", "--max-time", &timeout.as_secs().to_string(), "-K", "-"]);
        let done =
            crate::toolchain::run_process_with_input(cmd, &curl, cancel, timeout + std::time::Duration::from_secs(10), config.into_bytes())
                .map_err(|e| e.to_string())?;
        if done.stopped.is_some() {
            return Err(crate::t!("AI önerisi durduruldu.", "The AI suggestion was stopped."));
        }
        let out = String::from_utf8_lossy(&done.stdout).into_owned();
        let (body, status) = out.rsplit_once('\n').unwrap_or(("", &out));
        match status.trim().parse::<u16>() {
            Ok(status) if status > 0 => Ok((status, body.to_string())),
            _ => {
                let reason = String::from_utf8_lossy(&done.stderr).trim().to_string();
                Err(crate::t!("Sağlayıcıya ulaşılamadı: {reason}", "Couldn't reach the provider: {reason}"))
            }
        }
    }

    fn models(&self, cancel: &AtomicBool, timeout: std::time::Duration) -> Result<Vec<String>, String> {
        let url = match self.kind {
            Kind::Anthropic => format!("{}/v1/models", self.base()),
            Kind::Openai => format!("{}/models", self.base()),
        };
        let (status, body) = self.send(&Request { url, headers: self.headers(), body: None }, cancel, timeout)?;
        if !(200..300).contains(&status) {
            return Err(failure(status, &body));
        }
        let value: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
        let models = value["data"].as_array().ok_or_else(|| crate::t!("Model listesi okunamadı.", "Couldn't read the model list."))?;
        Ok(models.iter().filter_map(|m| m["id"].as_str()).map(str::to_string).collect())
    }

    /// Whether the key and address work, without spending tokens: it lists the models.
    pub fn check(&self, cancel: &AtomicBool) -> Result<(), String> {
        self.models(cancel, std::time::Duration::from_secs(20)).map(|_| ())
    }
}

impl Provider for Remote {
    fn name(&self) -> String {
        match self.kind {
            Kind::Anthropic => format!("Anthropic ({})", self.model),
            Kind::Openai => {
                let local = LOCAL_SERVERS.iter().find(|(_, url)| origin(url) == origin(&self.base_url));
                format!("{} ({})", local.map_or_else(|| self.host(), |(name, _)| name.to_string()), self.model)
            }
        }
    }

    fn complete(&self, prompt: &Prompt, cancel: &AtomicBool) -> Result<String, String> {
        let (status, body) = self.send(&self.completion(prompt), cancel, TIMEOUT)?;
        if !(200..300).contains(&status) {
            return Err(failure(status, &body));
        }
        match self.kind {
            Kind::Anthropic => anthropic_reply(&body),
            Kind::Openai => openai_reply(&body),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Answers one request with `reply` and hands back what it received.
    fn serve(reply: &'static str) -> (String, std::thread::JoinHandle<String>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut seen = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let n = stream.read(&mut chunk).unwrap();
                seen.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&seen).to_string();
                if let Some((head, body)) = text.split_once("\r\n\r\n") {
                    let length = head
                        .lines()
                        .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap()))
                        .unwrap_or(0);
                    if body.len() >= length {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
                reply.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            String::from_utf8_lossy(&seen).to_string()
        });
        (url, handle)
    }

    fn sample_prompt() -> Prompt {
        Prompt {
            system: "fix it".into(),
            user: "say \"hi\"".into(),
            file: "a.php".into(),
            first_line: 1,
            last_line: 1,
            original: String::new(),
            masked: Vec::new(),
        }
    }

    #[test]
    fn an_openai_compatible_server_gets_the_prompt_and_the_key_stays_off_the_command_line() {
        if crate::toolchain::find_executable("curl").is_none() {
            return;
        }
        let (url, server) = serve(r#"{"choices":[{"message":{"content":"```php\necho 1;\n```"}}]}"#);
        let remote = Remote { kind: Kind::Openai, model: "local-coder".into(), base_url: format!("{url}/v1"), key: Some("sk-test".into()) };
        let reply = remote.complete(&sample_prompt(), &AtomicBool::new(false)).unwrap();
        assert!(reply.contains("echo 1;"));
        let seen = server.join().unwrap();
        assert!(seen.starts_with("POST /v1/chat/completions"), "{seen}");
        assert!(seen.contains("Authorization: Bearer sk-test"), "{seen}");
        assert!(seen.contains(r#""model":"local-coder""#) && seen.contains(r#"say \"hi\""#), "{seen}");
        assert_eq!(remote.name(), format!("{} (local-coder)", url.trim_start_matches("http://")));
    }

    #[test]
    fn anthropic_requests_carry_the_version_and_a_fallback_except_on_haiku() {
        if crate::toolchain::find_executable("curl").is_none() {
            return;
        }
        let (url, server) =
            serve(r#"{"content":[{"type":"thinking","thinking":""},{"type":"text","text":"done"}],"stop_reason":"end_turn"}"#);
        let remote = Remote { base_url: url, ..Remote::anthropic("claude-opus-5-5", "sk-ant-test") };
        assert_eq!(remote.complete(&sample_prompt(), &AtomicBool::new(false)).unwrap(), "done");
        let seen = server.join().unwrap().to_ascii_lowercase();
        assert!(seen.starts_with("post /v1/messages"), "{seen}");
        for header in ["x-api-key: sk-ant-test", "anthropic-version: 2026-10-21", "anthropic-beta: server-side-fallback-2026-07-01"] {
            assert!(seen.contains(header), "{header}: {seen}");
        }
        assert!(seen.contains(r#""fallbacks":"default""#));
        let haiku = Remote::anthropic("claude-haiku-5-5", "k").completion(&sample_prompt());
        assert!(!haiku.body.unwrap().contains("fallbacks") && !haiku.headers.iter().any(|h| h.contains("beta")));
    }

    #[test]
    fn running_local_servers_are_found_with_their_chat_models() {
        if crate::toolchain::find_executable("curl").is_none() {
            return;
        }
        let (url, server) = serve(r#"{"object":"list","data":[{"id":"qwen2.5-coder:7b"},{"id":"nomic-embed-text:latest"}]}"#);
        let closed = format!("http://{}/v1", std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap());
        let base = format!("{url}/v1");
        let found = local_servers(&[("Ollama", base.as_str()), ("LM Studio", closed.as_str())], &AtomicBool::new(false));
        assert_eq!(found, vec![LocalServer { name: "Ollama".into(), base_url: base, models: vec!["qwen2.5-coder:7b".into()] }]);
        assert!(server.join().unwrap().starts_with("GET /v1/models"));
    }

    #[test]
    fn a_key_belongs_to_the_scheme_host_and_port() {
        assert_eq!(origin("HTTPS://API.OpenAI.com:443/v1/"), "https://api.openai.com");
        assert_eq!(origin(" http://127.0.0.1:11434/v1 "), "http://127.0.0.1:11434");
        assert_ne!(origin("http://127.0.0.1:11434/v1"), origin("http://127.0.0.1:1234/v1"));
        assert_eq!(origin("https://user@host.example/x?y"), "https://host.example");
        let ollama = Remote { kind: Kind::Openai, model: "qwen".into(), base_url: "http://127.0.0.1:11434/v1/".into(), key: None };
        assert_eq!(ollama.name(), "Ollama (qwen)");
    }

    #[test]
    fn replies_and_errors_read_like_the_providers_send_them() {
        assert!(anthropic_reply(r#"{"content":[],"stop_reason":"refusal"}"#).is_err());
        assert!(openai_reply(r#"{"choices":[{"message":{"content":""}}]}"#).is_err());
        let auth = failure(401, r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#);
        assert!(auth.contains("invalid x-api-key"));
        assert!(failure(529, "{}").contains("{}") || failure(529, "{}").contains("later") || failure(529, "{}").contains("sonra"));
        let config = curl_config(&Request { url: "http://h/\"x".into(), headers: vec!["a: b\\c".into()], body: None }, None);
        assert!(config.contains(r#"url = "http://h/\"x""#) && config.contains(r#"header = "a: b\\c""#), "{config}");
    }

    fn finding(root: &Path, src: &str) -> Finding {
        std::fs::write(root.join("a.php"), src).unwrap();
        crate::analyzer::analyze_source("a.php", src, PhpVersion::new(8, 4)).into_iter().find(|f| f.rule_id == "removed.each").unwrap()
    }

    #[test]
    fn only_the_lines_around_a_finding_are_sent_and_secrets_are_masked() {
        let dir = tempfile::tempdir().unwrap();
        let mut src = String::from("<?php\n");
        for n in 0..60 {
            src.push_str(&format!("$v{n} = {n};\n"));
        }
        src.push_str("$password = 'hunter2';\nwhile (list($k, $v) = each($a)) {}\n");
        let f = finding(dir.path(), &src);
        let p = prompt(dir.path(), &f, PhpVersion::new(8, 4)).unwrap();
        assert_eq!((p.first_line, p.last_line), (f.line - AROUND, f.line));
        assert!(!p.user.contains("$v0 = 0;") && p.user.contains("each($a)"));
        assert!(!p.user.contains("hunter2") && p.user.contains("[redacted]"));
    }

    #[test]
    fn a_reply_becomes_a_change_that_keeps_hidden_values_and_line_endings() {
        let dir = tempfile::tempdir().unwrap();
        let src = "<?php\r\n$password = 'hunter2';\r\nwhile (list($k, $v) = each($a)) {}\r\necho 1;\r\n";
        let f = finding(dir.path(), src);
        let p = prompt(dir.path(), &f, PhpVersion::new(8, 4)).unwrap();
        let reply = "Here:\n```php\n<?php\n$password = '[redacted]';\nforeach ($a as $k => $v) {}\necho 1;\n```";
        let change = suggestion(&f, &p, reply).unwrap();
        assert_eq!(change.new_content, "<?php\r\n$password = 'hunter2';\r\nforeach ($a as $k => $v) {}\r\necho 1;\r\n");
        assert_eq!(change.original_hash, crate::snapshot::fnv1a(src.as_bytes()));
        assert_eq!(change.rules[0].name, "removed.each");
    }

    #[test]
    fn a_reply_that_touches_a_hidden_value_or_says_nothing_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let f = finding(dir.path(), "<?php\n$password = 'hunter2';\nwhile (list($k, $v) = each($a)) {}\n");
        let p = prompt(dir.path(), &f, PhpVersion::new(8, 4)).unwrap();
        let changed = "```php\n<?php\n$password = '[redacted]x';\nforeach ($a as $k => $v) {}\n```";
        assert!(suggestion(&f, &p, changed).is_err());
        assert!(suggestion(&f, &p, "No code, sorry.").is_err());
        let same = "```php\n<?php\n$password = '[redacted]';\nwhile (list($k, $v) = each($a)) {}\n```";
        assert!(suggestion(&f, &p, same).is_err());
    }

    #[test]
    fn files_that_arent_php_or_utf8_are_never_sent() {
        let dir = tempfile::tempdir().unwrap();
        let mut f = finding(dir.path(), "<?php\nwhile (list($k, $v) = each($a)) {}\n");
        std::fs::write(dir.path().join("a.php"), b"<?php\n\xff each($a);\n").unwrap();
        assert!(prompt(dir.path(), &f, PhpVersion::new(8, 4)).is_err());
        f.file = ".env".into();
        std::fs::write(dir.path().join(".env"), "DB_PASSWORD=x\n").unwrap();
        assert!(prompt(dir.path(), &f, PhpVersion::new(8, 4)).is_err());
    }
}
