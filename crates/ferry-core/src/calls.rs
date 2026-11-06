use crate::declarations::{close_of, function_at, Function};
use crate::i18n::Text;
use crate::index::ProjectIndex;
use crate::lexer::{Token, TokenKind};
use crate::rules::{FixKind, PatternRule, Severity};
use std::collections::HashSet;

pub(crate) const BY_REF_ARGUMENT: &[&str] =
    &["array_pop", "array_push", "array_shift", "array_unshift", "end", "next", "prev", "reset", "rsort", "shuffle", "sort"];
const XML_HANDLERS: &[&str] = &[
    "xml_set_character_data_handler",
    "xml_set_default_handler",
    "xml_set_element_handler",
    "xml_set_end_namespace_decl_handler",
    "xml_set_external_entity_ref_handler",
    "xml_set_notation_decl_handler",
    "xml_set_processing_instruction_handler",
    "xml_set_start_namespace_decl_handler",
    "xml_set_unparsed_entity_decl_handler",
];
const SORTS: &[&str] = &["usort", "uasort", "uksort"];
const VALUE_CALLBACKS: &[(&str, usize)] =
    &[("array_filter", 1), ("array_map", 0), ("array_reduce", 1), ("preg_replace_callback", 1), ("uasort", 1), ("uksort", 1), ("usort", 1)];

fn starts(tokens: &[Token], open: usize) -> Vec<usize> {
    let Some(close) = close_of(tokens, open) else { return Vec::new() };
    let mut out = Vec::new();
    if open + 1 < close {
        out.push(open + 1);
    }
    let mut depth = 0usize;
    for (k, t) in tokens.iter().enumerate().take(close).skip(open + 1) {
        if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") || t.is_punct("#[") {
            depth += 1;
        } else if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            depth -= 1;
        } else if depth == 0 && t.is_punct(",") && k + 1 < close {
            out.push(k + 1);
        }
    }
    out
}

fn open_of(tokens: &[Token], close: usize) -> Option<usize> {
    let mut depth = 0usize;
    (0..=close).rev().find(|&k| {
        if tokens[k].is_punct(")") {
            depth += 1;
        } else if tokens[k].is_punct("(") {
            depth -= 1;
        }
        depth == 0
    })
}

fn closure_at(tokens: &[Token], k: usize) -> Option<Function> {
    let at = if tokens.get(k)?.is_ident_ci("static") { k + 1 } else { k };
    function_at(tokens, at).filter(|f| f.name.is_none())
}

fn top_level_at(tokens: &[Token], from: usize, to: usize, found: impl Fn(&Token) -> bool) -> Vec<usize> {
    let mut depth = 0usize;
    let mut out = Vec::new();
    for (k, t) in tokens.iter().enumerate().take(to).skip(from) {
        if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") {
            depth += 1;
        } else if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            depth = depth.saturating_sub(1);
        } else if depth == 0 && found(t) {
            out.push(k);
        }
    }
    out
}

fn top_level(tokens: &[Token], from: usize, to: usize, found: impl Fn(&Token) -> bool) -> bool {
    !top_level_at(tokens, from, to, found).is_empty()
}

/// The tokens of a callback that only evaluates one expression.
fn single_expression(tokens: &[Token], f: &Function, end: usize) -> Option<(usize, usize)> {
    match f.body {
        Some((open, close)) => {
            let semi = (open..close).find(|&k| tokens[k].is_punct(";"))?;
            (tokens[open + 1].is_ident_ci("return") && semi + 1 == close).then_some((open + 2, semi))
        }
        None => {
            let arrow = (f.keyword..end).find(|&k| tokens[k].is_punct("=>"))?;
            Some((arrow + 1, end))
        }
    }
}

fn boolean_comparison(tokens: &[Token], from: usize, to: usize) -> bool {
    let compares = |t: &Token| ["<", ">", "<=", ">=", "==", "!=", "===", "!==", "<>"].iter().any(|o| t.is_punct(o));
    let other = |t: &Token| ["?", "??", "<=>", "&&", "||", "+", "-", "*", "/"].iter().any(|o| t.is_punct(o));
    top_level(tokens, from, to, compares) && !top_level(tokens, from, to, other)
}

fn sort_comparison(tokens: &[Token], k: usize, close: usize) -> Option<(usize, usize)> {
    let args = starts(tokens, k + 1);
    let end = args.get(2).map_or(close, |&n| n - 1);
    let expr = closure_at(tokens, *args.get(1)?).and_then(|f| single_expression(tokens, &f, end))?;
    boolean_comparison(tokens, expr.0, expr.1).then_some(expr)
}

