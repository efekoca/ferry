use crate::i18n::Text;
use crate::lexer::{Token, TokenKind};
use crate::rules::{FixKind, PatternRule, Severity};
use std::collections::HashMap;

fn invalid_octal(text: &str) -> bool {
    let digits = text.replace('_', "");
    digits.len() > 1 && digits.starts_with('0') && digits.bytes().all(|b| b.is_ascii_digit()) && digits.contains(['8', '9'])
}

fn bad_unicode_escape(text: &str) -> bool {
    let double = text.starts_with('"') || text.starts_with("<<<") && !text[3..].trim_start().starts_with('\'');
    if !double {
        return false;
    }
    let b = text.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] != b'\\' {
            i += 1;
            continue;
        }
        if b[i + 1] == b'u' && b.get(i + 2) == Some(&b'{') {
            let hex = b[i + 3..].iter().take_while(|c| c.is_ascii_hexdigit()).count();
            if hex == 0 || b.get(i + 3 + hex) != Some(&b'}') {
                return true;
            }
        }
        i += 2;
    }
    false
}

#[derive(Clone, Copy, PartialEq)]
enum Block {
    Loop,
    Switch,
    Function,
    Other,
}

fn header_close(tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    (open..tokens.len()).find(|&k| {
        if tokens[k].is_punct("(") {
            depth += 1;
        } else if tokens[k].is_punct(")") {
            depth -= 1;
        }
        depth == 0
    })
}

fn continues_on_switch(tokens: &[Token]) -> Vec<usize> {
    let mut kinds: HashMap<usize, Block> = HashMap::new();
    for (k, t) in tokens.iter().enumerate() {
        let kind = if ["for", "foreach", "while"].iter().any(|w| t.is_ident_ci(w)) {
            Block::Loop
        } else if t.is_ident_ci("switch") {
            Block::Switch
        } else if t.is_ident_ci("function") || t.is_ident_ci("fn") {
            Block::Function
        } else if t.is_ident_ci("do") {
            if tokens.get(k + 1).is_some_and(|n| n.is_punct("{")) {
                kinds.insert(k + 1, Block::Loop);
            }
            continue;
        } else {
            continue;
        };
        let Some(open) = (k + 1..tokens.len().min(k + 4)).find(|&j| tokens[j].is_punct("(")) else { continue };
        let Some(close) = header_close(tokens, open) else { continue };
        let body = if kind == Block::Function {
            (close + 1..tokens.len()).find(|&j| tokens[j].is_punct("{") || tokens[j].is_punct(";") || tokens[j].is_punct("=>"))
        } else {
            Some(close + 1)
        };
        if let Some(b) = body.filter(|&b| tokens.get(b).is_some_and(|t| t.is_punct("{"))) {
            kinds.insert(b, kind);
        }
    }
    let mut out = Vec::new();
    let mut stack: Vec<Block> = Vec::new();
    for (k, t) in tokens.iter().enumerate() {
        if t.is_punct("{") {
            stack.push(kinds.get(&k).copied().unwrap_or(Block::Other));
        } else if t.is_punct("}") {
            stack.pop();
        } else if t.is_ident_ci("continue") {
            let plain = tokens.get(k + 1).is_some_and(|n| n.is_punct(";"))
                || tokens.get(k + 1).is_some_and(|n| n.text == "1") && tokens.get(k + 2).is_some_and(|n| n.is_punct(";"));
            let target = stack.iter().rev().find(|b| matches!(b, Block::Loop | Block::Switch | Block::Function));
            if plain && target == Some(&Block::Switch) {
                out.push(k);
            }
        }
    }
    out
}

fn is_int(t: &Token) -> bool {
    let hex = t.text.starts_with("0x") || t.text.starts_with("0X");
    t.kind == TokenKind::Number && (hex || !t.text.contains(['.', 'e', 'E']))
}

fn int_value(t: &Token) -> Option<u64> {
    let s = t.text.replace('_', "").to_ascii_lowercase();
    if let Some(h) = s.strip_prefix("0x") {
        u64::from_str_radix(h, 16).ok()
    } else if let Some(b) = s.strip_prefix("0b") {
        u64::from_str_radix(b, 2).ok()
    } else {
        s.parse().ok()
    }
}

