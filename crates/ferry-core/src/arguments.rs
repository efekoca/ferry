use crate::i18n::Text;
use crate::lexer::{Token, TokenKind};
use crate::rules::{FixKind, PatternRule, Severity};

#[derive(Debug, Clone, Copy)]
pub enum Check {
    Count(usize, usize),
    Null(usize),
    False(usize),
    Bool(usize),
    Int(usize),
    Str(usize),
    StrIs(usize, &'static [&'static str]),
    StrNot(usize, &'static [&'static str]),
    StrHas(usize, char),
    StrOutside(usize, &'static str),
    StrLenNot(usize, usize),
    NumIs(usize, f64),
    NumBelow(usize, f64),
    NumOutside(usize, f64, f64),
    Has(usize, &'static str),
    Lacks(usize, &'static str),
    Key(usize, &'static str),
    StringKeys(usize),
    StrStarts(usize, &'static [&'static str]),
    New(usize),
    StrContains(usize, &'static str),
    ArrayLit(usize),
    This(usize),
    InvalidForBase(usize, usize),
    /// The argument sets a locale, as opposed to reading it or resetting it to C.
    ChangesLocale(usize),
    ArrayLitEmpty(usize),
}

#[derive(Debug, Clone, Copy)]
pub struct ArgRule {
    pub functions: &'static [&'static str],
    pub checks: &'static [Check],
    pub rule: PatternRule,
}

enum Lit {
    Num(f64, bool),
    Str(Option<String>),
    Null,
    Bool(bool),
    Other,
}

fn number(text: &str) -> Option<(f64, bool)> {
    let t = text.replace('_', "").to_ascii_lowercase();
    let radix = |digits: &str, r| i64::from_str_radix(digits, r).ok().map(|v| (v as f64, true));
    if let Some(h) = t.strip_prefix("0x") {
        radix(h, 16)
    } else if let Some(b) = t.strip_prefix("0b") {
        radix(b, 2)
    } else if let Some(o) = t.strip_prefix("0o") {
        radix(o, 8)
    } else if t.contains(['.', 'e']) {
        t.parse().ok().map(|v| (v, false))
    } else if t.len() > 1 && t.starts_with('0') {
        radix(&t[1..], 8)
    } else {
        t.parse::<i64>().ok().map(|v| (v as f64, true))
    }
}

pub fn string_literal(text: &str) -> Option<String> {
    if let Some(body) = text.strip_prefix('\'').and_then(|t| t.strip_suffix('\'')) {
        return Some(body.replace("\\\\", "\u{0}").replace("\\'", "'").replace('\u{0}', "\\"));
    }
    let body = text.strip_prefix('"').and_then(|t| t.strip_suffix('"'))?;
    (!body.contains('\\')).then(|| body.to_string())
}

fn literal(arg: &[Token]) -> Lit {
    match arg {
        [t] if t.kind == TokenKind::Number => number(&t.text).map_or(Lit::Other, |(v, int)| Lit::Num(v, int)),
        [m, t] if m.is_punct("-") && t.kind == TokenKind::Number => number(&t.text).map_or(Lit::Other, |(v, int)| Lit::Num(-v, int)),
        [t] if t.literal().is_some() => Lit::Str(string_literal(&t.text)),
        [t] if t.is_ident_ci("null") => Lit::Null,
        [t] if t.is_ident_ci("true") => Lit::Bool(true),
        [t] if t.is_ident_ci("false") => Lit::Bool(false),
        _ => Lit::Other,
    }
}

impl Check {
    fn slot(self) -> Option<usize> {
        match self {
            Check::Count(..) => None,
            Check::Null(n)
            | Check::False(n)
            | Check::Bool(n)
            | Check::Int(n)
            | Check::Str(n)
            | Check::StrIs(n, _)
            | Check::StrNot(n, _)
            | Check::StrHas(n, _)
            | Check::StrOutside(n, _)
            | Check::StrLenNot(n, _)
            | Check::NumIs(n, _)
            | Check::NumBelow(n, _)
            | Check::NumOutside(n, ..)
            | Check::Has(n, _)
            | Check::Lacks(n, _)
            | Check::Key(n, _)
            | Check::StringKeys(n)
            | Check::StrStarts(n, _)
            | Check::New(n)
            | Check::StrContains(n, _)
            | Check::This(n)
            | Check::InvalidForBase(n, _)
            | Check::ChangesLocale(n)
            | Check::ArrayLitEmpty(n)
            | Check::ArrayLit(n) => Some(n),
        }
    }

    fn holds(self, args: &[&[Token]]) -> bool {
        let Some(n) = self.slot() else {
            let Check::Count(min, max) = self else { unreachable!() };
            return (min..=max).contains(&args.len());
        };
        let Some(arg) = args.get(n) else { return false };
        let text = || match literal(arg) {
            Lit::Str(Some(s)) => Some(s),
            _ => None,
        };
        let num = || match literal(arg) {
            Lit::Num(v, _) => Some(v),
            _ => None,
        };
        match self {
            Check::Count(..) => unreachable!(),
            Check::Null(_) => matches!(literal(arg), Lit::Null),
            Check::False(_) => matches!(literal(arg), Lit::Bool(false)),
            Check::Bool(_) => matches!(literal(arg), Lit::Bool(_)),
            Check::Int(_) => matches!(literal(arg), Lit::Num(_, true)),
            Check::Str(_) => matches!(literal(arg), Lit::Str(_)),
            Check::StrIs(_, list) => text().is_some_and(|s| list.iter().any(|v| v.eq_ignore_ascii_case(&s))),
            Check::StrNot(_, list) => text().is_some_and(|s| !list.iter().any(|v| v.eq_ignore_ascii_case(&s))),
            Check::StrHas(_, c) => text().is_some_and(|s| s.contains(c)),
            Check::StrOutside(_, set) => text().is_some_and(|s| {
                let s = s.to_ascii_lowercase();
                let digits = ["0x", "0b", "0o"].iter().find_map(|p| s.strip_prefix(p)).unwrap_or(&s);
                digits.chars().any(|c| !set.contains(c))
            }),
            Check::StrLenNot(_, len) => text().is_some_and(|s| s.len() != len),
            Check::NumIs(_, v) => num().is_some_and(|x| x == v),
            Check::NumBelow(_, v) => num().is_some_and(|x| x < v),
            Check::NumOutside(_, lo, hi) => num().is_some_and(|x| x < lo || x > hi),
            Check::Has(_, name) => {
                arg.iter().any(|t| t.kind == TokenKind::Ident && t.text.trim_start_matches('\\').eq_ignore_ascii_case(name))
            }
            Check::Lacks(_, name) => !Check::Has(n, name).holds(args),
            Check::StringKeys(_) => {
                arg.first().is_some_and(|t| t.is_punct("[") || t.is_ident_ci("array"))
                    && arg.windows(2).any(|w| matches!(w[0].kind, TokenKind::Str { .. }) && w[1].is_punct("=>"))
            }
            Check::StrStarts(_, list) => text().is_some_and(|s| list.iter().any(|p| s.to_ascii_lowercase().starts_with(p))),
            Check::InvalidForBase(_, from) => {
                let base = args.get(from).and_then(|b| match literal(b) {
                    Lit::Num(v, true) => Some(v as u32),
                    _ => None,
                });
                let digits = text().map(|s| s.to_ascii_lowercase());
                match (digits, base) {
                    (Some(s), Some(base @ 2..=36)) => {
                        let prefix = [(16, "0x"), (2, "0b"), (8, "0o")].iter().find(|(b, _)| *b == base).map_or("", |(_, p)| p);
                        s.strip_prefix(prefix).unwrap_or(&s).chars().any(|c| !c.is_digit(base))
                    }
                    _ => false,
                }
            }
            Check::This(_) => arg.len() == 1 && arg[0].kind == TokenKind::Variable && arg[0].text == "this",
            Check::ArrayLitEmpty(_) => {
                arg.len() == 2 && arg[0].is_punct("[") && arg[1].is_punct("]")
                    || arg.len() == 3 && arg[0].is_ident_ci("array") && arg[1].is_punct("(") && arg[2].is_punct(")")
            }
            Check::ChangesLocale(_) => match literal(arg) {
                Lit::Num(..) => false,
                Lit::Str(Some(s)) => !["C", "POSIX"].contains(&s.as_str()),
                _ => true,
            },
            Check::New(_) => arg.first().is_some_and(|t| t.is_ident_ci("new")),
            Check::StrContains(_, part) => arg.len() == 1 && matches!(arg[0].kind, TokenKind::Str { .. }) && arg[0].text.contains(part),
            Check::ArrayLit(_) => {
                arg.first().is_some_and(|t| t.is_punct("[") || t.is_ident_ci("array"))
                    && arg.last().is_some_and(|t| t.is_punct("]") || t.is_punct(")"))
            }
            Check::Key(_, key) => {
                arg.windows(2).any(|w| string_literal(&w[0].text).is_some_and(|s| s.eq_ignore_ascii_case(key)) && w[1].is_punct("=>"))
            }
        }
    }
}

pub(crate) const HTML_FLAGS_ID: &str = "behavior.html_default_flags";

pub(crate) const NEGATIVE_KEY: PatternRule = rule(
    "behavior.negative_array_key",
    Text::new("Negatif anahtardan sonraki anahtar", "Key after a negative key"),
    B,
    (8, 0),
    None,
    Text::new(
        "PHP 8.0'dan beri negatif n anahtarından sonraki otomatik anahtar 0 değil, n+1.",
        "Since PHP 8.0 the automatic key after a negative key n is n+1 instead of 0.",
    ),
);

const OBJECT_KEY_EXISTS: PatternRule = rule(
    "removed.array_key_exists_object",
    Text::new("array_key_exists() nesneyle", "array_key_exists() on an object"),
    R,
    (8, 0),
    code("property_exists($object, $key)"),
    Text::new(
        "array_key_exists() nesnelerde PHP 7.4'te kullanımdan kalktı ve PHP 8.0'dan beri TypeError fırlatıyor.",
        "array_key_exists() on objects was deprecated in PHP 7.4 and throws a TypeError since PHP 8.0.",
    ),
);

const OBJECT_POINTER: PatternRule = rule(
    "deprecated.array_pointer_on_object",
    Text::new("Dizi işaretçisi fonksiyonu nesneyle", "Array pointer function on an object"),
    D,
    (8, 1),
    tx("önce get_mangled_object_vars() ile diziye çevirin", "convert it with get_mangled_object_vars() first"),
    Text::new(
        "key(), current(), next(), prev(), reset() ve end() fonksiyonlarını nesnede kullanmak PHP 8.1'de kullanımdan kalktı.",
        "Calling key(), current(), next(), prev(), reset() or end() on an object is deprecated since PHP 8.1.",
    ),
);

/// The first rule whose checks all hold for a call, and the token to report it at.
pub fn find<'t>(function: &str, args: &[&'t [Token]]) -> Option<(&'static ArgRule, Option<&'t Token>)> {
    let rule = ARGUMENTS.iter().find(|r| r.functions.contains(&function) && r.checks.iter().all(|c| c.holds(args)))?;
    let at = rule.checks.iter().find_map(|c| c.slot()).and_then(|n| args.get(n)).and_then(|a| a.first());
    Some((rule, at))
}

