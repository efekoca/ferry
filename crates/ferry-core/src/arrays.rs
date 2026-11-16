//! Expressions, functions and methods whose value is always an array.

use crate::declarations::close_of;
use crate::lexer::{Token, TokenKind};

/// Built-in functions declared to return `array` on PHP 8.0 through 8.5.
const ARRAY_FUNCTIONS: [&str; 53] = [
    "array_change_key_case",
    "array_chunk",
    "array_column",
    "array_combine",
    "array_count_values",
    "array_diff",
    "array_diff_assoc",
    "array_diff_key",
    "array_diff_uassoc",
    "array_diff_ukey",
    "array_fill",
    "array_fill_keys",
    "array_filter",
    "array_flip",
    "array_intersect",
    "array_intersect_assoc",
    "array_intersect_key",
    "array_intersect_uassoc",
    "array_intersect_ukey",
    "array_keys",
    "array_map",
    "array_merge",
    "array_merge_recursive",
    "array_pad",
    "array_replace",
    "array_replace_recursive",
    "array_reverse",
    "array_slice",
    "array_splice",
    "array_udiff",
    "array_udiff_assoc",
    "array_udiff_uassoc",
    "array_uintersect",
    "array_uintersect_assoc",
    "array_uintersect_uassoc",
    "array_unique",
    "array_values",
    "compact",
    "debug_backtrace",
    "explode",
    "func_get_args",
    "get_class_methods",
    "get_class_vars",
    "get_declared_classes",
    "get_defined_vars",
    "get_object_vars",
    "getdate",
    "iterator_to_array",
    "localtime",
    "mb_str_split",
    "range",
    "str_getcsv",
    "str_split",
];

/// Operators that end the expression an array could be read from: what they give back is not the
/// array, or not only it.
const ENDS: [&str; 15] = ["=", "+=", "-=", ".=", "*=", "/=", "%=", "**=", "??=", "|=", "&=", "^=", "<<=", ">>=", ","];
const LOWER: [&str; 9] = ["and", "or", "xor", "print", "yield", "throw", "include", "require", "clone"];

/// What the expression `a..=b` being an array rests on: `None` when it may be something else,
/// otherwise the functions (`f:name`) and methods (`m:name`) that must always return arrays.
pub(crate) fn array_expression(tokens: &[Token], a: usize, b: usize) -> Option<Vec<String>> {
    array_in(tokens, a, b, 0)
}

fn is_cast(tokens: &[Token], a: usize) -> bool {
    tokens[a].is_punct("(")
        && tokens.get(a + 1).is_some_and(|t| t.is_ident_ci("array"))
        && tokens.get(a + 2).is_some_and(|t| t.is_punct(")"))
}

/// The tokens of `a..=b` outside any brackets; an opening bracket stands for its group.
fn top_level(tokens: &[Token], a: usize, b: usize) -> Option<Vec<usize>> {
    let mut out = Vec::new();
    let mut i = a;
    while i <= b {
        out.push(i);
        let t = &tokens[i];
        if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") || t.is_punct("#[") {
            i = close_of(tokens, i).filter(|&c| c <= b)?;
        } else if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            return None;
        }
        i += 1;
    }
    Some(out)
}

