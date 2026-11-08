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
and keep the behavior, style, indentation and blank lines. Don't add or change type declarations, return types, \
visibility or names, and don't add checks, comments or other code the problem doesn't need. Don't delete a \
statement, an array element or an argument unless the problem requires it: when a deprecated constant or flag has no \
effect, take it out of a | combination or write 0 in its place. Never replace a function call, a connection or another \
value the code uses with a placeholder such as 0, null or ''; when the code needs a migration these lines can't hold, \
reply with them unchanged. Prefer a fix that keeps the program's output the same. \
A hint written as Class::method() names a method; call it the way the class defines it. A value shown as [redacted] \
was hidden before sending; leave any line that holds one exactly as it is. If the problem can't be fixed safely with \
only the lines you're given, reply with them unchanged.";

/// Rules where Ferry's own local model gave a correct fix when every manual rule in
/// demo/kuzey-magaza was tried with Qwen3.5 9B and each accepted reply was reviewed. For the others
/// it isn't offered; a cloud provider still is.
pub const LOCAL_RULES: &[&str] = &[
    "behavior.concat_precedence",
    "behavior.html_default_flags",
    "behavior.indirect_access_order",
    "behavior.substr_false",
    "deprecated.assert_constants",
    "deprecated.assert_options",
    "deprecated.curlopt_binarytransfer",
    "deprecated.curlpipe_http1",
    "deprecated.date_wakeup",
    "deprecated.dom_php_err",
    "deprecated.dynamic_property",
    "deprecated.e_strict",
    "deprecated.e_user_error",
    "deprecated.false_to_array",
    "deprecated.file_binary",
    "deprecated.file_text",
    "deprecated.filter_sanitize_stripped",
    "deprecated.http_response_header",
    "deprecated.implicit_argument",
    "deprecated.ini_setting",
    "deprecated.intlgregcal_create_instance",
    "deprecated.lcg_value",
    "deprecated.libxml_disable_entity_loader",
    "deprecated.mt_rand_php",
    "deprecated.mysqli_is_mariadb",
    "deprecated.mysqli_refresh",
    "deprecated.mysqli_store_result_copy_data",
    "deprecated.mysqli_store_result_mode",
    "deprecated.mysqli_update_max_length",
    "deprecated.null_offset",
    "deprecated.null_to_internal",
    "deprecated.openssl_x509_free",
    "deprecated.partial_callable",
    "deprecated.shmop_close",
    "deprecated.splfixedarray_wakeup",
    "deprecated.u_multiple_decimal_seperators",
    "deprecated.xml_handler_string",
    "deprecated.zip_fl_recompress",
    "removed.__autoload",
    "removed.ai_idn_constant",
    "removed.break_outside_loop",
    "removed.datefmt_set_timezone_id",
    "removed.filter_flag_host_required",
    "removed.filter_flag_scheme_required",
    "removed.filter_sanitize_magic_quotes",
    "removed.get_magic_quotes_gpc",
    "removed.get_magic_quotes_runtime",
    "removed.gzgetss",
    "removed.intl_idna_variant_2003",
    "removed.magic_quotes_runtime",
    "removed.mysqli_embedded_server",
    "removed.pgsql_libpq_version_str",
    "removed.reflection_export",
    "removed.set_magic_quotes_runtime",
    "removed.unset_this",
];

pub fn local_supports(rule: &str) -> bool {
    LOCAL_RULES.contains(&rule)
}

/// Lines on each side of the finding where a reply may change anything. Further away, only a change
/// the fix plainly needs is kept.
const NEAR: usize = 2;

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
    #[serde(skip)]
    line: usize,
    #[serde(skip)]
    target: Option<PhpVersion>,
}

/// Anything that turns a prompt into a reply: a cloud model with the user's key, or a local one.
pub trait Provider: Send + Sync {
    fn name(&self) -> String;
    fn complete(&self, prompt: &Prompt, cancel: &AtomicBool) -> Result<String, String>;
}

impl Prompt {
    /// A prompt that isn't about a file, such as the check after a local model is installed.
    pub fn plain(system: &str, user: &str) -> Prompt {
        Prompt {
            system: system.into(),
            user: user.into(),
            file: String::new(),
            first_line: 1,
            last_line: 1,
            original: String::new(),
            masked: Vec::new(),
            line: 1,
            target: None,
        }
    }

