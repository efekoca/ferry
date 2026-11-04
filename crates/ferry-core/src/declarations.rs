use crate::i18n::Text;
use crate::lexer::{Token, TokenKind};
use crate::rules::{FixKind, PatternRule, Severity};
use std::collections::{HashMap, HashSet};

pub(crate) struct Issue {
    pub at: usize,
    pub rule: &'static PatternRule,
    pub since: (u32, u32),
    pub title: Option<String>,
}

fn issue(at: usize, rule: &'static PatternRule) -> Issue {
    Issue { at, rule, since: rule.since, title: None }
}

pub(crate) fn close_of(tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    (open..tokens.len()).find(|&k| {
        let t = &tokens[k];
        if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") || t.is_punct("#[") {
            depth += 1;
        } else if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            depth -= 1;
        }
        depth == 0
    })
}

fn type_parts(tokens: &[Token]) -> Vec<String> {
    tokens
        .iter()
        .filter(|t| t.kind == TokenKind::Ident)
        .map(|t| t.text.trim_start_matches('\\').to_ascii_lowercase())
        .chain(tokens.iter().any(|t| t.is_punct("?")).then(|| "null".to_string()))
        .collect()
}

pub(crate) struct Param {
    pub(crate) var: usize,
    types: Vec<String>,
    default: Option<usize>,
}

fn params(tokens: &[Token], open: usize, close: usize) -> Vec<Param> {
    let mut out = Vec::new();
    let mut start = open + 1;
    let mut depth = 0usize;
    for k in open + 1..=close {
        let t = &tokens[k];
        if depth == 0 && (k == close || t.is_punct(",")) {
            let part = &tokens[start..k];
            if let Some(v) = part.iter().position(|t| t.kind == TokenKind::Variable) {
                let modifiers = ["public", "protected", "private", "readonly"];
                let ty: Vec<Token> = part[..v].iter().filter(|t| !modifiers.iter().any(|m| t.is_ident_ci(m))).cloned().collect();
                let default = part.iter().position(|t| t.is_punct("=")).filter(|&e| e + 2 == part.len()).map(|e| start + e + 1);
                out.push(Param { var: start + v, types: type_parts(&ty), default });
            }
            start = k + 1;
        } else if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") || t.is_punct("#[") {
            depth += 1;
        } else if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            depth = depth.saturating_sub(1);
        }
    }
    out
}

pub(crate) struct Function {
    pub(crate) keyword: usize,
    pub(crate) name: Option<usize>,
    by_ref: bool,
    pub(crate) params: Vec<Param>,
    pub(crate) uses: Vec<usize>,
    returns: Option<Vec<String>>,
    pub(crate) body: Option<(usize, usize)>,
}

pub(crate) fn function_at(tokens: &[Token], k: usize) -> Option<Function> {
    let t = &tokens[k];
    if !(t.is_ident_ci("function") || t.is_ident_ci("fn")) || k > 0 && (tokens[k - 1].is_punct("->") || tokens[k - 1].is_punct("::")) {
        return None;
    }
    let mut j = k + 1;
    let by_ref = tokens.get(j).is_some_and(|t| t.is_punct("&"));
    if by_ref {
        j += 1;
    }
    let name = tokens.get(j).filter(|t| t.kind == TokenKind::Ident).map(|_| j);
    if name.is_some() {
        j += 1;
    }
    if !tokens.get(j)?.is_punct("(") {
        return None;
    }
    let close = close_of(tokens, j)?;
    let params = params(tokens, j, close);
    let mut after = close + 1;
    let mut uses = Vec::new();
    if tokens.get(after).is_some_and(|t| t.is_ident_ci("use")) && tokens.get(after + 1).is_some_and(|t| t.is_punct("(")) {
        let end = close_of(tokens, after + 1)?;
        uses = (after + 2..end).filter(|&u| tokens[u].kind == TokenKind::Variable).collect();
        after = end + 1;
    }
    let mut returns = None;
    let mut end = after;
    if tokens.get(after).is_some_and(|t| t.is_punct(":")) {
        end = (after + 1..tokens.len()).find(|&e| ["{", ";", "=>"].iter().any(|p| tokens[e].is_punct(p)))?;
        returns = Some(type_parts(&tokens[after + 1..end]));
    }
    let body = tokens.get(end).filter(|t| t.is_punct("{")).and_then(|_| close_of(tokens, end)).map(|c| (end, c));
    Some(Function { keyword: k, name, by_ref, params, uses, returns, body })
}

fn modifiers(tokens: &[Token], keyword: usize) -> Vec<String> {
    let list = ["public", "protected", "private", "static", "final", "abstract", "readonly"];
    (0..keyword).rev().map_while(|j| list.iter().find(|m| tokens[j].is_ident_ci(m)).map(|m| m.to_string())).collect()
}

struct TypeBody {
    keyword: usize,
    open: usize,
    close: usize,
}

fn type_bodies(tokens: &[Token]) -> Vec<TypeBody> {
    let mut out = Vec::new();
    for k in 0..tokens.len() {
        let t = &tokens[k];
        let kw = ["class", "interface", "trait", "enum"].iter().any(|w| t.is_ident_ci(w));
        if !kw || k > 0 && (tokens[k - 1].is_punct("::") || tokens[k - 1].is_punct("->")) {
            continue;
        }
        if t.is_ident_ci("enum") && !tokens.get(k + 1).is_some_and(|n| n.kind == TokenKind::Ident) {
            continue;
        }
        let Some(open) = (k + 1..tokens.len()).find(|&j| tokens[j].is_punct("{") || tokens[j].is_punct(";")) else { continue };
        if !tokens[open].is_punct("{") {
            continue;
        }
        if let Some(close) = close_of(tokens, open) {
            out.push(TypeBody { keyword: k, open, close });
        }
    }
    out
}