const fn rule(id: &'static str, title: Text, severity: Severity, since: (u32, u32), replacement: Option<Text>, note: Text) -> PatternRule {
    PatternRule { id, title, severity, since, replacement, fix: FixKind::Manual, note }
}

const fn arg(functions: &'static [&'static str], checks: &'static [Check], rule: PatternRule) -> ArgRule {
    ArgRule { functions, checks, rule }
}

use Check::*;
use Severity::{Behavior as B, Breaking as R, Deprecated as D};

const fn tx(tr: &'static str, en: &'static str) -> Option<Text> {
    Some(Text::new(tr, en))
}

const fn code(both: &'static str) -> Option<Text> {
    Some(Text::same(both))
}

const NON_CRYPTO_HASHES: &[&str] = &[
    "adler32", "crc32", "crc32b", "crc32c", "fnv132", "fnv1a32", "fnv164", "fnv1a64", "joaat", "murmur3a", "murmur3c", "murmur3f", "xxh32",
    "xxh64", "xxh3", "xxh128",
];

const VERSION_OPERATORS: &[&str] = &["<", "lt", "<=", "le", ">", "gt", ">=", "ge", "==", "=", "eq", "!=", "<>", "ne"];

const CHAR_CODE: Text = Text::new(
    "Sayı olarak verilen bu argüman eskiden karakter kodu sayılıyordu (65 = \"A\"). PHP 8.0 onu rakamlardan oluşan bir metin olarak arar, yani sonuç sessizce değişir.",
    "An integer here used to mean a character code (65 = \"A\"). PHP 8.0 searches for the digits as a string instead, so the result silently changes.",
);

const IMPLICIT_CONNECTION: Text = Text::new(
    "pgsql fonksiyonlarını bağlantı argümanı olmadan, son açılan bağlantıya güvenerek çağırmak kullanımdan kalktı.",
    "Calling pgsql functions without the connection argument, relying on the last opened connection, is deprecated.",
);

const NULL_LENGTH: Text = Text::new(
    "PHP 8.0'dan önce null uzunluk 0 sayılırdı; artık \"sona kadar\" anlamına geliyor, yani sonuç değişir.",
    "Before PHP 8.0 a null length counted as 0; it now means \"to the end\", so the result changes.",
);

pub(crate) const POW_ZERO_NEGATIVE: PatternRule = rule(
    "deprecated.pow_zero_negative",
    Text::new("0'ın negatif kuvveti", "Zero to a negative power"),
    D,
    (8, 4),
    code("fpow()"),
    Text::new(
        "0'ın negatif bir kuvvetini almak PHP 8.4'te kullanımdan kalktı.",
        "Raising zero to a negative power is deprecated since PHP 8.4.",
    ),
);

const DRIVER_METHODS: &[&str] = &[
    "pdo::sqlitecreateaggregate",
    "pdo::sqlitecreatecollation",
    "pdo::sqlitecreatefunction",
    "pdo::pgsqlcopyfromarray",
    "pdo::pgsqlcopyfromfile",
    "pdo::pgsqlcopytoarray",
    "pdo::pgsqlcopytofile",
    "pdo::pgsqlgetnotify",
    "pdo::pgsqlgetpid",
    "pdo::pgsqllobcreate",
    "pdo::pgsqllobopen",
    "pdo::pgsqllobunlink",
];

const fn value_error(tr: &'static str, en: &'static str, fix_tr: &'static str, fix_en: &'static str) -> PatternRule {
    rule(
        "removed.literal_value_error",
        Text::new(tr, en),
        R,
        (8, 0),
        tx(fix_tr, fix_en),
        Text::new(
            "Bu değer eskiden uyarı verip false ya da null döndürürdü; PHP 8.0'dan beri ValueError fırlatır.",
            "This value used to warn and return false or null; since PHP 8.0 it throws a ValueError.",
        ),
    )
}