/// The operator of a comparison that `<=>` can replace without regrouping its operands.
fn ordering_operator(tokens: &[Token], from: usize, to: usize) -> Option<usize> {
    let compares = |t: &Token| ["<", ">", "<=", ">=", "==", "!=", "===", "!==", "<>"].iter().any(|o| t.is_punct(o));
    let looser = |t: &Token| {
        ["&", "|", "^", "=", ".", "%", ","].iter().any(|o| t.is_punct(o)) || ["and", "or", "xor"].iter().any(|w| t.is_ident_ci(w))
    };
    let ops = top_level_at(tokens, from, to, compares);
    let op = *ops.first().filter(|_| ops.len() == 1)?;
    let ordering = ["<", ">", "<=", ">="].iter().any(|o| tokens[op].is_punct(o));
    (ordering && from < op && op + 1 < to && !top_level(tokens, from, to, looser)).then_some(op)
}

/// `(first operand, operator, end)` of each boolean sort comparator that `<=>` can replace.
pub(crate) fn ordering_comparators(tokens: &[Token]) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    for k in 0..tokens.len().saturating_sub(1) {
        let name = tokens[k].text.trim_start_matches('\\').to_ascii_lowercase();
        if tokens[k].kind != TokenKind::Ident || !tokens[k + 1].is_punct("(") || !SORTS.contains(&name.as_str()) {
            continue;
        }
        if k > 0 && ["->", "?->", "::"].iter().any(|p| tokens[k - 1].is_punct(p)) || k > 0 && tokens[k - 1].is_ident_ci("function") {
            continue;
        }
        let expr = close_of(tokens, k + 1).and_then(|close| sort_comparison(tokens, k, close));
        if let Some((from, op, to)) = expr.and_then(|(from, to)| ordering_operator(tokens, from, to).map(|op| (from, op, to))) {
            out.push((from, op, to));
        }
    }
    out
}

/// A variable after `->` or `::` that PHP 5 read together with its offsets and PHP 7 reads left to right.
pub(crate) fn indirect_access(tokens: &[Token], k: usize) -> bool {
    if tokens[k].kind != TokenKind::Variable || !tokens.get(k + 1).is_some_and(|n| n.is_punct("[")) || k == 0 {
        return false;
    }
    let then_call = close_of(tokens, k + 1).is_some_and(|c| tokens.get(c + 1).is_some_and(|n| n.is_punct("(")));
    let own = k >= 2 && ["self", "static", "parent"].iter().any(|w| tokens[k - 2].is_ident_ci(w));
    let created = k >= 3 && tokens[k - 3].is_ident_ci("new");
    tokens[k - 1].is_punct("->") || tokens[k - 1].is_punct("::") && then_call && !own && !created
}

fn fractional(t: &Token) -> bool {
    t.kind == TokenKind::Number
        && !t.text.starts_with("0x")
        && !t.text.starts_with("0X")
        && t.text.replace('_', "").parse::<f64>().is_ok_and(|v| v.fract() != 0.0)
}

fn uses_this(tokens: &[Token], f: &Function) -> bool {
    f.body.is_some_and(|(open, close)| tokens[open..close].iter().any(|t| t.kind == TokenKind::Variable && t.text == "this"))
}

fn is_null(tokens: &[Token], k: usize) -> bool {
    tokens[k].is_ident_ci("null") && tokens.get(k + 1).is_some_and(|t| t.is_punct(")") || t.is_punct(","))
}

