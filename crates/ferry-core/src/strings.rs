use crate::i18n::Text;
use crate::lexer::{Token, TokenKind};
use crate::rules::{FixKind, PatternRule, Severity};

/// The literal text of a quoted string, with only backslashes, quotes and \$ unescaped. Interpolated and
/// heredoc strings are skipped.
fn content(t: &Token) -> Option<String> {
    if !matches!(t.kind, TokenKind::Str { backtick: false, .. }) {
        return None;
    }
    let (quote, body) = if let Some(b) = t.text.strip_prefix('\'').and_then(|b| b.strip_suffix('\'')) {
        ('\'', b)
    } else {
        ('"', t.text.strip_prefix('"').and_then(|b| b.strip_suffix('"'))?)
    };
    if quote == '"' && (body.contains("{$") || body.contains("${")) {
        return None;
    }
    let mut out = String::new();
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek()) {
            ('\\', Some(&n)) if n == '\\' || n == quote || quote == '"' && n == '$' => {
                out.push(n);
                chars.next();
            }
            _ => out.push(c),
        }
    }
    Some(out)
}

/// The name of the function or method whose argument list contains token `k`, directly or inside an array
/// literal, and the argument's position.
fn enclosing_call(tokens: &[Token], k: usize) -> Option<(String, usize)> {
    let mut depth = 0usize;
    let mut position = 0;
    for j in (0..k).rev() {
        let t = &tokens[j];
        if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            depth += 1;
        } else if t.is_punct("{") {
            if depth == 0 {
                return None;
            }
            depth -= 1;
        } else if t.is_punct("[") {
            if depth == 0 {
                position = 0;
            } else {
                depth -= 1;
            }
        } else if t.is_punct("(") {
            if depth == 0 {
                let name = tokens.get(j.checked_sub(1)?).filter(|n| n.kind == TokenKind::Ident)?;
                return Some((name.text.trim_start_matches('\\').to_ascii_lowercase(), position));
            }
            depth -= 1;
        } else if depth == 0 && t.is_punct(",") {
            position += 1;
        } else if depth == 0 && t.is_punct(";") {
            return None;
        }
    }
    None
}

struct Regex<'a> {
    body: &'a str,
    modifiers: &'a str,
}

fn regex(pattern: &str) -> Option<Regex<'_>> {
    let start = pattern.find(|c: char| !c.is_whitespace())?;
    let open = pattern[start..].chars().next()?;
    if open.is_alphanumeric() || open == '\\' {
        return None;
    }
    let close = match open {
        '(' => ')',
        '{' => '}',
        '[' => ']',
        '<' => '>',
        c => c,
    };
    let end = pattern.rfind(close).filter(|&e| e > start)?;
    Some(Regex { body: &pattern[start + open.len_utf8()..end], modifiers: &pattern[end + close.len_utf8()..] })
}

fn class_range(body: &str) -> bool {
    let b = body.as_bytes();
    let mut i = 0;
    let mut in_class = false;
    while i < b.len() {
        match b[i] {
            b'\\' if i + 1 < b.len() => {
                let starts_range = in_class
                    && b"wWdDsShHvV".contains(&b[i + 1])
                    && b.get(i + 2) == Some(&b'-')
                    && b.get(i + 3).is_some_and(|&c| c != b']');
                if starts_range {
                    return true;
                }
                i += 2;
                continue;
            }
            b'[' if !in_class => {
                in_class = true;
                if b.get(i + 1) == Some(&b'^') {
                    i += 1;
                }
                if b.get(i + 1) == Some(&b']') {
                    i += 1;
                }
            }
            b']' => in_class = false,
            _ => {}
        }
        i += 1;
    }
    false
}

const INVALID_ESCAPES: &str = "ijlmquyFIJLMOTUY";

fn invalid_escape(body: &str) -> bool {
    let b = body.as_bytes();
    let mut i = 0;
    let mut in_class = false;
    while i < b.len() {
        match b[i] {
            b'\\' if i + 1 < b.len() => {
                if !in_class && INVALID_ESCAPES.as_bytes().contains(&b[i + 1]) {
                    return true;
                }
                i += 2;
                continue;
            }
            b'[' => in_class = true,
            b']' => in_class = false,
            _ => {}
        }
        i += 1;
    }
    false
}

