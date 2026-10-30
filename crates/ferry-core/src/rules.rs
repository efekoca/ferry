use crate::i18n::Text;
use crate::version::PhpVersion;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Breaking,
    Deprecated,
    Behavior,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FixKind {
    Auto,
    Likely,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Removed,
    Deprecated,
}

#[derive(Debug, Clone, Copy)]
pub struct SymbolRule {
    pub id: &'static str,
    pub name: &'static str,
    pub change: Change,
    pub since: (u32, u32),
    pub replacement: Option<Text>,
    pub fix: FixKind,
    pub note: Text,
}

impl SymbolRule {
    pub fn since(&self) -> PhpVersion {
        PhpVersion::new(self.since.0, self.since.1)
    }

    pub fn severity(&self) -> Severity {
        match self.change {
            Change::Removed => Severity::Breaking,
            Change::Deprecated => Severity::Deprecated,
        }
    }

    pub fn matches(&self, name: &str) -> bool {
        match self.name.strip_suffix('*') {
            Some(prefix) => name.starts_with(prefix),
            None => name == self.name,
        }
    }
}

use Change::{Deprecated as D, Removed as R};
use FixKind::{Auto as A, Likely as L, Manual as M};

const fn tx(tr: &'static str, en: &'static str) -> Option<Text> {
    Some(Text::new(tr, en))
}

const fn code(both: &'static str) -> Option<Text> {
    Some(Text::same(both))
}

const REMOVED_ALIAS: Text = Text::new("Bu takma ad kaldırıldı.", "This alias was removed.");
const REMOVED_FN: Text = Text::new("Bu fonksiyon kaldırıldı.", "This function was removed.");
const DEPRECATED_FN: Text = Text::new("Kullanımdan kalktı.", "Deprecated.");
const POSIX_REGEX: Text = Text::new(
    "POSIX regex fonksiyonları kaldırıldı. Desen, sınırlayıcılarla PCRE sözdizimine çevrilmelidir.",
    "POSIX regex functions were removed. The pattern has to be converted to PCRE syntax with delimiters.",
);
const MAGIC_QUOTES_SET: Text = Text::new(
    "Magic quotes artık yok; bu çağrı silinebilir.",
    "Magic quotes no longer exist; this call can be removed.",
);
const MAGIC_QUOTES_GET: Text = Text::new(
    "Magic quotes PHP 5.4'ten beri yok; çağrının yerine false yazılabilir.",
    "Magic quotes have been gone since PHP 5.4; replace the call with false.",
);
const NOOP_SINCE_80: Text = Text::new(
    "PHP 8.0'dan beri hiçbir şey yapmıyor, 8.5'te kullanımdan kalktı.",
    "Has done nothing since PHP 8.0 and is deprecated in 8.5.",
);
const REMOVE_CALL: Option<Text> = tx("çağrıyı silin", "remove the call");

macro_rules! rule {
    ($id:literal, $name:literal, $change:expr, $since:expr, $repl:expr, $fix:expr, $note:expr) => {
        SymbolRule {
            id: $id,
            name: $name,
            change: $change,
            since: $since,
            replacement: $repl,
            fix: $fix,
            note: $note,
        }
    };
}

