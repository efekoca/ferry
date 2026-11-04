use crate::declarations::{close_of, function_at};
use crate::i18n::Text;
use crate::lexer::{Token, TokenKind};
use crate::rules::{FixKind, PatternRule, Severity};
use std::collections::HashSet;

const RESTRICTED: &[&str] = &["compact", "extract", "func_get_arg", "func_get_args", "func_num_args", "get_defined_vars"];
const CALLERS: &[&str] = &["array_map", "call_user_func", "call_user_func_array", "forward_static_call", "forward_static_call_array"];
const SCOPE_READERS: &[&str] =
    &["extract", "parse_str", "mb_parse_str", "get_defined_vars", "eval", "include", "include_once", "require", "require_once"];

fn name_of(t: &Token) -> String {
    t.text.trim_start_matches('\\').to_ascii_lowercase()
}

fn called(tokens: &[Token], k: usize) -> bool {
    let t = &tokens[k];
    let member = k > 0 && ["->", "?->", "::", "function", "new"].iter().any(|p| tokens[k - 1].is_punct(p) || tokens[k - 1].is_ident_ci(p));
    t.kind == TokenKind::Ident && tokens.get(k + 1).is_some_and(|n| n.is_punct("(")) && !member
}

fn string_value(t: &Token) -> Option<&str> {
    matches!(t.kind, TokenKind::Str { interpolates: false, .. }).then(|| t.text.trim_matches(|c| c == '\'' || c == '"'))
}

fn modifies(tokens: &[Token], k: usize) -> bool {
    let next = tokens.get(k + 1);
    let compound = next.is_some_and(|n| ["+", "-", "*", "/", ".", "%", "&", "|", "^"].iter().any(|o| n.is_punct(o)))
        && tokens.get(k + 2).is_some_and(|n| n.is_punct("="));
    next.is_some_and(|n| n.is_punct("=") || n.is_punct("++") || n.is_punct("--"))
        || compound
        || k > 0 && (tokens[k - 1].is_punct("++") || tokens[k - 1].is_punct("--"))
}

/// The next time a variable appears after `from`, while still inside the block `from` belongs to.
fn next_mention(tokens: &[Token], from: usize, name: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (j, t) in tokens.iter().enumerate().skip(from + 1) {
        if t.is_punct("{") {
            depth += 1;
        } else if t.is_punct("}") {
            depth = depth.checked_sub(1)?;
        } else if t.kind == TokenKind::Variable && t.text == name {
            return Some(j);
        }
    }
    None
}

fn statement_start(tokens: &[Token], k: usize) -> bool {
    k == 0 || [";", "{", "}"].iter().any(|p| tokens[k - 1].is_punct(p)) || tokens[k - 1].kind == TokenKind::CloseTag
}

fn non_numeric(text: &str) -> bool {
    !text.is_empty() && text.trim().parse::<f64>().is_err() && text.chars().all(|c| c.is_ascii_alphanumeric())
}

fn negative_offset(tokens: &[Token], k: usize) -> bool {
    tokens.get(k + 1).is_some_and(|t| t.is_punct("["))
        && tokens.get(k + 2).is_some_and(|t| t.is_punct("-"))
        && tokens.get(k + 3).is_some_and(|t| t.kind == TokenKind::Number)
        && tokens.get(k + 4).is_some_and(|t| t.is_punct("]"))
        && tokens.get(k + 5).is_some_and(|t| t.is_punct("="))
}

fn appends(tokens: &[Token], k: usize) -> bool {
    tokens.get(k + 1).is_some_and(|t| t.is_punct("["))
        && tokens.get(k + 2).is_some_and(|t| t.is_punct("]"))
        && tokens.get(k + 3).is_some_and(|t| t.is_punct("="))
}

