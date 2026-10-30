use crate::lexer::{Token, TokenKind};
use crate::rules::{Change, FUNCTIONS};
use crate::version::PhpVersion;

pub type Span = ((usize, usize), (usize, usize));

const PECL: [&str; 6] = ["removed.mcrypt", "removed.imap", "removed.pspell", "removed.oci8", "removed.xmlrpc", "removed.interbase"];

pub fn dead_spans(tokens: &[Token], target: PhpVersion) -> Vec<Span> {
    let mut out = Vec::new();
    for i in 0..tokens.len() {
        if !(tokens[i].is_ident_ci("if") || tokens[i].is_ident_ci("elseif")) {
            continue;
        }
        if tokens[i].is_ident_ci("if") && i > 0 && tokens[i - 1].is_ident_ci("else") {
            continue;
        }
        let mut k = i;
        let mut taken = false;
        while let Some((cond, body_start, body_end)) = branch(tokens, k) {
            let value = if taken { Some(false) } else { evaluate(&tokens[cond.0..cond.1], target) };
            if value == Some(false) {
                out.push(span(tokens, body_start, body_end));
            }
            taken |= value == Some(true);
            let next = body_end + 1;
            let chained = tokens.get(next).is_some_and(|t| t.is_ident_ci("elseif") || t.is_ident_ci("else"));
            if k == i && value == Some(true) && !chained && exits(tokens, body_start, body_end) && !in_switch(tokens, i) {
                let end = block_end(tokens, next);
                if end > next {
                    out.push(span(tokens, next, end - 1));
                }
            }
            match tokens.get(next) {
                Some(t) if t.is_ident_ci("elseif") => k = next,
                Some(t) if t.is_ident_ci("else") && tokens.get(next + 1).is_some_and(|n| n.is_ident_ci("if")) => k = next + 1,
                Some(t) if t.is_ident_ci("else") => {
                    if taken {
                        if let Some(end) = statement_end(tokens, next + 1) {
                            out.push(span(tokens, next + 1, end));
                        }
                    }
                    break;
                }
                _ => break,
            }
        }
    }
    out
}

pub fn dead_expressions(tokens: &[Token], target: PhpVersion) -> Vec<Span> {
    let mut out = Vec::new();
    for (k, t) in tokens.iter().enumerate() {
        let and = t.is_punct("&&") || t.is_ident_ci("and");
        let or = t.is_punct("||") || t.is_ident_ci("or");
        if and || or {
            let left = expr_start(tokens, k, if and { &["||", "or"] } else { &[] });
            let right = expr_end(tokens, k + 1, if and { &["||", "or"] } else { &[] });
            if left < k && right > k + 1 && evaluate(&tokens[left..k], target) == Some(!and) {
                out.push(span(tokens, k + 1, right - 1));
            }
        } else if t.is_punct("?") && !tokens.get(k + 1).is_some_and(|n| n.is_punct(":")) {
            let left = expr_start(tokens, k, &["||", "or", "&&", "and"]);
            let left = if left < k && tokens[left..k].iter().any(|x| x.is_punct("&&") || x.is_punct("||")) {
                expr_start(tokens, k, &[])
            } else {
                left
            };
            let Some(colon) = ternary_colon(tokens, k) else { continue };
            let end = expr_end(tokens, colon + 1, &[]);
            match (left < k).then(|| evaluate(&tokens[left..k], target)).flatten() {
                Some(false) if colon > k + 1 => out.push(span(tokens, k + 1, colon - 1)),
                Some(true) if end > colon + 1 => out.push(span(tokens, colon + 1, end - 1)),
                _ => {}
            }
        }
    }
    out
}

const EXPR_STOPS: [&str; 16] = [";", ",", "=", "=>", "?", ":", "??=", "return", "echo", "print", "yield", "throw", "case", "and", "or", "xor"];

fn stops(t: &Token, extra: &[&str]) -> bool {
    t.kind == TokenKind::CloseTag
        || EXPR_STOPS.iter().chain(extra).any(|s| t.is_punct(s) || (t.kind == TokenKind::Ident && t.is_ident_ci(s)))
}

