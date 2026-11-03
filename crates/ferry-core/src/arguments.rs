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
    Key(usize, &'static str),
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
        [t] if matches!(t.kind, TokenKind::Str { interpolates: false, backtick: false }) => Lit::Str(string_literal(&t.text)),
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
            | Check::Key(n, _) => Some(n),
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
            Check::Key(_, key) => {
                arg.windows(2).any(|w| string_literal(&w[0].text).is_some_and(|s| s.eq_ignore_ascii_case(key)) && w[1].is_punct("=>"))
            }
        }
    }
}

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

pub const ARGUMENTS: &[ArgRule] = &[
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
];