/// Literal values followed through a block, and calls that depend on what a function's scope contains.
pub(crate) fn check(tokens: &[Token]) -> Vec<(usize, &'static PatternRule)> {
    let mut out = Vec::new();
    let mut by_ref = HashSet::new();
    for k in 0..tokens.len() {
        let Some(f) = function_at(tokens, k) else { continue };
        if let (Some(n), Some(p)) = (f.name, f.params.first()) {
            if tokens[p.var - 1].is_punct("&") {
                by_ref.insert(name_of(&tokens[n]));
            }
        }
        let Some((open, close)) = f.body else { continue };

        let changed: Vec<Option<usize>> = f
            .params
            .iter()
            .map(|p| {
                let same = |j: &usize| tokens[*j].kind == TokenKind::Variable && tokens[*j].text == tokens[p.var].text;
                let at = (open..close).filter(same).find(|&j| modifies(tokens, j));
                at.and_then(|m| (m..close).find(|&j| tokens[j].is_punct(";")))
            })
            .collect();
        for j in open..close {
            if !called(tokens, j) {
                continue;
            }
            let from = match name_of(&tokens[j]).as_str() {
                "func_get_args" if !(tokens[j - 1].is_punct("(") && tokens[j - 2].is_ident_ci("array_slice")) => {
                    changed.iter().flatten().min().copied()
                }
                "func_get_arg" => {
                    tokens.get(j + 2).and_then(|n| n.text.parse::<usize>().ok()).and_then(|n| changed.get(n).copied().flatten())
                }
                _ => None,
            };
            if from.is_some_and(|f| f < j) {
                out.push((j, &FUNC_GET_ARGS));
            }
        }

        let body = &tokens[open..close];
        let reads_scope =
            body.iter().any(|t| t.is_punct("$") || t.kind == TokenKind::Ident && SCOPE_READERS.contains(&name_of(t).as_str()));
        if !reads_scope {
            let known: HashSet<&str> = body
                .iter()
                .chain(f.params.iter().map(|p| &tokens[p.var]))
                .filter(|t| t.kind == TokenKind::Variable)
                .map(|t| t.text.as_str())
                .chain(f.uses.iter().map(|&u| tokens[u].text.as_str()))
                .collect();
            for j in open..close {
                if !(called(tokens, j) && tokens[j].is_ident_ci("compact")) {
                    continue;
                }
                let Some(end) = close_of(tokens, j + 1) else { continue };
                for (a, arg) in tokens.iter().enumerate().take(end).skip(j + 2) {
                    if string_value(arg).is_some_and(|s| s != "this" && !known.contains(s)) {
                        out.push((a, &COMPACT_UNDEFINED));
                    }
                }
            }
        }
    }

    for k in 0..tokens.len() {
        let t = &tokens[k];
        if t.is_ident_ci("foreach") && tokens.get(k + 1).is_some_and(|n| n.is_punct("(")) {
            let var = tokens.get(k + 2).filter(|v| v.kind == TokenKind::Variable && tokens.get(k + 3).is_some_and(|a| a.is_ident_ci("as")));
            let head = close_of(tokens, k + 1);
            if let (Some(var), Some(head)) = (var, head) {
                let end = if tokens.get(head + 1).is_some_and(|b| b.is_punct("{")) {
                    close_of(tokens, head + 1)
                } else {
                    (head + 1..tokens.len()).find(|&j| tokens[j].is_punct(";"))
                };
                for j in head + 1..end.unwrap_or(head + 1) {
                    let pointer = ["current", "key", "pos"].contains(&name_of(&tokens[j]).as_str()) && called(tokens, j);
                    let same = tokens.get(j + 2).is_some_and(|a| a.kind == TokenKind::Variable && a.text == var.text)
                        && tokens.get(j + 3).is_some_and(|c| c.is_punct(")"));
                    if pointer && same {
                        out.push((j, &FOREACH_POINTER));
                    }
                }
            }
        }

        if called(tokens, k) {
            let name = name_of(t);
            let first = tokens.get(k + 2);
            if CALLERS.contains(&name.as_str())
                && first.and_then(string_value).is_some_and(|s| RESTRICTED.contains(&s.to_ascii_lowercase().as_str()))
            {
                out.push((k + 2, &DYNAMIC_CALL));
            }
            if (name == "mt_srand" || name == "srand")
                && first.is_some_and(|a| a.kind == TokenKind::Number)
                && tokens.get(k + 3).is_some_and(|c| c.is_punct(")") || c.is_punct(","))
            {
                out.push((k, &SEEDED_RANDOM));
            }
            let reference = crate::calls::BY_REF_ARGUMENT.contains(&name.as_str()) || by_ref.contains(&name);
            if reference && first.is_some_and(|a| a.is_ident_ci("list")) {
                let assigned = tokens.get(k + 3).filter(|o| o.is_punct("(")).and_then(|_| close_of(tokens, k + 3));
                if assigned.is_some_and(|c| tokens.get(c + 1).is_some_and(|e| e.is_punct("="))) {
                    out.push((k + 2, &LIST_BY_REFERENCE));
                }
            }
        }

        if t.kind == TokenKind::Variable && statement_start(tokens, k) && tokens.get(k + 1).is_some_and(|n| n.is_punct("=")) {
            let Some(value) = tokens.get(k + 2) else { continue };
            let single = tokens.get(k + 3).is_some_and(|s| s.is_punct(";"));
            let empty_array = value.is_punct("[")
                && tokens.get(k + 3).is_some_and(|c| c.is_punct("]"))
                && tokens.get(k + 4).is_some_and(|s| s.is_punct(";"));
            let mention = next_mention(tokens, k, &t.text);
            if single && value.is_ident_ci("false") && mention.is_some_and(|m| tokens.get(m + 1).is_some_and(|n| n.is_punct("["))) {
                let m = mention.unwrap_or(k);
                let write = close_of(tokens, m + 1).is_some_and(|c| tokens.get(c + 1).is_some_and(|e| e.is_punct("=")));
                if write {
                    out.push((m, &FALSE_TO_ARRAY));
                }
            }
            if single && string_value(value).is_some_and(non_numeric) {
                if let Some(m) = mention {
                    let incremented = tokens.get(m + 1).is_some_and(|n| n.is_punct("++")) || tokens[m - 1].is_punct("++");
                    if incremented {
                        out.push((m, &STRING_INCREMENT));
                    }
                }
            }
            if single && string_value(value).is_some_and(|s| RESTRICTED.contains(&s.to_ascii_lowercase().as_str())) {
                if let Some(m) = mention.filter(|&m| tokens.get(m + 1).is_some_and(|n| n.is_punct("("))) {
                    out.push((m, &DYNAMIC_CALL));
                }
            }
            if empty_array {
                if let Some(m) = mention.filter(|&m| negative_offset(tokens, m)) {
                    if next_mention(tokens, m, &t.text).is_some_and(|n| appends(tokens, n)) {
                        out.push((m, &NEGATIVE_INDEX_ASSIGNMENT));
                    }
                }
            }
        }
    }
    out
}