/// Literal syntax that newer PHP versions reject or read differently, as (line, column, rule).
pub(crate) fn check(src: &str, tokens: &[Token], html: &[(usize, usize)]) -> Vec<(usize, usize, &'static PatternRule)> {
    let mut out = Vec::new();
    let at = |t: &Token, rule: &'static PatternRule| (t.line, t.column, rule);
    for (k, t) in tokens.iter().enumerate() {
        if t.kind == TokenKind::Number && invalid_octal(&t.text) {
            out.push(at(t, &INVALID_OCTAL));
        }
        if matches!(t.kind, TokenKind::Str { backtick: false, .. }) && bad_unicode_escape(&t.text) {
            out.push(at(t, &UNICODE_ESCAPE));
        }
        let next = tokens.get(k + 1);
        let pair = |c: &str| t.is_punct(c) && next.is_some_and(|n| n.is_punct(c) && n.line == t.line && n.column == t.column + 1);
        let operand = if pair("<") || pair(">") {
            Some(k + 2)
        } else if t.is_punct("<<=") || t.is_punct(">>=") {
            Some(k + 1)
        } else {
            None
        };
        if let Some(o) = operand {
            match (tokens.get(o), tokens.get(o + 1)) {
                (Some(m), Some(n)) if m.is_punct("-") && is_int(n) && int_value(n) != Some(0) => out.push(at(t, &NEGATIVE_SHIFT)),
                (Some(n), _) if is_int(n) && int_value(n).is_some_and(|v| v >= 64) => out.push(at(t, &WIDE_SHIFT)),
                _ => {}
            }
        }
        if pair("*")
            && k > 0
            && tokens[k - 1].kind == TokenKind::Number
            && tokens[k - 1].text.trim_end_matches(['.', '0']).is_empty()
            && tokens.get(k + 2).is_some_and(|m| m.is_punct("-"))
            && tokens.get(k + 3).is_some_and(|n| n.kind == TokenKind::Number && !n.text.trim_matches(['.', '0']).is_empty())
        {
            out.push(at(t, &crate::arguments::POW_ZERO_NEGATIVE));
        }
        if (t.is_punct("===") || t.is_punct("!==")) && k > 0 {
            let sides = [(&tokens[k - 1], next), (next.unwrap_or(t), Some(&tokens[k - 1]))];
            if sides.iter().any(|(a, b)| {
                (a.text.trim_start_matches('\\') == "PHP_DEBUG" || a.text.trim_start_matches('\\') == "PHP_ZTS")
                    && b.is_some_and(|b| b.kind == TokenKind::Number)
            }) {
                out.push(at(t, &DEBUG_ZTS_BOOL));
            }
        }
        if t.is_ident_ci("list") && next.is_some_and(|n| n.is_punct("(")) {
            let empty = tokens[k + 2..].iter().take_while(|n| !n.is_punct(")")).all(|n| n.is_punct(","));
            let close = (k + 2..tokens.len()).find(|&j| !tokens[j].is_punct(","));
            if empty && close.is_some_and(|c| tokens[c].is_punct(")") && tokens.get(c + 1).is_some_and(|e| e.is_punct("="))) {
                out.push(at(t, &EMPTY_LIST));
            }
        }
    }
    for k in continues_on_switch(tokens) {
        out.push(at(&tokens[k], &CONTINUE_SWITCH));
    }
    let lower = src.to_ascii_lowercase();
    for (pos, _) in lower.match_indices("<script") {
        let tag_end = lower[pos..].find('>').map_or(lower.len(), |e| pos + e);
        let tag: String = lower[pos..tag_end].chars().filter(|c| !c.is_whitespace() && *c != '"' && *c != '\'').collect();
        if tag.contains("language=php") && html.iter().any(|&(a, b)| a <= pos && pos < b) {
            let line = src[..pos].matches('\n').count() + 1;
            let column = pos - src[..pos].rfind('\n').map_or(0, |n| n + 1) + 1;
            out.push((line, column, &SCRIPT_TAG));
        }
    }
    if src.ends_with("<?php") {
        let pos = src.len() - 5;
        let line = src[..pos].matches('\n').count() + 1;
        let column = pos - src[..pos].rfind('\n').map_or(0, |n| n + 1) + 1;
        out.push((line, column, &TRAILING_OPEN_TAG));
    }
    out
}

const fn rule(id: &'static str, title: Text, severity: Severity, since: (u32, u32), replacement: Option<Text>, note: Text) -> PatternRule {
    PatternRule { id, title, severity, since, replacement, fix: FixKind::Manual, note }
}

pub(crate) const INVALID_OCTAL: PatternRule = rule(
    "removed.invalid_octal",
    Text::new("Geçersiz octal sayı", "Invalid octal literal"),
    Severity::Breaking,
    (7, 0),
    None,
    Text::new(
        "0 ile başlayan sayılar octal okunur. PHP 7.0'dan beri 8 ya da 9 içeren bir octal sayı derleme hatası verir; eskiden sessizce kesiliyordu.",
        "Numbers starting with 0 are octal. Since PHP 7.0 an octal literal containing 8 or 9 is a compile error; it used to be truncated silently.",
    ),
);