fn array_in(tokens: &[Token], a: usize, b: usize, depth: usize) -> Option<Vec<String>> {
    if a > b || depth > 16 {
        return None;
    }
    if tokens[a].is_punct("(") && !is_cast(tokens, a) && close_of(tokens, a) == Some(b) {
        return array_in(tokens, a + 1, b - 1, depth + 1);
    }
    if tokens[a].is_punct("@") {
        return array_in(tokens, a + 1, b, depth + 1);
    }
    let top = top_level(tokens, a, b)?;
    if top.iter().any(|&i| ENDS.iter().any(|e| tokens[i].is_punct(e)) || LOWER.iter().any(|l| tokens[i].is_ident_ci(l))) {
        return None;
    }
    if let Some(at) = top.iter().position(|&i| tokens[i].is_punct("?")) {
        let q = top[at];
        let mut nested = 0usize;
        let colon = top[at + 1..].iter().copied().find(|&i| {
            if tokens[i].is_punct("?") {
                nested += 1;
            } else if tokens[i].is_punct(":") {
                if nested == 0 {
                    return true;
                }
                nested -= 1;
            }
            false
        })?;
        if colon == q + 1 {
            return None;
        }
        let mut deps = array_in(tokens, q + 1, colon - 1, depth + 1)?;
        deps.extend(array_in(tokens, colon + 1, b, depth + 1)?);
        return Some(deps);
    }
    let plus: Vec<usize> = top.iter().copied().filter(|&i| tokens[i].is_punct("+")).collect();
    if !plus.is_empty() {
        let mut from = a;
        for to in plus.iter().map(|&p| p.checked_sub(1)).chain([Some(b)]) {
            let to = to.filter(|&t| t >= from)?;
            if let Some(deps) = array_in(tokens, from, to, depth + 1) {
                return Some(deps);
            }
            from = to + 2;
        }
        return None;
    }
    let joins = ["(", "[", "{", "->", "::"];
    if top.iter().any(|&i| tokens[i].kind == TokenKind::Punct && !joins.iter().any(|j| tokens[i].is_punct(j))) {
        return None;
    }
    if is_cast(tokens, a) || tokens[a].is_punct("[") && close_of(tokens, a) == Some(b) {
        return Some(Vec::new());
    }
    if tokens[a].is_ident_ci("array") && tokens.get(a + 1).is_some_and(|t| t.is_punct("(")) && close_of(tokens, a + 1) == Some(b) {
        return Some(Vec::new());
    }
    if !tokens[b].is_punct(")") {
        return None;
    }
    let open = crate::flow::opener_of(tokens, b)?;
    let name = open.checked_sub(1).filter(|&n| n >= a && tokens[n].kind == TokenKind::Ident)?;
    let lower = tokens[name].text.to_ascii_lowercase();
    if name == a {
        let plain = lower.trim_start_matches('\\');
        if !plain.contains('\\') && ARRAY_FUNCTIONS.binary_search(&plain).is_ok() {
            return Some(Vec::new());
        }
        return Some(vec![format!("f:{}", plain.rsplit('\\').next().unwrap_or(plain))]);
    }
    let join = name.checked_sub(1).filter(|&j| j > a && (tokens[j].is_punct("->") || tokens[j].is_punct("::")))?;
    let own = join == a + 1
        && (tokens[a].kind == TokenKind::Variable && tokens[a].text == "this"
            || ["self", "static", "parent"].iter().any(|s| tokens[a].is_ident_ci(s)));
    let builtin = || crate::builtin::CLASSES.iter().flat_map(|(_, _, methods)| methods.iter()).any(|m| m.0 == lower && m.5 != "array");
    if !own && builtin() {
        return None;
    }
    Some(vec![format!("m:{lower}")])
}

/// The functions and closures declared inside `open..close`, by their keyword and the end of their body.
pub(crate) fn nested_functions(tokens: &[Token], open: usize, close: usize) -> Vec<(usize, usize)> {
    let mut nested: Vec<(usize, usize)> = Vec::new();
    for i in open + 1..close {
        if tokens[i].is_ident_ci("function") && !nested.iter().any(|&(a, b)| a < i && i < b) {
            if let Some(body) = (i..close).find(|&j| tokens[j].is_punct("{")) {
                nested.push((i, close_of(tokens, body).unwrap_or(close)));
            }
        }
    }
    nested
}

/// Whether the last statement of the body `open..close` is a `return` or a `throw`.
fn ends_in_exit(tokens: &[Token], open: usize, close: usize) -> bool {
    if close < open + 2 || !tokens[close - 1].is_punct(";") {
        return false;
    }
    let mut k = close - 2;
    while k > open {
        let t = &tokens[k];
        if t.is_punct(";") || t.is_punct("{") || t.is_punct("}") || t.is_punct(":") {
            break;
        }
        if t.is_punct(")") || t.is_punct("]") {
            match crate::flow::opener_of(tokens, k) {
                Some(o) if o > open => k = o,
                _ => return false,
            }
        }
        k -= 1;
    }
    tokens.get(k + 1).is_some_and(|t| t.is_ident_ci("return") || t.is_ident_ci("throw"))
}

/// Every named function and method declared in the file with a body, keyed `f:name` or `m:name`, and
/// what its result being an array rests on, as [`array_expression`] gives it.
pub(crate) fn array_returns(tokens: &[Token]) -> Vec<(String, Option<Vec<String>>)> {
    let methods = crate::analyzer::method_tokens(tokens);
    let mut out = Vec::new();
    for j in 0..tokens.len() {
        if !tokens[j].is_ident_ci("function") {
            continue;
        }
        let Some(f) = crate::declarations::function_at(tokens, j) else { continue };
        let (Some(name), Some((open, close))) = (f.name, f.body) else { continue };
        let lower = tokens[name].text.to_ascii_lowercase();
        let key = if methods.contains(&j) { format!("m:{lower}") } else { format!("f:{lower}") };
        let shape = if f.returns.as_deref().is_some_and(|r| r == ["array"]) { Some(Vec::new()) } else { body_returns(tokens, open, close) };
        out.push((key, shape));
    }
    out
}

