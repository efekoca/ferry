use crate::analyzer::{call_args, Receivers};
use crate::i18n::Text;
use crate::lexer::{Token, TokenKind};
use crate::rules::{patterns, FixKind, PatternRule, Severity};

pub(crate) struct Issue {
    pub at: usize,
    pub rule: &'static PatternRule,
    pub since: (u32, u32),
}

fn issue(at: usize, rule: &'static PatternRule) -> Issue {
    Issue { at, rule, since: rule.since }
}

fn short(name: &str) -> String {
    name.rsplit('\\').next().unwrap_or_default().to_ascii_lowercase()
}

fn plain(args: &[&[Token]]) -> bool {
    !args.iter().any(|a| a.first().is_some_and(|t| t.is_punct("...")) || a.len() > 1 && a[1].is_punct(":") && a[0].kind == TokenKind::Ident)
}

fn argument_rule(tokens: &[Token], keys: &[String], open: usize, call: usize, out: &mut Vec<Issue>) {
    let args = call_args(tokens, open);
    if !plain(&args) {
        return;
    }
    for key in keys {
        if let Some((rule, at)) = crate::arguments::find(key, &args) {
            let at = at.map_or(call, |t| tokens.iter().position(|x| std::ptr::eq(x, t)).unwrap_or(call));
            out.push(issue(at, &rule.rule));
            return;
        }
    }
}

const PROPERTIES: &[(&str, &str, &PatternRule)] = &[
    ("mysqli", "stat", &MYSQLI_STAT),
    ("mysqli_driver", "driver_version", &DRIVER_VERSION),
    ("domdocument", "actualencoding", &DOM_DOCUMENT_PROPERTY),
    ("domdocument", "config", &DOM_DOCUMENT_PROPERTY),
];