fn depth_one(tokens: &[Token], body: &TypeBody) -> Vec<usize> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    for (k, t) in tokens.iter().enumerate().take(body.close).skip(body.open) {
        if t.is_punct("{") || t.is_punct("(") || t.is_punct("[") || t.is_punct("#[") {
            depth += 1;
        } else if t.is_punct("}") || t.is_punct(")") || t.is_punct("]") {
            depth -= 1;
        } else if depth == 1 {
            out.push(k);
        }
    }
    out
}

const MAGIC_PARAMS: &[(&str, &[&str])] = &[
    ("__call", &["string", "array"]),
    ("__callstatic", &["string", "array"]),
    ("__get", &["string"]),
    ("__isset", &["string"]),
    ("__set", &["string", "mixed"]),
    ("__set_state", &["array"]),
    ("__unserialize", &["array"]),
    ("__unset", &["string"]),
];

const MAGIC_RETURNS: &[(&str, &[&str])] = &[
    ("__clone", &["void"]),
    ("__debuginfo", &["array", "null"]),
    ("__isset", &["bool", "true", "false"]),
    ("__serialize", &["array"]),
    ("__set", &["void"]),
    ("__sleep", &["array"]),
    ("__tostring", &["string"]),
    ("__unserialize", &["void"]),
    ("__unset", &["void"]),
    ("__wakeup", &["void"]),
];

fn magic_mismatch(name: &str, f: &Function) -> bool {
    let wide = |types: &[String], need: &str| types.iter().any(|t| t == need || t == "mixed" || (need == "array" && t == "iterable"));
    let params = MAGIC_PARAMS.iter().find(|(n, _)| *n == name).is_some_and(|(_, need)| {
        f.params.iter().zip(need.iter()).any(|(p, need)| !p.types.is_empty() && *need != "mixed" && !wide(&p.types, need))
    });
    let returns = MAGIC_RETURNS
        .iter()
        .find(|(n, _)| *n == name)
        .zip(f.returns.as_ref())
        .is_some_and(|((_, allowed), got)| !got.is_empty() && got.iter().any(|t| !allowed.contains(&t.as_str())));
    params || returns
}

fn header_names(tokens: &[Token], body: &TypeBody, keyword: &str) -> Vec<usize> {
    let Some(at) = (body.keyword..body.open).find(|&k| tokens[k].is_ident_ci(keyword)) else { return Vec::new() };
    (at + 1..body.open)
        .take_while(|&k| !tokens[k].is_ident_ci("implements") && !tokens[k].is_ident_ci("extends"))
        .filter(|&k| tokens[k].kind == TokenKind::Ident)
        .collect()
}

struct Const {
    name: usize,
    typed: Option<Vec<String>>,
    visibility: Option<String>,
}

fn constants(tokens: &[Token], body: &TypeBody) -> Vec<Const> {
    let mut out = Vec::new();
    for k in depth_one(tokens, body) {
        if !tokens[k].is_ident_ci("const") {
            continue;
        }
        let Some(eq) = (k + 1..body.close).find(|&j| tokens[j].is_punct("=")) else { continue };
        if eq < k + 2 || tokens[eq - 1].kind != TokenKind::Ident {
            continue;
        }
        let typed = (eq > k + 2).then(|| type_parts(&tokens[k + 1..eq - 1]));
        let visibility = modifiers(tokens, k).into_iter().find(|m| ["public", "protected", "private"].contains(&m.as_str()));
        out.push(Const { name: eq - 1, typed, visibility });
    }
    out
}

const SUPERGLOBALS: &[&str] = &["this", "GLOBALS", "_SERVER", "_GET", "_POST", "_FILES", "_COOKIE", "_SESSION", "_REQUEST", "_ENV"];

fn literal(t: &Token) -> bool {
    matches!(t.kind, TokenKind::Number | TokenKind::Str { interpolates: false, .. })
        || ["true", "false", "null"].iter().any(|w| t.is_ident_ci(w))
}

fn statement_start(tokens: &[Token], k: usize) -> bool {
    k == 0 || [";", "{", "}"].iter().any(|p| tokens[k - 1].is_punct(p)) || tokens[k - 1].kind == TokenKind::CloseTag
}

fn string_value(t: &Token) -> Option<&str> {
    matches!(t.kind, TokenKind::Str { interpolates: false, .. }).then(|| t.text.trim_matches(|c| c == '\'' || c == '"'))
}