pub const FUNCTIONS: &[SymbolRule] = &[
    rule!("removed.mysql", "mysql_*", R, (7, 0), tx("mysqli_* ya da PDO", "mysqli_* or PDO"), M, Text::new(
        "ext/mysql PHP 7.0'da kaldırıldı. Veritabanı katmanının mysqli ya da PDO ile yeniden yazılması gerekiyor.",
        "ext/mysql was removed in PHP 7.0. The database layer needs to be rewritten with mysqli or PDO.")),
    rule!("removed.ereg", "ereg", R, (7, 0), code("preg_match"), L, POSIX_REGEX),
    rule!("removed.eregi", "eregi", R, (7, 0), tx("preg_match (i bayrağıyla)", "preg_match with the i flag"), L, POSIX_REGEX),
    rule!("removed.ereg_replace", "ereg_replace", R, (7, 0), code("preg_replace"), L, POSIX_REGEX),
    rule!("removed.eregi_replace", "eregi_replace", R, (7, 0), tx("preg_replace (i bayrağıyla)", "preg_replace with the i flag"), L, POSIX_REGEX),
    rule!("removed.split", "split", R, (7, 0), code("explode / preg_split"), L, Text::new(
        "split() kaldırıldı. Desen düz metinse explode(), değilse preg_split() kullanın.",
        "split() was removed. Use explode() for plain strings, preg_split() otherwise.")),
    rule!("removed.spliti", "spliti", R, (7, 0), tx("preg_split (i bayrağıyla)", "preg_split with the i flag"), L, REMOVED_FN),
    rule!("removed.sql_regcase", "sql_regcase", R, (7, 0), None, M, Text::new(
        "sql_regcase() kaldırıldı ve doğrudan bir karşılığı yok.",
        "sql_regcase() was removed and has no direct replacement.")),
    rule!("removed.call_user_method", "call_user_method", R, (7, 0), code("call_user_func([$obj, 'method'])"), A, REMOVED_FN),
    rule!("removed.call_user_method_array", "call_user_method_array", R, (7, 0), code("call_user_func_array([$obj, 'method'], $args)"), A, REMOVED_FN),
    rule!("removed.set_magic_quotes_runtime", "set_magic_quotes_runtime", R, (7, 0), REMOVE_CALL, M, MAGIC_QUOTES_SET),
    rule!("removed.magic_quotes_runtime", "magic_quotes_runtime", R, (7, 0), REMOVE_CALL, M, MAGIC_QUOTES_SET),
    rule!("removed.mcrypt", "mcrypt_*", R, (7, 2), tx("openssl_* ya da sodium_*", "openssl_* or sodium_*"), M, Text::new(
        "ext/mcrypt PHP 7.2'de çekirdekten çıkarıldı. Şifreleme openssl ya da sodium ile yeniden yazılmalı; eski şifreli verinin hâlâ çözülebildiğini doğrulayın.",
        "ext/mcrypt was dropped from core in PHP 7.2. Rewrite the encryption with openssl or sodium and make sure existing encrypted data can still be decrypted.")),
    rule!("removed.create_function", "create_function", R, (8, 0), tx("anonim fonksiyon (closure)", "an anonymous function (closure)"), L, Text::new(
        "create_function() kaldırıldı. Kod dizesi bir closure'a çevrilmeli; dize çalışma anında üretiliyorsa elle inceleyin.",
        "create_function() was removed. The code string has to become a closure; review it by hand if the string is built at runtime.")),
    rule!("removed.each", "each", R, (8, 0), tx("foreach ya da key()/current()/next()", "foreach or key()/current()/next()"), L, Text::new(
        "each() kaldırıldı. while (list() = each()) döngüleri foreach'e çevrilir.",
        "each() was removed. while (list() = each()) loops become foreach.")),
    rule!("removed.read_exif_data", "read_exif_data", R, (8, 0), code("exif_read_data"), A, REMOVED_ALIAS),
    rule!("removed.gmp_random", "gmp_random", R, (8, 0), tx("gmp_random_bits ya da gmp_random_range", "gmp_random_bits or gmp_random_range"), L, REMOVED_FN),
    rule!("removed.image2wbmp", "image2wbmp", R, (8, 0), code("imagewbmp"), A, REMOVED_FN),
    rule!("removed.png2wbmp", "png2wbmp", R, (8, 0), None, M, REMOVED_FN),
    rule!("removed.jpeg2wbmp", "jpeg2wbmp", R, (8, 0), None, M, REMOVED_FN),
    rule!("removed.fgetss", "fgetss", R, (8, 0), code("strip_tags(fgets(...))"), M, REMOVED_FN),
    rule!("removed.money_format", "money_format", R, (8, 0), code("NumberFormatter"), M, Text::new(
        "money_format() kaldırıldı. Yerine intl eklentisindeki NumberFormatter kullanılmalı.",
        "money_format() was removed. Use NumberFormatter from the intl extension instead.")),
    rule!("removed.restore_include_path", "restore_include_path", R, (8, 0), code("ini_restore('include_path')"), A, REMOVED_FN),
    rule!("removed.ezmlm_hash", "ezmlm_hash", R, (8, 0), None, M, REMOVED_FN),
    rule!("removed.hebrevc", "hebrevc", R, (8, 0), code("nl2br(hebrev(...))"), A, REMOVED_FN),
    rule!("removed.convert_cyr_string", "convert_cyr_string", R, (8, 0), tx("mb_convert_encoding ya da iconv", "mb_convert_encoding or iconv"), M, REMOVED_FN),
    rule!("removed.get_magic_quotes_gpc", "get_magic_quotes_gpc", R, (8, 0), code("false"), M, MAGIC_QUOTES_GET),
    rule!("removed.get_magic_quotes_runtime", "get_magic_quotes_runtime", R, (8, 0), code("false"), M, MAGIC_QUOTES_GET),
    rule!("removed.is_real", "is_real", R, (8, 0), code("is_float"), A, REMOVED_ALIAS),
    rule!("removed.ldap_sort", "ldap_sort", R, (8, 0), None, M, REMOVED_FN),
    rule!("removed.ldap_control_paged_result", "ldap_control_paged_result", R, (8, 0), tx("ldap_search'teki $controls", "$controls in ldap_search"), M, REMOVED_FN),
    rule!("removed.mbereg", "mbereg*", R, (8, 0), code("mb_ereg*"), A, Text::new(
        "mbereg* takma adları kaldırıldı.",
        "The mbereg* aliases were removed.")),
    rule!("removed.mbsplit", "mbsplit", R, (8, 0), code("mb_split"), A, REMOVED_ALIAS),
    rule!("deprecated.strftime", "strftime", D, (8, 1), tx("IntlDateFormatter ya da date()", "IntlDateFormatter or date()"), M, Text::new(
        "strftime() kullanımdan kalktı ve PHP 9'da kaldırılacak. Biçim karakterleri date() ile birebir eşleşmez.",
        "strftime() is deprecated and will be removed in PHP 9. Its format characters don't map one-to-one to date().")),
    rule!("deprecated.gmstrftime", "gmstrftime", D, (8, 1), tx("IntlDateFormatter ya da gmdate()", "IntlDateFormatter or gmdate()"), M, DEPRECATED_FN),
    rule!("deprecated.strptime", "strptime", D, (8, 1), tx("date_parse_from_format ya da IntlDateFormatter::parse", "date_parse_from_format or IntlDateFormatter::parse"), M, DEPRECATED_FN),
    rule!("deprecated.date_sunrise", "date_sunrise", D, (8, 1), code("date_sun_info"), M, DEPRECATED_FN),
    rule!("deprecated.date_sunset", "date_sunset", D, (8, 1), code("date_sun_info"), M, DEPRECATED_FN),
    rule!("deprecated.mhash", "mhash*", D, (8, 1), code("hash_*"), M, Text::new(
        "mhash fonksiyonları kullanımdan kalktı.",
        "The mhash functions are deprecated.")),
    rule!("deprecated.odbc_result_all", "odbc_result_all", D, (8, 1), None, M, DEPRECATED_FN),
    rule!("deprecated.utf8_encode", "utf8_encode", D, (8, 2), code("mb_convert_encoding($s, 'UTF-8', 'ISO-8859-1')"), L, Text::new(
        "utf8_encode() kullanımdan kalktı. Kaynak metnin gerçekten ISO-8859-1 olduğundan emin olun.",
        "utf8_encode() is deprecated. Make sure the source text really is ISO-8859-1.")),
    rule!("deprecated.utf8_decode", "utf8_decode", D, (8, 2), code("mb_convert_encoding($s, 'ISO-8859-1', 'UTF-8')"), L, DEPRECATED_FN),
    rule!("deprecated.lcg_value", "lcg_value", D, (8, 4), code("Random\\Randomizer::getFloat()"), M, DEPRECATED_FN),
    rule!("deprecated.mysqli_ping", "mysqli_ping", D, (8, 4), None, M, Text::new(
        "mysqli_ping() kullanımdan kalktı; kopan bağlantı artık kendiliğinden yeniden kurulmuyor.",
        "mysqli_ping() is deprecated; dropped connections are no longer reconnected automatically.")),
    rule!("deprecated.mysqli_kill", "mysqli_kill", D, (8, 4), tx("KILL CONNECTION sorgusu", "a KILL CONNECTION query"), M, DEPRECATED_FN),
    rule!("deprecated.mysqli_refresh", "mysqli_refresh", D, (8, 4), tx("FLUSH sorguları", "FLUSH queries"), M, DEPRECATED_FN),
    rule!("deprecated.xml_set_object", "xml_set_object", D, (8, 4), tx("xml_set_*_handler'a doğrudan callable verin", "pass a callable to xml_set_*_handler"), M, DEPRECATED_FN),
    rule!("deprecated.curl_close", "curl_close", D, (8, 5), REMOVE_CALL, A, NOOP_SINCE_80),
    rule!("deprecated.curl_share_close", "curl_share_close", D, (8, 5), REMOVE_CALL, A, NOOP_SINCE_80),
    rule!("deprecated.finfo_close", "finfo_close", D, (8, 5), REMOVE_CALL, A, Text::new(
        "PHP 8.1'den beri hiçbir şey yapmıyor, 8.5'te kullanımdan kalktı.",
        "Has done nothing since PHP 8.1 and is deprecated in 8.5.")),
    rule!("deprecated.imagedestroy", "imagedestroy", D, (8, 5), REMOVE_CALL, A, NOOP_SINCE_80),
    rule!("deprecated.xml_parser_free", "xml_parser_free", D, (8, 5), REMOVE_CALL, A, NOOP_SINCE_80),
];