    /// The same request with the reason the previous reply wasn't used.
    fn again(&self, reason: &str) -> Prompt {
        let user = format!("{}\n\nYour previous reply wasn't used: {reason} Reply again, changing only what the problem needs.", self.user);
        Prompt { user, ..self.clone() }
    }
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
    let problem = excerpt[finding.line - first].trim();
    let user = format!(
        "Ferry found this problem in a project moving to PHP {target}:\n{}: {}{hint}\n\nFile {}, lines {first}-{last}. \
         The problem is on line {}:\n{problem}\n\n```php\n{}\n```\n\nReply with lines {first}-{last} corrected, as one ```php \
         block and nothing else. Keep every line that doesn't need to change exactly as it is, and use only syntax and \
         functions that PHP {target} has.",
        finding.title.en,
        finding.message.en,
        finding.file,
        finding.line,
        excerpt.join("\n"),
    );
    Ok(Prompt {
        system: SYSTEM.to_string(),
        user,
        file: finding.file.clone(),
        first_line: first,
        last_line: last,
        original,
        masked,
        line: finding.line,
        target: Some(target),
    })
}

pub fn code_block(reply: &str) -> Option<&str> {
    let open = reply.find("```")?;
    let body = &reply[open + 3..];
    let body = &body[body.find('\n')? + 1..];
    let close = body.rfind("```")?;
    Some(body[..close].strip_suffix('\n').unwrap_or(&body[..close]))
}

/// Why a reply wasn't used: in the user's language, and in English for the model when it's asked
/// again. A reply that changes nothing isn't asked again: the model found no safe fix.
#[derive(Debug)]
enum Rejected {
    Unchanged,
    Retry { user: String, model: String },
}

impl Rejected {
    fn retry(user: String, model: impl Into<String>) -> Rejected {
        Rejected::Retry { user, model: model.into() }
    }

    fn message(self) -> String {
        match self {
            Rejected::Unchanged => crate::t!(
                "Model bu satırlarla güvenli bir düzeltme bulamadı; düzeltme büyük olasılıkla gösterilen satırların dışındaki koda bağlı. Elle düzeltin ya da başka bir modelle deneyin.",
                "The model found no safe fix within these lines; the fix likely depends on code outside them. Fix it by hand or try another model."
            ),
            Rejected::Retry { user, .. } => user,
        }
    }
}

/// Puts back changes that only move whitespace or blank lines away from the finding: noise a model
/// adds that would otherwise show up in the diff.
fn without_noise(old: &[String], new: Vec<String>, near: std::ops::RangeInclusive<usize>) -> Vec<String> {
    let squash = |lines: &[String]| lines.concat().split_whitespace().collect::<String>();
    let mut out = Vec::new();
    for op in similar::capture_diff_slices(similar::Algorithm::Myers, old, &new) {
        let (tag, before, after) = op.as_tag_tuple();
        let close = before.start <= *near.end() && before.end.max(before.start + 1) > *near.start();
        if tag != similar::DiffTag::Equal && !close && squash(&old[before.clone()]) == squash(&new[after.clone()]) {
            out.extend_from_slice(&old[before]);
        } else {
            out.extend_from_slice(&new[after]);
        }
    }
    out
}

/// A function declaration changed away from the finding: models like to add types there, and PHP
/// may refuse to load the file for it.
fn touches_far_signature(old: &[String], new: &[String], near: std::ops::RangeInclusive<usize>) -> bool {
    let declares = |l: &String| {
        let t = l.trim_start();
        (t.starts_with("function ") || t.contains(" function ")) && t.contains('(') && !t.starts_with("//") && !t.starts_with('*')
    };
    similar::capture_diff_slices(similar::Algorithm::Myers, old, new).iter().any(|op| {
        let (tag, before, _) = op.as_tag_tuple();
        tag != similar::DiffTag::Equal
            && old[before.clone()].iter().enumerate().any(|(i, l)| !near.contains(&(before.start + i)) && declares(l))
    })
}