/// Method calls, constructors, property reads and clones on built-in classes whose behavior changed.
/// `is_user_type` says whether the project declares a class with that short name.
pub(crate) fn check(tokens: &[Token], is_user_type: impl Fn(&str) -> bool) -> Vec<Issue> {
    let mut out = Vec::new();
    let receivers = Receivers::new(tokens);
    let field_call = |j: usize| {
        let name = tokens[j].text.to_ascii_lowercase();
        tokens[j].kind == TokenKind::Ident
            && (name.starts_with("mysqli_fetch_field") || name.starts_with("fetch_field") && j > 0 && tokens[j - 1].is_punct("->"))
    };
    let mut field_vars = std::collections::HashSet::new();
    for k in 0..tokens.len() {
        let assigned = tokens.get(k + 1).is_some_and(|n| n.is_punct("=")) && (k + 2..tokens.len().min(k + 6)).any(field_call);
        let looped = k > 0 && tokens[k - 1].is_ident_ci("as") && (k.saturating_sub(10)..k).any(field_call);
        if tokens[k].kind == TokenKind::Variable && (assigned || looped) {
            field_vars.insert(tokens[k].text.as_str());
        }
    }
    for k in 0..tokens.len() {
        let t = &tokens[k];
        let prev = k.checked_sub(1).map(|p| &tokens[p]);
        let next = tokens.get(k + 1);
        let called = next.is_some_and(|n| n.is_punct("("));
        if t.kind == TokenKind::Ident && prev.is_some_and(|p| p.is_punct("->")) && k >= 2 && tokens[k - 2].kind == TokenKind::Variable {
            let types = receivers.at(tokens, k - 2, &tokens[k - 2].text);
            let builtin: Vec<&String> = types.iter().filter(|c| !is_user_type(c)).collect();
            let name = t.text.to_ascii_lowercase();
            if called {
                let keys: Vec<String> = builtin.iter().map(|c| format!("{c}::{name}")).collect();
                argument_rule(tokens, &keys, k + 1, k, &mut out);
                if name == "connect" && builtin.iter().any(|c| *c == "mysqli") {
                    let close = crate::declarations::close_of(tokens, k + 1);
                    let null = close.is_some_and(|c| {
                        tokens.get(c + 1).is_some_and(|o| ["===", "!==", "==", "!="].iter().any(|p| o.is_punct(p)))
                            && tokens.get(c + 2).is_some_and(|v| v.is_ident_ci("null"))
                    });
                    if null {
                        out.push(issue(k, &MYSQLI_CONNECT_RESULT));
                    }
                }
            } else if let Some((_, _, rule)) = PROPERTIES.iter().find(|(c, p, _)| *p == name && builtin.iter().any(|b| b == c)) {
                out.push(issue(k, rule));
            }
        }
        let compound = next.is_some_and(|n| ["+", "-", "*", "/", ".", "%"].iter().any(|o| n.is_punct(o)))
            && tokens.get(k + 2).is_some_and(|n| n.is_punct("="));
        let read = !called && !compound && !next.is_some_and(|n| n.is_punct("="));
        let from_field = k >= 2 && tokens[k - 2].kind == TokenKind::Variable && field_vars.contains(tokens[k - 2].text.as_str());
        if t.is_ident_ci("max_length") && prev.is_some_and(|p| p.is_punct("->")) && read && from_field {
            out.push(issue(k, &MAX_LENGTH));
        }
        let static_call = t.kind == TokenKind::Ident
            && next.is_some_and(|n| n.is_punct("::"))
            && tokens.get(k + 3).is_some_and(|n| n.is_punct("("))
            && !prev.is_some_and(|p| p.is_ident_ci("new"));
        if static_call {
            let class = short(&t.text);
            if !["self", "static", "parent"].contains(&class.as_str()) && !is_user_type(&class) {
                let method = tokens[k + 2].text.to_ascii_lowercase();
                let keys = [format!("static {class}::{method}"), format!("{class}::{method}")];
                argument_rule(tokens, &keys, k + 3, k + 2, &mut out);
            }
        }
        if prev.is_some_and(|p| p.is_ident_ci("new")) && t.kind == TokenKind::Ident && called {
            let class = short(&t.text);
            if !is_user_type(&class) {
                argument_rule(tokens, &[format!("{class}::__construct")], k + 1, k, &mut out);
            }
        }
        if t.is_ident_ci("clone") {
            let var = next.filter(|n| n.kind == TokenKind::Variable);
            let member = tokens.get(k + 3).filter(|_| tokens.get(k + 2).is_some_and(|a| a.is_punct("->")));
            if let Some(v) = var {
                let classes = receivers.at(tokens, k + 1, &v.text);
                if member.is_none() && classes.contains("domxpath") && !is_user_type("domxpath") {
                    out.push(issue(k, &CLONE_XPATH));
                }
                let list = member.is_some_and(|m| {
                    let after = tokens.get(k + 4).is_some_and(|n| n.is_punct("("));
                    (m.is_ident_ci("childNodes") || m.is_ident_ci("attributes")) && !after
                        || (m.is_ident_ci("getElementsByTagName") || m.is_ident_ci("getElementsByTagNameNS")) && after
                });
                if list {
                    out.push(issue(k, &CLONE_NODE_LIST));
                }
            }
        }
        let soap_handle = t.is_ident_ci("is_resource")
            && called
            && tokens.get(k + 2).is_some_and(|v| v.kind == TokenKind::Variable)
            && tokens.get(k + 3).is_some_and(|a| a.is_punct("->"))
            && tokens.get(k + 4).is_some_and(|p| ["httpurl", "sdl", "typemap"].iter().any(|n| p.text.eq_ignore_ascii_case(n)))
            && tokens.get(k + 5).is_some_and(|c| c.is_punct(")"));
        if soap_handle {
            out.push(Issue { at: k, rule: &patterns::RESOURCE_CHECK, since: (8, 4) });
        }
    }
    out
}

const fn rule(id: &'static str, title: Text, severity: Severity, since: (u32, u32), replacement: Option<Text>, note: Text) -> PatternRule {
    PatternRule { id, title, severity, since, replacement, fix: FixKind::Manual, note }
}