fn expr_start(tokens: &[Token], at: usize, extra: &[&str]) -> usize {
    let mut depth = 0usize;
    let mut k = at;
    while k > 0 {
        let t = &tokens[k - 1];
        if t.is_punct(")") || t.is_punct("]") {
            depth += 1;
        } else if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") || t.is_punct("}") {
            if depth == 0 || t.is_punct("{") || t.is_punct("}") {
                break;
            }
            depth -= 1;
        } else if depth == 0 && stops(t, extra) {
            break;
        }
        k -= 1;
    }
    k
}

fn expr_end(tokens: &[Token], from: usize, extra: &[&str]) -> usize {
    let mut depth = 0usize;
    let mut k = from;
    while k < tokens.len() {
        let t = &tokens[k];
        if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") {
            depth += 1;
        } else if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            if depth == 0 {
                break;
            }
            depth -= 1;
        } else if depth == 0 && stops(t, extra) {
            break;
        }
        k += 1;
    }
    k
}

fn ternary_colon(tokens: &[Token], question: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut nested = 0usize;
    for (k, t) in tokens.iter().enumerate().skip(question + 1) {
        if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") {
            depth += 1;
        } else if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            depth = depth.checked_sub(1)?;
        } else if depth == 0 && t.is_punct("?") {
            nested += 1;
        } else if depth == 0 && t.is_punct(":") {
            if nested == 0 {
                return Some(k);
            }
            nested -= 1;
        } else if depth == 0 && (t.is_punct(";") || t.kind == TokenKind::CloseTag) {
            return None;
        }
    }
    None
}

fn exits(tokens: &[Token], start: usize, end: usize) -> bool {
    let leaves = |t: &Token| ["return", "throw", "exit", "die"].iter().any(|k| t.is_ident_ci(k));
    if !tokens[start].is_punct("{") {
        return leaves(&tokens[start]);
    }
    if end < 2 || !tokens[end - 1].is_punct(";") {
        return false;
    }
    let mut depth = 0usize;
    let mut stmt = start + 1;
    for k in start + 1..end {
        let t = &tokens[k];
        if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") {
            depth += 1;
        } else if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            depth = depth.saturating_sub(1);
            if depth == 0 && t.is_punct("}") {
                stmt = k + 1;
            }
        } else if depth == 0 && t.is_punct(";") {
            if k == end - 1 {
                return tokens.get(stmt).is_some_and(leaves);
            }
            stmt = k + 1;
        }
    }
    false
}

fn block_end(tokens: &[Token], from: usize) -> usize {
    let mut depth = 0usize;
    for (k, t) in tokens.iter().enumerate().skip(from) {
        if t.is_punct("{") {
            depth += 1;
        } else if t.is_punct("}") {
            if depth == 0 {
                return k;
            }
            depth -= 1;
        }
    }
    tokens.len()
}

fn in_switch(tokens: &[Token], at: usize) -> bool {
    let mut depth = 0usize;
    for k in (0..at).rev() {
        if tokens[k].is_punct("}") {
            depth += 1;
        } else if tokens[k].is_punct("{") {
            if depth == 0 {
                if k == 0 || !tokens[k - 1].is_punct(")") {
                    return false;
                }
                let mut d = 0usize;
                let open = (0..k).rev().find(|&j| {
                    if tokens[j].is_punct(")") {
                        d += 1;
                    } else if tokens[j].is_punct("(") {
                        d -= 1;
                    }
                    d == 0
                });
                return open.is_some_and(|o| o > 0 && tokens[o - 1].is_ident_ci("switch"));
            }
            depth -= 1;
        }
    }
    false
}

fn span(tokens: &[Token], a: usize, b: usize) -> Span {
    ((tokens[a].line, tokens[a].column), (tokens[b].line, tokens[b].column))
}