/// Words that look like a function or a constant but are part of PHP itself.
const KEYWORDS: &[&str] = &[
    "__class__",
    "__dir__",
    "__file__",
    "__function__",
    "__halt_compiler",
    "__line__",
    "__method__",
    "__namespace__",
    "__property__",
    "__trait__",
    "abstract",
    "and",
    "array",
    "as",
    "break",
    "callable",
    "case",
    "catch",
    "class",
    "clone",
    "const",
    "continue",
    "declare",
    "default",
    "die",
    "do",
    "echo",
    "else",
    "elseif",
    "empty",
    "enddeclare",
    "endfor",
    "endforeach",
    "endif",
    "endswitch",
    "endwhile",
    "enum",
    "eval",
    "exit",
    "extends",
    "false",
    "final",
    "finally",
    "fn",
    "for",
    "foreach",
    "function",
    "global",
    "goto",
    "if",
    "implements",
    "include",
    "include_once",
    "instanceof",
    "insteadof",
    "interface",
    "isset",
    "list",
    "match",
    "namespace",
    "new",
    "null",
    "or",
    "parent",
    "print",
    "private",
    "protected",
    "public",
    "readonly",
    "require",
    "require_once",
    "return",
    "self",
    "static",
    "switch",
    "throw",
    "trait",
    "true",
    "try",
    "unset",
    "use",
    "var",
    "while",
    "xor",
    "yield",
];

/// Words after which a name is declared or names a class, not used as a function or constant.
const DECLARERS: &[&str] = &[
    "function",
    "const",
    "class",
    "interface",
    "trait",
    "enum",
    "new",
    "extends",
    "implements",
    "instanceof",
    "insteadof",
    "use",
    "namespace",
    "case",
    "goto",
    "as",
];

#[derive(Default)]
struct Symbols {
    calls: std::collections::HashSet<String>,
    constants: std::collections::HashSet<String>,
    declared: std::collections::HashSet<String>,
}

/// The global functions a file calls and the constants it uses, with the names it declares
/// itself. Functions are lowercased; constants keep their case.
fn symbols(src: &str) -> Symbols {
    use crate::lexer::TokenKind;
    let tokens = crate::lexer::tokenize(src);
    let mut found = Symbols::default();
    for (i, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Ident {
            continue;
        }
        let prev = i.checked_sub(1).map(|j| &tokens[j]);
        let next = tokens.get(i + 1);
        if token.text.eq_ignore_ascii_case("define") && next.is_some_and(|n| n.is_punct("(")) {
            if let Some(TokenKind::Str { .. }) = tokens.get(i + 2).map(|t| &t.kind) {
                found.declared.insert(tokens[i + 2].text.trim_matches(|c| c == '\'' || c == '"').to_string());
            }
        }
        let name = token.text.trim_start_matches('\\');
        let lower = name.to_ascii_lowercase();
        if name.contains('\\') || KEYWORDS.contains(&lower.as_str()) {
            continue;
        }
        if prev.is_some_and(|p| p.kind == TokenKind::Ident && DECLARERS.contains(&p.text.to_ascii_lowercase().as_str())) {
            found.declared.insert(name.to_string());
            found.declared.insert(lower);
            continue;
        }
        if prev.is_some_and(|p| p.is_punct("->") || p.is_punct("?->") || p.is_punct("::"))
            || next.is_some_and(|n| n.is_punct("::") || n.is_punct(":") || n.kind == TokenKind::Variable)
        {
            continue;
        }
        if next.is_some_and(|n| n.is_punct("(")) {
            found.calls.insert(lower);
        } else if name.starts_with(|c: char| c.is_ascii_uppercase() || c == '_')
            && name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        {
            found.constants.insert(name.to_string());
        }
    }
    found
}

/// A function or constant the changed code uses that PHP doesn't define, the file didn't use before
/// and doesn't declare: a name the model made up.
fn invented(original: &str, changed: &str, target: PhpVersion) -> Option<String> {
    let (before, after) = (symbols(original), symbols(changed));
    let known_function = |f: &str| crate::arity::ARITY.iter().any(|(name, _, _)| name.eq_ignore_ascii_case(f));
    let mut calls: Vec<&String> = after.calls.difference(&before.calls).collect();
    calls.sort();
    if let Some(call) = calls.into_iter().find(|f| !known_function(f) && !after.declared.contains(*f)) {
        return Some(format!("{call}()"));
    }
    let removed_later = |c: &str| {
        crate::symbols::REMOVED_CONSTANTS.iter().any(|(name, (major, minor), _)| *name == c && PhpVersion::new(*major, *minor) > target)
    };
    let mut constants: Vec<&String> = after.constants.difference(&before.constants).collect();
    constants.sort();
    constants
        .into_iter()
        .find(|c| crate::constants::CONSTANTS.binary_search(&c.as_str()).is_err() && !removed_later(c) && !after.declared.contains(*c))
        .cloned()
}