const fn rule(id: &'static str, title: Text, severity: Severity, since: (u32, u32), replacement: Option<Text>, note: Text) -> PatternRule {
    PatternRule { id, title, severity, since, replacement, fix: FixKind::Manual, note }
}

const FOREACH_POINTER: PatternRule = rule(
    "behavior.foreach_array_pointer",
    Text::new("foreach içinde dizi işaretçisi", "Array pointer inside foreach"),
    Severity::Behavior,
    (7, 0),
    Some(Text::new("döngünün anahtar ve değerini kullanın", "use the loop's key and value")),
    Text::new(
        "PHP 7.0'dan beri foreach dizinin iç işaretçisini ilerletmez; döngü içindeki current() ve key() hep aynı öğeyi gösterir.",
        "Since PHP 7.0 foreach doesn't move the array's internal pointer, so current() and key() inside the loop keep pointing at the same element.",
    ),
);

const FUNC_GET_ARGS: PatternRule = rule(
    "behavior.func_get_args_current_value",
    Text::new("Değişen parametreden sonra func_get_args()", "func_get_args() after changing a parameter"),
    Severity::Behavior,
    (7, 0),
    Some(Text::new("func_get_args()'ı parametreyi değiştirmeden önce çağırın", "call func_get_args() before changing the parameter")),
    Text::new(
        "PHP 7.0'dan beri func_get_arg() ve func_get_args() parametrenin ilk değerini değil, güncel değerini verir.",
        "Since PHP 7.0 func_get_arg() and func_get_args() return the parameter's current value instead of the original one.",
    ),
);