pub const CONSTANTS: &[SymbolRule] = &[
    rule!("deprecated.filter_sanitize_string", "filter_sanitize_string", D, (8, 1), code("htmlspecialchars()"), M, Text::new(
        "FILTER_SANITIZE_STRING kullanımdan kalktı. İhtiyacınıza göre htmlspecialchars() ya da strip_tags() kullanın.",
        "FILTER_SANITIZE_STRING is deprecated. Use htmlspecialchars() or strip_tags(), depending on what you need.")),
    rule!("deprecated.filter_sanitize_stripped", "filter_sanitize_stripped", D, (8, 1), code("htmlspecialchars()"), M, Text::new(
        "FILTER_SANITIZE_STRIPPED kullanımdan kalktı.",
        "FILTER_SANITIZE_STRIPPED is deprecated.")),
    rule!("deprecated.e_strict", "e_strict", D, (8, 4), tx("E_STRICT'i kaldırın", "remove E_STRICT"), M, Text::new(
        "E_STRICT kullanımdan kalktı; bu hata seviyesi artık hiç tetiklenmiyor.",
        "E_STRICT is deprecated; this error level is never raised anymore.")),
    rule!("deprecated.e_user_error", "e_user_error", D, (8, 4), tx("exception fırlatın ya da exit() kullanın", "throw an exception or call exit()"), M, Text::new(
        "trigger_error() ile E_USER_ERROR kullanmak 8.4'te kullanımdan kalktı.",
        "Passing E_USER_ERROR to trigger_error() is deprecated in 8.4.")),
];