/// Global and namespaced constants declared with `const` or `define()` outside any block, in order.
fn top_constants(tokens: &[Token], names: &crate::index::Names, inside: impl Fn(usize) -> bool) -> Vec<(usize, String, usize)> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    for k in 0..tokens.len() {
        let t = &tokens[k];
        if t.is_punct("{") {
            depth += 1;
        } else if t.is_punct("}") {
            depth = depth.saturating_sub(1);
        } else if t.is_ident_ci("const") && !inside(k) && statement_start(tokens, k) {
            let mut j = k + 1;
            while let (Some(name), Some(eq)) = (tokens.get(j), tokens.get(j + 1)) {
                if name.kind != TokenKind::Ident || !eq.is_punct("=") {
                    break;
                }
                let value = j + 2;
                out.push((j, names.declared(j, &name.text), value));
                let mut d = 0usize;
                j = (value..tokens.len())
                    .find(|&e| {
                        let p = &tokens[e];
                        if p.is_punct("(") || p.is_punct("[") {
                            d += 1;
                        } else if p.is_punct(")") || p.is_punct("]") {
                            d = d.saturating_sub(1);
                        }
                        d == 0 && (p.is_punct(",") || p.is_punct(";"))
                    })
                    .filter(|&e| tokens[e].is_punct(","))
                    .map_or(tokens.len(), |e| e + 1);
            }
        } else if depth == 0 && t.is_ident_ci("define") && statement_start(tokens, k) && tokens.get(k + 1).is_some_and(|n| n.is_punct("("))
        {
            let name = tokens.get(k + 2).and_then(string_value);
            if let (Some(name), Some(comma)) = (name, tokens.get(k + 3)) {
                if comma.is_punct(",") {
                    out.push((k, name.trim_start_matches('\\').to_ascii_lowercase(), k + 4));
                }
            }
        }
    }
    out
}

fn property_defaults(tokens: &[Token], body: &TypeBody) -> Vec<(usize, Option<usize>)> {
    depth_one(tokens, body)
        .into_iter()
        .filter(|&k| tokens[k].kind == TokenKind::Variable)
        .map(|k| {
            let value = (tokens.get(k + 1).is_some_and(|n| n.is_punct("="))
                && tokens.get(k + 3).is_some_and(|n| n.is_punct(";") || n.is_punct(",")))
            .then_some(k + 2)
            .filter(|&v| literal(&tokens[v]));
            (k, value)
        })
        .collect()
}

struct TraitUse {
    keyword: usize,
    traits: Vec<usize>,
    block: Option<(usize, usize)>,
}

fn trait_uses(tokens: &[Token], body: &TypeBody) -> Vec<TraitUse> {
    let mut out = Vec::new();
    for k in depth_one(tokens, body) {
        if !tokens[k].is_ident_ci("use") {
            continue;
        }
        let Some(end) = (k + 1..body.close).find(|&j| tokens[j].is_punct(";") || tokens[j].is_punct("{")) else { continue };
        let traits = (k + 1..end).filter(|&j| tokens[j].kind == TokenKind::Ident).collect();
        let block = tokens[end].is_punct("{").then(|| close_of(tokens, end).map(|c| (end, c))).flatten();
        out.push(TraitUse { keyword: k, traits, block });
    }
    out
}