fn branch(tokens: &[Token], kw: usize) -> Option<((usize, usize), usize, usize)> {
    let open = kw + 1;
    if !tokens.get(open)?.is_punct("(") {
        return None;
    }
    let close = matching(tokens, open)?;
    let body = close + 1;
    let end = statement_end(tokens, body)?;
    Some(((open + 1, close), body, end))
}

fn matching(tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (k, t) in tokens.iter().enumerate().skip(open) {
        if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") || t.is_punct("#[") {
            depth += 1;
        } else if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(k);
            }
        }
    }
    None
}

fn statement_end(tokens: &[Token], start: usize) -> Option<usize> {
    let first = tokens.get(start)?;
    if first.is_punct("{") {
        return matching(tokens, start);
    }
    if first.is_punct(":") {
        return None;
    }
    let mut depth = 0usize;
    for (k, t) in tokens.iter().enumerate().skip(start) {
        if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") {
            depth += 1;
        } else if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            depth = depth.checked_sub(1)?;
        } else if depth == 0 && (t.is_punct(";") || t.kind == TokenKind::CloseTag) {
            return Some(k);
        }
    }
    None
}

fn split<'a>(cond: &'a [Token], ops: &[&str]) -> Vec<&'a [Token]> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (k, t) in cond.iter().enumerate() {
        if t.is_punct("(") || t.is_punct("[") {
            depth += 1;
        } else if t.is_punct(")") || t.is_punct("]") {
            depth = depth.saturating_sub(1);
        } else if depth == 0 && (ops.iter().any(|op| t.is_punct(op) || t.is_ident_ci(op))) {
            parts.push(&cond[start..k]);
            start = k + 1;
        }
    }
    parts.push(&cond[start..]);
    parts
}

fn evaluate(cond: &[Token], target: PhpVersion) -> Option<bool> {
    let any = split(cond, &["||", "or", "xor"]);
    if any.len() > 1 {
        if cond.iter().any(|t| t.is_ident_ci("xor")) {
            return None;
        }
        let values: Vec<_> = any.iter().map(|p| evaluate(p, target)).collect();
        return if values.contains(&Some(true)) {
            Some(true)
        } else if values.iter().all(|v| *v == Some(false)) {
            Some(false)
        } else {
            None
        };
    }
    let all = split(cond, &["&&", "and"]);
    if all.len() > 1 {
        let values: Vec<_> = all.iter().map(|p| evaluate(p, target)).collect();
        return if values.contains(&Some(false)) {
            Some(false)
        } else if values.iter().all(|v| *v == Some(true)) {
            Some(true)
        } else {
            None
        };
    }
    atom(cond, target)
}

fn atom(cond: &[Token], target: PhpVersion) -> Option<bool> {
    let first = cond.first()?;
    if first.is_punct("!") {
        return atom(&cond[1..], target).map(|v| !v);
    }
    if first.is_punct("(") && matching(cond, 0) == Some(cond.len() - 1) {
        return evaluate(&cond[1..cond.len() - 1], target);
    }
    if let Some(k) = cond.iter().position(|t| ["<", "<=", ">", ">="].iter().any(|op| t.is_punct(op))) {
        let (left, right) = (&cond[..k], &cond[k + 1..]);
        let op = cond[k].text.as_str();
        if let (Some(scale), Some(n)) = (version_constant(left), number(right)) {
            return compare(target, scale, op, n);
        }
        if let (Some(n), Some(scale)) = (number(left), version_constant(right)) {
            return compare(target, scale, flip(op), n);
        }
        if let (Some(vc), Some(n)) = (version_compare_call(left), number(right)) {
            if n == 0 {
                let op = match (vc.1, op) {
                    (None, op) => op,
                    _ => return None,
                };
                return compare_id(target, op, vc.0);
            }
        }
        return None;
    }
    if let Some((id, Some(op))) = version_compare_call(cond) {
        return compare_id(target, op, id);
    }
    function_exists(cond, target)
}

fn flip(op: &str) -> &str {
    match op {
        "<" => ">",
        "<=" => ">=",
        ">" => "<",
        _ => "<=",
    }
}