/// Literal calls and operators whose result or meaning changed: what a call is compared with, what callbacks
fn unchecked_pdo(tokens: &[Token]) -> Vec<usize> {
    let creates = tokens.windows(2).any(|w| w[0].is_ident_ci("new") && w[1].text.trim_start_matches('\\').eq_ignore_ascii_case("pdo"));
    if !creates || tokens.iter().any(|t| t.text.ends_with("ATTR_ERRMODE")) {
        return Vec::new();
    }
    (2..tokens.len().saturating_sub(1))
        .filter(|&k| {
            let method = ["query", "exec", "prepare"].iter().any(|m| tokens[k].is_ident_ci(m));
            let Some(close) =
                (method && tokens[k - 1].is_punct("->") && tokens[k + 1].is_punct("(")).then(|| close_of(tokens, k + 1)).flatten()
            else {
                return false;
            };
            let after = tokens.get(close + 1);
            let then = tokens.get(close + 2);
            let compared = after.is_some_and(|o| o.is_punct("===")) && then.is_some_and(|v| v.is_ident_ci("false"));
            let dies = after.is_some_and(|o| o.is_ident_ci("or")) && then.is_some_and(|v| v.is_ident_ci("die") || v.is_ident_ci("exit"));
            let kept = k >= 4 && tokens[k - 3].is_punct("=") && tokens[k - 4].kind == TokenKind::Variable;
            let checked_later = kept
                && (close + 2..tokens.len().min(close + 10)).any(|i| {
                    let same = tokens[i].text == tokens[k - 4].text;
                    same && (tokens[i - 1].is_punct("!")
                        || tokens.get(i + 1).is_some_and(|o| o.is_punct("==="))
                            && tokens.get(i + 2).is_some_and(|v| v.is_ident_ci("false")))
                });
            compared || dies || checked_later || k >= 3 && tokens[k - 3].is_punct("!")
        })
        .collect()
}

fn inherited_statics(tokens: &[Token], index: &ProjectIndex) -> Vec<usize> {
    let mut out = Vec::new();
    for (class, open, close) in crate::analyzer::class_bodies(tokens) {
        let Some(name) = tokens.get(class + 1) else { continue };
        let mut k = open + 1;
        while k < close {
            if !tokens[k].is_ident_ci("function") {
                k += 1;
                continue;
            }
            let Some(start) = (k..close).find(|&i| tokens[i].is_punct("{") || tokens[i].is_punct(";")) else { break };
            if tokens[start].is_punct(";") {
                k = start + 1;
                continue;
            }
            let end = close_of(tokens, start).unwrap_or(close);
            let private = tokens[..k]
                .iter()
                .rev()
                .take_while(|t| !t.is_punct(";") && !t.is_punct("{") && !t.is_punct("}"))
                .any(|t| t.is_ident_ci("private"));
            if !private && tokens.get(k + 1).is_some_and(|m| m.kind == TokenKind::Ident && index.inherits_unchanged(&name.text, &m.text)) {
                out.extend((start..end).filter(|&i| tokens[i].is_ident_ci("static") && tokens[i + 1].kind == TokenKind::Variable));
            }
            k = end + 1;
        }
    }
    out
}