/// Declarations that newer PHP versions reject or warn about, checked against the project index for traits and parents.
pub(crate) fn check(tokens: &[Token], index: &crate::index::ProjectIndex) -> Vec<Issue> {
    let mut out = Vec::new();
    let names = crate::index::Names::new(tokens);
    let bodies = type_bodies(tokens);
    let inside = |k: usize| bodies.iter().any(|b| b.open < k && k < b.close);
    let user_type = |fqn: &str| index.type_by_fqn(fqn).is_some() || index.interface_by_fqn(fqn).is_some();
    let constants_here = top_constants(tokens, &names, inside);
    let null_constants: HashSet<&str> = constants_here
        .iter()
        .filter(|(_, _, v)| tokens.get(*v).is_some_and(|t| t.is_ident_ci("null")))
        .map(|(_, n, _)| n.as_str())
        .collect();
    let mut seen_constants = HashSet::new();
    for (at, name, _) in &constants_here {
        let guarded = tokens.windows(3).any(|w| {
            w[0].is_ident_ci("defined")
                && w[1].is_punct("(")
                && string_value(&w[2]).is_some_and(|n| n.trim_start_matches('\\').eq_ignore_ascii_case(name))
        });
        if !seen_constants.insert(name.as_str()) && !guarded {
            out.push(issue(*at, &CONSTANT_REDECLARED));
        }
    }
    for k in 0..tokens.len() {
        let Some(f) = function_at(tokens, k) else { continue };
        let mut seen = HashSet::new();
        for p in &f.params {
            if !seen.insert(tokens[p.var].text.as_str()) {
                out.push(issue(p.var, &DUPLICATE_PARAMETER));
            }
        }
        if f.by_ref && f.returns.as_deref() == Some(&["void".to_string()]) {
            out.push(issue(f.keyword, &VOID_BY_REFERENCE));
        }
        for &u in &f.uses {
            let text = tokens[u].text.as_str();
            if SUPERGLOBALS.contains(&text) || f.params.iter().any(|p| tokens[p.var].text == text) {
                out.push(issue(u, &CLOSURE_USE));
            }
        }
        if let Some(name) = f.name.filter(|&n| tokens[n].is_ident_ci("assert") && !inside(n)) {
            if names.declared(name, "assert") != "assert" {
                out.push(issue(name, &CUSTOM_ASSERT));
            }
        }
        for p in &f.params {
            let Some(d) = p.default.filter(|&d| tokens[d].kind == TokenKind::Ident && !literal(&tokens[d])) else { continue };
            let nullable = p.types.is_empty() || p.types.iter().any(|t| t == "null" || t == "mixed");
            let text = &tokens[d].text;
            let resolved = names.declared(d, text);
            let global = text.trim_start_matches('\\').to_ascii_lowercase();
            if !nullable && (null_constants.contains(resolved.as_str()) || !text.contains('\\') && null_constants.contains(global.as_str()))
            {
                out.push(issue(d, &NULL_CONSTANT_DEFAULT));
            }
        }
    }

    let mut trait_defaults: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for b in bodies.iter().filter(|b| tokens[b.keyword].is_ident_ci("trait")) {
        if let Some(name) = tokens.get(b.keyword + 1) {
            let props = property_defaults(tokens, b)
                .into_iter()
                .filter_map(|(var, value)| value.map(|v| (tokens[var].text.clone(), tokens[v].text.clone())))
                .collect();
            trait_defaults.insert(names.declared(b.keyword, &name.text), props);
        }
    }

    for b in &bodies {
        let kind = tokens[b.keyword].text.to_ascii_lowercase();
        if tokens.get(b.keyword + 1).is_some_and(|n| n.text == "_") {
            out.push(issue(b.keyword + 1, &UNDERSCORE_CLASS));
        }
        for k in depth_one(tokens, b) {
            let Some(f) = function_at(tokens, k) else { continue };
            let Some(name_at) = f.name else { continue };
            let name = tokens[name_at].text.to_ascii_lowercase();
            let mods = modifiers(tokens, k);
            if kind != "trait" && name != "__construct" && mods.iter().any(|m| m == "final") && mods.iter().any(|m| m == "private") {
                out.push(issue(name_at, &FINAL_PRIVATE));
            }
            if name.starts_with("__") && magic_mismatch(&name, &f) {
                out.push(issue(name_at, &MAGIC_SIGNATURE));
            }
            if name == "__debuginfo" {
                if let Some((open, close)) = f.body {
                    for r in open..close {
                        if tokens[r].is_ident_ci("return") && tokens.get(r + 1).is_some_and(|t| t.is_ident_ci("null")) {
                            out.push(issue(r, &DEBUG_INFO_NULL));
                        }
                    }
                }
            }
            if name == "__sleep" {
                if let Some((open, close)) = f.body {
                    for r in open..close {
                        let scalar = tokens.get(r + 1).is_some_and(literal) && tokens.get(r + 2).is_some_and(|t| t.is_punct(";"));
                        if tokens[r].is_ident_ci("return") && scalar {
                            out.push(issue(r, &SLEEP_NON_ARRAY));
                        }
                    }
                }
            }
        }

        let uses = trait_uses(tokens, b);
        for u in &uses {
            let Some((open, close)) = u.block else { continue };
            let methods: Vec<Option<&HashSet<String>>> = u
                .traits
                .iter()
                .map(|&t| index.type_by_fqn(&names.resolve(t, &tokens[t].text)).filter(|i| i.is_trait).map(|i| &i.methods))
                .collect();
            for j in open + 1..close {
                let starts = tokens[j - 1].is_punct("{") || tokens[j - 1].is_punct(";");
                if !starts || tokens[j].kind != TokenKind::Ident || !tokens.get(j + 1).is_some_and(|t| t.is_ident_ci("as")) {
                    continue;
                }
                let method = tokens[j].text.to_ascii_lowercase();
                if methods.iter().filter(|m| m.is_some_and(|m| m.contains(&method))).count() > 1 {
                    out.push(issue(j, &AMBIGUOUS_TRAIT_ALIAS));
                }
            }
        }
        let own_props = property_defaults(tokens, b);
        for u in &uses {
            for &t in &u.traits {
                let Some(defaults) = trait_defaults.get(&names.resolve(t, &tokens[t].text)) else { continue };
                for &(var, value) in &own_props {
                    let Some(value) = value else { continue };
                    let clash = defaults.iter().any(|(name, other)| *name == tokens[var].text && *other != tokens[value].text);
                    if clash {
                        out.push(issue(var, &TRAIT_PROPERTY_DEFAULT));
                    }
                }
            }
        }

        if kind == "class" {
            let parent = header_names(tokens, b, "extends").first().copied();
            let parent_fqn = parent.map(|p| names.resolve(p, &tokens[p].text));
            if let (Some(p), Some(fqn)) = (parent, parent_fqn.as_deref().filter(|f| !user_type(f))) {
                let shown = tokens[p].text.trim_start_matches('\\');
                if let Ok(i) = crate::symbols::FINAL_CLASSES.binary_search_by(|r| r.0.cmp(fqn)) {
                    let (_, since, class) = crate::symbols::FINAL_CLASSES[i];
                    out.push(Issue { at: p, rule: &FINAL_BUILTIN, since, title: Some(format!("extends {class}")) });
                }
                for k in depth_one(tokens, b) {
                    let Some(f) = function_at(tokens, k) else { continue };
                    let Some(name_at) = f.name else { continue };
                    let method = tokens[name_at].text.to_ascii_lowercase();
                    let since = match (fqn, method.as_str()) {
                        ("tidy", "repairstring" | "repairfile") if !modifiers(tokens, k).iter().any(|m| m == "static") => (8, 0),
                        ("splfileobject", "fwrite")
                            if f.params
                                .get(1)
                                .is_some_and(|p| !p.types.is_empty() && !p.types.iter().any(|t| t == "null" || t == "mixed")) =>
                        {
                            (8, 5)
                        }
                        ("soapclient", "__dorequest")
                            if f.params.len() < 6 && !f.params.iter().any(|p| tokens[p.var - 1].is_punct("...")) =>
                        {
                            (8, 5)
                        }
                        _ => continue,
                    };
                    out.push(Issue {
                        at: name_at,
                        rule: &BUILTIN_OVERRIDE,
                        since,
                        title: Some(format!("{shown}::{}()", tokens[name_at].text)),
                    });
                }
            }
            let mut inherited = HashSet::new();
            let mut ancestor = parent_fqn.as_deref().and_then(|f| index.type_by_fqn(f));
            for _ in 0..32 {
                let Some(a) = ancestor else { break };
                inherited.extend(a.static_props.iter().cloned());
                ancestor = a.parent_fqn.as_deref().and_then(|f| index.type_by_fqn(f));
            }
            let declared: HashSet<&str> = own_props.iter().map(|&(v, _)| tokens[v].text.as_str()).collect();
            for u in &uses {
                let shared = u
                    .traits
                    .iter()
                    .filter_map(|&t| index.type_by_fqn(&names.resolve(t, &tokens[t].text)))
                    .any(|t| t.is_trait && t.static_props.iter().any(|p| inherited.contains(p) && !declared.contains(p.as_str())));
                if shared {
                    out.push(issue(u.keyword, &TRAIT_STATIC_REDECLARED));
                }
            }

            let class_readonly = (0..b.keyword)
                .rev()
                .take_while(|&j| ["final", "abstract", "readonly"].iter().any(|m| tokens[j].is_ident_ci(m)))
                .any(|j| tokens[j].is_ident_ci("readonly"));
            let mut readonly = HashSet::new();
            let mut clone_body = None;
            for k in depth_one(tokens, b) {
                if tokens[k].kind == TokenKind::Variable {
                    let start = (b.open..k).rev().find(|&j| [";", "{", "}"].iter().any(|p| tokens[j].is_punct(p))).unwrap_or(b.open);
                    if class_readonly || tokens[start..k].iter().any(|t| t.is_ident_ci("readonly")) {
                        readonly.insert(tokens[k].text.clone());
                    }
                }
                let Some(f) = function_at(tokens, k) else { continue };
                let Some(name_at) = f.name else { continue };
                if tokens[name_at].is_ident_ci("__construct") {
                    for p in &f.params {
                        let start = (0..p.var).rev().find(|&j| tokens[j].is_punct("(") || tokens[j].is_punct(",")).unwrap_or(0);
                        if class_readonly || tokens[start..p.var].iter().any(|t| t.is_ident_ci("readonly")) {
                            readonly.insert(tokens[p.var].text.clone());
                        }
                    }
                } else if tokens[name_at].is_ident_ci("__clone") {
                    clone_body = f.body;
                }
            }
            if let Some((open, close)) = clone_body {
                for r in open..close.saturating_sub(3) {
                    let this = tokens[r + 1].kind == TokenKind::Variable && tokens[r + 1].text == "this" && tokens[r + 2].is_punct("->");
                    if tokens[r].is_punct("&") && this && readonly.contains(tokens[r + 3].text.as_str()) {
                        out.push(issue(r, &READONLY_CLONE_REFERENCE));
                    }
                }
            }
        }

        if kind == "class" || kind == "enum" {
            let consts = constants(tokens, b);
            let parents: Vec<usize> = header_names(tokens, b, "extends").into_iter().chain(header_names(tokens, b, "implements")).collect();
            for c in &consts {
                let name = &tokens[c.name].text;
                let hidden = c.visibility.as_deref().is_some_and(|v| v != "public");
                let interface_const = parents.iter().any(|&p| index.interface_declares_constant(&names.resolve(p, &tokens[p].text), name));
                if hidden && interface_const {
                    out.push(issue(c.name, &INTERFACE_CONSTANT_VISIBILITY));
                }
                for &p in &parents {
                    let class = names.resolve(p, &tokens[p].text);
                    let key = format!("{class}::{name}");
                    let Ok(i) = crate::symbols::TYPED_CONSTANTS.binary_search_by(|r| r.0.cmp(key.as_str())) else { continue };
                    let (_, since, ty) = crate::symbols::TYPED_CONSTANTS[i];
                    let want = type_parts(&crate::lexer::lex(&format!("<?php {ty};")).tokens);
                    let mut got = c.typed.clone().unwrap_or_default();
                    got.sort();
                    let mut want = want;
                    want.sort();
                    if got != want {
                        let shown = tokens[p].text.trim_start_matches('\\');
                        out.push(Issue {
                            at: c.name,
                            rule: &TYPED_CONSTANT,
                            since,
                            title: Some(format!("const {ty} {name} ({shown}::{name})")),
                        });
                    }
                    break;
                }
            }
        }
    }

    for k in 0..tokens.len().saturating_sub(2) {
        if !tokens[k].is_punct("#[") {
            continue;
        }
        let Some(close) = close_of(tokens, k) else { continue };
        let has_attribute = (k + 1..close).any(|j| {
            tokens[j].kind == TokenKind::Ident
                && names.resolve(j, &tokens[j].text) == "attribute"
                && (tokens[j - 1].is_punct("#[") || tokens[j - 1].is_punct(","))
        });
        if !has_attribute {
            continue;
        }
        let mut j = close + 1;
        let mut is_abstract = false;
        loop {
            match tokens.get(j) {
                Some(t) if t.is_punct("#[") => j = close_of(tokens, j).map_or(tokens.len(), |c| c + 1),
                Some(t) if t.is_ident_ci("abstract") => {
                    is_abstract = true;
                    j += 1;
                }
                Some(t) if t.is_ident_ci("final") || t.is_ident_ci("readonly") => j += 1,
                _ => break,
            }
        }
        let target = tokens.get(j);
        let wrong = target.is_some_and(|t| t.is_ident_ci("interface") || t.is_ident_ci("trait") || t.is_ident_ci("enum"))
            || is_abstract && target.is_some_and(|t| t.is_ident_ci("class"));
        if wrong {
            out.push(issue(k, &ATTRIBUTE_TARGET));
        }
    }

    for k in 0..tokens.len().saturating_sub(2) {
        let t = &tokens[k];
        if t.kind != TokenKind::Ident || !tokens[k + 1].is_punct("::") || ["self", "static", "parent"].iter().any(|w| t.is_ident_ci(w)) {
            continue;
        }
        let member = &tokens[k + 2];
        if member.kind == TokenKind::Variable
            && tokens.get(k + 3).is_some_and(|n| n.is_punct("="))
            && tokens.get(k + 4).is_some_and(|n| n.is_punct("&"))
        {
            let class =
                index.type_by_fqn(&names.resolve(k, &t.text)).filter(|c| c.traits.is_empty() && !c.static_props.contains(&member.text));
            let mut ancestor = class.and_then(|c| c.parent_fqn.as_deref()).and_then(|f| index.type_by_fqn(f));
            for _ in 0..32 {
                let Some(a) = ancestor else { break };
                if a.static_props.contains(&member.text) {
                    out.push(issue(k, &STATIC_REFERENCE));
                    break;
                }
                ancestor = a.parent_fqn.as_deref().and_then(|f| index.type_by_fqn(f));
            }
        }
        let static_member = member.kind == TokenKind::Variable
            || member.kind == TokenKind::Ident && tokens.get(k + 3).is_some_and(|n| n.is_punct("(")) && !member.is_ident_ci("class");
        if static_member && index.type_by_fqn(&names.resolve(k, &t.text)).is_some_and(|t| t.is_trait) {
            out.push(issue(k, &TRAIT_STATIC_ACCESS));
        }
    }
    out
}