fn name(t: &Token) -> &str {
    t.text.trim_start_matches('\\')
}

fn version_constant(side: &[Token]) -> Option<u32> {
    match side {
        [t] if name(t).eq_ignore_ascii_case("PHP_VERSION_ID") => Some(1),
        [t] if name(t).eq_ignore_ascii_case("PHP_MAJOR_VERSION") => Some(10000),
        _ => None,
    }
}

fn number(side: &[Token]) -> Option<u32> {
    match side {
        [t] if t.kind == TokenKind::Number => t.text.replace('_', "").parse().ok(),
        _ => None,
    }
}

fn compare(target: PhpVersion, scale: u32, op: &str, n: u32) -> Option<bool> {
    let low = target.major * 10000 + target.minor * 100;
    let (low, high) = if scale == 1 { (low, low + 99) } else { (target.major, target.major) };
    let test = |v: u32| match op {
        "<" => v < n,
        "<=" => v <= n,
        ">" => v > n,
        _ => v >= n,
    };
    match (test(low), test(high)) {
        (true, true) => Some(true),
        (false, false) => Some(false),
        _ => None,
    }
}

fn compare_id(target: PhpVersion, op: &str, id: u32) -> Option<bool> {
    let op = match op {
        "<" | "lt" => "<",
        "<=" | "le" => "<=",
        ">" | "gt" => ">",
        ">=" | "ge" => ">=",
        _ => return None,
    };
    compare(target, 1, op, id)
}

fn strip_quotes(t: &Token) -> Option<&str> {
    matches!(t.kind, TokenKind::Str { .. }).then(|| t.text.get(1..t.text.len().saturating_sub(1)).unwrap_or_default())
}

fn args(cond: &[Token]) -> Option<Vec<&[Token]>> {
    let close = matching(cond, 1)?;
    if close != cond.len() - 1 {
        return None;
    }
    let inner = &cond[2..close];
    Some(if inner.is_empty() { Vec::new() } else { split(inner, &[","]) })
}

fn version_compare_call(cond: &[Token]) -> Option<(u32, Option<&str>)> {
    let head = cond.first()?;
    if !name(head).eq_ignore_ascii_case("version_compare") || !cond.get(1)?.is_punct("(") {
        return None;
    }
    let a = args(cond)?;
    let current = match a.first()? {
        [t] => name(t).eq_ignore_ascii_case("PHP_VERSION"),
        [t, open, close] => name(t).eq_ignore_ascii_case("phpversion") && open.is_punct("(") && close.is_punct(")"),
        _ => false,
    };
    if !current {
        return None;
    }
    let version = match a.get(1)? {
        [t] => strip_quotes(t)?,
        _ => return None,
    };
    let mut parts = version.split('.').map(|p| p.parse::<u32>().ok());
    let id = parts.next()??.checked_mul(10000)? + parts.next().flatten().unwrap_or(0) * 100 + parts.next().flatten().unwrap_or(0);
    let op = match a.get(2) {
        Some([t]) => Some(strip_quotes(t)?),
        Some(_) => return None,
        None => None,
    };
    Some((id, op))
}

fn function_exists(cond: &[Token], target: PhpVersion) -> Option<bool> {
    let head = cond.first()?;
    if !name(head).eq_ignore_ascii_case("function_exists") || !cond.get(1)?.is_punct("(") {
        return None;
    }
    let a = args(cond)?;
    let func = match a.as_slice() {
        [[t]] => strip_quotes(t)?.trim_start_matches('\\').to_ascii_lowercase(),
        _ => return None,
    };
    let rule = FUNCTIONS.iter().find(|r| r.matches(&func))?;
    let gone = rule.change == Change::Removed && rule.since() <= target && !PECL.contains(&rule.id);
    gone.then_some(false)
}

#[cfg(test)]
mod tests {
    use crate::analyzer::analyze_source;
    use crate::version::PhpVersion;