pub const ARGUMENTS: &[ArgRule] = &[
    arg(&["htmlspecialchars", "htmlentities", "htmlspecialchars_decode", "html_entity_decode"], &[Count(1, 1)], rule(
        HTML_FLAGS_ID,
        Text::new("HTML kodlamanın varsayılan bayrakları", "Default flags of HTML encoding"),
        B, (8, 1), code("ENT_COMPAT | ENT_HTML401"),
        Text::new(
            "PHP 8.1'den beri varsayılan bayraklar ENT_QUOTES | ENT_SUBSTITUTE | ENT_HTML401: tek tırnak da kodlanır, geçersiz UTF-8 boş sonuç yerine � olur. Bu dosyadaki bayraksız çağrıların hepsi için geçerlidir; yalnızca ilki gösterilir.",
            "Since PHP 8.1 the default flags are ENT_QUOTES | ENT_SUBSTITUTE | ENT_HTML401: single quotes are encoded too, and invalid UTF-8 becomes � instead of an empty result. This applies to every call without flags in this file; only the first is shown.",
        ),
    )),
    arg(&["setlocale"], &[Has(0, "LC_ALL"), ChangesLocale(1)], rule(
        "behavior.locale_conversions",
        Text::new("Locale artık dönüşümleri etkilemiyor", "The locale no longer affects conversions"),
        B, (8, 0), tx("biçimlendirme için NumberFormatter, Türkçe harfler için mb_strtolower() kullanın", "use NumberFormatter for formatting and mb_strtolower() for non-ASCII letters"),
        Text::new(
            "PHP 8.0'dan beri ondalık sayılar metne locale'den bağımsız noktayla çevrilir, 8.2'den beri strtolower(), strtoupper(), ucfirst() ve benzerleri yalnızca ASCII harfleri değiştirir. tr_TR ile I/ı ve İ/i dönüşümü artık yapılmaz.",
            "Since PHP 8.0 floats convert to strings with a dot whatever the locale, and since 8.2 strtolower(), strtoupper(), ucfirst() and similar functions change ASCII letters only. With tr_TR, I/ı and İ/i are no longer converted.",
        ),
    )),
    arg(&["setlocale"], &[Has(0, "LC_NUMERIC"), ChangesLocale(1)], rule(
        "behavior.locale_float",
        Text::new("Locale artık sayı biçimini etkilemiyor", "The locale no longer formats floats"),
        B, (8, 0), tx("biçimlendirme için number_format() ya da NumberFormatter kullanın", "use number_format() or NumberFormatter to format numbers"),
        Text::new("PHP 8.0'dan beri ondalık sayılar metne locale'den bağımsız noktayla çevrilir; 1,5 değil 1.5 yazılır.", "Since PHP 8.0 floats convert to strings with a dot whatever the locale, so 1.5 is no longer written as 1,5."),
    )),
    arg(&["setlocale"], &[Has(0, "LC_CTYPE"), ChangesLocale(1)], rule(
        "behavior.locale_case",
        Text::new("Locale artık harf dönüşümünü etkilemiyor", "The locale no longer changes letter case"),
        B, (8, 2), code("mb_strtolower()"),
        Text::new(
            "PHP 8.2'den beri strtolower(), strtoupper(), ucfirst() ve benzerleri yalnızca ASCII harfleri değiştirir. tr_TR ile I/ı ve İ/i dönüşümü artık yapılmaz.",
            "Since PHP 8.2 strtolower(), strtoupper(), ucfirst() and similar functions change ASCII letters only. With tr_TR, I/ı and İ/i are no longer converted.",
        ),
    )),
    arg(&["explode"], &[StrIs(0, &[""])], value_error("explode() boş ayraçla", "explode() with an empty separator", "tek karakterlere ayırmak için str_split() kullanın", "use str_split() to split into characters")),
    arg(&["str_repeat"], &[NumBelow(1, 0.0)], value_error("str_repeat() negatif sayıyla", "str_repeat() with a negative count", "sayıyı 0'dan küçük olmayacak şekilde sınırlayın", "keep the count at 0 or above")),
    arg(&["array_fill"], &[NumBelow(1, 0.0)], value_error("array_fill() negatif sayıyla", "array_fill() with a negative count", "sayıyı 0'dan küçük olmayacak şekilde sınırlayın", "keep the count at 0 or above")),
    arg(&["array_chunk", "str_split"], &[NumBelow(1, 1.0)], value_error("Sıfır ya da negatif parça boyu", "Zero or negative chunk length", "boyu en az 1 yapın", "use a length of at least 1")),
    arg(&["str_pad"], &[StrIs(2, &[""])], value_error("str_pad() boş dolgu metniyle", "str_pad() with an empty padding", "boş olmayan bir metin verin", "pass a non-empty string")),
    arg(&["mb_substr_count"], &[StrIs(1, &[""])], value_error("mb_substr_count() boş metinle", "mb_substr_count() with an empty needle", "boş olmayan bir metin verin", "pass a non-empty string")),
    arg(&["array_rand"], &[ArrayLitEmpty(0)], value_error("array_rand() boş diziyle", "array_rand() with an empty array", "önce dizinin boş olmadığını denetleyin", "check that the array isn't empty first")),
    arg(&["vsprintf", "vprintf"], &[Str(1)], rule(
        "removed.vsprintf_string",
        Text::new("Metin verilen vsprintf()", "vsprintf() given a string"),
        R, (8, 0), code("[$value]"),
        Text::new("vsprintf() ve vprintf() PHP 8.0'dan beri dizi dışında bir değer verilirse TypeError fırlatır.", "vsprintf() and vprintf() throw a TypeError since PHP 8.0 when given anything but an array."),
    )),
    arg(&["filesystemiterator::__construct"], &[Count(2, 2), Lacks(1, "SKIP_DOTS"), Has(1, "FilesystemIterator")], rule(
        "behavior.filesystem_iterator_dots",
        Text::new("FilesystemIterator bayrakları", "FilesystemIterator flags"),
        B, (8, 2), code("FilesystemIterator::SKIP_DOTS"),
        Text::new(
            "PHP 8.2'den beri bayrak verildiğinde SKIP_DOTS kendiliğinden eklenmez; döngüye . ve .. da girer.",
            "Since PHP 8.2 SKIP_DOTS is no longer added when flags are passed, so the loop also sees . and ..",
        ),
    )),
    arg(&["reflectionproperty::setvalue"], &[Count(1, 1)], rule(
        "deprecated.reflection_set_value_single",
        Text::new("Tek argümanlı ReflectionProperty::setValue()", "ReflectionProperty::setValue() with one argument"),
        D, (8, 3), code("setValue(null, $value)"),
        Text::new("ReflectionProperty::setValue() tek argümanla çağrılması PHP 8.3'te kullanımdan kalktı.", "Calling ReflectionProperty::setValue() with one argument is deprecated since PHP 8.3."),
    )),
    arg(DRIVER_METHODS, &[], rule(
        "deprecated.pdo_driver_method",
        Text::new("PDO'nun sürücüye özel metodu", "Driver-specific PDO method"),
        D, (8, 5), code("Pdo\\Sqlite / Pdo\\Pgsql"),
        Text::new(
            "PDO::sqlite* ve PDO::pgsql* metotları PHP 8.5'te kullanımdan kalktı; Pdo\\Sqlite ve Pdo\\Pgsql sınıflarındaki karşılıklarını kullanın.",
            "The PDO::sqlite* and PDO::pgsql* methods are deprecated since PHP 8.5; use their counterparts on Pdo\\Sqlite and Pdo\\Pgsql.",
        ),
    )),
    arg(&["curl_setopt"], &[Has(1, "CURLOPT_SAFE_UPLOAD"), False(2)], rule(
        "removed.curl_safe_upload_off",
        Text::new("CURLOPT_SAFE_UPLOAD kapatma", "Turning off CURLOPT_SAFE_UPLOAD"),
        R, (7, 0), code("new CURLFile($path)"),
        Text::new(
            "CURLOPT_SAFE_UPLOAD PHP 7.0'da kapatılamaz oldu; \"@dosya\" ile yükleme artık çalışmıyor.",
            "CURLOPT_SAFE_UPLOAD can't be turned off since PHP 7.0, so \"@file\" uploads no longer work.",
        ),
    )),
    arg(&["setcookie", "setrawcookie"], &[StrIs(0, &[""])], rule(
        "behavior.setcookie_empty_name",
        Text::new("Boş adlı çerez", "Cookie with an empty name"),
        B, (7, 0), tx("çereze bir ad verin", "give the cookie a name"),
        Text::new(
            "PHP 7.0'dan beri boş adlı bir çerez gönderilmez; çağrı uyarı verip false döner.",
            "Since PHP 7.0 a cookie with an empty name isn't sent; the call warns and returns false.",
        ),
    )),
    arg(&["gmp_setbit"], &[NumBelow(1, 0.0)], rule(
        "removed.gmp_setbit_negative",
        Text::new("Negatif bit indeksi", "Negative bit index"),
        R, (7, 0), None,
        Text::new("gmp_setbit() PHP 7.0'dan beri negatif indeksi kabul etmez.", "gmp_setbit() doesn't accept a negative index since PHP 7.0."),
    )),
    arg(&["setlocale"], &[Str(0)], rule(
        "removed.setlocale_string_category",
        Text::new("Metin olarak locale kategorisi", "Locale category as a string"),
        R, (7, 0), code("LC_ALL"),
        Text::new(
            "setlocale() kategoriyi PHP 7.0'dan beri yalnızca LC_* sabitiyle kabul eder; metin verildiğinde çalışmaz.",
            "setlocale() only accepts the category as an LC_* constant since PHP 7.0; a string doesn't work.",
        ),
    )),
    arg(&["hash_hmac", "hash_hmac_file", "hash_pbkdf2", "hash_hkdf"], &[StrIs(0, NON_CRYPTO_HASHES)], rule(
        "removed.hash_hmac_non_crypto",
        Text::new("Kriptografik olmayan HMAC algoritması", "Non-cryptographic HMAC algorithm"),
        R, (7, 2), code("'sha256'"),
        Text::new(
            "crc32 ya da fnv gibi kriptografik olmayan algoritmalar PHP 7.2'den beri HMAC ile kullanılamaz.",
            "Non-cryptographic algorithms such as crc32 or fnv can't be used for HMAC since PHP 7.2.",
        ),
    )),
    arg(&["hash_init"], &[StrIs(0, NON_CRYPTO_HASHES), Has(1, "HASH_HMAC")], rule(
        "removed.hash_hmac_non_crypto",
        Text::new("Kriptografik olmayan HMAC algoritması", "Non-cryptographic HMAC algorithm"),
        R, (7, 2), code("'sha256'"),
        Text::new(
            "crc32 ya da fnv gibi kriptografik olmayan algoritmalar PHP 7.2'den beri HMAC ile kullanılamaz.",
            "Non-cryptographic algorithms such as crc32 or fnv can't be used for HMAC since PHP 7.2.",
        ),
    )),
    arg(&["json_decode"], &[Null(1), Has(3, "JSON_OBJECT_AS_ARRAY")], rule(
        "behavior.json_decode_null_assoc",
        Text::new("null assoc ile JSON_OBJECT_AS_ARRAY", "JSON_OBJECT_AS_ARRAY with a null assoc"),
        B, (7, 2), code("json_decode($json, true)"),
        Text::new(
            "PHP 7.2'den beri assoc null olduğunda JSON_OBJECT_AS_ARRAY dikkate alınır; nesne yerine dizi döner.",
            "Since PHP 7.2 JSON_OBJECT_AS_ARRAY is honored when assoc is null, so arrays come back instead of objects.",
        ),
    )),
    arg(&["strpos", "stripos", "strrpos", "strripos", "strstr", "stristr", "strrchr", "strchr"], &[Int(1)], rule(
        "behavior.int_as_char_code",
        Text::new("Sayı olarak verilen needle", "Integer needle"),
        B, (7, 3), code("chr(65)"), CHAR_CODE,
    )),
    arg(&["mb_ereg_replace", "mb_eregi_replace"], &[Int(0)], rule(
        "behavior.int_as_char_code",
        Text::new("Sayı olarak verilen desen", "Integer pattern"),
        B, (7, 4), code("chr(65)"), CHAR_CODE,
    )),
    arg(&["stream_filter_append", "stream_filter_prepend"], &[StrIs(1, &["string.strip_tags"])], rule(
        "removed.strip_tags_filter",
        Text::new("string.strip_tags akış filtresi", "string.strip_tags stream filter"),
        R, (7, 3), code("strip_tags()"),
        Text::new(
            "string.strip_tags filtresi PHP 7.3'te kullanımdan kalktı ve 8.0'da kaldırıldı.",
            "The string.strip_tags filter was deprecated in PHP 7.3 and removed in 8.0.",
        ),
    )),
    arg(&["mb_strrpos"], &[Str(2)], rule(
        "removed.mb_strrpos_encoding",
        Text::new("Üçüncü argümanda encoding", "Encoding as the third argument"),
        R, (7, 4), code("mb_strrpos($haystack, $needle, 0, 'UTF-8')"),
        Text::new(
            "Encoding'i offset yerine üçüncü argüman olarak vermek PHP 7.4'te kullanımdan kalktı ve 8.0'da TypeError verir.",
            "Passing the encoding as the third argument instead of the offset was deprecated in PHP 7.4 and throws a TypeError in 8.0.",
        ),
    )),
    arg(&["bindec"], &[StrOutside(0, "01")], rule(
        "deprecated.invalid_digits",
        Text::new("Geçersiz basamak", "Invalid digit"),
        D, (7, 4), None,
        Text::new(
            "Geçersiz basamak içeren bir metni çevirmek PHP 7.4'ten beri uyarı verir; bu basamaklar yok sayılır.",
            "Converting a string with invalid digits raises a deprecation since PHP 7.4; those digits are ignored.",
        ),
    )),
    arg(&["base_convert"], &[InvalidForBase(0, 1)], rule(
        "deprecated.invalid_digits",
        Text::new("Geçersiz basamak", "Invalid digit"),
        D, (7, 4), None,
        Text::new("base_convert()'e tabanda bulunmayan basamak vermek PHP 7.4'te kullanımdan kalktı; bu basamaklar yok sayılır.", "Passing digits that aren't in the base to base_convert() is deprecated since PHP 7.4; they're ignored."),
    )),
    arg(&["intldateformatter::settimezoneid"], &[], rule(
        "removed.datefmt_set_timezone_id",
        Text::new("IntlDateFormatter::setTimeZoneID()", "IntlDateFormatter::setTimeZoneID()"),
        R, (7, 0), code("setTimeZone()"),
        Text::new("IntlDateFormatter::setTimeZoneID() PHP 7.0'da kaldırıldı; setTimeZone() kullanın.", "IntlDateFormatter::setTimeZoneID() was removed in PHP 7.0; use setTimeZone()."),
    )),
    arg(&["octdec"], &[StrOutside(0, "01234567")], rule(
        "deprecated.invalid_digits",
        Text::new("Geçersiz basamak", "Invalid digit"),
        D, (7, 4), None,
        Text::new(
            "Geçersiz basamak içeren bir metni çevirmek PHP 7.4'ten beri uyarı verir; bu basamaklar yok sayılır.",
            "Converting a string with invalid digits raises a deprecation since PHP 7.4; those digits are ignored.",
        ),
    )),
    arg(&["hexdec"], &[StrOutside(0, "0123456789abcdef")], rule(
        "deprecated.invalid_digits",
        Text::new("Geçersiz basamak", "Invalid digit"),
        D, (7, 4), None,
        Text::new(
            "Geçersiz basamak içeren bir metni çevirmek PHP 7.4'ten beri uyarı verir; bu basamaklar yok sayılır.",
            "Converting a string with invalid digits raises a deprecation since PHP 7.4; those digits are ignored.",
        ),
    )),
    arg(&["com_load_typelib"], &[False(1)], rule(
        "deprecated.com_typelib_case_insensitive",
        Text::new("Büyük/küçük harfe duyarsız typelib", "Case-insensitive type library"),
        D, (7, 4), tx("argümanı silin", "remove the argument"),
        Text::new(
            "Tür kütüphanesini büyük/küçük harfe duyarsız kaydetmek PHP 7.4'te kullanımdan kalktı ve 8.0'da kaldırıldı.",
            "Registering a type library case-insensitively was deprecated in PHP 7.4 and removed in 8.0.",
        ),
    )),
    arg(&["imagecropauto"], &[NumIs(1, -1.0)], rule(
        "removed.imagecropauto_default",
        Text::new("imagecropauto() modu -1", "imagecropauto() mode -1"),
        R, (8, 0), code("IMG_CROP_DEFAULT"),
        Text::new("imagecropauto() PHP 8.0'dan beri -1 modunu kabul etmez.", "imagecropauto() no longer accepts the mode -1 since PHP 8.0."),
    )),
    arg(&["ldap_set_rebind_proc"], &[StrIs(1, &[""])], rule(
        "deprecated.ldap_rebind_empty",
        Text::new("Boş metin callback", "Empty string callback"),
        D, (8, 0), code("null"),
        Text::new(
            "Callback'i kaldırmak için boş metin vermek PHP 8.0'da kullanımdan kalktı; null verin.",
            "Passing an empty string to remove the callback is deprecated since PHP 8.0; pass null.",
        ),
    )),
    arg(&["mb_ereg_replace", "mb_eregi_replace"], &[StrHas(3, 'e')], rule(
        "removed.mb_ereg_e_option",
        Text::new("mb_ereg_replace() e seçeneği", "mb_ereg_replace() e option"),
        R, (8, 0), code("mb_ereg_replace_callback()"),
        Text::new(
            "Değiştirme metnini kod olarak çalıştıran e seçeneği PHP 8.0'da kaldırıldı.",
            "The e option, which ran the replacement as code, was removed in PHP 8.0.",
        ),
    )),
    arg(&["spl_autoload_register"], &[False(1)], rule(
        "behavior.spl_autoload_register_throw",
        Text::new("spl_autoload_register() throw=false", "spl_autoload_register() throw=false"),
        B, (8, 0), tx("argümanı silin", "remove the argument"),
        Text::new(
            "PHP 8.0'dan beri bu fonksiyon her zaman exception fırlatır; false verilirse yok sayılır ve uyarı verir.",
            "Since PHP 8.0 this function always throws; false is ignored and raises a warning.",
        ),
    )),
    arg(&["substr", "array_splice"], &[Null(2)], rule(
        "behavior.null_length",
        Text::new("null uzunluk", "Null length"),
        B, (8, 0), tx("argümanı silin", "remove the argument"), NULL_LENGTH,
    )),
    arg(&["substr_compare"], &[Null(3)], rule(
        "behavior.null_length",
        Text::new("null uzunluk", "Null length"),
        B, (8, 0), tx("argümanı silin", "remove the argument"), NULL_LENGTH,
    )),
    arg(&["iconv_substr"], &[Null(2)], rule(
        "behavior.null_length",
        Text::new("null uzunluk", "Null length"),
        B, (8, 0), tx("argümanı silin", "remove the argument"), NULL_LENGTH,
    )),
    arg(&["substr_count"], &[Null(3)], rule(
        "behavior.null_length",
        Text::new("null uzunluk", "Null length"),
        B, (8, 0), tx("argümanı silin", "remove the argument"), NULL_LENGTH,
    )),
    arg(&["password_hash"], &[Key(2, "salt")], rule(
        "behavior.password_hash_salt",
        Text::new("password_hash() salt seçeneği", "password_hash() salt option"),
        B, (8, 0), tx("salt seçeneğini silin", "remove the salt option"),
        Text::new(
            "salt seçeneği PHP 8.0'da kaldırıldı; yok sayılır, uyarı verilir ve rastgele bir salt kullanılır.",
            "The salt option was removed in PHP 8.0; it's ignored with a warning and a random salt is used.",
        ),
    )),
    arg(&["get_defined_functions"], &[False(0)], rule(
        "deprecated.get_defined_functions_arg",
        Text::new("get_defined_functions() argümanı", "get_defined_functions() argument"),
        D, (8, 0), tx("argümanı silin", "remove the argument"),
        Text::new(
            "Devre dışı fonksiyonları da istemek için false vermek PHP 8.0'da kullanımdan kalktı; argüman 8.5'te tamamen kullanımdan kalktı.",
            "Passing false to include disabled functions is deprecated since PHP 8.0, and the argument itself is deprecated in 8.5.",
        ),
    )),
    arg(&["get_defined_functions"], &[Count(1, 1)], rule(
        "deprecated.get_defined_functions_arg",
        Text::new("get_defined_functions() argümanı", "get_defined_functions() argument"),
        D, (8, 5), tx("argümanı silin", "remove the argument"),
        Text::new("get_defined_functions()'ın argümanı PHP 8.5'te kullanımdan kalktı.", "The argument of get_defined_functions() is deprecated in PHP 8.5."),
    )),
    arg(&["version_compare"], &[StrNot(2, VERSION_OPERATORS)], rule(
        "removed.version_compare_operator",
        Text::new("Geçersiz version_compare() operatörü", "Invalid version_compare() operator"),
        R, (8, 1), code("'<'"),
        Text::new(
            "version_compare() PHP 8.1'den beri belgelenmemiş kısaltmaları kabul etmez; ValueError fırlatır.",
            "version_compare() rejects undocumented operator abbreviations since PHP 8.1 and throws a ValueError.",
        ),
    )),
    arg(
        &["ctype_alnum", "ctype_alpha", "ctype_cntrl", "ctype_digit", "ctype_graph", "ctype_lower", "ctype_print", "ctype_punct", "ctype_space", "ctype_upper", "ctype_xdigit"],
        &[Int(0)],
        rule(
            "deprecated.ctype_non_string",
            Text::new("ctype_*() fonksiyonuna sayı", "Integer passed to ctype_*()"),
            D, (8, 1), code("chr(65)"),
            Text::new(
                "ctype_*() fonksiyonlarına sayı vermek PHP 8.1'de kullanımdan kalktı; -128 ile 255 arası sayılar karakter kodu sayılıyordu.",
                "Passing an integer to ctype_*() is deprecated since PHP 8.1; integers from -128 to 255 were treated as character codes.",
            ),
        ),
    ),
    arg(&["imagepolygon", "imagefilledpolygon", "imageopenpolygon"], &[Count(4, 4)], rule(
        "deprecated.polygon_num_points",
        Text::new("num_points argümanı", "num_points argument"),
        D, (8, 1), tx("nokta sayısı argümanını silin", "remove the point count argument"),
        Text::new("Nokta sayısını ayrıca vermek PHP 8.1'de kullanımdan kalktı.", "Passing the number of points separately is deprecated since PHP 8.1."),
    )),
    arg(&["intlcal_roll"], &[Bool(2)], rule(
        "deprecated.intlcal_roll_bool",
        Text::new("intlcal_roll() bool değer", "intlcal_roll() bool value"),
        D, (8, 1), code("1 / -1"),
        Text::new("intlcal_roll()'a bool vermek PHP 8.1'de kullanımdan kalktı; 1 ya da -1 verin.", "Passing a bool to intlcal_roll() is deprecated since PHP 8.1; pass 1 or -1."),
    )),
    arg(&["pg_query", "pg_escape_string", "pg_escape_literal", "pg_escape_identifier", "pg_escape_bytea", "pg_set_client_encoding", "pg_parameter_status", "pg_lo_unlink", "pg_put_line"], &[Count(1, 1)], rule(
        "deprecated.pg_implicit_connection",
        Text::new("Bağlantısız pgsql çağrısı", "pgsql call without a connection"),
        D, (8, 1), tx("bağlantıyı ilk argüman olarak verin", "pass the connection as the first argument"), IMPLICIT_CONNECTION,
    )),
    arg(&["pg_query_params", "pg_prepare", "pg_execute", "pg_lo_export"], &[Count(2, 2)], rule(
        "deprecated.pg_implicit_connection",
        Text::new("Bağlantısız pgsql çağrısı", "pgsql call without a connection"),
        D, (8, 1), tx("bağlantıyı ilk argüman olarak verin", "pass the connection as the first argument"), IMPLICIT_CONNECTION,
    )),
    arg(&["pg_last_error", "pg_errormessage", "pg_dbname", "pg_host", "pg_options", "pg_port", "pg_tty", "pg_ping", "pg_close", "pg_version", "pg_client_encoding", "pg_untrace", "pg_lo_create", "pg_end_copy"], &[Count(0, 0)], rule(
        "deprecated.pg_implicit_connection",
        Text::new("Bağlantısız pgsql çağrısı", "pgsql call without a connection"),
        D, (8, 1), tx("bağlantıyı ilk argüman olarak verin", "pass the connection as the first argument"), IMPLICIT_CONNECTION,
    )),
    arg(&["pg_fetch_result", "pg_field_prtlen", "pg_field_is_null"], &[Count(2, 2)], rule(
        "deprecated.pg_implicit_row",
        Text::new("Satır numarası olmayan çağrı", "Call without a row number"),
        D, (8, 4), tx("satır numarasını ikinci argüman olarak verin", "pass the row number as the second argument"),
        Text::new("Satırı vermeden iki argümanla çağırmak PHP 8.4'te kullanımdan kalktı.", "Calling it with two arguments, without the row, is deprecated since PHP 8.4."),
    )),
    arg(&["openssl_pkcs7_encrypt"], &[Count(4, 5)], rule(
        "behavior.openssl_default_cipher",
        Text::new("Varsayılan şifreleme algoritması", "Default cipher"),
        B, (8, 1), code("OPENSSL_CIPHER_AES_128_CBC"),
        Text::new(
            "Varsayılan algoritma PHP 8.1'de RC2-40'tan AES-128-CBC'ye değişti; eski alıcılar mesajı açamayabilir.",
            "The default cipher changed from RC2-40 to AES-128-CBC in PHP 8.1; old recipients may not be able to decrypt it.",
        ),
    )),
    arg(&["openssl_cms_encrypt"], &[Count(3, 6)], rule(
        "behavior.openssl_default_cipher",
        Text::new("Varsayılan şifreleme algoritması", "Default cipher"),
        B, (8, 1), code("OPENSSL_CIPHER_AES_128_CBC"),
        Text::new(
            "Varsayılan algoritma PHP 8.1'de RC2-40'tan AES-128-CBC'ye değişti; eski alıcılar mesajı açamayabilir.",
            "The default cipher changed from RC2-40 to AES-128-CBC in PHP 8.1; old recipients may not be able to decrypt it.",
        ),
    )),
    arg(&["mysqli_get_client_info"], &[Count(1, 1)], rule(
        "deprecated.mysqli_get_client_info_arg",
        Text::new("mysqli_get_client_info() argümanı", "mysqli_get_client_info() argument"),
        D, (8, 1), tx("argümanı silin", "remove the argument"),
        Text::new("mysqli_get_client_info()'ya bağlantı vermek PHP 8.1'de kullanımdan kalktı.", "Passing a connection to mysqli_get_client_info() is deprecated since PHP 8.1."),
    )),
    arg(&["file"], &[Has(1, "FILE_APPEND")], rule(
        "removed.file_append_flag",
        Text::new("file() ile FILE_APPEND", "FILE_APPEND with file()"),
        R, (8, 3), None,
        Text::new("file() PHP 8.3'ten beri geçersiz flag'lerde ValueError fırlatır; FILE_APPEND bunlardan biri.", "file() throws a ValueError for invalid flags since PHP 8.3, and FILE_APPEND is one of them."),
    )),
    arg(&["dba_fetch"], &[Count(3, 3), Int(1)], rule(
        "deprecated.dba_fetch_order",
        Text::new("Eski dba_fetch() argüman sırası", "Old dba_fetch() argument order"),
        D, (8, 3), code("dba_fetch($key, $dba, $skip)"),
        Text::new("dba_fetch()'te handle'ı üçüncü argüman olarak vermek PHP 8.3'te kullanımdan kalktı.", "Passing the handle as the third argument of dba_fetch() is deprecated since PHP 8.3."),
    )),
    arg(&["curl_getinfo"], &[Has(1, "CURLINFO_CERTINFO")], rule(
        "behavior.curl_certinfo_format",
        Text::new("CURLINFO_CERTINFO sonucunun biçimi", "CURLINFO_CERTINFO result format"),
        B, (7, 0), None,
        Text::new("PHP 7.0'dan beri sertifikanın Subject ve Issuer alanları dizi yerine metin olarak döner.", "Since PHP 7.0 the certificate's Subject and Issuer come back as strings instead of arrays."),
    )),
    arg(&["idn_to_ascii", "idn_to_utf8"], &[Count(1, 2)], rule(
        "behavior.idn_default_variant",
        Text::new("Değişen IDN varsayılanı", "Changed IDN default"),
        B, (7, 4), code("INTL_IDNA_VARIANT_UTS46"),
        Text::new("PHP 7.4'ten beri varsayılan varyant INTL_IDNA_VARIANT_2003 değil, INTL_IDNA_VARIANT_UTS46; bazı alan adları farklı dönüşebilir.", "Since PHP 7.4 the default variant is INTL_IDNA_VARIANT_UTS46 instead of INTL_IDNA_VARIANT_2003, so some domains can convert differently."),
    )),
    arg(&["call_user_func_array"], &[StringKeys(1)], rule(
        "behavior.call_user_func_array_named",
        Text::new("Dizi anahtarları parametre adı oluyor", "Array keys become parameter names"),
        B, (8, 0), tx("array_values() ile sarın", "wrap the array in array_values()"),
        Text::new("PHP 8.0'dan beri call_user_func_array() metin anahtarlarını parametre adı sayar; değerler sırayla değil adlarına göre geçer, eşleşmeyen ad Error fırlatır.", "Since PHP 8.0 call_user_func_array() treats string keys as parameter names, so values go by name instead of by position and an unknown name throws an Error."),
    )),
    arg(&["array_fill"], &[NumBelow(0, 0.0)], NEGATIVE_KEY),
    arg(&["array_key_exists", "key_exists"], &[This(1)], OBJECT_KEY_EXISTS),
    arg(&["array_key_exists", "key_exists"], &[New(1)], OBJECT_KEY_EXISTS),
    arg(&["key", "current", "pos", "next", "prev", "reset", "end"], &[This(0)], OBJECT_POINTER),
    arg(&["key", "current", "pos", "next", "prev", "reset", "end"], &[New(0)], OBJECT_POINTER),
    arg(&["ldap_connect"], &[Count(2, 2)], rule(
        "deprecated.ldap_connect_port",
        Text::new("Ayrı host ve port", "Separate host and port"),
        D, (8, 3), code("ldap_connect('ldap://host:389')"),
        Text::new("ldap_connect()'e host ve portu ayrı vermek PHP 8.3'te kullanımdan kalktı; bir URI verin.", "Passing host and port separately to ldap_connect() is deprecated since PHP 8.3; pass a URI."),
    )),
    arg(&["mb_strimwidth"], &[NumBelow(2, 0.0)], rule(
        "deprecated.mb_strimwidth_negative",
        Text::new("Negatif genişlik", "Negative width"),
        D, (8, 3), None,
        Text::new("mb_strimwidth()'e negatif genişlik vermek PHP 8.3'te kullanımdan kalktı.", "Passing a negative width to mb_strimwidth() is deprecated since PHP 8.3."),
    )),
    arg(&["curl_multi_select"], &[NumBelow(1, 0.0)], rule(
        "removed.curl_multi_select_negative",
        Text::new("Negatif zaman aşımı", "Negative timeout"),
        R, (8, 4), None,
        Text::new("curl_multi_select() PHP 8.4'ten beri negatif zaman aşımında ValueError fırlatır.", "curl_multi_select() throws a ValueError for a negative timeout since PHP 8.4."),
    )),
    arg(&["imagejpeg", "imagewebp", "imageavif"], &[NumOutside(2, -1.0, 100.0)], rule(
        "removed.image_quality_range",
        Text::new("Aralık dışı kalite", "Quality out of range"),
        R, (8, 4), None,
        Text::new("GD PHP 8.4'ten beri aralık dışındaki kalite değerlerinde ValueError fırlatır.", "GD throws a ValueError for quality values out of range since PHP 8.4."),
    )),
    arg(&["imagepng"], &[NumOutside(2, -1.0, 9.0)], rule(
        "removed.image_quality_range",
        Text::new("Aralık dışı kalite", "Quality out of range"),
        R, (8, 4), None,
        Text::new("GD PHP 8.4'ten beri aralık dışındaki kalite değerlerinde ValueError fırlatır.", "GD throws a ValueError for quality values out of range since PHP 8.4."),
    )),
    arg(&["textdomain", "dgettext", "dcgettext", "dngettext", "dcngettext", "bindtextdomain", "bind_textdomain_codeset"], &[StrIs(0, &[""])], rule(
        "removed.gettext_empty_domain",
        Text::new("Boş gettext domain'i", "Empty gettext domain"),
        R, (8, 4), None,
        Text::new("gettext fonksiyonları PHP 8.4'ten beri boş domain'de ValueError fırlatır.", "gettext functions throw a ValueError for an empty domain since PHP 8.4."),
    )),
    arg(&["odbc_fetch_row"], &[NumBelow(1, 1.0)], rule(
        "removed.odbc_row_zero",
        Text::new("1'den küçük satır numarası", "Row number below 1"),
        R, (8, 4), None,
        Text::new("odbc_fetch_row() PHP 8.4'ten beri 1'den küçük satır numarasında ValueError fırlatır.", "odbc_fetch_row() throws a ValueError for a row number below 1 since PHP 8.4."),
    )),
    arg(&["round"], &[NumOutside(2, 1.0, 4.0)], rule(
        "removed.round_invalid_mode",
        Text::new("Geçersiz round() modu", "Invalid round() mode"),
        R, (8, 4), code("PHP_ROUND_HALF_UP"),
        Text::new("round() PHP 8.4'ten beri geçersiz modda ValueError fırlatır.", "round() throws a ValueError for an invalid mode since PHP 8.4."),
    )),
    arg(&["php_uname"], &[StrNot(0, &["a", "m", "n", "r", "s", "v"])], rule(
        "removed.php_uname_mode",
        Text::new("Geçersiz php_uname() modu", "Invalid php_uname() mode"),
        R, (8, 4), code("'a'"),
        Text::new("php_uname() PHP 8.4'ten beri geçersiz modda ValueError fırlatır.", "php_uname() throws a ValueError for an invalid mode since PHP 8.4."),
    )),
    arg(&["pow"], &[NumIs(0, 0.0), NumBelow(1, 0.0)], POW_ZERO_NEGATIVE),
    arg(&["dba_key_split"], &[Null(0)], rule(
        "deprecated.dba_key_split_null",
        Text::new("dba_key_split() null/false", "dba_key_split() null/false"),
        D, (8, 4), None,
        Text::new("dba_key_split()'e null ya da false vermek PHP 8.4'te kullanımdan kalktı.", "Passing null or false to dba_key_split() is deprecated since PHP 8.4."),
    )),
    arg(&["dba_key_split"], &[False(0)], rule(
        "deprecated.dba_key_split_null",
        Text::new("dba_key_split() null/false", "dba_key_split() null/false"),
        D, (8, 4), None,
        Text::new("dba_key_split()'e null ya da false vermek PHP 8.4'te kullanımdan kalktı.", "Passing null or false to dba_key_split() is deprecated since PHP 8.4."),
    )),
    arg(&["ldap_exop"], &[Count(5, 6)], rule(
        "deprecated.ldap_exop_response",
        Text::new("ldap_exop() yanıt argümanları", "ldap_exop() response arguments"),
        D, (8, 4), code("ldap_exop_sync()"),
        Text::new("ldap_exop()'un yanıt argümanları PHP 8.4'te kullanımdan kalktı.", "The response arguments of ldap_exop() are deprecated since PHP 8.4."),
    )),
    arg(&["stream_context_set_option"], &[Count(2, 2)], rule(
        "deprecated.stream_context_option_array",
        Text::new("Dizi ile stream_context_set_option()", "stream_context_set_option() with an array"),
        D, (8, 4), code("stream_context_set_options()"),
        Text::new("stream_context_set_option()'a dizi vermek PHP 8.4'te kullanımdan kalktı.", "Passing an array to stream_context_set_option() is deprecated since PHP 8.4."),
    )),
    arg(&["mysqli_store_result"], &[Count(2, 2)], rule(
        "deprecated.mysqli_store_result_mode",
        Text::new("mysqli_store_result() modu", "mysqli_store_result() mode"),
        D, (8, 4), tx("argümanı silin", "remove the argument"),
        Text::new("mysqli_store_result()'a mod vermek PHP 8.4'te kullanımdan kalktı.", "Passing a mode to mysqli_store_result() is deprecated since PHP 8.4."),
    )),
    arg(&["bzcompress"], &[NumOutside(1, 1.0, 9.0)], rule(
        "removed.bzcompress_range",
        Text::new("Aralık dışı bzcompress() değeri", "bzcompress() value out of range"),
        R, (8, 5), None,
        Text::new("bzcompress() PHP 8.5'ten beri aralık dışındaki block size ve work factor değerlerinde ValueError fırlatır.", "bzcompress() throws a ValueError for an out-of-range block size or work factor since PHP 8.5."),
    )),
    arg(&["bzcompress"], &[NumOutside(2, 0.0, 250.0)], rule(
        "removed.bzcompress_range",
        Text::new("Aralık dışı bzcompress() değeri", "bzcompress() value out of range"),
        R, (8, 5), None,
        Text::new("bzcompress() PHP 8.5'ten beri aralık dışındaki block size ve work factor değerlerinde ValueError fırlatır.", "bzcompress() throws a ValueError for an out-of-range block size or work factor since PHP 8.5."),
    )),
    arg(&["finfo_buffer"], &[Count(4, 4)], rule(
        "deprecated.finfo_buffer_context",
        Text::new("finfo_buffer() context argümanı", "finfo_buffer() context argument"),
        D, (8, 5), tx("argümanı silin", "remove the argument"),
        Text::new("finfo_buffer()'ın yok sayılan $context argümanı PHP 8.5'te kullanımdan kalktı.", "The ignored $context argument of finfo_buffer() is deprecated in PHP 8.5."),
    )),
    arg(&["openssl_pkey_derive"], &[Count(3, 3)], rule(
        "deprecated.openssl_pkey_derive_length",
        Text::new("openssl_pkey_derive() uzunluk argümanı", "openssl_pkey_derive() length argument"),
        D, (8, 5), tx("argümanı silin", "remove the argument"),
        Text::new("openssl_pkey_derive()'ın $key_length argümanı PHP 8.5'te kullanımdan kalktı.", "The $key_length argument of openssl_pkey_derive() is deprecated in PHP 8.5."),
    )),
    arg(&["spl_autoload_unregister"], &[StrIs(0, &["spl_autoload_call"])], rule(
        "deprecated.unregister_autoload_call",
        Text::new("spl_autoload_call'ı kaldırma", "Unregistering spl_autoload_call"),
        D, (8, 5), None,
        Text::new(
            "spl_autoload_call'ı kaldırarak bütün autoloader'ları silmek PHP 8.5'te kullanımdan kalktı.",
            "Unregistering spl_autoload_call to remove every autoloader is deprecated in PHP 8.5.",
        ),
    )),
    arg(&["readdir", "rewinddir", "closedir"], &[Null(0)], rule(
        "deprecated.implicit_argument",
        Text::new("null handle", "Null handle"),
        D, (8, 5), tx("handle'ı açıkça verin", "pass the handle explicitly"),
        Text::new("Son açılan dizine güvenerek null vermek PHP 8.5'te kullanımdan kalktı.", "Passing null to use the last opened directory is deprecated in PHP 8.5."),
    )),
    arg(&["chr"], &[NumOutside(0, 0.0, 255.0)], rule(
        "deprecated.chr_out_of_range",
        Text::new("Aralık dışı chr() değeri", "chr() value out of range"),
        D, (8, 5), None,
        Text::new("chr()'a 0–255 dışında bir sayı vermek PHP 8.5'te kullanımdan kalktı.", "Passing a number outside 0–255 to chr() is deprecated in PHP 8.5."),
    )),
    arg(&["ord"], &[StrLenNot(0, 1)], rule(
        "deprecated.ord_length",
        Text::new("Tek karakter olmayan ord() argümanı", "ord() argument that isn't one character"),
        D, (8, 5), code("ord($s[0])"),
        Text::new("ord()'a boş ya da birden uzun bir metin vermek PHP 8.5'te kullanımdan kalktı.", "Passing an empty or longer string to ord() is deprecated in PHP 8.5."),
    )),
    arg(&["splpriorityqueue::setextractflags"], &[NumIs(0, 0.0)], rule(
        "removed.extract_flags_zero",
        Text::new("setExtractFlags(0)", "setExtractFlags(0)"),
        R, (7, 4), code("SplPriorityQueue::EXTR_DATA"),
        Text::new("SplPriorityQueue::setExtractFlags() PHP 7.4'ten beri 0 verildiğinde exception fırlatır.", "SplPriorityQueue::setExtractFlags() throws for 0 since PHP 7.4."),
    )),
    arg(&["pdostatement::bindcolumn"], &[Has(2, "PARAM_LOB")], rule(
        "behavior.bind_column_lob",
        Text::new("PARAM_LOB ile bindColumn()", "bindColumn() with PARAM_LOB"),
        B, (8, 1), None,
        Text::new("PHP 8.1'den beri PDO::PARAM_LOB ile bağlanan sütun her zaman bir stream verir; metin bekleyen kod bozulur.", "Since PHP 8.1 a column bound with PDO::PARAM_LOB is always a stream, which breaks code expecting a string."),
    )),
    arg(&["splfileobject::ftell", "splfileobject::fflush", "splfileobject::getcsvcontrol", "splfileobject::eof", "splfileobject::valid", "splfileobject::fgetc", "splfileobject::fpassthru", "splfileobject::getmaxlinelen"], &[Count(1, 9)], rule(
        "removed.splfileobject_extra_argument",
        Text::new("Fazladan SplFileObject argümanı", "Extra SplFileObject argument"),
        R, (8, 2), tx("argümanı silin", "remove the argument"),
        Text::new("Bu SplFileObject metotları PHP 8.2'den beri argüman verildiğinde ArgumentCountError fırlatır.", "These SplFileObject methods throw an ArgumentCountError for any argument since PHP 8.2."),
    )),
    arg(&["phar::setstub", "phardata::setstub"], &[Count(2, 2)], rule(
        "deprecated.phar_setstub_length",
        Text::new("setStub() uzunluk argümanı", "setStub() length argument"),
        D, (8, 3), tx("stub'ı metin olarak verin", "pass the stub as a string"),
        Text::new("Phar::setStub()'a resource ve uzunluk vermek PHP 8.3'te kullanımdan kalktı.", "Passing a resource and a length to Phar::setStub() is deprecated since PHP 8.3."),
    )),
    arg(&["sqlite3::enableexceptions"], &[False(0)], rule(
        "deprecated.sqlite3_exceptions_off",
        Text::new("SQLite3 exception'larını kapatma", "Turning off SQLite3 exceptions"),
        D, (8, 3), None,
        Text::new("SQLite3::enableExceptions(false) PHP 8.3'te kullanımdan kalktı; ileride exception'lar hep açık olacak.", "SQLite3::enableExceptions(false) is deprecated since PHP 8.3; exceptions will always be on."),
    )),
    arg(&["reflectionmethod::__construct"], &[Count(1, 1)], rule(
        "deprecated.reflection_method_string",
        Text::new("Tek argümanlı new ReflectionMethod()", "new ReflectionMethod() with one argument"),
        D, (8, 4), code("ReflectionMethod::createFromMethodName()"),
        Text::new("new ReflectionMethod('Sınıf::metot') PHP 8.4'te kullanımdan kalktı.", "new ReflectionMethod('Class::method') is deprecated since PHP 8.4."),
    )),
    arg(&["splfileobject::fgetcsv", "splfileobject::setcsvcontrol"], &[Count(0, 2)], rule(
        "deprecated.csv_escape",
        Text::new("Varsayılan CSV escape karakteri", "Default CSV escape character"),
        D, (8, 4), tx("$escape argümanını açıkça verin", "pass $escape explicitly"),
        Text::new("CSV fonksiyonlarını $escape vermeden çağırmak PHP 8.4'te kullanımdan kalktı.", "Calling CSV functions without $escape is deprecated since PHP 8.4."),
    )),
    arg(&["splfileobject::fputcsv"], &[Count(1, 3)], rule(
        "deprecated.csv_escape",
        Text::new("Varsayılan CSV escape karakteri", "Default CSV escape character"),
        D, (8, 4), tx("$escape argümanını açıkça verin", "pass $escape explicitly"),
        Text::new("CSV fonksiyonlarını $escape vermeden çağırmak PHP 8.4'te kullanımdan kalktı.", "Calling CSV functions without $escape is deprecated since PHP 8.4."),
    )),
    arg(&["pdostatement::setfetchmode", "pdostatement::fetchall", "pdo::query"], &[Has(0, "FETCH_CLASS"), StringKeys(2)], rule(
        "removed.fetch_class_named_args",
        Text::new("Anahtarlı constructor argümanları", "Keyed constructor arguments"),
        R, (8, 5), tx("anahtarsız bir dizi verin", "pass a list without keys"),
        Text::new("PHP 8.5'ten beri FETCH_CLASS'ın constructor argümanlarında metin anahtar kullanılamaz.", "Since PHP 8.5 the constructor arguments of FETCH_CLASS can't have string keys."),
    )),
    arg(&["pdostatement::fetchall", "pdostatement::fetch", "pdostatement::setfetchmode"], &[Has(0, "FETCH_PROPS_LATE"), Lacks(0, "FETCH_CLASS")], rule(
        "removed.fetch_props_late_mode",
        Text::new("FETCH_CLASS olmadan FETCH_PROPS_LATE", "FETCH_PROPS_LATE without FETCH_CLASS"),
        R, (8, 5), None,
        Text::new("PHP 8.5'ten beri FETCH_PROPS_LATE yalnızca FETCH_CLASS ile kullanılabilir.", "Since PHP 8.5 FETCH_PROPS_LATE can only be used with FETCH_CLASS."),
    )),
    arg(&["pdostatement::fetchall"], &[Has(0, "FETCH_INTO")], rule(
        "removed.fetch_all_into",
        Text::new("fetchAll() ile FETCH_INTO", "FETCH_INTO with fetchAll()"),
        R, (8, 5), code("fetch()"),
        Text::new("PHP 8.5'ten beri fetchAll() FETCH_INTO modunu kabul etmez.", "Since PHP 8.5 fetchAll() doesn't accept the FETCH_INTO mode."),
    )),
    arg(&["simplexmlelement::xpath"], &[StrStarts(0, &["count(", "sum(", "string(", "number(", "boolean(", "concat(", "name(", "local-name(", "string-length("])], rule(
        "removed.xpath_non_node_set",
        Text::new("Düğüm dönmeyen XPath ifadesi", "XPath expression that doesn't return nodes"),
        R, (8, 5), code("DOMXPath::evaluate()"),
        Text::new("SimpleXMLElement::xpath() PHP 8.5'ten beri düğüm kümesi dönmeyen ifadelerde hata fırlatır.", "SimpleXMLElement::xpath() throws for expressions that don't return a node set since PHP 8.5."),
    )),
    arg(
        &[
            "pdo::sqlitecreatefunction", "pdo::sqlitecreateaggregate", "pdo::sqlitecreatecollation", "pdo::pgsqlcopyfromarray", "pdo::pgsqlcopyfromfile",
            "pdo::pgsqlcopytoarray", "pdo::pgsqlcopytofile", "pdo::pgsqlgetnotify", "pdo::pgsqlgetpid", "pdo::pgsqllobcreate", "pdo::pgsqllobopen",
            "pdo::pgsqllobunlink",
        ],
        &[Count(0, 9)],
        rule(
            "deprecated.pdo_driver_method",
            Text::new("PDO üzerinde sürücüye özel metot", "Driver-specific method on PDO"),
            D, (8, 5), code("Pdo\\Sqlite / Pdo\\Pgsql"),
            Text::new(
                "Sürücüye özel PDO metotları PHP 8.5'te kullanımdan kalktı; Pdo\\Sqlite ya da Pdo\\Pgsql alt sınıfındaki karşılığını kullanın.",
                "Driver-specific PDO methods are deprecated in PHP 8.5; use the method on the Pdo\\Sqlite or Pdo\\Pgsql subclass.",
            ),
        ),
    ),
    arg(&["intlcalendar::roll", "intlgregoriancalendar::roll"], &[Bool(1)], rule(
        "deprecated.intlcal_roll_bool",
        Text::new("roll() bool değer", "roll() bool value"),
        D, (8, 1), code("1 / -1"),
        Text::new("IntlCalendar::roll()'a bool vermek PHP 8.1'de kullanımdan kalktı; 1 ya da -1 verin.", "Passing a bool to IntlCalendar::roll() is deprecated since PHP 8.1; pass 1 or -1."),
    )),
    arg(&["intlcalendar::set", "intlgregoriancalendar::set"], &[Count(3, 6)], rule(
        "deprecated.intlcal_set",
        Text::new("Çok argümanlı IntlCalendar::set()", "IntlCalendar::set() with several arguments"),
        D, (8, 4), code("setDate() / setDateTime()"),
        Text::new("IntlCalendar::set()'i tarih parçalarıyla çağırmak PHP 8.4'te kullanımdan kalktı.", "Calling IntlCalendar::set() with date parts is deprecated since PHP 8.4."),
    )),
    arg(&["intlgregoriancalendar::__construct"], &[Count(3, 6)], rule(
        "deprecated.intlgregcal_create_instance",
        Text::new("Tarih parçalarıyla IntlGregorianCalendar", "IntlGregorianCalendar from date parts"),
        D, (8, 4), code("IntlGregorianCalendar::createFromDate()"),
        Text::new("new IntlGregorianCalendar()'ı tarih parçalarıyla çağırmak PHP 8.4'te kullanımdan kalktı.", "Calling new IntlGregorianCalendar() with date parts is deprecated since PHP 8.4."),
    )),
    arg(&["mysqli::store_result"], &[Count(1, 1)], rule(
        "deprecated.mysqli_store_result_mode",
        Text::new("store_result() modu", "store_result() mode"),
        D, (8, 4), tx("argümanı silin", "remove the argument"),
        Text::new("mysqli::store_result()'a mod vermek PHP 8.4'te kullanımdan kalktı.", "Passing a mode to mysqli::store_result() is deprecated since PHP 8.4."),
    )),
    arg(&["soapserver::addfunction"], &[Int(0)], rule(
        "deprecated.soap_functions_all",
        Text::new("addFunction()'a sayı", "Integer passed to addFunction()"),
        D, (8, 4), tx("fonksiyon adlarını verin", "pass function names"),
        Text::new("SoapServer::addFunction()'a sayı vermek PHP 8.4'te kullanımdan kalktı.", "Passing an integer to SoapServer::addFunction() is deprecated since PHP 8.4."),
    )),
    arg(&["dateperiod::__construct"], &[Str(0)], rule(
        "deprecated.date_period_iso_string",
        Text::new("ISO metniyle DatePeriod", "DatePeriod from an ISO string"),
        D, (8, 4), code("DatePeriod::createFromISO8601String()"),
        Text::new("new DatePeriod()'u ISO 8601 metniyle çağırmak PHP 8.4'te kullanımdan kalktı.", "Calling new DatePeriod() with an ISO 8601 string is deprecated since PHP 8.4."),
    )),
    arg(&["arrayobject::__construct", "arrayiterator::__construct"], &[New(0)], rule(
        "deprecated.array_object_object",
        Text::new("Nesneyle ArrayObject", "ArrayObject wrapping an object"),
        D, (8, 5), tx("bir dizi verin", "pass an array"),
        Text::new("ArrayObject ve ArrayIterator'a nesne vermek PHP 8.5'te kullanımdan kalktı.", "Passing an object to ArrayObject or ArrayIterator is deprecated in PHP 8.5."),
    )),
    arg(&["soapclient::__construct"], &[Key(1, "ssl_method")], rule(
        "deprecated.soap_ssl_method",
        Text::new("SoapClient ssl_method seçeneği", "SoapClient ssl_method option"),
        D, (8, 1), tx("stream context'in crypto_method seçeneği", "the crypto_method option of a stream context"),
        Text::new("SoapClient'ın ssl_method seçeneği PHP 8.1'de kullanımdan kalktı.", "The ssl_method option of SoapClient is deprecated since PHP 8.1."),
    )),
    arg(&["pdo::__construct"], &[StrStarts(0, &["oci:"])], rule(
        "removed.pdo_oci",
        Text::new("PDO_OCI bağlantısı", "PDO_OCI connection"),
        R, (8, 4), tx("PECL pdo_oci", "PECL pdo_oci"),
        Text::new("PDO_OCI sürücüsü PHP 8.4'te çekirdekten çıkarıldı; ancak PECL'den ayrıca kurulursa çalışır.", "The PDO_OCI driver was unbundled in PHP 8.4; it only works if you install it from PECL."),
    )),
    arg(&["pdo::__construct"], &[StrStarts(0, &["uri:"])], rule(
        "deprecated.pdo_uri_dsn",
        Text::new("uri: DSN", "uri: DSN"),
        D, (8, 5), tx("DSN'i doğrudan verin", "pass the DSN directly"),
        Text::new("DSN'i uri: ile bir dosyadan okumak PHP 8.5'te kullanımdan kalktı.", "Reading the DSN from a file with uri: is deprecated in PHP 8.5."),
    )),
    arg(&["static ffi::cast", "static ffi::new", "static ffi::type"], &[Count(0, 9)], rule(
        "deprecated.ffi_static_call",
        Text::new("FFI'ın statik çağrısı", "Static FFI call"),
        D, (8, 3), code("FFI::cdef()->cast()"),
        Text::new("FFI::cast(), FFI::new() ve FFI::type()'ı statik çağırmak PHP 8.3'te kullanımdan kalktı.", "Calling FFI::cast(), FFI::new() and FFI::type() statically is deprecated since PHP 8.3."),
    )),
    arg(&["mb_ereg", "mb_eregi", "mb_ereg_replace", "mb_eregi_replace", "mb_ereg_match", "mb_split", "mb_ereg_search_init"], &[StrContains(0, "(?<")], rule(
        "behavior.mb_ereg_named_capture",
        Text::new("Adlandırılmış grup içeren mb_ereg deseni", "mb_ereg pattern with a named group"),
        B, (7, 3), None,
        Text::new(
            "PHP 7.3'ten beri mb_ereg_*() adlandırılmış grupları destekler; bu gruplar eşleşmelere eklenir ve değiştirme metni yeni sözdizimini yorumlar.",
            "Since PHP 7.3 mb_ereg_*() supports named groups; they're added to the matches and the replacement interprets new syntax.",
        ),
    )),
    arg(&["file_put_contents"], &[StrStarts(0, &["data:"])], rule(
        "removed.data_stream_write",
        Text::new("data: akışına yazma", "Writing to a data: stream"),
        R, (8, 0), None,
        Text::new("data: akışları PHP 8.0'dan beri yazılamaz.", "data: streams can't be written to since PHP 8.0."),
    )),
    arg(&["fopen"], &[StrStarts(0, &["data:"]), StrNot(1, &["r", "rb", "rt"])], rule(
        "removed.data_stream_write",
        Text::new("data: akışına yazma", "Writing to a data: stream"),
        R, (8, 0), None,
        Text::new("data: akışları PHP 8.0'dan beri yazılamaz.", "data: streams can't be written to since PHP 8.0."),
    )),
    arg(&["range"], &[NumIs(2, 0.0)], rule(
        "removed.range_invalid",
        Text::new("Geçersiz range() argümanı", "Invalid range() argument"),
        R, (8, 3), None,
        Text::new("range() PHP 8.3'ten beri 0 adımda, boş metinde ve dizi sınırda hata fırlatır.", "Since PHP 8.3 range() throws for a step of 0, an empty string or an array as a boundary."),
    )),
    arg(&["range"], &[StrIs(0, &[""])], rule(
        "removed.range_invalid",
        Text::new("Geçersiz range() argümanı", "Invalid range() argument"),
        R, (8, 3), None,
        Text::new("range() PHP 8.3'ten beri 0 adımda, boş metinde ve dizi sınırda hata fırlatır.", "Since PHP 8.3 range() throws for a step of 0, an empty string or an array as a boundary."),
    )),
    arg(&["range"], &[StrIs(1, &[""])], rule(
        "removed.range_invalid",
        Text::new("Geçersiz range() argümanı", "Invalid range() argument"),
        R, (8, 3), None,
        Text::new("range() PHP 8.3'ten beri 0 adımda, boş metinde ve dizi sınırda hata fırlatır.", "Since PHP 8.3 range() throws for a step of 0, an empty string or an array as a boundary."),
    )),
    arg(&["range"], &[ArrayLit(0)], rule(
        "removed.range_invalid",
        Text::new("Geçersiz range() argümanı", "Invalid range() argument"),
        R, (8, 3), None,
        Text::new("range() PHP 8.3'ten beri 0 adımda, boş metinde ve dizi sınırda hata fırlatır.", "Since PHP 8.3 range() throws for a step of 0, an empty string or an array as a boundary."),
    )),
    arg(&["str_getcsv"], &[StrLenNot(1, 1)], rule(
        "removed.csv_separator_length",
        Text::new("Tek karakter olmayan CSV ayırıcısı", "CSV separator that isn't one character"),
        R, (8, 4), None,
        Text::new("CSV fonksiyonları PHP 8.4'ten beri boş ya da birden uzun ayırıcıda ValueError fırlatır.", "CSV functions throw a ValueError for an empty or longer separator since PHP 8.4."),
    )),
    arg(&["fgetcsv", "fputcsv"], &[StrLenNot(2, 1)], rule(
        "removed.csv_separator_length",
        Text::new("Tek karakter olmayan CSV ayırıcısı", "CSV separator that isn't one character"),
        R, (8, 4), None,
        Text::new("CSV fonksiyonları PHP 8.4'ten beri boş ya da birden uzun ayırıcıda ValueError fırlatır.", "CSV functions throw a ValueError for an empty or longer separator since PHP 8.4."),
    )),
];