const fn rule(id: &'static str, title: Text, severity: Severity, since: (u32, u32), replacement: Option<Text>, note: Text) -> PatternRule {
    PatternRule { id, title, severity, since, replacement, fix: FixKind::Manual, note }
}

pub(crate) const DUPLICATE_PARAMETER: PatternRule = rule(
    "removed.duplicate_parameter",
    Text::new("Aynı adlı parametre", "Duplicate parameter name"),
    Severity::Breaking,
    (7, 0),
    Some(Text::new("parametreyi yeniden adlandırın", "rename the parameter")),
    Text::new(
        "PHP 7.0'dan beri iki parametre aynı adı taşıyamaz; dosya derlenmez.",
        "Since PHP 7.0 two parameters can't share a name; the file doesn't compile.",
    ),
);

pub(crate) const VOID_BY_REFERENCE: PatternRule = rule(
    "deprecated.void_by_reference",
    Text::new("void dönen fonksiyonda referans", "By-reference void function"),
    Severity::Deprecated,
    (8, 1),
    Some(Text::new("& işaretini silin", "remove the &")),
    Text::new(
        "void dönen bir fonksiyonu referansla döndürmek PHP 8.1'de kullanımdan kalktı.",
        "Returning by reference from a void function is deprecated since PHP 8.1.",
    ),
);

