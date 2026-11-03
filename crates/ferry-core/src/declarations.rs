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

fn close_of(tokens: &[Token], open: usize) -> Option<usize> {
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

struct Param {
    var: usize,
    types: Vec<String>,
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
                out.push(Param { var: start + v, types: type_parts(&ty) });
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

struct Function {
    keyword: usize,
    name: Option<usize>,
    by_ref: bool,
    params: Vec<Param>,
    returns: Option<Vec<String>>,
    body: Option<(usize, usize)>,
}

fn function_at(tokens: &[Token], k: usize) -> Option<Function> {
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
    if tokens.get(after).is_some_and(|t| t.is_ident_ci("use")) && tokens.get(after + 1).is_some_and(|t| t.is_punct("(")) {
        after = close_of(tokens, after + 1)? + 1;
    }
    let mut returns = None;
    let mut end = after;
    if tokens.get(after).is_some_and(|t| t.is_punct(":")) {
        end = (after + 1..tokens.len()).find(|&e| ["{", ";", "=>"].iter().any(|p| tokens[e].is_punct(p)))?;
        returns = Some(type_parts(&tokens[after + 1..end]));
    }
    let body = tokens.get(end).filter(|t| t.is_punct("{")).and_then(|_| close_of(tokens, end)).map(|c| (end, c));
    Some(Function { keyword: k, name, by_ref, params, returns, body })
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

/// Declarations that newer PHP versions reject or warn about. `is_trait` says whether the project declares
/// a fully qualified name as a trait and not as a class.
pub(crate) fn check(tokens: &[Token], is_trait: impl Fn(&str) -> bool) -> Vec<Issue> {
    let mut out = Vec::new();
    let names = crate::index::Names::new(tokens);
    let bodies = type_bodies(tokens);
    let mut interfaces: HashMap<String, HashSet<String>> = HashMap::new();
    for b in bodies.iter().filter(|b| tokens[b.keyword].is_ident_ci("interface")) {
        if let Some(name) = tokens.get(b.keyword + 1) {
            let consts = constants(tokens, b).iter().map(|c| tokens[c.name].text.clone()).collect();
            interfaces.insert(names.declared(b.keyword, &name.text), consts);
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
        }

        if kind == "class" || kind == "enum" {
            let consts = constants(tokens, b);
            let parents: Vec<usize> = header_names(tokens, b, "extends").into_iter().chain(header_names(tokens, b, "implements")).collect();
            for c in &consts {
                let name = &tokens[c.name].text;
                let hidden = c.visibility.as_deref().is_some_and(|v| v != "public");
                let interface_const =
                    parents.iter().any(|&p| interfaces.get(&names.resolve(p, &tokens[p].text)).is_some_and(|s| s.contains(name)));
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
        let static_member = member.kind == TokenKind::Variable
            || member.kind == TokenKind::Ident && tokens.get(k + 3).is_some_and(|n| n.is_punct("(")) && !member.is_ident_ci("class");
        if static_member && is_trait(&names.resolve(k, &t.text)) {
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