    fn ids(src: &str, target: (u32, u32)) -> Vec<String> {
        analyze_source("a.php", src, PhpVersion::new(target.0, target.1)).into_iter().map(|f| f.rule_id).collect()
    }

    #[test]
    fn code_for_older_versions_is_ignored() {
        let src = "<?php\nif (PHP_VERSION_ID < 80100) { $r->setAccessible(true); each($a); }\nif (\\PHP_VERSION_ID >= 80000) { $x = 1; } else { each($b); }\nif (80000 > PHP_VERSION_ID) each($c);\nif (version_compare(PHP_VERSION, '8.0', '<')) { each($d); }\nif (version_compare(phpversion(), '8.0.0') < 0) { each($e); }\nif (PHP_MAJOR_VERSION < 8) { each($f); }\nif (defined('PHP_VERSION_ID') && PHP_VERSION_ID < 70400) { each($g); }\neach($live);";
        assert_eq!(ids(src, (8, 4)), ["removed.each"]);
    }

    #[test]
    fn branches_that_still_run_are_kept() {
        let src = "<?php\nif (PHP_VERSION_ID < 80500) { each($a); }\nif (PHP_VERSION_ID < 80000 || $legacy) { each($b); }\nif (PHP_VERSION_ID >= 80000) { each($c); } elseif ($x) { each($d); } else { each($e); }\nif ($y) { each($f); } elseif (PHP_VERSION_ID < 70000) { each($g); } else { each($h); }";
        let found = analyze_source("a.php", src, PhpVersion::new(8, 4));
        let lines: Vec<usize> = found.iter().map(|f| f.line).collect();
        assert_eq!(lines, [2, 3, 4, 5, 5]);
    }

    #[test]
    fn code_after_an_early_return_guard() {
        let src = "<?php\nfunction a() {\n    if (PHP_VERSION_ID >= 80000) {\n        return modern();\n    }\n    return each($x);\n}\nfunction b() {\n    if (\\PHP_VERSION_ID >= 70400) return;\n    each($y);\n}\nfunction c() {\n    if (PHP_VERSION_ID >= 80000) {\n        if ($z) { return 1; }\n    }\n    each($kept1);\n}\nfunction d() {\n    if (PHP_VERSION_ID >= 80000) {\n        return 1;\n    } else {\n        each($deadElse);\n    }\n    each($kept2);\n}\nswitch ($m) {\n    case 1:\n        if (PHP_VERSION_ID >= 80000) { return; }\n        break;\n    case 2:\n        each($kept3);\n}\nif (PHP_VERSION_ID >= 80000) { throw new Exception(); }\neach($topLevelDead);";
        let lines: Vec<usize> = analyze_source("a.php", src, PhpVersion::new(8, 4)).iter().map(|f| f.line).collect();
        assert_eq!(lines, [16, 24, 31]);
    }

    #[test]
    fn short_circuits_and_ternaries() {
        let src = "<?php\n(\\PHP_VERSION_ID < 80100) && $p->setAccessible(true);\ntrigger_error('x', PHP_VERSION_ID < 80400 ? E_USER_ERROR : E_USER_WARNING);\n$f = PHP_VERSION_ID >= 80000 ? 'mb_strlen' : each($a);\nPHP_VERSION_ID >= 80000 || each($b);\n$ok && each($c);\n$v = $cond ? each($d) : 1;";
        let lines: Vec<usize> = analyze_source("a.php", src, PhpVersion::new(8, 5)).iter().map(|f| f.line).collect();
        assert_eq!(lines, [6, 7]);
    }

    #[test]
    fn removed_functions_guarded_by_function_exists() {
        let src = "<?php\nif (function_exists('mysql_connect')) { mysql_connect(); }\nif (!function_exists('each')) { function each($a) {} }\nif (function_exists('mcrypt_encrypt')) { mcrypt_encrypt(); }\nif (function_exists('utf8_encode')) { utf8_encode($s); }";
        assert_eq!(ids(src, (8, 4)), ["removed.mcrypt", "deprecated.utf8_encode"]);
    }
}