pub(crate) const UNDERSCORE_CLASS: PatternRule = rule(
    "deprecated.underscore_class_name",
    Text::new("_ adlı sınıf", "Class named _"),
    Severity::Deprecated,
    (8, 4),
    Some(Text::new("sınıfı yeniden adlandırın", "rename the class")),
    Text::new("Sınıf adı olarak _ kullanmak PHP 8.4'te kullanımdan kalktı.", "Using _ as a class name is deprecated since PHP 8.4."),
);

pub(crate) const FINAL_PRIVATE: PatternRule = rule(
    "deprecated.final_private_method",
    Text::new("final private metot", "final private method"),
    Severity::Deprecated,
    (8, 0),
    Some(Text::new("final'ı silin", "remove final")),
    Text::new(
        "Private metotlar hiçbir zaman ezilmediği için PHP 8.0'dan beri final private bir metot derlenirken uyarı verir.",
        "Private methods are never overridden, so since PHP 8.0 a final private method raises a warning when compiled.",
    ),
);

pub(crate) const MAGIC_SIGNATURE: PatternRule = rule(
    "removed.magic_method_signature",
    Text::new("Sihirli metot tipi uyuşmuyor", "Magic method type mismatch"),
    Severity::Breaking,
    (8, 0),
    Some(Text::new("tipi PHP'nin beklediği tiple değiştirin", "use the type PHP expects")),
    Text::new(
        "PHP 8.0'dan beri sihirli metotların parametre ve dönüş tipleri, yazıldıklarında kontrol edilir; uymayan bir tip derleme hatası verir.",
        "Since PHP 8.0 the declared parameter and return types of magic methods are checked, and a mismatch is a compile error.",
    ),
);

pub(crate) const DEBUG_INFO_NULL: PatternRule = rule(
    "deprecated.debug_info_null",
    Text::new("__debugInfo() null dönüyor", "__debugInfo() returns null"),
    Severity::Deprecated,
    (8, 5),
    Some(Text::new("return [];", "return [];")),
    Text::new(
        "__debugInfo()'dan null döndürmek PHP 8.5'te kullanımdan kalktı; boş dizi döndürün.",
        "Returning null from __debugInfo() is deprecated in PHP 8.5; return an empty array.",
    ),
);