fn body_returns(tokens: &[Token], open: usize, close: usize) -> Option<Vec<String>> {
    let nested = nested_functions(tokens, open, close);
    let own = |i: usize| !nested.iter().any(|&(a, b)| a <= i && i <= b);
    if (open..close).any(|i| own(i) && tokens[i].is_ident_ci("yield")) || !ends_in_exit(tokens, open, close) {
        return None;
    }
    let mut deps = Vec::new();
    for r in open + 1..close {
        if !own(r) || !tokens[r].is_ident_ci("return") {
            continue;
        }
        let mut e = r + 1;
        while e < close && !tokens[e].is_punct(";") {
            if tokens[e].is_punct("(") || tokens[e].is_punct("[") || tokens[e].is_punct("{") {
                e = close_of(tokens, e)?;
            }
            e += 1;
        }
        deps.extend(array_expression(tokens, r + 1, e.checked_sub(1)?)?);
    }
    Some(deps)
}

/// Whether the first argument of the `extract(` call at `k` is an array whenever the call is reached.
pub(crate) fn extracts_array(tokens: &[Token], k: usize, index: &crate::index::ProjectIndex) -> bool {
    let open = k + 1;
    let Some(close) = close_of(tokens, open) else { return false };
    let Some(top) = top_level(tokens, open + 1, close.saturating_sub(1)) else { return false };
    let end = top.into_iter().find(|&i| tokens[i].is_punct(",")).unwrap_or(close);
    end > open + 1 && array_expression(tokens, open + 1, end - 1).is_some_and(|deps| deps.iter().all(|d| index.returns_array(d)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_functions_are_sorted() {
        assert!(ARRAY_FUNCTIONS.windows(2).all(|w| w[0] < w[1]));
    }

    fn shape(expr: &str) -> Option<Vec<String>> {
        let tokens = crate::lexer::tokenize(&format!("<?php {expr};"));
        array_expression(&tokens, 0, tokens.len() - 2)
    }

    #[test]
    fn expressions_that_are_arrays() {
        for expr in [
            "array(1)",
            "['a' => 1]",
            "(array) $x",
            "compact('a', 'b')",
            "\\array_merge($a, $b)",
            "$full + array('escape' => false)",
            "$a ? [] : array_keys($b)",
            "(array_slice($a, 1))",
            "@array_combine($a, $b)",
        ] {
            assert_eq!(shape(expr), Some(Vec::new()), "{expr}");
        }
        assert_eq!(shape("$this->vars()"), Some(vec!["m:vars".to_string()]));
        assert_eq!(shape("$d->trigger_event('x', compact($vars))"), Some(vec!["m:trigger_event".to_string()]));
        assert_eq!(shape("helper($x)"), Some(vec!["f:helper".to_string()]));
        assert_eq!(shape("unpack('N', $s)"), Some(vec!["f:unpack".to_string()]), "an array only if the project's own unpack() is one");
    }

    #[test]
    fn expressions_that_may_be_something_else() {
        for expr in [
            "$x",
            "$this->settings",
            "$a ?: []",
            "$a ?? []",
            "$a = []",
            "(object) $a",
            "$a + $b",
            "$o?->vars()",
            "$q->fetch()",
            "new ArrayObject([])",
            "[1][0]",
            "$f()",
            "$a . []",
        ] {
            assert_eq!(shape(expr), None, "{expr}");
        }
    }

    #[test]
    fn returns_follow_the_functions_they_call() {
        let src = "<?php\nclass D {\n    public function trigger_event($n, $data = array()) {\n        $e = new Data($data);\n        return $e->filtered(array_keys($data));\n    }\n}\nclass Data {\n    public function filtered($keys) { return array_intersect_key($this->data, array_flip($keys)); }\n    public function maybe($a) { if ($a) { return []; } }\n    public function gen() { yield 1; return []; }\n    public function typed(): array { return $this->x; }\n    public function closure() { $f = function () { return 1; }; return []; }\n}\ninterface I { public function trigger_event($n, $data = array()); }\n";
        let found = array_returns(&crate::lexer::tokenize(src));
        let get = |k: &str| found.iter().find(|(key, _)| key == k).map(|(_, s)| s.clone());
        assert_eq!(get("m:trigger_event"), Some(Some(vec!["m:filtered".to_string()])));
        assert_eq!(get("m:filtered"), Some(Some(Vec::new())));
        assert_eq!(get("m:maybe"), Some(None), "it can fall through to null");
        assert_eq!(get("m:gen"), Some(None));
        assert_eq!(get("m:typed"), Some(Some(Vec::new())));
        assert_eq!(get("m:closure"), Some(Some(Vec::new())), "a closure's return isn't the method's");
        assert_eq!(found.iter().filter(|(k, _)| k == "m:trigger_event").count(), 1, "the interface has no body");
    }
}