pub(crate) const UNICODE_ESCAPE: PatternRule = rule(
    "removed.invalid_unicode_escape",
    Text::new("Geçersiz \\u{} kaçışı", "Invalid \\u{} escape"),
    Severity::Breaking,
    (7, 0),
    Some(Text::new("ters bölüyü kaçırın: \\\\u{", "escape the backslash: \\\\u{")),
    Text::new(
        "PHP 7.0'dan beri çift tırnaklı metinlerde \\u{ bir Unicode kaçışı başlatır; arkasından geçerli bir onaltılık sayı gelmezse dosya derlenmez.",
        "Since PHP 7.0 \\u{ starts a Unicode escape in double-quoted strings, and the file doesn't compile unless a valid hex code point follows.",
    ),
);

pub(crate) const NEGATIVE_SHIFT: PatternRule = rule(
    "removed.negative_shift",
    Text::new("Negatif bit kaydırma", "Negative bit shift"),
    Severity::Breaking,
    (7, 0),
    None,
    Text::new(
        "PHP 7.0'dan beri negatif bir sayı kadar bit kaydırmak ArithmeticError fırlatır.",
        "Since PHP 7.0 shifting by a negative number throws an ArithmeticError.",
    ),
);

pub(crate) const WIDE_SHIFT: PatternRule = rule(
    "behavior.wide_shift",
    Text::new("64 bit ya da daha fazla kaydırma", "Shift by 64 bits or more"),
    Severity::Behavior,
    (7, 0),
    None,
    Text::new(
        "PHP 7.0'dan beri tamsayı genişliği kadar ya da daha fazla kaydırmak her zaman 0 (ya da sağa kaydırmada -1) verir; eskiden sonuç işlemciye göre değişirdi.",
        "Since PHP 7.0 shifting by the integer width or more always gives 0 (or -1 for a negative right shift); it used to depend on the CPU.",
    ),
);

pub(crate) const EMPTY_LIST: PatternRule = rule(
    "removed.empty_list",
    Text::new("Boş list() ataması", "Empty list() assignment"),
    Severity::Breaking,
    (7, 0),
    None,
    Text::new(
        "PHP 7.0'dan beri boş bir list() ataması derleme hatası verir.",
        "Since PHP 7.0 an empty list() assignment is a compile error.",
    ),
);

pub(crate) const CONTINUE_SWITCH: PatternRule = rule(
    "deprecated.continue_switch",
    Text::new("switch'i hedefleyen continue", "continue targeting a switch"),
    Severity::Deprecated,
    (7, 3),
    Some(Text::new("break ya da continue 2", "break or continue 2")),
    Text::new(
        "switch içindeki continue break gibi davranır. PHP 7.3'ten beri bu derlenirken uyarı verir; döngüye devam etmek için continue 2 yazın.",
        "continue inside a switch acts like break. Since PHP 7.3 it raises a warning when compiled; use continue 2 to continue the loop.",
    ),
);

pub(crate) const SCRIPT_TAG: PatternRule = rule(
    "removed.script_tag",
    Text::new("<script language=\"php\"> etiketi", "<script language=\"php\"> tag"),
    Severity::Breaking,
    (7, 0),
    Some(Text::new("<?php ... ?>", "<?php ... ?>")),
    Text::new(
        "<script language=\"php\"> etiketleri PHP 7.0'da kaldırıldı; içindeki kod artık çalışmaz ve olduğu gibi çıktıya yazılır.",
        "<script language=\"php\"> tags were removed in PHP 7.0; the code inside no longer runs and is output as text.",
    ),
);

pub(crate) const TRAILING_OPEN_TAG: PatternRule = rule(
    "behavior.trailing_open_tag",
    Text::new("Dosya sonunda <?php", "<?php at the end of the file"),
    Severity::Behavior,
    (7, 4),
    None,
    Text::new(
        "PHP 7.4'ten beri dosyanın en sonundaki, arkasında satır sonu olmayan <?php bir açılış etiketi sayılır; eskiden metin olarak yazılıyordu.",
        "Since PHP 7.4 a <?php at the very end of a file, without a newline after it, is an opening tag; it used to be output as text.",
    ),
);

pub(crate) const DEBUG_ZTS_BOOL: PatternRule = rule(
    "behavior.php_debug_bool",
    Text::new("PHP_DEBUG/PHP_ZTS ile sayı karşılaştırması", "PHP_DEBUG/PHP_ZTS compared with a number"),
    Severity::Behavior,
    (8, 4),
    Some(Text::new("true ya da false ile karşılaştırın", "compare with true or false")),
    Text::new(
        "PHP 8.4'te PHP_DEBUG ve PHP_ZTS tamsayı yerine bool oldu; sayıyla === karşılaştırması artık hep false döner.",
        "In PHP 8.4 PHP_DEBUG and PHP_ZTS became bool instead of int, so a strict comparison with a number is now always false.",
    ),
);