pub(crate) const INTERFACE_CONSTANT_VISIBILITY: PatternRule = rule(
    "removed.interface_constant_visibility",
    Text::new("Arayüz sabitinin görünürlüğü", "Interface constant visibility"),
    Severity::Breaking,
    (8, 3),
    Some(Text::new("sabiti public yapın", "make the constant public")),
    Text::new(
        "Bir arayüz sabitini private ya da protected olarak yeniden tanımlamak PHP 8.3'ten beri derleme hatası verir.",
        "Redeclaring an interface constant as private or protected is a compile error since PHP 8.3.",
    ),
);

pub(crate) const TYPED_CONSTANT: PatternRule = rule(
    "removed.typed_constant_override",
    Text::new("Tipli yerleşik sabiti ezme", "Overriding a typed built-in constant"),
    Severity::Breaking,
    (8, 3),
    Some(Text::new("sabite yerleşik sınıftaki tipi verin", "give the constant the built-in type")),
    Text::new(
        "Bu yerleşik sınıfın sabitleri artık tipli. Alt sınıfta aynı sabiti aynı tip olmadan yeniden tanımlamak derleme hatası verir.",
        "This built-in class's constants are typed now. Redeclaring one in a subclass without the same type is a compile error.",
    ),
);

pub(crate) const ATTRIBUTE_TARGET: PatternRule = rule(
    "removed.attribute_on_non_class",
    Text::new("#[Attribute] sınıf olmayan bir yerde", "#[Attribute] on something that isn't a class"),
    Severity::Breaking,
    (8, 5),
    Some(Text::new("#[Attribute]'u silin", "remove #[Attribute]")),
    Text::new(
        "PHP 8.5'ten beri #[Attribute] soyut sınıflara, arayüzlere, trait'lere ve enum'lara uygulanamaz; dosya derlenmez.",
        "Since PHP 8.5 #[Attribute] can't be applied to abstract classes, interfaces, traits or enums; the file doesn't compile.",
    ),
);

pub(crate) const TRAIT_STATIC_ACCESS: PatternRule = rule(
    "deprecated.trait_static_access",
    Text::new("Trait üzerinden statik erişim", "Static access on a trait"),
    Severity::Deprecated,
    (8, 1),
    Some(Text::new("trait'i kullanan sınıf üzerinden erişin", "go through a class that uses the trait")),
    Text::new(
        "Bir trait'in statik metodunu çağırmak ya da statik özelliğine doğrudan trait üzerinden erişmek PHP 8.1'de kullanımdan kalktı.",
        "Calling a static method or accessing a static property directly on a trait is deprecated since PHP 8.1.",
    ),
);

pub(crate) const STATIC_REFERENCE: PatternRule = rule(
    "behavior.inherited_static_reference",
    Text::new("Miras alınan statik özelliğe referans", "Reference to an inherited static property"),
    Severity::Behavior,
    (7, 3),
    None,
    Text::new(
        "Bu statik özellik üst sınıftan geliyor. PHP 7.3'ten beri referans ataması özelliği ayırmaz, üst sınıfı da etkiler.",
        "This static property comes from the parent class. Since PHP 7.3 assigning a reference no longer separates it, so the parent changes too.",
    ),
);

pub(crate) const CONSTANT_REDECLARED: PatternRule = rule(
    "deprecated.constant_redeclared",
    Text::new("Sabit yeniden tanımlanıyor", "Constant redeclared"),
    Severity::Deprecated,
    (8, 5),
    Some(Text::new("ikinci tanımı silin", "remove the second declaration")),
    Text::new(
        "Bu sabit aynı dosyada daha önce tanımlandı. İkinci tanım her zaman uyarı verip yok sayıldı; PHP 8.5'ten beri kullanımdan kalktı ve PHP 9'da hata olacak.",
        "This constant is already declared earlier in the file. The second declaration always warned and was ignored; since PHP 8.5 it's deprecated and PHP 9 will make it an error.",
    ),
);

pub(crate) const CLOSURE_USE: PatternRule = rule(
    "removed.closure_use_variable",
    Text::new("Closure use() ile yasak değişken", "Forbidden variable in closure use()"),
    Severity::Breaking,
    (7, 1),
    Some(Text::new("değişkeni use() listesinden çıkarın", "remove the variable from use()")),
    Text::new(
        "PHP 7.1'den beri $this, süper global değişkenler ve parametreyle aynı adı taşıyan değişkenler use() ile closure'a bağlanamaz; dosya derlenmez.",
        "Since PHP 7.1 $this, superglobals and variables named like a parameter can't be bound with use(); the file doesn't compile.",
    ),
);

pub(crate) const CUSTOM_ASSERT: PatternRule = rule(
    "removed.custom_assert",
    Text::new("Namespace içinde assert() tanımı", "assert() declared in a namespace"),
    Severity::Breaking,
    (8, 0),
    Some(Text::new("fonksiyonu yeniden adlandırın", "rename the function")),
    Text::new(
        "assert() adlı bir fonksiyon tanımlamak PHP 7.3'te kullanımdan kalktı ve PHP 8.0'dan beri derleme hatası veriyor; assert'in özel bir anlamı var.",
        "Declaring a function named assert() was deprecated in PHP 7.3 and is a compile error since PHP 8.0, because assert has special semantics.",
    ),
);

pub(crate) const NULL_CONSTANT_DEFAULT: PatternRule = rule(
    "removed.null_constant_default",
    Text::new("null değerli sabit varsayılan değer", "Default value is a null constant"),
    Severity::Breaking,
    (8, 0),
    Some(Text::new("tipin önüne ? koyun", "add ? before the type")),
    Text::new(
        "Bu sabitin değeri null. PHP 8.0'dan önce böyle bir varsayılan değer parametre tipini nullable yapıyordu; artık yapmıyor ve parametre verilmeden yapılan çağrı TypeError fırlatır.",
        "This constant is null. Before PHP 8.0 such a default made the parameter type nullable; now it doesn't, and a call that omits the argument throws a TypeError.",
    ),
);