const DYNAMIC_CALL: PatternRule = rule(
    "removed.dynamic_scope_call",
    Text::new("Kapsam fonksiyonu dinamik çağrılıyor", "Scope function called dynamically"),
    Severity::Breaking,
    (7, 1),
    Some(Text::new("fonksiyonu doğrudan çağırın", "call the function directly")),
    Text::new(
        "extract(), compact(), func_get_args() ve benzerleri PHP 7.1'den beri $f() ya da call_user_func() gibi dinamik yollarla çağrılamaz; 7.1'de uyarı verir, PHP 8.0'dan beri Error fırlatır.",
        "extract(), compact(), func_get_args() and similar functions can't be called dynamically, through $f() or call_user_func(), since PHP 7.1. It warns on 7.1 and throws an Error since PHP 8.0.",
    ),
);

const SEEDED_RANDOM: PatternRule = rule(
    "behavior.seeded_random_sequence",
    Text::new("Sabit tohumla rastgele sayı dizisi", "Random sequence from a fixed seed"),
    Severity::Behavior,
    (7, 1),
    None,
    Text::new(
        "mt_rand() PHP 7.1'de düzeltildi, rand() ve srand() onun takma adı oldu, PHP 7.2'de aralıklı üretim de değişti. Aynı tohum eski sürümlerdekinden farklı bir dizi üretir; kayıtlı bir diziye güvenen kodu kontrol edin.",
        "mt_rand() was fixed in PHP 7.1, rand() and srand() became its aliases, and PHP 7.2 changed range generation too. The same seed gives a different sequence than on older versions, so check code that relies on a stored sequence.",
    ),
);

const COMPACT_UNDEFINED: PatternRule = rule(
    "behavior.compact_undefined_variable",
    Text::new("compact() tanımsız değişkene bakıyor", "compact() names an undefined variable"),
    Severity::Behavior,
    (7, 3),
    Some(Text::new("adı düzeltin ya da listeden çıkarın", "fix the name or remove it")),
    Text::new(
        "Bu ad fonksiyonda hiç geçmiyor. compact() tanımsız değişkenler için PHP 7.3'ten beri notice, PHP 8.0'dan beri uyarı verir.",
        "This name doesn't appear in the function. compact() raises a notice for undefined variables since PHP 7.3 and a warning since PHP 8.0.",
    ),
);

const LIST_BY_REFERENCE: PatternRule = rule(
    "removed.list_result_by_reference",
    Text::new("list() sonucu referansla geçiyor", "list() result passed by reference"),
    Severity::Breaking,
    (7, 4),
    Some(Text::new("atamayı çağrıdan önce yapın", "do the assignment before the call")),
    Text::new(
        "Referans almayan bir list() atamasının sonucunu referansla geçmek PHP 7.4'ten beri hata verir.",
        "Passing the result of a non-reference list() assignment by reference is an error since PHP 7.4.",
    ),
);

const FALSE_TO_ARRAY: PatternRule = rule(
    "deprecated.false_to_array",
    Text::new("false değeri diziye dönüşüyor", "false converted to an array"),
    Severity::Deprecated,
    (8, 1),
    Some(Text::new("değişkeni [] ile başlatın", "initialize the variable with []")),
    Text::new(
        "false değerini yazarken otomatik olarak diziye çevirmek PHP 8.1'de kullanımdan kalktı.",
        "Automatically converting false to an array on write is deprecated since PHP 8.1.",
    ),
);

const STRING_INCREMENT: PatternRule = rule(
    "deprecated.string_increment",
    Text::new("Sayı olmayan metni artırma", "Incrementing a non-numeric string"),
    Severity::Deprecated,
    (8, 5),
    Some(Text::new("str_increment($s)", "str_increment($s)")),
    Text::new(
        "Sayı olmayan bir metni ++ ile artırmak PHP 8.5'te kullanımdan kalktı; str_increment() kullanın.",
        "Incrementing a non-numeric string with ++ is deprecated since PHP 8.5; use str_increment().",
    ),
);

const NEGATIVE_INDEX_ASSIGNMENT: PatternRule = rule(
    "behavior.negative_index_assignment",
    Text::new("Boş diziye negatif anahtar", "Negative key on an empty array"),
    Severity::Behavior,
    (8, 3),
    None,
    Text::new(
        "PHP 8.3'ten beri boş diziye negatif n anahtarı atandıktan sonraki otomatik anahtar 0 değil, n+1.",
        "Since PHP 8.3 the automatic key after assigning a negative key n to an empty array is n+1 instead of 0.",
    ),
);