/// do, and how literal keys and operands behave.
pub(crate) fn check(tokens: &[Token], index: &ProjectIndex) -> Vec<(usize, &'static PatternRule)> {
    let mut out: Vec<(usize, &'static PatternRule)> = unchecked_pdo(tokens).into_iter().map(|k| (k, &PDO_EXCEPTIONS)).collect();
    out.extend(inherited_statics(tokens, index).into_iter().map(|k| (k, &INHERITED_STATIC)));
    let names = crate::index::Names::new(tokens);
    let spaces = crate::index::namespaces(tokens);
    let mut closures: Vec<(&str, bool, bool)> = Vec::new();
    for k in 0..tokens.len().saturating_sub(2) {
        if tokens[k].kind == TokenKind::Variable && tokens[k + 1].is_punct("=") {
            if let Some(f) = closure_at(tokens, k + 2) {
                closures.push((&tokens[k].text, tokens[k + 2].is_ident_ci("static"), uses_this(tokens, &f)));
            }
        }
    }
    let xml_object = tokens.iter().any(|t| t.is_ident_ci("xml_set_object"));
    let list_seen = |base: &mut HashSet<String>, name: &str| !base.insert(name.to_string());

    for k in 0..tokens.len() {
        let t = &tokens[k];
        let prev = k.checked_sub(1).map(|p| &tokens[p]);
        let next = tokens.get(k + 1);
        let open_call = next.is_some_and(|n| n.is_punct("("));
        let member = prev.is_some_and(|p| p.is_punct("->") || p.is_punct("::") || p.is_punct("?->"));
        let declared = prev.is_some_and(|p| p.is_ident_ci("function") || p.is_ident_ci("new"));

        if t.is_punct("%") {
            if prev.is_some_and(fractional) || next.is_some_and(fractional) {
                out.push((k, &FLOAT_TO_INT));
            }
        } else if fractional(t) && prev.is_some_and(|p| p.is_punct("[")) && next.is_some_and(|n| n.is_punct("]")) {
            let base = k.checked_sub(2).map(|b| &tokens[b]);
            if base.is_some_and(|b| b.kind == TokenKind::Variable || b.is_punct("]")) {
                out.push((k, &FLOAT_TO_INT));
            }
        }

        if indirect_access(tokens, k) {
            out.push((k, &INDIRECT_ORDER));
        }

        if t.kind == TokenKind::Ident && !member && !declared && !open_call && !next.is_some_and(|n| n.is_punct("::")) {
            let word = !prev.is_some_and(|p| p.is_ident_ci("const") || p.is_ident_ci("class") || p.is_ident_ci("function"));
            if (t.text == "SID" || t.text == "NIL") && word && !index.defines_constant_in(crate::index::namespace_at(&spaces, k), &t.text) {
                out.push((k, if t.text == "SID" { &SID_CONSTANT } else { &NIL_CONSTANT }));
            }
        }

        let literal_array = t.is_punct("[") && !prev.is_some_and(|p| p.kind == TokenKind::Variable || p.is_punct("]") || p.is_punct(")"));
        let array_call = t.is_ident_ci("array") && open_call && !member && !declared;
        if literal_array || array_call {
            let open = if array_call { k + 1 } else { k };
            let items = starts(tokens, open);
            let end = close_of(tokens, open).unwrap_or(open);
            let keyed = |i: usize| {
                let to = items.get(i + 1).map_or(end, |&n| n - 1);
                top_level(tokens, items[i], to, |t| t.is_punct("=>"))
            };
            let negative = items.first().is_some_and(|&f| tokens[f].is_punct("-") && tokens[f + 1].kind == TokenKind::Number);
            if negative && keyed(0) && (1..items.len()).any(|i| !keyed(i)) {
                out.push((k, &crate::arguments::NEGATIVE_KEY));
            }
        }

        if t.is_ident_ci("list") && open_call {
            let Some(close) = close_of(tokens, k + 1) else { continue };
            let mut bases = HashSet::new();
            let appended = (k + 2..close.saturating_sub(2)).any(|j| {
                tokens[j].kind == TokenKind::Variable
                    && tokens[j + 1].is_punct("[")
                    && tokens[j + 2].is_punct("]")
                    && list_seen(&mut bases, &tokens[j].text)
            });
            if appended && tokens.get(close + 1).is_some_and(|n| n.is_punct("=")) {
                out.push((k, &LIST_ORDER));
            }
        }

        if t.is_ident_ci("bindto") && open_call && prev.is_some_and(|p| p.is_punct("->")) {
            let null_arg = tokens.get(k + 2).is_some_and(|_| is_null(tokens, k + 2));
            let receiver = tokens.get(k.wrapping_sub(2));
            let (is_static, this) = match receiver {
                Some(r) if r.kind == TokenKind::Variable => {
                    closures.iter().rev().find(|c| c.0 == r.text).map_or((false, false), |c| (c.1, c.2))
                }
                Some(r) if r.is_punct(")") => {
                    let open = open_of(tokens, k - 2);
                    let f = open.and_then(|o| closure_at(tokens, o + 1));
                    (open.is_some_and(|o| tokens[o + 1].is_ident_ci("static")) && f.is_some(), f.is_some_and(|f| uses_this(tokens, &f)))
                }
                _ => (false, false),
            };
            if is_static && !null_arg || !is_static && this && null_arg {
                out.push((k, &CLOSURE_BIND));
            }
        }

        if t.text.trim_start_matches('\\').eq_ignore_ascii_case("closure")
            && next.is_some_and(|n| n.is_punct("::"))
            && tokens.get(k + 2).is_some_and(|m| m.is_ident_ci("bind"))
            && tokens.get(k + 3).is_some_and(|o| o.is_punct("("))
        {
            let args = starts(tokens, k + 3);
            let f = args.first().and_then(|&a| closure_at(tokens, a));
            if let (Some(f), Some(&a), Some(&b)) = (f, args.first(), args.get(1)) {
                if tokens[a].is_ident_ci("static") != is_null(tokens, b) && (!is_null(tokens, b) || uses_this(tokens, &f)) {
                    out.push((k, &CLOSURE_BIND));
                }
            }
        }

        if t.is_ident_ci("arrayobject") && open_call && prev.is_some_and(|p| p.is_ident_ci("new")) {
            let case = tokens.get(k + 2).filter(|c| c.kind == TokenKind::Ident && tokens.get(k + 3).is_some_and(|n| n.is_punct("::")));
            if case.is_some_and(|c| index.is_enum(&names.resolve(k + 2, &c.text))) {
                out.push((k, &ARRAY_OBJECT_ENUM));
            }
        }

        if t.kind != TokenKind::Ident || !open_call || member || declared {
            continue;
        }
        let name = t.text.trim_start_matches('\\').to_ascii_lowercase();
        let Some(close) = close_of(tokens, k + 1) else { continue };
        let args = starts(tokens, k + 1);
        let after = tokens.get(close + 1);
        let before = (k >= 2).then(|| (&tokens[k - 1], &tokens[k - 2]));

        if name == "substr" {
            let strict = |o: &Token| o.is_punct("===") || o.is_punct("!==");
            let right = after.is_some_and(strict) && tokens.get(close + 2).is_some_and(|v| v.is_ident_ci("false"));
            let left = before.is_some_and(|(o, v)| strict(o) && v.is_ident_ci("false"));
            if right || left {
                out.push((k, &SUBSTR_FALSE));
            }
        }
        if name == "pcntl_sigwaitinfo" || name == "pcntl_sigtimedwait" {
            let equal = |o: &Token| ["===", "!==", "==", "!="].iter().any(|p| o.is_punct(p));
            let minus = tokens.get(close + 2).is_some_and(|m| m.is_punct("-")) && tokens.get(close + 3).is_some_and(|n| n.text == "1");
            if after.is_some_and(equal) && minus {
                out.push((k, &PCNTL_FAILURE));
            }
        }
        if BY_REF_ARGUMENT.contains(&name.as_str())
            && args.len() == 1
            && tokens[k + 2].is_punct("(")
            && close_of(tokens, k + 2) == Some(close - 1)
            && tokens[k + 3].kind == TokenKind::Ident
            && tokens[k + 4].is_punct("(")
        {
            out.push((k, &PARENTHESIZED_REFERENCE));
        }
        if ["mysqli_connect", "mysqli_query", "mysqli_real_connect"].contains(&name.as_str()) && !index.sets_mysqli_report() {
            let compared = after.is_some_and(|o| o.is_punct("===")) && tokens.get(close + 2).is_some_and(|v| v.is_ident_ci("false"));
            let dies = after.is_some_and(|o| o.is_ident_ci("or"))
                && tokens.get(close + 2).is_some_and(|v| v.is_ident_ci("die") || v.is_ident_ci("exit"));
            if compared || dies || prev.is_some_and(|p| p.is_punct("!")) {
                out.push((k, &MYSQLI_EXCEPTIONS));
            }
        }
        if name == "is_callable" {
            let first = args.first().copied().unwrap_or(close);
            let quoted = |t: &Token| t.literal().map(str::to_string);
            let at = if tokens[first].is_ident_ci("array") { first + 1 } else { first };
            let pair = (tokens[at].is_punct("[") || tokens[at].is_punct("("))
                .then(|| starts(tokens, at))
                .filter(|items| items.len() == 2)
                .and_then(|items| {
                    let class = quoted(&tokens[items[0]]).or_else(|| {
                        (tokens[items[0]].kind == TokenKind::Ident && tokens.get(items[0] + 2).is_some_and(|c| c.is_ident_ci("class")))
                            .then(|| names.resolve(items[0], &tokens[items[0]].text))
                    });
                    class.zip(quoted(&tokens[items[1]]))
                })
                .or_else(|| quoted(&tokens[first]).and_then(|s| s.split_once("::").map(|(c, m)| (c.to_string(), m.to_string()))));
            if let Some((class, method)) = pair {
                let fqn = class.trim_start_matches('\\').to_ascii_lowercase();
                if index.instance_method(&fqn, &method.to_ascii_lowercase()) == Some(true) {
                    out.push((k, &crate::rules::patterns::STATIC_CALL_TO_INSTANCE_METHOD));
                }
            }
        }
        if XML_HANDLERS.contains(&name.as_str()) {
            let text = |a: &usize| tokens[*a].literal();
            let stale = args.iter().skip(1).filter_map(text).any(|s| s.is_empty() || xml_object && !index.declares_function("", s, true));
            if stale {
                out.push((k, &XML_HANDLER_STRING));
            }
        }
        if name == "ob_start" {
            if let Some((open, end)) = args.first().and_then(|&a| closure_at(tokens, a)).and_then(|f| f.body) {
                if tokens[open..end].iter().any(|b| b.is_ident_ci("echo") || b.is_ident_ci("print")) {
                    out.push((k, &HANDLER_OUTPUT));
                }
            }
        }
        if SORTS.contains(&name.as_str()) {
            if let Some((from, to)) = sort_comparison(tokens, k, close) {
                match ordering_operator(tokens, from, to) {
                    Some(op) => out.push((op, &BOOL_COMPARATOR)),
                    None => out.push((k, &BOOL_EQUALITY_COMPARATOR)),
                }
            }
        }
        if let Some((_, slot)) = VALUE_CALLBACKS.iter().find(|(f, _)| *f == name) {
            let by_ref = args
                .get(*slot)
                .and_then(|&a| closure_at(tokens, a))
                .is_some_and(|f| f.params.iter().any(|p| tokens[p.var - 1].is_punct("&")));
            if by_ref {
                out.push((k, &REFERENCE_CALLBACK));
            }
        }
        if name == "stream_wrapper_register" {
            let class = args.get(1).and_then(|&a| {
                let c = &tokens[a];
                match c.kind {
                    TokenKind::Ident if tokens.get(a + 1).is_some_and(|n| n.is_punct("::")) => Some(c.text.clone()),
                    _ => c.literal().map(str::to_string),
                }
            });
            let short = class.map(|c| c.rsplit('\\').next().unwrap_or_default().to_ascii_lowercase());
            let lacks = short
                .and_then(|s| index.class(&s).cloned())
                .is_some_and(|c| c.parent.is_none() && c.traits.is_empty() && !c.methods.contains("stream_set_option"));
            if lacks {
                out.push((k, &STREAM_SET_OPTION));
            }
        }
    }
    out
}

const fn rule(id: &'static str, title: Text, severity: Severity, since: (u32, u32), replacement: Option<Text>, note: Text) -> PatternRule {
    PatternRule { id, title, severity, since, replacement, fix: FixKind::Manual, note }
}

const SUBSTR_FALSE: PatternRule = rule(
    "behavior.substr_false",
    Text::new("substr() false ile karşılaştırılıyor", "substr() compared with false"),
    Severity::Behavior,
    (7, 0),
    Some(Text::new("=== '' ile karşılaştırın", "compare with === ''")),
    Text::new(
        "PHP 7.0'da substr() sınırda kesilen metin için false yerine boş metin döndürmeye başladı; PHP 8.0'dan beri hiç false dönmüyor.",
        "PHP 7.0 made substr() return an empty string instead of false when it cuts on the boundary, and since PHP 8.0 it never returns false.",
    ),
);

const INDIRECT_ORDER: PatternRule = rule(
    "behavior.indirect_access_order",
    Text::new("Dolaylı erişim sırası", "Indirect access order"),
    Severity::Behavior,
    (7, 0),
    Some(Text::new("$foo->{$bar['baz']}", "$foo->{$bar['baz']}")),
    Text::new(
        "PHP 7.0'dan beri dolaylı erişim soldan sağa okunur: $foo->$bar['baz'] artık ($foo->$bar)['baz'] demek. Eski anlam için süslü parantez kullanın.",
        "Since PHP 7.0 indirect access reads left to right, so $foo->$bar['baz'] now means ($foo->$bar)['baz']. Use braces for the old meaning.",
    ),
);

const SID_CONSTANT: PatternRule = rule(
    "deprecated.sid_constant",
    Text::new("SID sabiti", "SID constant"),
    Severity::Deprecated,
    (8, 4),
    Some(Text::new("oturum kimliğini çerezle taşıyın", "keep the session ID in a cookie")),
    Text::new(
        "SID sabiti, GET ve POST üzerinden oturumlarla birlikte PHP 8.4'te kullanımdan kalktı.",
        "The SID constant is deprecated since PHP 8.4, along with GET and POST sessions.",
    ),
);

const NIL_CONSTANT: PatternRule = rule(
    "deprecated.imap_nil_constant",
    Text::new("NIL sabiti", "NIL constant"),
    Severity::Deprecated,
    (8, 1),
    None,
    Text::new(
        "imap eklentisinin NIL sabiti PHP 8.1'de kullanımdan kalktı.",
        "The imap extension's NIL constant is deprecated since PHP 8.1.",
    ),
);

const XML_HANDLER_STRING: PatternRule = rule(
    "deprecated.xml_handler_string",
    Text::new("xml_set_*_handler() çağrılamayan metinle", "xml_set_*_handler() with a non-callable string"),
    Severity::Deprecated,
    (8, 4),
    Some(Text::new("[$object, 'method'] ya da null verin", "pass [$object, 'method'] or null")),
    Text::new(
        "xml_set_*_handler() fonksiyonlarına çağrılamayan bir metin, boş metin ya da xml_set_object() ile çözülecek metot adı vermek PHP 8.4'te kullanımdan kalktı.",
        "Passing a non-callable string, an empty string or a method name left for xml_set_object() to resolve to xml_set_*_handler() is deprecated since PHP 8.4.",
    ),
);

const ARRAY_OBJECT_ENUM: PatternRule = rule(
    "removed.array_object_enum",
    Text::new("ArrayObject enum ile", "ArrayObject with an enum"),
    Severity::Breaking,
    (8, 5),
    None,
    Text::new(
        "PHP 8.5'ten beri ArrayObject enum kabul etmez; InvalidArgumentException fırlatır.",
        "Since PHP 8.5 ArrayObject doesn't accept enums and throws an InvalidArgumentException.",
    ),
);

const MYSQLI_EXCEPTIONS: PatternRule = rule(
    "behavior.mysqli_default_exceptions",
    Text::new("mysqli hatası artık istisna", "mysqli errors are exceptions now"),
    Severity::Behavior,
    (8, 1),
    Some(Text::new("try/catch kullanın ya da mysqli_report(MYSQLI_REPORT_OFF) çağırın", "use try/catch or call mysqli_report(MYSQLI_REPORT_OFF)")),
    Text::new(
        "PHP 8.1'den beri mysqli varsayılan olarak hata durumunda false döndürmek yerine mysqli_sql_exception fırlatır, bu yüzden bu kontrole hiç ulaşılmaz.",
        "Since PHP 8.1 mysqli throws mysqli_sql_exception by default instead of returning false on failure, so this check is never reached.",
    ),
);

const PDO_EXCEPTIONS: PatternRule = rule(
    "behavior.pdo_default_exceptions",
    Text::new("PDO hatası artık istisna", "PDO errors are exceptions now"),
    Severity::Behavior,
    (8, 0),
    Some(Text::new("try/catch kullanın ya da PDO::ATTR_ERRMODE ayarlayın", "use try/catch or set PDO::ATTR_ERRMODE")),
    Text::new(
        "PHP 8.0'dan beri PDO varsayılan olarak hata durumunda false döndürmek yerine PDOException fırlatır, bu yüzden bu kontrole hiç ulaşılmaz.",
        "Since PHP 8.0 PDO throws PDOException by default instead of returning false on failure, so this check is never reached.",
    ),
);

const INHERITED_STATIC: PatternRule = rule(
    "behavior.inherited_static_variable",
    Text::new("Alt sınıfla paylaşılan static değişken", "Static variable shared with a subclass"),
    Severity::Behavior,
    (8, 1),
    Some(Text::new("sınıfa göre anahtarlı bir static dizi kullanın", "key a static array by static::class")),
    Text::new(
        "PHP 8.1'den beri miras alınan metottaki static değişken üst sınıfla ortaktır; alt sınıf başına ayrı kopya olmaz. Sınıf başına singleton ve önbellekler aynı değeri paylaşır.",
        "Since PHP 8.1 a static variable in an inherited method is shared with the parent, with no separate copy per subclass. Per-class singletons and caches end up sharing one value.",
    ),
);

const PCNTL_FAILURE: PatternRule = rule(
    "behavior.pcntl_failure_value",
    Text::new("pcntl hata değeri", "pcntl failure value"),
    Severity::Behavior,
    (8, 4),
    Some(Text::new("=== false ile karşılaştırın", "compare with === false")),
    Text::new(
        "PHP 8.4'ten beri pcntl_sigwaitinfo() ve pcntl_sigtimedwait() hata durumunda -1 değil, false döndürür.",
        "Since PHP 8.4 pcntl_sigwaitinfo() and pcntl_sigtimedwait() return false on failure instead of -1.",
    ),
);

const FLOAT_TO_INT: PatternRule = rule(
    "deprecated.implicit_float_to_int",
    Text::new("Kesirli sayı tamsayıya dönüşüyor", "Fractional number converted to an integer"),
    Severity::Deprecated,
    (8, 1),
    Some(Text::new("(int) ile açıkça dönüştürün", "cast it with (int)")),
    Text::new(
        "Kesirli bir sayının dizi anahtarı ya da % işleneni olarak örtük şekilde tamsayıya dönüşmesi PHP 8.1'de kullanımdan kalktı.",
        "Implicitly converting a fractional number to an integer, as an array key or % operand, is deprecated since PHP 8.1.",
    ),
);

const LIST_ORDER: PatternRule = rule(
    "behavior.list_assignment_order",
    Text::new("list() atama sırası", "list() assignment order"),
    Severity::Behavior,
    (7, 0),
    None,
    Text::new(
        "PHP 7.0'dan beri list() soldan sağa atar. list($a[], $a[]) artık öğeleri PHP 5'tekinin tersi sırayla ekler.",
        "Since PHP 7.0 list() assigns left to right, so list($a[], $a[]) appends in the opposite order to PHP 5.",
    ),
);

const CLOSURE_BIND: PatternRule = rule(
    "deprecated.closure_bind",
    Text::new("Closure yanlış bağlanıyor", "Invalid closure binding"),
    Severity::Deprecated,
    (8, 5),
    None,
    Text::new(
        "Statik bir closure'a nesne bağlamak ya da $this kullanan bir closure'dan $this'i kaldırmak hep uyarı verip null döndürüyordu; PHP 8.5'te kullanımdan kalktı ve PHP 9'da hata olacak.",
        "Binding an object to a static closure, or unbinding $this from a closure that uses it, always warned and returned null; it's deprecated in PHP 8.5 and will be an error in PHP 9.",
    ),
);

const PARENTHESIZED_REFERENCE: PatternRule = rule(
    "behavior.parenthesized_reference",
    Text::new("Parantez referans uyarısını gizlemiyor", "Parentheses no longer hide the reference notice"),
    Severity::Behavior,
    (7, 0),
    Some(Text::new("sonucu önce bir değişkene atayın", "assign the result to a variable first")),
    Text::new(
        "PHP 7.0'dan beri fazladan parantez davranışı değiştirmez; fonksiyon sonucunu referansla geçmek E_NOTICE verir.",
        "Since PHP 7.0 extra parentheses change nothing, so passing a function result by reference raises an E_NOTICE.",
    ),
);

const HANDLER_OUTPUT: PatternRule = rule(
    "deprecated.output_in_ob_handler",
    Text::new("Çıktı tamponu işleyicisinde çıktı", "Output inside an output handler"),
    Severity::Deprecated,
    (8, 5),
    Some(Text::new("metni dönüş değerine ekleyin", "add the text to the return value")),
    Text::new(
        "Kullanıcı tanımlı çıktı işleyicisinde echo ya da print ile çıktı üretmek PHP 8.5'te kullanımdan kalktı.",
        "Producing output with echo or print inside a user output handler is deprecated since PHP 8.5.",
    ),
);

const BOOL_COMPARATOR: PatternRule = PatternRule { fix: FixKind::Likely, ..BOOL_EQUALITY_COMPARATOR };

const BOOL_EQUALITY_COMPARATOR: PatternRule = rule(
    "deprecated.bool_sort_comparator",
    Text::new("Sıralama fonksiyonu bool dönüyor", "Sort comparator returns a boolean"),
    Severity::Deprecated,
    (8, 0),
    Some(Text::new("$a <=> $b", "$a <=> $b")),
    Text::new(
        "Karşılaştırma fonksiyonundan true ya da false döndürmek PHP 8.0'da kullanımdan kalktı; sıfırdan küçük, sıfır ya da sıfırdan büyük bir tamsayı döndürün.",
        "Returning true or false from a comparison function is deprecated since PHP 8.0; return an integer below, equal to or above zero.",
    ),
);

const REFERENCE_CALLBACK: PatternRule = rule(
    "behavior.reference_callback",
    Text::new("Callback parametresi referansla", "Callback parameter by reference"),
    Severity::Behavior,
    (8, 0),
    Some(Text::new("& işaretini silin", "remove the &")),
    Text::new(
        "Bu fonksiyon callback'e referans geçirmez. PHP 8.0'dan beri referans bekleyen bir callback her çağrıda uyarı verir.",
        "This function doesn't pass references to its callback. Since PHP 8.0 a callback that expects a reference warns on every call.",
    ),
);

const STREAM_SET_OPTION: PatternRule = rule(
    "behavior.stream_set_option_missing",
    Text::new("Akış sarmalayıcısında stream_set_option() yok", "Stream wrapper without stream_set_option()"),
    Severity::Behavior,
    (7, 4),
    Some(Text::new(
        "public function stream_set_option($option, $arg1, $arg2) { return false; }",
        "public function stream_set_option($option, $arg1, $arg2) { return false; }",
    )),
    Text::new(
        "PHP 7.4'ten beri include ve require akışlarda stream_set_option() çağırır; sarmalayıcıda yoksa uyarı verir.",
        "Since PHP 7.4 include and require call stream_set_option() on streams, and a wrapper without it raises a warning.",
    ),
);