pub(crate) const SLEEP_NON_ARRAY: PatternRule = rule(
    "behavior.sleep_non_array",
    Text::new("__sleep() dizi dönmüyor", "__sleep() doesn't return an array"),
    Severity::Behavior,
    (8, 0),
    Some(Text::new("return [];", "return [];")),
    Text::new(
        "__sleep() bir özellik adları dizisi döndürmeli. Başka bir değer döndüğünde serialize() PHP 8.0'dan beri notice yerine uyarı verir ve null serileştirir.",
        "__sleep() must return an array of property names. Since PHP 8.0 serialize() raises a warning instead of a notice for anything else and serializes null.",
    ),
);

pub(crate) const AMBIGUOUS_TRAIT_ALIAS: PatternRule = rule(
    "removed.ambiguous_trait_alias",
    Text::new("Belirsiz trait takma adı", "Ambiguous trait alias"),
    Severity::Breaking,
    (8, 0),
    Some(Text::new("Trait::metot as yeniAd", "Trait::method as newName")),
    Text::new(
        "Bu metot kullanılan birden fazla trait'te var. PHP 8.0'dan beri takma ad hangi trait'in metodunu kastettiğini Trait::metot biçiminde belirtmeli; yoksa sınıf derlenmez.",
        "This method exists in more than one of the used traits. Since PHP 8.0 the alias must say which one with Trait::method, or the class doesn't compile.",
    ),
);

pub(crate) const TRAIT_PROPERTY_DEFAULT: PatternRule = rule(
    "removed.trait_property_default",
    Text::new("Trait özelliğiyle farklı varsayılan değer", "Default value differs from the trait property"),
    Severity::Breaking,
    (7, 2),
    Some(Text::new("trait'teki varsayılan değerin aynısını yazın", "use the same default as the trait")),
    Text::new(
        "Sınıf, kullandığı trait'teki bir özelliği farklı bir varsayılan değerle yeniden tanımlıyor. PHP 7.2'den beri değerler dönüştürülmeden karşılaştırılır; 1 ile '1' bile uyumsuz sayılır ve sınıf derlenmez.",
        "The class redeclares a property of a trait it uses with a different default. Since PHP 7.2 the defaults are compared without casting, so even 1 and '1' are incompatible and the class doesn't compile.",
    ),
);

pub(crate) const TRAIT_STATIC_REDECLARED: PatternRule = rule(
    "behavior.trait_static_property",
    Text::new("Trait üst sınıfın statik özelliğini ayırıyor", "Trait separates a parent's static property"),
    Severity::Behavior,
    (8, 3),
    None,
    Text::new(
        "Bu trait, üst sınıfta da tanımlı bir statik özellik içeriyor. PHP 8.3'ten beri trait'i kullanan sınıf özelliği yeniden tanımlar ve üst sınıfla aynı değeri paylaşmaz.",
        "This trait declares a static property the parent class also has. Since PHP 8.3 the class using the trait redeclares it and no longer shares the parent's value.",
    ),
);

pub(crate) const READONLY_CLONE_REFERENCE: PatternRule = rule(
    "removed.readonly_clone_reference",
    Text::new("__clone() içinde readonly özelliğe referans", "Reference to a readonly property in __clone()"),
    Severity::Breaking,
    (8, 4),
    Some(Text::new("özelliğe doğrudan değer atayın", "assign the property directly")),
    Text::new(
        "PHP 8.4'ten beri __clone() içinde readonly bir özelliğe referans almak dolaylı değişiklik sayılır ve Error fırlatır.",
        "Since PHP 8.4 taking a reference to a readonly property inside __clone() counts as indirect modification and throws an Error.",
    ),
);

pub(crate) const FINAL_BUILTIN: PatternRule = rule(
    "removed.extends_final_class",
    Text::new("final olan yerleşik sınıftan türetme", "Extending a built-in class that's final now"),
    Severity::Breaking,
    (8, 4),
    Some(Text::new("sınıfı türetmek yerine içinde tutun", "wrap the class instead of extending it")),
    Text::new(
        "Bu yerleşik sınıf artık final; ondan türeyen bir sınıf derlenmez.",
        "This built-in class is final now, so a class that extends it doesn't compile.",
    ),
);

pub(crate) const BUILTIN_OVERRIDE: PatternRule = rule(
    "removed.builtin_override_signature",
    Text::new("Yerleşik metodun değişen imzası", "Changed built-in method signature"),
    Severity::Breaking,
    (8, 0),
    Some(Text::new("imzayı yerleşik metoda uydurun", "match the built-in signature")),
    Text::new(
        "Yerleşik metodun imzası değişti: tidy::repairString() ve repairFile() PHP 8.0'da statik oldu, SplFileObject::fwrite()'ın $length parametresi PHP 8.5'te ?int oldu, SoapClient::__doRequest() PHP 8.5'te altıncı bir $uriParserClass parametresi aldı. Uymayan bir alt sınıf metodu derlenmez.",
        "The built-in method's signature changed: tidy::repairString() and repairFile() became static in PHP 8.0, SplFileObject::fwrite()'s $length became ?int in PHP 8.5, and SoapClient::__doRequest() gained a sixth $uriParserClass parameter in PHP 8.5. A subclass method that doesn't match doesn't compile.",
    ),
);