#[derive(Debug, Clone, Copy)]
pub struct PatternRule {
    pub id: &'static str,
    pub title: Text,
    pub severity: Severity,
    pub since: (u32, u32),
    pub replacement: Option<Text>,
    pub fix: FixKind,
    pub note: Text,
}

impl PatternRule {
    pub fn since(&self) -> PhpVersion {
        PhpVersion::new(self.since.0, self.since.1)
    }
}

pub mod patterns {
    use super::*;

    pub const AUTOLOAD: PatternRule = PatternRule {
        id: "removed.__autoload",
        title: Text::new("__autoload() tanımı", "__autoload() definition"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: code("spl_autoload_register()"),
        fix: FixKind::Manual,
        note: Text::new(
            "__autoload() PHP 8.0'da kaldırıldı; artık hiç çağrılmıyor.",
            "__autoload() was removed in PHP 8.0 and is never called anymore.",
        ),
    };
    pub const CURLY_OFFSET: PatternRule = PatternRule {
        id: "removed.curly_offset",
        title: Text::new("Süslü parantezle dizi erişimi", "Curly brace offset access"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: code("$x[0]"),
        fix: FixKind::Auto,
        note: Text::new(
            "$x{0} yazımı PHP 8.0'da kaldırıldı ve ayrıştırma hatası verir.",
            "The $x{0} syntax was removed in PHP 8.0 and is now a parse error.",
        ),
    };
    pub const HASH_BRACKET_COMMENT: PatternRule = PatternRule {
        id: "removed.hash_bracket_comment",
        title: Text::new("#[ ile başlayan yorum", "Comment starting with #["),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: code("// ..."),
        fix: FixKind::Manual,
        note: Text::new(
            "PHP 8.0'dan itibaren #[ bir attribute başlatır; bu satır artık yorum sayılmaz ve ayrıştırma hatası verir. Yorumu // ile yazın.",
            "Since PHP 8.0, #[ starts an attribute, so this line is no longer a comment and becomes a parse error. Write the comment with // instead.",
        ),
    };
    pub const REAL_CAST: PatternRule = PatternRule {
        id: "removed.real_cast",
        title: Text::new("(real) dönüşümü", "(real) cast"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: code("(float)"),
        fix: FixKind::Auto,
        note: Text::new("(real) dönüşümü PHP 8.0'da kaldırıldı.", "The (real) cast was removed in PHP 8.0."),
    };
    pub const UNSET_CAST: PatternRule = PatternRule {
        id: "removed.unset_cast",
        title: Text::new("(unset) dönüşümü", "(unset) cast"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: code("null"),
        fix: FixKind::Auto,
        note: Text::new("(unset) dönüşümü PHP 8.0'da kaldırıldı.", "The (unset) cast was removed in PHP 8.0."),
    };
    pub const DOLLAR_BRACE_INTERPOLATION: PatternRule = PatternRule {
        id: "deprecated.dollar_brace_interpolation",
        title: Text::new("\"${var}\" yazımı", "\"${var}\" interpolation"),
        severity: Severity::Deprecated,
        since: (8, 2),
        replacement: code("\"{$var}\""),
        fix: FixKind::Auto,
        note: Text::new(
            "Metin içinde ${...} yazımı PHP 8.2'de kullanımdan kalktı.",
            "${...} interpolation in strings is deprecated in PHP 8.2.",
        ),
    };
    pub const GET_CLASS_NO_ARGS: PatternRule = PatternRule {
        id: "deprecated.get_class_no_args",
        title: Text::new("Parametresiz get_class()", "get_class() without arguments"),
        severity: Severity::Deprecated,
        since: (8, 3),
        replacement: code("self::class"),
        fix: FixKind::Auto,
        note: Text::new(
            "Parametresiz get_class() ve get_parent_class() çağrıları 8.3'te kullanımdan kalktı.",
            "Calling get_class() or get_parent_class() without arguments is deprecated in 8.3.",
        ),
    };
    pub const IMPLICIT_NULLABLE: PatternRule = PatternRule {
        id: "deprecated.implicit_nullable",
        title: Text::new("Örtük nullable parametre", "Implicitly nullable parameter"),
        severity: Severity::Deprecated,
        since: (8, 4),
        replacement: tx("?Tip $param = null", "?Type $param = null"),
        fix: FixKind::Auto,
        note: Text::new(
            "Varsayılan değeri null olan tipli parametreler açıkça nullable (?Tip) yazılmalı.",
            "Typed parameters that default to null must be declared nullable (?Type).",
        ),
    };
    pub const NON_CANONICAL_CAST: PatternRule = PatternRule {
        id: "deprecated.non_canonical_cast",
        title: Text::new("Standart dışı tip dönüşümü", "Non-canonical cast"),
        severity: Severity::Deprecated,
        since: (8, 5),
        replacement: code("(bool) / (int) / (float) / (string)"),
        fix: FixKind::Auto,
        note: Text::new(
            "(boolean), (integer), (double) ve (binary) dönüşümleri 8.5'te kullanımdan kalktı.",
            "The (boolean), (integer), (double) and (binary) casts are deprecated in 8.5.",
        ),
    };
    pub const BACKTICK: PatternRule = PatternRule {
        id: "deprecated.backtick",
        title: Text::new("Ters tırnakla komut çalıştırma", "Backtick shell execution"),
        severity: Severity::Deprecated,
        since: (8, 5),
        replacement: code("shell_exec()"),
        fix: FixKind::Auto,
        note: Text::new(
            "Ters tırnak operatörü 8.5'te kullanımdan kalktı.",
            "The backtick operator is deprecated in 8.5.",
        ),
    };
    pub const EVAL: PatternRule = PatternRule {
        id: "review.eval",
        title: Text::new("eval() kullanımı", "eval() usage"),
        severity: Severity::Behavior,
        since: (7, 0),
        replacement: None,
        fix: FixKind::Manual,
        note: Text::new(
            "eval() içindeki kod statik olarak incelenemez; çalıştırdığı kodun yeni sürümde de çalıştığını elle doğrulayın.",
            "Code inside eval() can't be analyzed statically; check by hand that what it runs still works on the new version.",
        ),
    };
    pub const VARIABLE_VARIABLE: PatternRule = PatternRule {
        id: "review.variable_variable",
        title: Text::new("Değişken değişken ($$)", "Variable variable ($$)"),
        severity: Severity::Behavior,
        since: (7, 0),
        replacement: None,
        fix: FixKind::Manual,
        note: Text::new(
            "$$ kullanımı statik analizden kaçar ve PHP 7'den beri farklı bir öncelikle değerlendirilir.",
            "Variable variables escape static analysis and have been evaluated with different precedence since PHP 7.",
        ),
    };
    pub const EXTRACT: PatternRule = PatternRule {
        id: "review.extract",
        title: Text::new("extract() kullanımı", "extract() usage"),
        severity: Severity::Behavior,
        since: (7, 0),
        replacement: None,
        fix: FixKind::Manual,
        note: Text::new(
            "extract() hangi değişkenleri oluşturduğunu gizler; tip ve null davranışını elle kontrol edin.",
            "extract() hides which variables it creates; check types and nulls by hand.",
        ),
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_unique_and_names_lowercase() {
        let mut seen = HashSet::new();
        for r in FUNCTIONS.iter().chain(CONSTANTS) {
            assert!(seen.insert(r.id), "duplicate id {}", r.id);
            assert_eq!(r.name, r.name.to_lowercase(), "{} must be lowercase", r.id);
            assert!(!r.note.tr.is_empty() && !r.note.en.is_empty(), "{} needs both languages", r.id);
        }
    }

    #[test]
    fn prefix_matching() {
        let mysql = FUNCTIONS.iter().find(|r| r.id == "removed.mysql").unwrap();
        assert!(mysql.matches("mysql_connect"));
        assert!(!mysql.matches("mysqli_connect"));
        let each = FUNCTIONS.iter().find(|r| r.id == "removed.each").unwrap();
        assert!(each.matches("each"));
        assert!(!each.matches("eachx"));
    }
}