fn empty_min_quantifier(body: &str) -> bool {
    let b = body.as_bytes();
    (0..b.len()).any(|i| {
        b[i] == b'{'
            && b.get(i + 1) == Some(&b',')
            && (i == 0 || b[i - 1] != b'\\')
            && b[i + 2..].iter().take_while(|c| c.is_ascii_digit()).count() > 0
            && b[i + 2..].iter().find(|c| !c.is_ascii_digit()) == Some(&b'}')
    })
}

fn keep_in_lookaround(body: &str) -> bool {
    ["(?=", "(?!", "(?<=", "(?<!"].iter().any(|open| {
        body.match_indices(open).any(|(at, _)| {
            let mut depth = 0i32;
            let rest = &body[at..];
            let b = rest.as_bytes();
            let mut i = 0;
            while i < b.len() {
                match b[i] {
                    b'\\' => {
                        if b.get(i + 1) == Some(&b'K') && depth == 1 {
                            return true;
                        }
                        i += 2;
                        continue;
                    }
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            return false;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            false
        })
    })
}

fn bare_precision(format: &str) -> bool {
    let b = format.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'%' {
            i += 1;
            continue;
        }
        if b.get(i + 1) == Some(&b'%') {
            i += 2;
            continue;
        }
        let mut j = i + 1;
        while j < b.len() && (b[j].is_ascii_digit() || b"-+ 0'$".contains(&b[j])) {
            if b[j] == b'\'' {
                j += 1;
            }
            j += 1;
        }
        if b.get(j) == Some(&b'.') && b.get(j + 1).is_some_and(|c| c.is_ascii_alphabetic()) {
            return true;
        }
        i = j.max(i + 1);
    }
    false
}

fn serialized_upper_s(s: &str) -> bool {
    let b = s.as_bytes();
    (0..b.len()).any(|i| {
        b[i] == b'S'
            && b.get(i + 1) == Some(&b':')
            && (i == 0 || b";{:".contains(&b[i - 1]))
            && b.get(i + 2).is_some_and(u8::is_ascii_digit)
    })
}

fn escaped_question_in_dollar_quote(sql: &str) -> bool {
    let mut parts = sql.split("$$");
    parts.next();
    parts.step_by(2).any(|inside| inside.contains("??"))
}

const PREG: &[&str] = &["preg_match", "preg_match_all", "preg_replace", "preg_replace_callback", "preg_split", "preg_grep", "preg_filter"];
const PRINTF: &[(&str, usize)] = &[("printf", 0), ("sprintf", 0), ("vprintf", 0), ("vsprintf", 0), ("fprintf", 1), ("vfprintf", 1)];
const SSL_REMOVED: &[&str] = &["rsa_key_size", "cn_match", "sni_server_name"];

/// String literals whose content newer PHP versions read differently, as (token, rule).
pub(crate) fn check(tokens: &[Token]) -> Vec<(usize, &'static PatternRule)> {
    let mut out = Vec::new();
    for (k, t) in tokens.iter().enumerate() {
        let Some(text) = content(t) else { continue };
        let prev = k.checked_sub(1).map(|p| &tokens[p]);
        let next = tokens.get(k + 1);
        let key = next.is_some_and(|n| n.is_punct("=>"));
        let lower = text.to_ascii_lowercase();
        let compared = |n: Option<&Token>| n.is_some_and(|n| n.is_punct("==") || n.is_punct("!=") || n.is_punct("<>"));
        let call = enclosing_call(tokens, k);
        let call_is = |names: &[&str], at: usize| call.as_ref().is_some_and(|(n, p)| *p == at && names.contains(&n.as_str()));

        let hex = lower.trim().strip_prefix("0x").is_some_and(|h| !h.is_empty() && h.bytes().all(|c| c.is_ascii_hexdigit()));
        if hex && (compared(prev) || compared(next) || call_is(&["is_numeric"], 0)) {
            out.push((k, &HEX_STRING));
        }
        if key && SSL_REMOVED.contains(&lower.as_str()) {
            out.push((k, &SSL_OPTION_REMOVED));
        }
        if key && lower == "capture_session_meta" {
            out.push((k, &SSL_SESSION_META));
        }
        if lower.starts_with("sslv2://") {
            out.push((k, &SSLV2));
        }
        if lower == "unknown type"
            && (prev.is_some_and(|p| p.is_punct("===") || p.is_punct("!=="))
                || compared(prev)
                || compared(next)
                || next.is_some_and(|n| n.is_punct("===") || n.is_punct("!==")))
        {
            out.push((k, &UNKNOWN_TYPE));
        }
        if text == "OCI-Lob" || text == "OCI-Collection" {
            out.push((k, &OCI_CLASS_NAME));
        }
        let iso = lower.strip_prefix("iso_8859").or_else(|| lower.strip_prefix("iso8859_")).is_some_and(|r| {
            let r = r.trim_start_matches(['-', '_']);
            !r.is_empty() && r.bytes().all(|c| c.is_ascii_digit())
        });
        if iso && call.as_ref().is_some_and(|(n, _)| n.starts_with("mb_")) {
            out.push((k, &ISO_ALIAS));
        }
        if call_is(PREG, 0) {
            if let Some(re) = regex(&text) {
                if class_range(re.body) {
                    out.push((k, &REGEX_RANGE));
                }
                if re.modifiers.contains('X') || invalid_escape(re.body) {
                    out.push((k, &REGEX_ESCAPE));
                }
                if empty_min_quantifier(re.body) {
                    out.push((k, &REGEX_QUANTIFIER));
                }
                if keep_in_lookaround(re.body) {
                    out.push((k, &REGEX_KEEP));
                }
            }
        }
        if PRINTF.iter().any(|&(f, at)| call_is(&[f], at)) && bare_precision(&text) {
            out.push((k, &PRINTF_PRECISION));
        }
        if call_is(&["unserialize"], 0) && serialized_upper_s(&text) {
            out.push((k, &SERIALIZED_UPPER_S));
        }
        if call_is(&["prepare", "query", "exec"], 0) && escaped_question_in_dollar_quote(&text) {
            out.push((k, &PGSQL_ESCAPED_QUESTION));
        }
        let session =
            k >= 2 && tokens[k - 1].is_punct("[") && tokens[k - 2].kind == TokenKind::Variable && tokens[k - 2].text == "_SESSION";
        if session && text.contains('|') {
            out.push((k, &SESSION_PIPE));
        }
        if lower == "allowed_classes" && key {
            let value = tokens.get(k + 2);
            if value.is_some_and(|v| matches!(v.kind, TokenKind::Str { .. }) || v.kind == TokenKind::Number) {
                out.push((k + 2, &ALLOWED_CLASSES));
            }
        }
        if lower == "seed"
            && key
            && call.as_ref().is_some_and(|(n, _)| n.starts_with("hash"))
            && tokens.get(k + 2).is_some_and(|v| matches!(v.kind, TokenKind::Str { .. }))
        {
            out.push((k + 2, &HASH_OPTION_TYPE));
        }
    }
    out
}

const fn rule(id: &'static str, title: Text, severity: Severity, since: (u32, u32), replacement: Option<Text>, note: Text) -> PatternRule {
    PatternRule { id, title, severity, since, replacement, fix: FixKind::Manual, note }
}

const HEX_STRING: PatternRule = rule(
    "behavior.hex_numeric_string",
    Text::new("Onaltılık sayı içeren metin", "String containing a hex number"),
    Severity::Behavior,
    (7, 0),
    Some(Text::new("hexdec()", "hexdec()")),
    Text::new(
        "PHP 7.0'dan beri \"0x1A\" gibi metinler sayı sayılmaz: is_numeric() false döner ve == karşılaştırması sayısal yapılmaz.",
        "Since PHP 7.0 strings such as \"0x1A\" aren't numeric: is_numeric() returns false and == doesn't compare them as numbers.",
    ),
);

const SSL_OPTION_REMOVED: PatternRule = rule(
    "removed.ssl_context_option",
    Text::new("Kaldırılan SSL context seçeneği", "Removed SSL context option"),
    Severity::Behavior,
    (7, 0),
    Some(Text::new("'peer_name'", "'peer_name'")),
    Text::new(
        "rsa_key_size, CN_match ve SNI_server_name PHP 7.0'da kaldırıldı ve sessizce yok sayılıyor; sertifika adı için peer_name kullanın.",
        "rsa_key_size, CN_match and SNI_server_name were removed in PHP 7.0 and are silently ignored; use peer_name for the certificate name.",
    ),
);

const SSL_SESSION_META: PatternRule = rule(
    "deprecated.ssl_capture_session_meta",
    Text::new("capture_session_meta seçeneği", "capture_session_meta option"),
    Severity::Deprecated,
    (7, 0),
    Some(Text::new("stream_get_meta_data()", "stream_get_meta_data()")),
    Text::new(
        "capture_session_meta PHP 7.0'da kullanımdan kalktı; bilgi stream_get_meta_data()'dan okunur.",
        "capture_session_meta is deprecated since PHP 7.0; read the data from stream_get_meta_data().",
    ),
);

const SSLV2: PatternRule = rule(
    "removed.sslv2_stream",
    Text::new("sslv2:// akışı", "sslv2:// stream"),
    Severity::Breaking,
    (7, 1),
    Some(Text::new("tls://", "tls://")),
    Text::new("sslv2:// akış taşıyıcısı PHP 7.1'de kaldırıldı.", "The sslv2:// stream transport was removed in PHP 7.1."),
);

const UNKNOWN_TYPE: PatternRule = rule(
    "behavior.gettype_closed_resource",
    Text::new("gettype() 'unknown type' karşılaştırması", "gettype() compared with 'unknown type'"),
    Severity::Behavior,
    (7, 2),
    Some(Text::new("'resource (closed)'", "'resource (closed)'")),
    Text::new(
        "PHP 7.2'den beri gettype() kapalı resource'lar için 'unknown type' yerine 'resource (closed)' döner.",
        "Since PHP 7.2 gettype() returns 'resource (closed)' instead of 'unknown type' for closed resources.",
    ),
);

const OCI_CLASS_NAME: PatternRule = rule(
    "behavior.oci_class_name",
    Text::new("Eski OCI sınıf adı", "Old OCI class name"),
    Severity::Behavior,
    (8, 0),
    Some(Text::new("'OCILob' / 'OCICollection'", "'OCILob' / 'OCICollection'")),
    Text::new(
        "PHP 8.0'da OCI-Lob sınıfının adı OCILob, OCI-Collection'ın adı OCICollection oldu.",
        "In PHP 8.0 the OCI-Lob class was renamed OCILob and OCI-Collection was renamed OCICollection.",
    ),
);

const ISO_ALIAS: PatternRule = rule(
    "removed.mbstring_iso_alias",
    Text::new("Kaldırılan ISO 8859 adı", "Removed ISO 8859 alias"),
    Severity::Breaking,
    (8, 0),
    Some(Text::new("'ISO-8859-1'", "'ISO-8859-1'")),
    Text::new(
        "mbstring'in ISO_8859-* ve ISO8859_* adları PHP 8.0'da kaldırıldı; ISO-8859-* yazın.",
        "mbstring's ISO_8859-* and ISO8859_* aliases were removed in PHP 8.0; write ISO-8859-*.",
    ),
);

const REGEX_ESCAPE: PatternRule = rule(
    "removed.regex_invalid_escape",
    Text::new("Geçersiz regex kaçışı", "Invalid regex escape"),
    Severity::Breaking,
    (8, 0),
    Some(Text::new("karakteri kaçırmadan yazın", "write the character without the backslash")),
    Text::new(
        "PHP 8.0'dan beri \\i gibi bilinmeyen kaçışlar düz karakter sayılmaz, derleme hatası verir ve X bayrağı yok sayılır; desen hiç eşleşmez.",
        "Since PHP 8.0 unknown escapes such as \\i are no longer literal characters but a compile error, and the X modifier is ignored, so the pattern never matches.",
    ),
);

const REGEX_RANGE: PatternRule = rule(
    "removed.regex_class_range",
    Text::new("Karakter sınıfı aralığı", "Character class range"),
    Severity::Breaking,
    (7, 3),
    Some(Text::new("tireyi kaçırın ya da sınıfın sonuna yazın: [\\w.-]", "escape the hyphen or put it last: [\\w.-]")),
    Text::new(
        "PHP 7.3'teki PCRE2 [\\w-.] gibi \\w ile başlayan aralıkları derlemez; desen uyarı verip false döner.",
        "PCRE2 in PHP 7.3 doesn't compile ranges that start at a class such as [\\w-.]; the pattern warns and returns false.",
    ),
);

const REGEX_QUANTIFIER: PatternRule = rule(
    "behavior.regex_empty_min_quantifier",
    Text::new("{,n} regex niceleyicisi", "{,n} regex quantifier"),
    Severity::Behavior,
    (8, 4),
    Some(Text::new("{0,n} ya da \\{,n}", "{0,n} or \\{,n}")),
    Text::new(
        "PHP 8.4'teki PCRE2 10.44'ten beri {,3} düz metin değil, \"en fazla 3\" niceleyicisi.",
        "Since PCRE2 10.44 in PHP 8.4, {,3} is an \"up to 3\" quantifier instead of literal text.",
    ),
);

const REGEX_KEEP: PatternRule = rule(
    "removed.regex_keep_in_lookaround",
    Text::new("Lookaround içinde \\K", "\\K inside a lookaround"),
    Severity::Breaking,
    (8, 5),
    None,
    Text::new(
        "PHP 8.5'ten beri lookaround içinde \\K kullanmak derleme hatası verir.",
        "Since PHP 8.5 using \\K inside a lookaround is a compile error.",
    ),
);

const PRINTF_PRECISION: PatternRule = rule(
    "behavior.printf_bare_precision",
    Text::new("Rakamsız printf hassasiyeti", "printf precision without digits"),
    Severity::Behavior,
    (8, 5),
    Some(Text::new("%.0f", "%.0f")),
    Text::new(
        "PHP 8.5'ten beri %.f gibi rakamsız bir hassasiyet 0 sayılır; eskiden varsayılan hassasiyete dönüyordu.",
        "Since PHP 8.5 a precision without digits, such as %.f, means 0; it used to fall back to the default precision.",
    ),
);

const SERIALIZED_UPPER_S: PatternRule = rule(
    "deprecated.unserialize_upper_s",
    Text::new("Serileştirilmiş S: etiketi", "Serialized S: tag"),
    Severity::Deprecated,
    (8, 4),
    Some(Text::new("s:", "s:")),
    Text::new(
        "Büyük S etiketli metinleri unserialize() ile açmak PHP 8.4'te kullanımdan kalktı.",
        "Unserializing strings with the uppercase S tag is deprecated since PHP 8.4.",
    ),
);

const PGSQL_ESCAPED_QUESTION: PatternRule = rule(
    "deprecated.pgsql_escaped_question",
    Text::new("$$ içinde ?? kaçışı", "?? escape inside $$"),
    Severity::Deprecated,
    (8, 4),
    Some(Text::new("?", "?")),
    Text::new(
        "PDO_PGSQL $$ ile alıntılanan metinleri artık kendisi ayırt ediyor; içlerinde soru işaretini ?? ile kaçırmak PHP 8.4'te kullanımdan kalktı.",
        "PDO_PGSQL now understands $$-quoted strings, so escaping question marks inside them as ?? is deprecated since PHP 8.4.",
    ),
);

const SESSION_PIPE: PatternRule = rule(
    "behavior.session_key_pipe",
    Text::new("| içeren oturum anahtarı", "Session key containing |"),
    Severity::Behavior,
    (8, 5),
    Some(Text::new("anahtarda | kullanmayın", "don't use | in the key")),
    Text::new(
        "Varsayılan oturum formatı | karakterini ayırıcı olarak kullanır. Böyle bir anahtar oturumu sessizce kaydedemiyordu; PHP 8.5'ten beri uyarı verir.",
        "The default session format uses | as a separator. A key containing it silently failed to save the session; since PHP 8.5 it raises a warning.",
    ),
);

const ALLOWED_CLASSES: PatternRule = rule(
    "behavior.unserialize_allowed_classes_type",
    Text::new("allowed_classes için metin", "String for allowed_classes"),
    Severity::Behavior,
    (7, 1),
    Some(Text::new("['Foo']", "['Foo']")),
    Text::new(
        "unserialize()'ın allowed_classes seçeneği PHP 7.1'den beri yalnızca dizi ya da bool kabul eder; metin verildiğinde false döner, 8.4'te ValueError fırlatır.",
        "unserialize()'s allowed_classes option only accepts an array or a bool since PHP 7.1; a string makes it return false, and in 8.4 it throws a ValueError.",
    ),
);

const HASH_OPTION_TYPE: PatternRule = rule(
    "deprecated.hash_option_type",
    Text::new("Yanlış tipte hash seçeneği", "Hash option of the wrong type"),
    Severity::Deprecated,
    (8, 4),
    Some(Text::new("tamsayı verin", "pass an integer")),
    Text::new(
        "hash fonksiyonlarının seçeneklerine yanlış tipte değer vermek PHP 8.4'te kullanımdan kalktı.",
        "Passing values of the wrong type to hash function options is deprecated since PHP 8.4.",
    ),
);