fn counts(findings: &[Finding]) -> std::collections::HashMap<&str, usize> {
    let mut counts = std::collections::HashMap::new();
    for f in findings {
        *counts.entry(f.rule_id.as_str()).or_insert(0) += 1;
    }
    counts
}

/// The reply as a change to the file. A line the model was shown masked has to come back
/// unchanged, and then gets its real value back.
pub fn suggestion(finding: &Finding, prompt: &Prompt, reply: &str) -> Result<FileChange, String> {
    checked(finding, prompt, reply).map_err(Rejected::message)
}

fn checked(finding: &Finding, prompt: &Prompt, reply: &str) -> Result<FileChange, Rejected> {
    let code = code_block(reply)
        .ok_or_else(|| Rejected::retry(crate::t!("Yanıtta kod bulunamadı.", "The reply had no code in it."), "It had no ```php block."))?
        .trim_end_matches('\r');
    let lines: Vec<&str> = prompt.original.lines().collect();
    let window = prompt.last_line - prompt.first_line + 1;
    let mut replaced = Vec::new();
    for line in code.lines() {
        let line = line.trim_end_matches('\r');
        if line.contains(crate::redact::MASK) {
            let Some(&(n, _)) = prompt.masked.iter().find(|(_, shown)| shown == line) else {
                return Err(Rejected::retry(
                    crate::t!(
                        "Öneri gizlenmiş bir değeri içeren satırı değiştirdi; uygulanmadı.",
                        "The suggestion changed a line that held a hidden value, so it wasn't used."
                    ),
                    "It changed a line that holds [redacted]; those lines must stay exactly as they are.",
                ));
            };
            replaced.push(lines[n - 1].to_string());
        } else {
            replaced.push(line.to_string());
        }
    }
    if replaced.len() > window * 3 + 20 {
        return Err(Rejected::retry(
            crate::t!("Öneri beklenenden çok uzun; uygulanmadı.", "The suggestion was much longer than expected, so it wasn't used."),
            "It was much longer than the lines you were given.",
        ));
    }
    let shown: Vec<String> = lines[prompt.first_line - 1..prompt.last_line].iter().map(|l| l.to_string()).collect();
    let at = prompt.line.clamp(prompt.first_line, prompt.last_line) - prompt.first_line;
    let near = at.saturating_sub(NEAR)..=at + NEAR;
    let replaced = without_noise(&shown, replaced, near.clone());
    if touches_far_signature(&shown, &replaced, near) {
        return Err(Rejected::retry(
            crate::t!(
                "Öneri, bulgudan uzaktaki bir fonksiyon tanımını değiştirdi; uygulanmadı.",
                "The suggestion changed a function declaration away from the finding, so it wasn't used."
            ),
            "It changed a function declaration that the problem doesn't involve.",
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
        return Err(Rejected::Unchanged);
    }
    if let Some(target) = prompt.target {
        if let Some(name) = invented(&prompt.original, &new_content, target) {
            return Err(Rejected::retry(
                crate::t!(
                    "Öneri PHP'nin tanımadığı bir ad kullanıyor ({name}); uygulanmadı.",
                    "The suggestion uses a name PHP doesn't define ({name}), so it wasn't used."
                ),
                format!("It uses {name}, which PHP {target} doesn't define."),
            ));
        }
        let before = crate::analyzer::analyze_source(&finding.file, &prompt.original, target);
        let after = crate::analyzer::analyze_source(&finding.file, &new_content, target);
        let (before_counts, after_counts) = (counts(&before), counts(&after));
        let had = before_counts.get(finding.rule_id.as_str()).copied().unwrap_or(0);
        if had > 0 && after_counts.get(finding.rule_id.as_str()).copied().unwrap_or(0) >= had {
            return Err(Rejected::retry(
                crate::t!(
                    "Öneri bu sorunu çözmedi: Ferry düzeltilmiş kodda aynı bulguyu yine buldu.",
                    "The suggestion didn't solve the problem: Ferry still finds it in the changed code."
                ),
                "Ferry still finds the same problem in the changed code.",
            ));
        }
        if let Some(new) =
            after.iter().find(|f| after_counts[f.rule_id.as_str()] > before_counts.get(f.rule_id.as_str()).copied().unwrap_or(0))
        {
            let title = new.title.get(crate::i18n::lang()).to_string();
            return Err(Rejected::retry(
                crate::t!(
                    "Öneri yeni bir sorun ekledi ({title}); uygulanmadı.",
                    "The suggestion added a new problem ({title}), so it wasn't used."
                ),
                "It added a new PHP compatibility problem.",
            ));
        }
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

/// Asks the provider, and when the reply is rejected or `syntax` finds errors in it, asks once more
/// with the reason. Gives the change with the syntax errors and whether the syntax was checked.
pub fn suggest(
    provider: &dyn Provider,
    finding: &Finding,
    prompt: &Prompt,
    cancel: &AtomicBool,
    syntax: &dyn Fn(&FileChange) -> (Vec<String>, bool),
) -> Result<(FileChange, Vec<String>, bool), String> {
    let stopped = || crate::t!("AI önerisi durduruldu.", "The AI suggestion was stopped.");
    let mut current = prompt.clone();
    for attempt in 0..2 {
        let reply = provider.complete(&current, cancel)?;
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(stopped());
        }
        let last = attempt == 1;
        match checked(finding, prompt, &reply) {
            Ok(change) => {
                let (errors, ran) = syntax(&change);
                if errors.is_empty() || last {
                    return Ok((change, errors, ran));
                }
                current = prompt.again(&format!("PHP reported a syntax error in it: {}", errors.join("; ")));
            }
            Err(Rejected::Retry { user, .. }) if last => return Err(user),
            Err(Rejected::Retry { model, .. }) => current = prompt.again(&model),
            Err(unchanged) => return Err(unchanged.message()),
        }
    }
    unreachable!("the loop returns on its second attempt")
}

/// The providers Ferry can call with the user's own key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Anthropic,
    /// Anything that speaks OpenAI's chat completions: OpenAI itself, or a local server such as
    /// Ollama or LM Studio.
    Openai,
    /// The model Ferry downloads and runs itself with llama.cpp.
    Local,
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

/// Drops the reasoning some local models put before the answer, which can hold code of its own.
fn without_thinking(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<think>") {
        out.push_str(&rest[..start]);
        match rest[start..].find("</think>") {
            Some(end) => rest = &rest[start + end + "</think>".len()..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

fn openai_reply(body: &str) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let text = without_thinking(value["choices"][0]["message"]["content"].as_str().unwrap_or_default());
    if text.trim().is_empty() {
        return Err(crate::t!("Yanıt boş geldi.", "The reply was empty."));
    }
    Ok(text)
}

/// Whether an address is this computer, where Ferry can choose the decoding settings: a server
/// in the cloud may reject some of them.
fn is_loopback(url: &str) -> bool {
    let origin = origin(url);
    let host = origin.split("://").nth(1).unwrap_or_default();
    let host =
        host.rsplit_once(':').filter(|(h, port)| !h.ends_with(':') && port.chars().all(|c| c.is_ascii_digit())).map_or(host, |(h, _)| h);
    matches!(host, "127.0.0.1" | "localhost" | "[::1]")
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
            Kind::Openai | Kind::Local => key.map(|k| format!("Authorization: Bearer {k}")).into_iter().collect(),
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
            Kind::Openai | Kind::Local => {
                let messages = [("system", &prompt.system), ("user", &prompt.user)];
                let messages: Vec<_> = messages.iter().map(|(role, text)| serde_json::json!({ "role": role, "content": text })).collect();
                let mut body = serde_json::json!({ "model": self.model, "messages": messages });
                if is_loopback(&self.base_url) {
                    body["temperature"] = 0.into();
                    body["max_tokens"] = 2048.into();
                }
                (format!("{}/chat/completions", self.base()), body)
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
            Kind::Openai | Kind::Local => format!("{}/models", self.base()),
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
            Kind::Openai | Kind::Local => {
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
            Kind::Openai | Kind::Local => openai_reply(&body),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

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
            line: 1,
            target: None,
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
    fn a_server_on_this_computer_gets_steady_decoding_and_a_cloud_one_its_defaults() {
        let local = Remote { kind: Kind::Openai, model: "m".into(), base_url: "http://127.0.0.1:11434/v1".into(), key: None };
        let body: serde_json::Value = serde_json::from_str(&local.completion(&sample_prompt()).body.unwrap()).unwrap();
        assert_eq!((body["temperature"].as_i64(), body["max_tokens"].as_i64()), (Some(0), Some(2048)));
        let cloud = Remote { base_url: "https://api.openai.com/v1".into(), ..local.clone() };
        let body: serde_json::Value = serde_json::from_str(&cloud.completion(&sample_prompt()).body.unwrap()).unwrap();
        assert!(body.get("temperature").is_none() && body.get("max_tokens").is_none());
        assert!(
            is_loopback("http://localhost:1234/v1")
                && is_loopback("http://[::1]:8080/v1")
                && !is_loopback("http://127.0.0.1.example.com/v1")
        );
        let reply =
            openai_reply(r#"{"choices":[{"message":{"content":"<think>maybe ```php\nx\n```</think>```php\necho 1;\n```"}}]}"#).unwrap();
        assert_eq!(code_block(&reply), Some("echo 1;"));
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

    const LOOP: &str = "<?php\nfunction total($a, $b) {\n    $sum = 0;\n\n    $x = 1;\n    $y = 2;\n    $z = 3;\n\n    while (list($k, $v) = each($a)) {\n        $sum += $v;\n    }\n    return $sum;\n}\n";
    const FIXED: &str = "    foreach ($a as $k => $v) {";

    fn reply_with(edit: impl Fn(&str) -> String) -> String {
        let window: Vec<String> = LOOP.lines().map(|l| if l.contains("each($a)") { FIXED.to_string() } else { l.to_string() }).collect();
        format!("```php\n{}\n```", edit(&window.join("\n")))
    }

    #[test]
    fn a_reply_that_leaves_the_problem_or_adds_one_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let f = finding(dir.path(), LOOP);
        let p = prompt(dir.path(), &f, PhpVersion::new(8, 4)).unwrap();
        assert!(p.user.contains("The problem is on line 9:\nwhile (list($k, $v) = each($a)) {"), "{}", p.user);
        let unsolved = LOOP.replace("$sum = 0;", "$sum = 0.0;");
        assert!(
            matches!(checked(&f, &p, &format!("```php\n{}```", unsolved)), Err(Rejected::Retry { model, .. }) if model.contains("still"))
        );
        let worse = reply_with(|w| w.replace("$sum += $v;", "$sum += strlen(utf8_encode('a'));"));
        assert!(matches!(checked(&f, &p, &worse), Err(Rejected::Retry { model, .. }) if model.contains("new")));
        assert!(checked(&f, &p, &reply_with(str::to_string)).is_ok());
    }

    #[test]
    fn whitespace_moved_away_from_the_finding_is_put_back() {
        let dir = tempfile::tempdir().unwrap();
        let f = finding(dir.path(), LOOP);
        let p = prompt(dir.path(), &f, PhpVersion::new(8, 4)).unwrap();
        let noisy = reply_with(|w| w.replacen("$sum = 0;\n\n", "$sum = 0;\n", 1).replace("    $x = 1;", "  $x = 1;"));
        let change = checked(&f, &p, &noisy).unwrap();
        assert_eq!(change.new_content, LOOP.replace("    while (list($k, $v) = each($a)) {", FIXED));
    }

    #[test]
    fn names_are_read_as_calls_constants_or_declarations() {
        let src =
            "<?php\nnamespace App;\nuse Foo\\Bar;\ndefine('LIMIT', 3);\nconst SIZE = 2;\nfunction helper(int $a): int { return $a; }\n\
                   class Box { const MAX = 1; public function run(Bar $b) { $b->go(PHP_EOL); self::MAX; \\strlen('x'); new Bar(E_ALL); \
                   helper(a: LIMIT); goto done; done: return Box::MAX + SIZE; } }\n";
        let found = symbols(src);
        let mut calls: Vec<_> = found.calls.iter().map(String::as_str).collect();
        calls.sort();
        assert_eq!(calls, ["define", "helper", "strlen"]);
        let mut constants: Vec<_> = found.constants.iter().map(String::as_str).collect();
        constants.sort();
        assert_eq!(constants, ["E_ALL", "LIMIT", "PHP_EOL", "SIZE"]);
        for name in ["LIMIT", "SIZE", "helper", "MAX", "Box"] {
            assert!(found.declared.contains(name), "{name}");
        }
    }

    #[test]
    fn a_reply_that_uses_a_name_php_doesnt_have_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let f = finding(dir.path(), LOOP);
        let p = prompt(dir.path(), &f, PhpVersion::new(8, 4)).unwrap();
        let constant = reply_with(|w| w.replace("$sum += $v;", "$sum += $v * ZEND_SCALE;"));
        assert!(matches!(checked(&f, &p, &constant), Err(Rejected::Retry { model, .. }) if model.contains("ZEND_SCALE")));
        let function = reply_with(|w| w.replace("$sum += $v;", "$sum += array_sum_fast($v);"));
        assert!(matches!(checked(&f, &p, &function), Err(Rejected::Retry { model, .. }) if model.contains("array_sum_fast()")));
        let known = reply_with(|w| w.replace("$sum += $v;", "$sum += intdiv($v, PHP_INT_SIZE);"));
        assert!(checked(&f, &p, &known).is_ok());
    }

    #[test]
    fn a_function_declaration_away_from_the_finding_stays_as_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let f = finding(dir.path(), LOOP);
        let p = prompt(dir.path(), &f, PhpVersion::new(8, 4)).unwrap();
        let typed = reply_with(|w| w.replace("function total($a, $b) {", "function total(array $a, $b): int {"));
        assert!(matches!(checked(&f, &p, &typed), Err(Rejected::Retry { model, .. }) if model.contains("declaration")));
    }

    /// Answers with each reply in turn and keeps the prompts it was sent.
    struct Replies(Mutex<Vec<String>>, Mutex<Vec<String>>);

    impl Provider for Replies {
        fn name(&self) -> String {
            "replies".into()
        }

        fn complete(&self, prompt: &Prompt, _: &AtomicBool) -> Result<String, String> {
            self.1.lock().unwrap().push(prompt.user.clone());
            Ok(self.0.lock().unwrap().remove(0))
        }
    }

    #[test]
    fn a_refused_reply_is_asked_again_once_with_the_reason() {
        let dir = tempfile::tempdir().unwrap();
        let f = finding(dir.path(), LOOP);
        let p = prompt(dir.path(), &f, PhpVersion::new(8, 4)).unwrap();
        let clean = |_: &FileChange| (Vec::new(), true);
        let unsolved = format!("```php\n{}```", LOOP.replace("$sum = 0;", "$sum = 0.0;"));
        let provider = Replies(Mutex::new(vec![unsolved.clone(), reply_with(str::to_string)]), Mutex::default());
        let (change, errors, ran) = suggest(&provider, &f, &p, &AtomicBool::new(false), &clean).unwrap();
        assert!(change.new_content.contains(FIXED) && errors.is_empty() && ran);
        let sent = provider.1.lock().unwrap();
        assert!(!sent[0].contains("wasn't used") && sent[1].contains("wasn't used: Ferry still finds the same problem"), "{sent:?}");
        drop(sent);

        let provider = Replies(Mutex::new(vec![unsolved.clone(), unsolved]), Mutex::default());
        assert!(suggest(&provider, &f, &p, &AtomicBool::new(false), &clean).is_err());
        assert_eq!(provider.1.lock().unwrap().len(), 2);

        let provider = Replies(Mutex::new(vec![format!("```php\n{LOOP}```")]), Mutex::default());
        assert!(suggest(&provider, &f, &p, &AtomicBool::new(false), &clean).is_err());
        assert_eq!(provider.1.lock().unwrap().len(), 1, "a reply that changes nothing isn't asked again");

        let lint =
            |c: &FileChange| (if c.new_content.contains("$sum += $v;") { vec!["syntax error".to_string()] } else { Vec::new() }, true);
        let provider = Replies(
            Mutex::new(vec![reply_with(str::to_string), reply_with(|w| w.replace("$sum += $v;", "$sum = $sum + $v;"))]),
            Mutex::default(),
        );
        let (change, errors, _) = suggest(&provider, &f, &p, &AtomicBool::new(false), &lint).unwrap();
        assert!(errors.is_empty() && change.new_content.contains("$sum = $sum + $v;"));
        assert!(provider.1.lock().unwrap()[1].contains("PHP reported a syntax error in it: syntax error"));
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