const MYSQLI_CONNECT_RESULT: PatternRule = rule(
    "behavior.mysqli_connect_result",
    Text::new("mysqli::connect() sonucu", "mysqli::connect() result"),
    Severity::Behavior,
    (8, 1),
    Some(Text::new("sonucu true ile karşılaştırın", "compare the result with true")),
    Text::new(
        "PHP 8.1'den beri mysqli::connect() başarılı olduğunda null değil, true döndürür.",
        "Since PHP 8.1 mysqli::connect() returns true on success instead of null.",
    ),
);

const MYSQLI_STAT: PatternRule = rule(
    "removed.mysqli_stat_property",
    Text::new("mysqli::$stat özelliği", "mysqli::$stat property"),
    Severity::Breaking,
    (7, 4),
    Some(Text::new("$mysqli->stat()", "$mysqli->stat()")),
    Text::new(
        "Belgelenmemiş mysqli::$stat özelliği PHP 7.4'te kaldırıldı; stat() metodunu kullanın.",
        "The undocumented mysqli::$stat property was removed in PHP 7.4; use the stat() method.",
    ),
);

const DRIVER_VERSION: PatternRule = rule(
    "deprecated.mysqli_driver_version",
    Text::new("mysqli_driver::$driver_version", "mysqli_driver::$driver_version"),
    Severity::Deprecated,
    (8, 1),
    None,
    Text::new(
        "mysqli_driver::$driver_version PHP 8.1'de kullanımdan kalktı; yıllardır güncellenmiyordu.",
        "mysqli_driver::$driver_version is deprecated since PHP 8.1; it hadn't been updated in years.",
    ),
);

const DOM_DOCUMENT_PROPERTY: PatternRule = rule(
    "deprecated.dom_document_property",
    Text::new("DOMDocument'ın eski özelliği", "Old DOMDocument property"),
    Severity::Deprecated,
    (8, 4),
    Some(Text::new("$doc->encoding", "$doc->encoding")),
    Text::new(
        "DOMDocument::$actualEncoding ve $config PHP 8.4'te kullanımdan kalktı.",
        "DOMDocument::$actualEncoding and $config are deprecated since PHP 8.4.",
    ),
);

const MAX_LENGTH: PatternRule = rule(
    "behavior.mysqli_max_length",
    Text::new("Alan bilgisindeki max_length", "max_length in field metadata"),
    Severity::Behavior,
    (8, 1),
    None,
    Text::new(
        "PHP 8.1'den beri mysqli alan bilgisindeki max_length her zaman 0; değeri hesaplamak için sonuçları kendiniz dolaşın.",
        "Since PHP 8.1 max_length in mysqli field metadata is always 0; compute it from the results yourself.",
    ),
);

const CLONE_XPATH: PatternRule = rule(
    "removed.uncloneable_object",
    Text::new("DOMXPath klonlama", "Cloning DOMXPath"),
    Severity::Breaking,
    (8, 4),
    Some(Text::new("yeni bir DOMXPath oluşturun", "create a new DOMXPath")),
    Text::new("PHP 8.4'ten beri DOMXPath klonlanamaz; clone hata fırlatır.", "Since PHP 8.4 DOMXPath can't be cloned; clone throws."),
);

const CLONE_NODE_LIST: PatternRule = rule(
    "removed.uncloneable_object",
    Text::new("DOM listesi klonlama", "Cloning a DOM list"),
    Severity::Breaking,
    (8, 5),
    Some(Text::new("iterator_to_array() ile kopyalayın", "copy it with iterator_to_array()")),
    Text::new(
        "PHP 8.5'ten beri DOMNodeList ve DOMNamedNodeMap klonlanamaz; clone hata fırlatır.",
        "Since PHP 8.5 DOMNodeList and DOMNamedNodeMap can't be cloned; clone throws.",
    ),
);
