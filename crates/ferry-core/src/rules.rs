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
        if self.id == "removed.mysql" {
            return MYSQL_FUNCTIONS.contains(&name);
        }
        if self.id == "deprecated.pg_alias" {
            return PG_ALIASES.iter().any(|(old, _)| *old == name);
        }
        match self.name.strip_suffix('*') {
            Some(prefix) => name.starts_with(prefix),
            None => name == self.name,
        }
    }
}

use Change::{Deprecated as D, Removed as R};

const MYSQL_FUNCTIONS: [&str; 65] = [
    "mysql_affected_rows",
    "mysql_client_encoding",
    "mysql_close",
    "mysql_connect",
    "mysql_create_db",
    "mysql_createdb",
    "mysql_data_seek",
    "mysql_db_name",
    "mysql_dbname",
    "mysql_db_query",
    "mysql_drop_db",
    "mysql_dropdb",
    "mysql_errno",
    "mysql_error",
    "mysql_escape_string",
    "mysql_fetch_array",
    "mysql_fetch_assoc",
    "mysql_fetch_field",
    "mysql_fetch_lengths",
    "mysql_fetch_object",
    "mysql_fetch_row",
    "mysql_field_flags",
    "mysql_fieldflags",
    "mysql_field_len",
    "mysql_fieldlen",
    "mysql_field_name",
    "mysql_fieldname",
    "mysql_field_seek",
    "mysql_field_table",
    "mysql_fieldtable",
    "mysql_field_type",
    "mysql_fieldtype",
    "mysql_free_result",
    "mysql_freeresult",
    "mysql_get_client_info",
    "mysql_get_host_info",
    "mysql_get_proto_info",
    "mysql_get_server_info",
    "mysql_info",
    "mysql_insert_id",
    "mysql_list_dbs",
    "mysql_listdbs",
    "mysql_list_fields",
    "mysql_listfields",
    "mysql_list_processes",
    "mysql_list_tables",
    "mysql_listtables",
    "mysql_num_fields",
    "mysql_numfields",
    "mysql_num_rows",
    "mysql_numrows",
    "mysql_pconnect",
    "mysql_ping",
    "mysql_query",
    "mysql_real_escape_string",
    "mysql_result",
    "mysql_select_db",
    "mysql_selectdb",
    "mysql_set_charset",
    "mysql_stat",
    "mysql_tablename",
    "mysql_table_name",
    "mysql_thread_id",
    "mysql_unbuffered_query",
    "mysql",
];
pub const PG_ALIASES: [(&str, &str); 24] = [
    ("pg_clientencoding", "pg_client_encoding"),
    ("pg_cmdtuples", "pg_affected_rows"),
    ("pg_errormessage", "pg_last_error"),
    ("pg_fieldisnull", "pg_field_is_null"),
    ("pg_fieldname", "pg_field_name"),
    ("pg_fieldnum", "pg_field_num"),
    ("pg_fieldprtlen", "pg_field_prtlen"),
    ("pg_fieldsize", "pg_field_size"),
    ("pg_fieldtype", "pg_field_type"),
    ("pg_freeresult", "pg_free_result"),
    ("pg_getlastoid", "pg_last_oid"),
    ("pg_loclose", "pg_lo_close"),
    ("pg_locreate", "pg_lo_create"),
    ("pg_loexport", "pg_lo_export"),
    ("pg_loimport", "pg_lo_import"),
    ("pg_loopen", "pg_lo_open"),
    ("pg_loread", "pg_lo_read"),
    ("pg_loreadall", "pg_lo_read_all"),
    ("pg_lounlink", "pg_lo_unlink"),
    ("pg_lowrite", "pg_lo_write"),
    ("pg_numfields", "pg_num_fields"),
    ("pg_numrows", "pg_num_rows"),
    ("pg_result", "pg_fetch_result"),
    ("pg_setclientencoding", "pg_set_client_encoding"),
];

pub const COMPILE_TIME: &[&str] = &[
    "removed.parent_without_parent",
    "removed.curly_offset",
    "removed.real_cast",
    "removed.unset_cast",
    "removed.nested_ternary",
    "removed.hash_bracket_comment",
    "removed.__autoload",
    "removed.reserved_name",
    "removed.assign_new_by_ref",
    "removed.unset_this",
    "removed.globals_write",
    "removed.reserved_alias",
    "removed.builtin_result_by_ref",
    "removed.break_outside_loop",
    "deprecated.dollar_brace_interpolation",
    "deprecated.implicit_nullable",
    "deprecated.optional_before_required",
    "deprecated.case_semicolon",
    "deprecated.non_canonical_cast",
    "deprecated.backtick",
];

pub const NULL_STRING_ARGS: &[(&str, &[usize])] = &[
    ("strlen", &[0]),
    ("trim", &[0]),
    ("ltrim", &[0]),
    ("rtrim", &[0]),
    ("strtolower", &[0]),
    ("strtoupper", &[0]),
    ("ucfirst", &[0]),
    ("lcfirst", &[0]),
    ("ucwords", &[0]),
    ("str_replace", &[2]),
    ("str_ireplace", &[2]),
    ("strpos", &[0, 1]),
    ("stripos", &[0, 1]),
    ("strrpos", &[0, 1]),
    ("str_contains", &[0, 1]),
    ("str_starts_with", &[0, 1]),
    ("str_ends_with", &[0, 1]),
    ("strstr", &[0, 1]),
    ("substr", &[0]),
    ("substr_count", &[0, 1]),
    ("str_pad", &[0]),
    ("str_repeat", &[0]),
    ("str_split", &[0]),
    ("strrev", &[0]),
    ("strcmp", &[0, 1]),
    ("strcasecmp", &[0, 1]),
    ("strncmp", &[0, 1]),
    ("htmlspecialchars", &[0]),
    ("htmlentities", &[0]),
    ("html_entity_decode", &[0]),
    ("htmlspecialchars_decode", &[0]),
    ("strip_tags", &[0]),
    ("addslashes", &[0]),
    ("stripslashes", &[0]),
    ("nl2br", &[0]),
    ("explode", &[0, 1]),
    ("sprintf", &[0]),
    ("urlencode", &[0]),
    ("rawurlencode", &[0]),
    ("urldecode", &[0]),
    ("rawurldecode", &[0]),
    ("base64_encode", &[0]),
    ("base64_decode", &[0]),
    ("md5", &[0]),
    ("sha1", &[0]),
    ("crc32", &[0]),
    ("json_decode", &[0]),
    ("preg_match", &[0, 1]),
    ("preg_split", &[0, 1]),
    ("preg_quote", &[0]),
    ("mb_strlen", &[0]),
    ("mb_substr", &[0]),
    ("mb_strtolower", &[0]),
    ("mb_strtoupper", &[0]),
    ("mb_strpos", &[0, 1]),
    ("number_format", &[0]),
    ("strtotime", &[0]),
];

pub const DEPRECATED_INI: [(&str, u32, u32); 18] = [
    ("allow_url_include", 7, 4),
    ("mbstring.func_overload", 8, 0),
    ("track_errors", 8, 0),
    ("filter.default", 8, 1),
    ("filter.default_flags", 8, 1),
    ("auto_detect_line_endings", 8, 1),
    ("assert.active", 8, 3),
    ("assert.bail", 8, 3),
    ("assert.callback", 8, 3),
    ("assert.exception", 8, 3),
    ("assert.warning", 8, 3),
    ("session.sid_length", 8, 4),
    ("session.sid_bits_per_character", 8, 4),
    ("session.use_trans_sid", 8, 4),
    ("session.trans_sid_hosts", 8, 4),
    ("session.trans_sid_tags", 8, 4),
    ("session.referer_check", 8, 4),
    ("report_memleaks", 8, 5),
];

pub const REMOVED_INI: [(&str, u32, u32); 11] = [
    ("allow_call_time_pass_reference", 7, 0),
    ("asp_tags", 7, 0),
    ("define_syslog_variables", 7, 0),
    ("magic_quotes_gpc", 7, 0),
    ("magic_quotes_runtime", 7, 0),
    ("magic_quotes_sybase", 7, 0),
    ("register_globals", 7, 0),
    ("register_long_arrays", 7, 0),
    ("safe_mode", 7, 0),
    ("zend.ze1_compatibility_mode", 7, 0),
    ("track_errors", 8, 0),
];

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
const MAGIC_QUOTES_SET: Text =
    Text::new("Magic quotes artık yok; bu çağrı silinebilir.", "Magic quotes no longer exist; this call can be removed.");
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
        SymbolRule { id: $id, name: $name, change: $change, since: $since, replacement: $repl, fix: $fix, note: $note }
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
    rule!("deprecated.openssl_x509_free", "openssl_x509_free", D, (8, 0), REMOVE_CALL, M, NOOP_SINCE_80),
    rule!("deprecated.openssl_pkey_free", "openssl_pkey_free", D, (8, 0), REMOVE_CALL, M, NOOP_SINCE_80),
    rule!("deprecated.openssl_free_key", "openssl_free_key", D, (8, 0), REMOVE_CALL, M, NOOP_SINCE_80),
    rule!("deprecated.shmop_close", "shmop_close", D, (8, 0), REMOVE_CALL, M, NOOP_SINCE_80),
    rule!("deprecated.intlcal_set", "intlcal_set", D, (8, 4), tx("IntlCalendar::set(), setDate() ya da setDateTime()", "IntlCalendar::set(), setDate() or setDateTime()"), M, Text::new(
        "intlcal_set() 8.4'te kullanımdan kalktı.",
        "intlcal_set() is deprecated in 8.4.")),
    rule!("deprecated.intlgregcal_create_instance", "intlgregcal_create_instance", D, (8, 4), tx("new IntlGregorianCalendar(), createFromDate() ya da createFromDateTime()", "new IntlGregorianCalendar(), createFromDate() or createFromDateTime()"), M, Text::new(
        "intlgregcal_create_instance() 8.4'te kullanımdan kalktı.",
        "intlgregcal_create_instance() is deprecated in 8.4.")),
    rule!("removed.mbregex_encoding", "mbregex_encoding", R, (8, 0), code("mb_regex_encoding"), A, REMOVED_ALIAS),
    rule!("deprecated.pg_alias", "pg_*", D, (8, 0), tx("pg_ fonksiyonunun yeni adı", "the new pg_ function name"), A, Text::new(
        "Eski pgsql takma adları 8.0'da kullanımdan kalktı; alt çizgili yeni adlarını kullanın.",
        "The old pgsql aliases are deprecated in 8.0; use the new underscored names.")),
    rule!("deprecated.zip_procedural", "zip_*", D, (8, 0), code("ZipArchive"), M, Text::new(
        "Prosedürel zip_* API'si 8.0'da kullanımdan kalktı; ZipArchive sınıfını kullanın.",
        "The procedural zip_* API is deprecated in 8.0; use the ZipArchive class.")),
    rule!("removed.gzgetss", "gzgetss", R, (8, 0), code("strip_tags(gzgets(...))"), M, REMOVED_FN),
    rule!("removed.ldap_control_paged_result_response", "ldap_control_paged_result_response", R, (8, 0), tx("ldap_search()'e kontrol dizisi verin", "pass controls to ldap_search()"), M, REMOVED_FN),
    rule!("removed.xmlrpc", "xmlrpc_*", R, (8, 0), tx("PECL xmlrpc ya da bir XML-RPC kütüphanesi", "PECL xmlrpc or an XML-RPC library"), M, Text::new(
        "xmlrpc eklentisi PHP 8.0'da çekirdekten çıkarıldı.",
        "The xmlrpc extension was unbundled in PHP 8.0.")),
    rule!("removed.wddx", "wddx_*", R, (7, 4), tx("JSON ya da serialize()", "JSON or serialize()"), M, Text::new(
        "wddx eklentisi PHP 7.4'te kaldırıldı.",
        "The wddx extension was removed in PHP 7.4.")),
    rule!("removed.recode", "recode_*", R, (7, 4), code("iconv / mb_convert_encoding"), M, Text::new(
        "recode eklentisi PHP 7.4'te kaldırıldı.",
        "The recode extension was removed in PHP 7.4.")),
    rule!("removed.interbase", "ibase_*", R, (7, 4), tx("PECL interbase ya da PDO_Firebird", "PECL interbase or PDO_Firebird"), M, Text::new(
        "interbase eklentisi PHP 7.4'te çekirdekten çıkarıldı.",
        "The interbase extension was unbundled in PHP 7.4.")),
    rule!("removed.imageps", "imageps*", R, (7, 0), code("imagettftext / imagefttext"), M, Text::new(
        "PostScript Type1 font desteği PHP 7.0'da kaldırıldı.",
        "PostScript Type1 font support was removed in PHP 7.0.")),
    rule!("removed.set_socket_blocking", "set_socket_blocking", R, (7, 0), code("stream_set_blocking"), A, REMOVED_ALIAS),
    rule!("removed.datefmt_set_timezone_id", "datefmt_set_timezone_id", R, (7, 0), code("datefmt_set_timezone"), M, REMOVED_FN),
    rule!("deprecated.assert_options", "assert_options", D, (8, 3), tx("ini ayarları ya da zend.assertions", "ini settings or zend.assertions"), M, DEPRECATED_FN),
    rule!("deprecated.mysqli_execute", "mysqli_execute", D, (8, 5), code("mysqli_stmt_execute"), A, REMOVED_ALIAS),
    rule!("deprecated.socket_set_timeout", "socket_set_timeout", D, (8, 5), code("stream_set_timeout"), A, REMOVED_ALIAS),
    rule!("removed.mssql", "mssql_*", R, (7, 0), tx("sqlsrv_* ya da PDO", "sqlsrv_* or PDO"), M, Text::new(
        "mssql eklentisi PHP 7.0'da kaldırıldı.",
        "The mssql extension was removed in PHP 7.0.")),
    rule!("removed.sybase", "sybase_*", R, (7, 0), tx("PDO", "PDO"), M, Text::new(
        "sybase eklentisi PHP 7.0'da kaldırıldı.",
        "The sybase extension was removed in PHP 7.0.")),
    rule!("deprecated.libxml_disable_entity_loader", "libxml_disable_entity_loader", D, (8, 0), REMOVE_CALL, M, Text::new(
        "libxml 2.9'dan beri dış varlıklar zaten kapalı; bu çağrı hiçbir şey yapmıyor ve silinebilir.",
        "External entities are already disabled since libxml 2.9; this call does nothing and can be removed.")),
    rule!("removed.imap", "imap_*", R, (8, 4), tx("PECL imap ya da bir IMAP kütüphanesi", "PECL imap or an IMAP library"), M, Text::new(
        "imap eklentisi PHP 8.4'te çekirdekten çıkarıldı; ancak PECL'den ayrıca kurulursa çalışır.",
        "The imap extension was unbundled in PHP 8.4; it only works if you install it from PECL.")),
    rule!("removed.pspell", "pspell_*", R, (8, 4), tx("PECL pspell", "PECL pspell"), M, Text::new(
        "pspell eklentisi PHP 8.4'te çekirdekten çıkarıldı; ancak PECL'den ayrıca kurulursa çalışır.",
        "The pspell extension was unbundled in PHP 8.4; it only works if you install it from PECL.")),
    rule!("removed.oci8", "oci_*", R, (8, 4), tx("PECL oci8", "PECL oci8"), M, Text::new(
        "oci8 eklentisi PHP 8.4'te çekirdekten çıkarıldı; ancak PECL'den ayrıca kurulursa çalışır.",
        "The oci8 extension was unbundled in PHP 8.4; it only works if you install it from PECL.")),
];

pub const CONSTANTS: &[SymbolRule] = &[
    rule!(
        "deprecated.filter_sanitize_string",
        "filter_sanitize_string",
        D,
        (8, 1),
        code("htmlspecialchars()"),
        M,
        Text::new(
            "FILTER_SANITIZE_STRING kullanımdan kalktı. İhtiyacınıza göre htmlspecialchars() ya da strip_tags() kullanın.",
            "FILTER_SANITIZE_STRING is deprecated. Use htmlspecialchars() or strip_tags(), depending on what you need."
        )
    ),
    rule!(
        "deprecated.filter_sanitize_stripped",
        "filter_sanitize_stripped",
        D,
        (8, 1),
        code("htmlspecialchars()"),
        M,
        Text::new("FILTER_SANITIZE_STRIPPED kullanımdan kalktı.", "FILTER_SANITIZE_STRIPPED is deprecated.")
    ),
    rule!(
        "deprecated.e_strict",
        "e_strict",
        D,
        (8, 4),
        tx("E_STRICT'i kaldırın", "remove E_STRICT"),
        M,
        Text::new(
            "E_STRICT kullanımdan kalktı; bu hata seviyesi artık hiç tetiklenmiyor.",
            "E_STRICT is deprecated; this error level is never raised anymore."
        )
    ),
    rule!(
        "deprecated.e_user_error",
        "e_user_error",
        D,
        (8, 4),
        tx("exception fırlatın ya da exit() kullanın", "throw an exception or call exit()"),
        M,
        Text::new(
            "trigger_error() ile E_USER_ERROR kullanmak 8.4'te kullanımdan kalktı.",
            "Passing E_USER_ERROR to trigger_error() is deprecated in 8.4."
        )
    ),
    rule!(
        "removed.filter_sanitize_magic_quotes",
        "filter_sanitize_magic_quotes",
        R,
        (8, 0),
        code("FILTER_SANITIZE_ADD_SLASHES"),
        M,
        Text::new("FILTER_SANITIZE_MAGIC_QUOTES PHP 8.0'da kaldırıldı.", "FILTER_SANITIZE_MAGIC_QUOTES was removed in PHP 8.0.")
    ),
    rule!(
        "removed.intl_idna_variant_2003",
        "intl_idna_variant_2003",
        R,
        (8, 0),
        code("INTL_IDNA_VARIANT_UTS46"),
        M,
        Text::new("INTL_IDNA_VARIANT_2003 PHP 8.0'da kaldırıldı.", "INTL_IDNA_VARIANT_2003 was removed in PHP 8.0.")
    ),
    rule!(
        "removed.input_request",
        "input_request",
        R,
        (8, 0),
        code("$_REQUEST"),
        M,
        Text::new(
            "INPUT_REQUEST hiç uygulanmamıştı ve PHP 8.0'da kaldırıldı.",
            "INPUT_REQUEST was never implemented and was removed in PHP 8.0."
        )
    ),
    rule!(
        "removed.input_session",
        "input_session",
        R,
        (8, 0),
        code("$_SESSION"),
        M,
        Text::new(
            "INPUT_SESSION hiç uygulanmamıştı ve PHP 8.0'da kaldırıldı.",
            "INPUT_SESSION was never implemented and was removed in PHP 8.0."
        )
    ),
    rule!(
        "removed.assert_quiet_eval",
        "assert_quiet_eval",
        R,
        (8, 0),
        tx("sabiti silin", "remove the constant"),
        M,
        Text::new("ASSERT_QUIET_EVAL PHP 8.0'da kaldırıldı.", "ASSERT_QUIET_EVAL was removed in PHP 8.0.")
    ),
    rule!(
        "removed.mb_overload",
        "mb_overload_*",
        R,
        (8, 0),
        tx("sabiti silin", "remove the constant"),
        M,
        Text::new("mbstring fonksiyon aşırı yüklemesi PHP 8.0'da kaldırıldı.", "mbstring function overloading was removed in PHP 8.0.")
    ),
    rule!(
        "removed.pgsql_libpq_version_str",
        "pgsql_libpq_version_str",
        R,
        (8, 0),
        code("PGSQL_LIBPQ_VERSION"),
        M,
        Text::new("PGSQL_LIBPQ_VERSION_STR PHP 8.0'da kaldırıldı.", "PGSQL_LIBPQ_VERSION_STR was removed in PHP 8.0.")
    ),
    rule!("deprecated.file_binary", "file_binary", D, (8, 1), tx("bayrağı silin", "remove the flag"), M, NO_EFFECT),
    rule!("deprecated.file_text", "file_text", D, (8, 1), tx("bayrağı silin", "remove the flag"), M, NO_EFFECT),
    rule!(
        "deprecated.assert_constants",
        "assert_*",
        D,
        (8, 3),
        tx("zend.assertions ve assert.exception", "zend.assertions and assert.exception"),
        M,
        Text::new("ASSERT_* sabitleri 8.3'te kullanımdan kalktı.", "The ASSERT_* constants are deprecated in 8.3.")
    ),
    rule!(
        "deprecated.mt_rand_php",
        "mt_rand_php",
        D,
        (8, 3),
        code("MT_RAND_MT19937"),
        M,
        Text::new(
            "MT_RAND_PHP hatalı bir modulo uygulamasıdır ve 8.3'te kullanımdan kalktı.",
            "MT_RAND_PHP is a broken modulo implementation and is deprecated in 8.3."
        )
    ),
    rule!("deprecated.sunfuncs_ret", "sunfuncs_ret_*", D, (8, 4), code("date_sun_info()"), M, DEPRECATED_FN),
    rule!("deprecated.mysqli_refresh_constant", "mysqli_refresh_*", D, (8, 4), tx("FLUSH sorguları", "FLUSH queries"), M, DEPRECATED_FN),
    rule!("deprecated.mysqli_no_data", "mysqli_no_data", D, (8, 1), REMOVE_USE, M, UNUSED_CONSTANT),
    rule!("deprecated.mysqli_data_truncated", "mysqli_data_truncated", D, (8, 1), REMOVE_USE, M, UNUSED_CONSTANT),
    rule!("deprecated.mysqli_server_query", "mysqli_server_query_*", D, (8, 1), REMOVE_USE, M, UNUSED_CONSTANT),
    rule!("deprecated.mysqli_server_ps_out_params", "mysqli_server_ps_out_params", D, (8, 1), REMOVE_USE, M, UNUSED_CONSTANT),
    rule!(
        "deprecated.mysqli_is_mariadb",
        "mysqli_is_mariadb",
        D,
        (8, 2),
        None,
        M,
        Text::new(
            "MYSQLI_IS_MARIADB 8.2'de kullanımdan kalktı; değeri her zaman false olduğu için güvenilir değil. Sunucu türünü SELECT VERSION() ile kontrol edin.",
            "MYSQLI_IS_MARIADB is deprecated in 8.2; it's always false, so it can't be relied on. Check the server with SELECT VERSION()."
        )
    ),
    rule!(
        "deprecated.mysqli_store_result_copy_data",
        "mysqli_store_result_copy_data",
        D,
        (8, 4),
        REMOVE_USE,
        M,
        Text::new(
            "MYSQLI_STORE_RESULT_COPY_DATA, mysqli_store_result()'ın kullanımdan kalkan mode parametresiyle birlikte 8.4'te kullanımdan kalktı.",
            "MYSQLI_STORE_RESULT_COPY_DATA is deprecated in 8.4 together with the mode parameter of mysqli_store_result()."
        )
    ),
    rule!(
        "deprecated.u_multiple_decimal_seperators",
        "u_multiple_decimal_seperators",
        D,
        (8, 3),
        code("U_MULTIPLE_DECIMAL_SEPARATORS"),
        M,
        Text::new("Yazım hatalı bu sabit 8.3'te kullanımdan kalktı; doğru yazılmış adını kullanın.", "This misspelled constant is deprecated in 8.3; use the correctly spelled name.")
    ),
    rule!("deprecated.dom_php_err", "dom_php_err", D, (8, 4), None, M, DEPRECATED_FN),
    rule!(
        "deprecated.soap_functions_all",
        "soap_functions_all",
        D,
        (8, 4),
        tx("fonksiyonları tek tek ekleyin", "add the functions one by one"),
        M,
        DEPRECATED_FN
    ),
    rule!(
        "deprecated.date_rfc7231",
        "date_rfc7231",
        D,
        (8, 5),
        code("DATE_RFC1123"),
        M,
        Text::new(
            "DATE_RFC7231 saat dilimini yok sayıyordu ve 8.5'te kullanımdan kalktı.",
            "DATE_RFC7231 ignored the time zone and is deprecated in 8.5."
        )
    ),
    rule!("deprecated.mhash_constant", "mhash_*", D, (8, 5), code("hash_*"), M, DEPRECATED_FN),
    rule!(
        "removed.filter_flag_scheme_required",
        "filter_flag_scheme_required",
        R,
        (8, 0),
        tx("bayrağı silin", "remove the flag"),
        M,
        Text::new(
            "Bu bayrak PHP 8.0'da kaldırıldı; FILTER_VALIDATE_URL şemayı zaten zorunlu tutuyor.",
            "This flag was removed in PHP 8.0; FILTER_VALIDATE_URL already requires a scheme."
        )
    ),
    rule!(
        "removed.filter_flag_host_required",
        "filter_flag_host_required",
        R,
        (8, 0),
        tx("bayrağı silin", "remove the flag"),
        M,
        Text::new(
            "Bu bayrak PHP 8.0'da kaldırıldı; FILTER_VALIDATE_URL host'u zaten zorunlu tutuyor.",
            "This flag was removed in PHP 8.0; FILTER_VALIDATE_URL already requires a host."
        )
    ),
    rule!(
        "deprecated.curlopt_binarytransfer",
        "curlopt_binarytransfer",
        D,
        (8, 4),
        tx("seçeneği silin", "remove the option"),
        M,
        Text::new(
            "CURLOPT_BINARYTRANSFER uzun zamandır hiçbir şey yapmıyor ve 8.4'te kullanımdan kalktı.",
            "CURLOPT_BINARYTRANSFER has done nothing for years and is deprecated in 8.4."
        )
    ),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    Method,
    StaticCall,
    Constant,
}

#[derive(Debug, Clone, Copy)]
pub struct MemberRule {
    pub kind: MemberKind,
    pub class: &'static str,
    pub rule: SymbolRule,
}

const fn member(kind: MemberKind, class: &'static str, rule: SymbolRule) -> MemberRule {
    MemberRule { kind, class, rule }
}

use MemberKind::{Constant as K, Method as Mt, StaticCall as St};

pub const MEMBERS: &[MemberRule] = &[
    member(
        Mt,
        "mysqli",
        rule!(
            "deprecated.mysqli_method",
            "ping",
            D,
            (8, 4),
            None,
            M,
            Text::new(
                "mysqli::ping() kullanımdan kalktı; kopan bağlantı artık kendiliğinden yeniden kurulmuyor.",
                "mysqli::ping() is deprecated; dropped connections are no longer reconnected automatically."
            )
        ),
    ),
    member(
        Mt,
        "mysqli",
        rule!("deprecated.mysqli_method", "kill", D, (8, 4), tx("KILL CONNECTION sorgusu", "a KILL CONNECTION query"), M, DEPRECATED_FN),
    ),
    member(Mt, "mysqli", rule!("deprecated.mysqli_method", "refresh", D, (8, 4), tx("FLUSH sorguları", "FLUSH queries"), M, DEPRECATED_FN)),
    member(
        Mt,
        "reflection",
        rule!(
            "deprecated.set_accessible",
            "setaccessible",
            D,
            (8, 5),
            REMOVE_CALL,
            A,
            Text::new(
                "setAccessible() PHP 8.1'den beri hiçbir şey yapmıyor ve 8.5'te kullanımdan kalktı; çağrı silinebilir.",
                "setAccessible() has done nothing since PHP 8.1 and is deprecated in 8.5; the call can be removed."
            )
        ),
    ),
    member(Mt, "splobjectstorage", rule!("deprecated.spl_object_storage", "attach", D, (8, 5), code("offsetSet()"), A, DEPRECATED_FN)),
    member(Mt, "splobjectstorage", rule!("deprecated.spl_object_storage", "detach", D, (8, 5), code("offsetUnset()"), A, DEPRECATED_FN)),
    member(Mt, "splobjectstorage", rule!("deprecated.spl_object_storage", "contains", D, (8, 5), code("offsetExists()"), A, DEPRECATED_FN)),
    member(
        St,
        "reflection",
        rule!(
            "removed.reflection_export",
            "export",
            R,
            (8, 0),
            tx("(string) new ReflectionX(...)", "(string) new ReflectionX(...)"),
            M,
            Text::new(
                "Reflection*::export() PHP 8.0'da kaldırıldı; nesneyi metne çevirip yazdırın.",
                "Reflection*::export() was removed in PHP 8.0; cast the object to a string and print it instead."
            )
        ),
    ),
    member(K, "pdo", rule!("deprecated.pdo_driver_constant", "mysql_attr_*", D, (8, 5), code("Pdo\\Mysql::ATTR_*"), A, PDO_DRIVER)),
    member(
        K,
        "pgsql",
        rule!(
            "deprecated.pgsql_transaction_constant",
            "transaction_*",
            D,
            (8, 5),
            REMOVE_USE,
            M,
            PGSQL_TRANSACTION
        ),
    ),
    member(
        K,
        "numberformatter",
        rule!(
            "deprecated.numberformatter_type_currency",
            "type_currency",
            D,
            (8, 3),
            code("NumberFormatter::formatCurrency()"),
            M,
            Text::new("NumberFormatter::TYPE_CURRENCY 8.3'te kullanımdan kalktı; formatCurrency() ve parseCurrency() kullanın.", "NumberFormatter::TYPE_CURRENCY is deprecated in 8.3; use formatCurrency() and parseCurrency().")
        ),
    ),
    member(
        K,
        "ziparchive",
        rule!(
            "deprecated.zip_fl_recompress",
            "fl_recompress",
            D,
            (8, 3),
            REMOVE_USE,
            M,
            Text::new("ZipArchive::FL_RECOMPRESS 8.3'te kullanımdan kalktı; libzip'in yeni sürümlerinde kaldırılacak.", "ZipArchive::FL_RECOMPRESS is deprecated in 8.3 and will be removed in future libzip versions.")
        ),
    ),
    member(
        K,
        "datetime",
        rule!(
            "deprecated.date_rfc7231",
            "rfc7231",
            D,
            (8, 5),
            tx("UTC'de 'D, d M Y H:i:s \\G\\M\\T' biçimi", "the 'D, d M Y H:i:s \\G\\M\\T' format in UTC"),
            M,
            Text::new(
                "RFC7231 biçimi saat dilimini yok sayıp her zaman GMT yazdığı için 8.5'te kullanımdan kalktı. Tarihi UTC'ye çevirip biçimi kendiniz yazın.",
                "The RFC7231 format ignores the time zone and always writes GMT, so it's deprecated in 8.5. Convert the date to UTC and spell out the format."
            )
        ),
    ),
    member(Mt, "datetime", rule!("deprecated.date_wakeup", "__wakeup", D, (8, 5), code("__unserialize()"), M, WAKEUP)),
    member(Mt, "dateinterval", rule!("deprecated.date_wakeup", "__wakeup", D, (8, 5), code("__unserialize()"), M, WAKEUP)),
    member(Mt, "dateperiod", rule!("deprecated.date_wakeup", "__wakeup", D, (8, 5), code("__unserialize()"), M, WAKEUP)),
    member(Mt, "splfixedarray", rule!("deprecated.splfixedarray_wakeup", "__wakeup", D, (8, 4), code("__unserialize()"), M, WAKEUP)),
    member(Mt, "splfileinfo", rule!("deprecated.splfileinfo_bad_state_ex", "_bad_state_ex", D, (8, 2), REMOVE_CALL, M, DEPRECATED_FN)),
    member(
        Mt,
        "mysqli",
        rule!(
            "deprecated.mysqli_method",
            "get_client_info",
            D,
            (8, 1),
            code("mysqli_get_client_info()"),
            M,
            Text::new("mysqli::get_client_info() 8.1'de kullanımdan kalktı; argümansız mysqli_get_client_info() fonksiyonunu çağırın.", "mysqli::get_client_info() is deprecated in 8.1; call mysqli_get_client_info() without arguments.")
        ),
    ),
    member(
        Mt,
        "mysqli",
        rule!(
            "deprecated.mysqli_method",
            "init",
            D,
            (8, 1),
            code("parent::__construct()"),
            M,
            Text::new("mysqli::init() 8.1'de kullanımdan kalktı; alt sınıflarda parent::init() yerine parent::__construct() çağırın.", "mysqli::init() is deprecated in 8.1; in subclasses call parent::__construct() instead of parent::init().")
        ),
    ),
    member(
        Mt,
        "reflectionfunction",
        rule!(
            "deprecated.reflection_method",
            "isdisabled",
            D,
            (8, 0),
            REMOVE_CALL,
            M,
            Text::new("ReflectionFunction::isDisabled() 8.0'da kullanımdan kalktı; artık her zaman false döner.", "ReflectionFunction::isDisabled() is deprecated in 8.0; it always returns false now.")
        ),
    ),
    member(Mt, "reflectionparameter", rule!("deprecated.reflection_method", "getclass", D, (8, 0), code("getType()"), M, REFLECTION_TYPE)),
    member(Mt, "reflectionparameter", rule!("deprecated.reflection_method", "isarray", D, (8, 0), code("getType()"), M, REFLECTION_TYPE)),
    member(Mt, "reflectionparameter", rule!("deprecated.reflection_method", "iscallable", D, (8, 0), code("getType()"), M, REFLECTION_TYPE)),
    member(K, "pdo", rule!("deprecated.pgsql_transaction_constant", "pgsql_transaction_*", D, (8, 5), REMOVE_USE, M, PGSQL_TRANSACTION)),
    member(K, "pdo", rule!("deprecated.pdo_driver_constant", "pgsql_*", D, (8, 5), code("Pdo\\Pgsql::*"), A, PDO_DRIVER)),
    member(K, "pdo", rule!("deprecated.pdo_driver_constant", "sqlite_*", D, (8, 5), code("Pdo\\Sqlite::*"), A, PDO_DRIVER)),
    member(K, "pdo", rule!("deprecated.pdo_driver_constant", "odbc_*", D, (8, 5), code("Pdo\\Odbc::*"), A, PDO_DRIVER)),
    member(K, "pdo", rule!("deprecated.pdo_driver_constant", "dblib_*", D, (8, 5), code("Pdo\\Dblib::*"), A, PDO_DRIVER)),
    member(K, "pdo", rule!("deprecated.pdo_driver_constant", "fb_*", D, (8, 5), code("Pdo\\Firebird::*"), A, PDO_DRIVER)),
    member(
        K,
        "pdo",
        rule!(
            "deprecated.pdo_fetch_serialize",
            "fetch_serialize",
            D,
            (8, 1),
            None,
            M,
            Text::new(
                "PDO::FETCH_SERIALIZE, Serializable arayüzüne dayandığı için 8.1'de kullanımdan kalktı.",
                "PDO::FETCH_SERIALIZE relies on the Serializable interface and is deprecated in 8.1."
            )
        ),
    ),
];

const NO_EFFECT: Text = Text::new("Hiçbir etkisi yok; silinebilir.", "Has no effect and can be removed.");

pub const LARAVEL_HELPERS: [&str; 41] = [
    "array_add",
    "array_collapse",
    "array_divide",
    "array_dot",
    "array_except",
    "array_first",
    "array_flatten",
    "array_forget",
    "array_get",
    "array_has",
    "array_last",
    "array_only",
    "array_pluck",
    "array_prepend",
    "array_pull",
    "array_random",
    "array_set",
    "array_sort",
    "array_sort_recursive",
    "array_where",
    "array_wrap",
    "camel_case",
    "ends_with",
    "kebab_case",
    "snake_case",
    "starts_with",
    "str_after",
    "str_before",
    "str_finish",
    "str_is",
    "str_limit",
    "str_plural",
    "str_random",
    "str_replace_array",
    "str_replace_first",
    "str_replace_last",
    "str_singular",
    "str_slug",
    "str_start",
    "studly_case",
    "title_case",
];

pub const LARAVEL_HELPER: SymbolRule = rule!("framework.laravel_helper", "laravel_helper", R, (8, 0), tx(
    "Illuminate\\Support\\Str ve Arr sınıfları ya da laravel/helpers paketi",
    "the Illuminate\\Support\\Str and Arr classes, or the laravel/helpers package"), M, Text::new(
    "Laravel 6 str_ ve array_ yardımcılarını çıkardı. PHP 8 en az Laravel 6.20 gerektirdiği için bu çağrı çalışmaz. Str ya da Arr sınıfını kullanın veya composer require laravel/helpers ile paketi ekleyin.",
    "Laravel 6 removed the str_ and array_ helpers. PHP 8 needs at least Laravel 6.20, so this call breaks. Use the Str or Arr class, or add the package with composer require laravel/helpers."));

pub const LARAVEL_INPUT: SymbolRule = rule!(
    "framework.laravel_input",
    "input",
    R,
    (8, 0),
    code("Request::input()"),
    M,
    Text::new(
        "Input facade'ı Laravel 6'da kaldırıldı. PHP 8 en az Laravel 6.20 gerektirdiği için Request facade'ını kullanın.",
        "The Input facade was removed in Laravel 6. PHP 8 needs at least Laravel 6.20, so use the Request facade instead."
    )
);

const UNUSED_CONSTANT: Text = Text::new(
    "Bu sabit hiçbir yerde kullanılmadığı için 8.1'de kullanımdan kalktı; kullanımı silinebilir.",
    "This constant was never used by PHP and is deprecated in 8.1; its use can be removed.",
);
const REMOVE_USE: Option<Text> = tx("kullanımı silin", "remove its use");
const WAKEUP: Text = Text::new(
    "Bu sınıfın __wakeup() metodu kullanımdan kalktı; serileştirme artık __unserialize() ile yapılıyor. Alt sınıfta parent::__wakeup() çağrısını kaldırın ya da __unserialize()'a taşıyın.",
    "This class's __wakeup() method is deprecated; unserialization now goes through __unserialize(). In a subclass, drop the parent::__wakeup() call or move it to __unserialize().",
);
const REFLECTION_TYPE: Text = Text::new(
    "Bu ReflectionParameter metodu 8.0'da kullanımdan kalktı; parametrenin tipini getType() ve ReflectionNamedType ile okuyun.",
    "This ReflectionParameter method is deprecated in 8.0; read the parameter type with getType() and ReflectionNamedType.",
);
const PGSQL_TRANSACTION: Text = Text::new(
    "PDO'nun PostgreSQL işlem durumu sabitleri hiçbir işe yaramadığı için 8.5'te kullanımdan kalktı. Pdo\\Pgsql::TRANSACTION_* da kullanımdan kalktığı için yerine geçen bir sabit yok; kullanımı silin, işlem durumunu PDO::inTransaction() ile kontrol edin.",
    "PDO's PostgreSQL transaction state constants have no effect and are deprecated in 8.5. Pdo\\Pgsql::TRANSACTION_* is deprecated too, so there is no replacement; remove the use and check the state with PDO::inTransaction().",
);
const PDO_DRIVER: Text = Text::new(
    "PDO'nun sürücüye özel sabitleri 8.5'te kullanımdan kalktı; sürücünün kendi sınıfındaki sabiti kullanın.",
    "PDO's driver-specific constants are deprecated in 8.5; use the constant on the driver's own class.",
);

pub const ADDED_FUNCTIONS: &[(&str, (u32, u32))] = &[
    ("array_all", (8, 4)),
    ("array_any", (8, 4)),
    ("array_find", (8, 4)),
    ("array_find_key", (8, 4)),
    ("array_first", (8, 5)),
    ("array_is_list", (8, 1)),
    ("array_key_first", (7, 3)),
    ("array_key_last", (7, 3)),
    ("array_last", (8, 5)),
    ("bcceil", (8, 4)),
    ("bcdivmod", (8, 4)),
    ("bcfloor", (8, 4)),
    ("bcround", (8, 4)),
    ("curl_upkeep", (8, 2)),
    ("enum_exists", (8, 1)),
    ("error_clear_last", (7, 0)),
    ("fdatasync", (8, 1)),
    ("fdiv", (8, 0)),
    ("fpow", (8, 4)),
    ("fsync", (8, 1)),
    ("gc_mem_caches", (7, 0)),
    ("get_debug_type", (8, 0)),
    ("get_error_handler", (8, 5)),
    ("get_exception_handler", (8, 5)),
    ("get_mangled_object_vars", (7, 4)),
    ("get_resource_id", (8, 0)),
    ("hash_hmac_algos", (7, 2)),
    ("hrtime", (7, 3)),
    ("http_clear_last_response_headers", (8, 4)),
    ("http_get_last_response_headers", (8, 4)),
    ("ini_parse_quantity", (8, 2)),
    ("intdiv", (7, 0)),
    ("is_countable", (7, 3)),
    ("is_iterable", (7, 1)),
    ("json_validate", (8, 3)),
    ("libxml_get_external_entity_loader", (8, 2)),
    ("mb_chr", (7, 2)),
    ("mb_lcfirst", (8, 4)),
    ("mb_ltrim", (8, 4)),
    ("mb_ord", (7, 2)),
    ("mb_rtrim", (8, 4)),
    ("mb_scrub", (7, 2)),
    ("mb_str_pad", (8, 3)),
    ("mb_str_split", (7, 4)),
    ("mb_trim", (8, 4)),
    ("mb_ucfirst", (8, 4)),
    ("memory_reset_peak_usage", (8, 2)),
    ("mysqli_execute_query", (8, 2)),
    ("net_get_interfaces", (7, 3)),
    ("openssl_cipher_key_length", (8, 2)),
    ("password_algos", (7, 4)),
    ("preg_last_error_msg", (8, 0)),
    ("preg_replace_callback_array", (7, 0)),
    ("random_bytes", (7, 0)),
    ("random_int", (7, 0)),
    ("session_create_id", (7, 1)),
    ("spl_object_id", (7, 2)),
    ("str_contains", (8, 0)),
    ("str_decrement", (8, 3)),
    ("str_ends_with", (8, 0)),
    ("str_increment", (8, 3)),
    ("str_starts_with", (8, 0)),
    ("stream_context_set_options", (8, 3)),
    ("stream_isatty", (7, 2)),
];

pub const ADDED_CLASSES: &[(&str, (u32, u32))] = &[
    ("allowdynamicproperties", (8, 2)),
    ("argumentcounterror", (7, 1)),
    ("arithmeticerror", (7, 0)),
    ("assertionerror", (7, 0)),
    ("attribute", (8, 0)),
    ("compileerror", (7, 3)),
    ("deprecated", (8, 4)),
    ("divisionbyzeroerror", (7, 0)),
    ("error", (7, 0)),
    ("fiber", (8, 1)),
    ("fibererror", (8, 1)),
    ("jsonexception", (7, 3)),
    ("nodiscard", (8, 5)),
    ("override", (8, 3)),
    ("parseerror", (7, 0)),
    ("phptoken", (8, 0)),
    ("returntypewillchange", (8, 1)),
    ("sensitiveparameter", (8, 2)),
    ("stringable", (8, 0)),
    ("throwable", (7, 0)),
    ("typeerror", (7, 0)),
    ("unhandledmatcherror", (8, 0)),
    ("valueerror", (8, 0)),
    ("weakmap", (8, 0)),
    ("weakreference", (7, 4)),
];

pub fn added_function(name: &str) -> Option<PhpVersion> {
    ADDED_FUNCTIONS
        .binary_search_by(|(n, _)| (*n).cmp(name))
        .ok()
        .map(|i| PhpVersion::new(ADDED_FUNCTIONS[i].1 .0, ADDED_FUNCTIONS[i].1 .1))
}

pub fn added_class(name: &str) -> Option<PhpVersion> {
    ADDED_CLASSES.binary_search_by(|(n, _)| (*n).cmp(name)).ok().map(|i| PhpVersion::new(ADDED_CLASSES[i].1 .0, ADDED_CLASSES[i].1 .1))
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
        note: Text::new("Ters tırnak operatörü 8.5'te kullanımdan kalktı.", "The backtick operator is deprecated in 8.5."),
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
    pub const PHP4_CONSTRUCTOR: PatternRule = PatternRule {
        id: "removed.php4_constructor",
        title: Text::new("PHP 4 tarzı kurucu", "PHP 4 style constructor"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: code("__construct()"),
        fix: FixKind::Likely,
        note: Text::new(
            "Sınıfla aynı adı taşıyan metot PHP 8.0'dan itibaren kurucu sayılmıyor; nesne oluşturulurken artık çağrılmaz.",
            "Since PHP 8.0 a method named after its class is no longer a constructor and isn't called when the object is created.",
        ),
    };
    pub const NESTED_TERNARY: PatternRule = PatternRule {
        id: "removed.nested_ternary",
        title: Text::new("Parantezsiz iç içe ternary", "Unparenthesized nested ternary"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: code("$a ? $b : ($c ? $d : $e)"),
        fix: FixKind::Likely,
        note: Text::new(
            "Parantezsiz iç içe ternary PHP 8.0'da derleme hatası verir. Rector eski PHP 7 davranışını koruyan parantezleri ekler; bu çoğu zaman kodun asıl niyeti değildir, uygulamadan önce kontrol edin.",
            "Nested ternaries without parentheses are a compile error in PHP 8.0. Rector adds parentheses that keep the old PHP 7 behaviour, which is often not what the code meant, so check it before applying.",
        ),
    };
    pub const CASE_INSENSITIVE_CONSTANT: PatternRule = PatternRule {
        id: "removed.case_insensitive_constant",
        title: Text::new("Büyük/küçük harfe duyarsız sabit", "Case-insensitive constant"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: code("define('NAME', $value)"),
        fix: FixKind::Likely,
        note: Text::new(
            "define()'ın üçüncü parametresi PHP 8.0'da kaldırıldı. Sabite farklı harf büyüklüğüyle erişen yerler artık çalışmaz.",
            "The third argument of define() was removed in PHP 8.0. Code that reads the constant with different casing stops working.",
        ),
    };
    pub const STATIC_CALL_TO_INSTANCE_METHOD: PatternRule = PatternRule {
        id: "removed.static_call_to_instance_method",
        title: Text::new("Statik olmayan metodun statik çağrılması", "Non-static method called statically"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: tx("nesne oluşturup ->metot() ile çağırın ya da metodu static yapın", "call it on an instance with ->method(), or make the method static"),
        fix: FixKind::Manual,
        note: Text::new(
            "PHP 7'de yalnızca uyarı veren Sinif::metot() çağrısı, metot static değilse PHP 8'de Error fırlatır. Metot $this kullanmıyorsa static yapın; kullanıyorsa bir nesne üzerinden çağırın.",
            "Calling Class::method() on a non-static method only warned in PHP 7; PHP 8 throws an Error. Make the method static if it doesn't use $this, otherwise call it on an instance.",
        ),
    };
    pub const TOO_FEW_ARGUMENTS: PatternRule = PatternRule {
        id: "removed.too_few_arguments",
        title: Text::new("Eksik argümanla fonksiyon çağrısı", "Function called with too few arguments"),
        severity: Severity::Breaking,
        since: (7, 1),
        replacement: tx("eksik argümanları verin ya da parametrelere varsayılan değer ekleyin", "pass the missing arguments or give the parameters defaults"),
        fix: FixKind::Manual,
        note: Text::new(
            "PHP 7.0'a kadar eksik argüman yalnızca uyarı verip parametreyi null yapıyordu. PHP 7.1'den beri çağrı ArgumentCountError fırlatır.",
            "Until PHP 7.0 a missing argument only warned and left the parameter null. Since PHP 7.1 the call throws ArgumentCountError.",
        ),
    };
    pub const PREG_EVAL_MODIFIER: PatternRule = PatternRule {
        id: "removed.preg_eval_modifier",
        title: Text::new("preg_replace() içinde /e belirteci", "/e modifier in preg_replace()"),
        severity: Severity::Breaking,
        since: (7, 0),
        replacement: code("preg_replace_callback()"),
        fix: FixKind::Manual,
        note: Text::new(
            "/e belirteci PHP 7.0'da kaldırıldı. Çağrı artık uyarı verip null döndürür, yani metin sessizce boşalır. Değiştirme ifadesini bir fonksiyona taşıyıp preg_replace_callback() kullanın.",
            "The /e modifier was removed in PHP 7.0. The call now warns and returns null, so the text is silently emptied. Move the replacement expression into a function and use preg_replace_callback().",
        ),
    };
    pub const RESOURCE_CHECK: PatternRule = PatternRule {
        id: "removed.resource_check",
        title: Text::new("Artık nesne olan değer için is_resource()", "is_resource() on a handle that is now an object"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: tx("$handle !== false ya da instanceof kontrolü", "a $handle !== false or instanceof check"),
        fix: FixKind::Manual,
        note: Text::new(
            "Bu fonksiyon artık resource yerine nesne döndürüyor (örneğin CurlHandle, GdImage, PgSql\\Connection). is_resource() bu değer için her zaman false döner, get_resource_type() TypeError fırlatır. Hata kontrolünü $handle !== false ile yapın.",
            "This function now returns an object instead of a resource (for example CurlHandle, GdImage, PgSql\\Connection). is_resource() is always false for it and get_resource_type() throws a TypeError. Check for failure with $handle !== false.",
        ),
    };
    pub const REDECLARED_BUILTIN: PatternRule = PatternRule {
        id: "removed.redeclared_builtin",
        title: Text::new("PHP'nin kendi fonksiyonunu ya da sınıfını yeniden tanımlama", "Redeclares a PHP built-in"),
        severity: Severity::Breaking,
        since: (7, 0),
        replacement: tx("tanımı if (!function_exists(...)) ya da class_exists() ile koruyun", "wrap it in if (!function_exists(...)) or class_exists()"),
        fix: FixKind::Manual,
        note: Text::new(
            "Bu ad sonradan PHP'nin kendisine eklendi. Korumasız tanım hedef sürümde \"Cannot redeclare\" hatasıyla dosyanın hiç yüklenmemesine yol açar. Tanımı function_exists() ya da class_exists() kontrolüyle sarın veya PHP'nin kendi sürümüne geçip tanımı silin.",
            "PHP itself added this name later. An unguarded declaration stops the file from loading on the target with a \"Cannot redeclare\" error. Wrap it in a function_exists() or class_exists() check, or switch to PHP's own version and delete it.",
        ),
    };
    pub const LARAVEL_NATIVE_ARRAY_FIRST: PatternRule = PatternRule {
        id: "framework.laravel_native_array_first",
        title: Text::new("Callback'li array_first() / array_last()", "array_first() / array_last() with a callback"),
        severity: Severity::Breaking,
        since: (8, 5),
        replacement: code("Arr::first() / Arr::last()"),
        fix: FixKind::Manual,
        note: Text::new(
            "PHP 8.5 kendi array_first() ve array_last() fonksiyonlarını ekledi; bunlar yalnızca diziyi alır. laravel/helpers aynı adı tanımlamadan önce function_exists() ile kontrol ettiği için PHP'nin fonksiyonu çağrılır ve callback ya da varsayılan değer verilen çağrı ArgumentCountError fırlatır. Illuminate\\Support\\Arr::first() ya da Arr::last() kullanın.",
            "PHP 8.5 adds its own array_first() and array_last(), which take only the array. laravel/helpers checks function_exists() before defining them, so PHP's function is called and a call with a callback or default throws ArgumentCountError. Use Illuminate\\Support\\Arr::first() or Arr::last().",
        ),
    };
    pub const ERROR_HANDLER_CONTEXT: PatternRule = PatternRule {
        id: "removed.error_handler_context",
        title: Text::new("Beş zorunlu parametreli hata işleyici", "Error handler with five required parameters"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: tx("beşinci parametreye varsayılan değer verin ya da silin", "give the fifth parameter a default or remove it"),
        fix: FixKind::Manual,
        note: Text::new(
            "PHP 8.0 hata işleyicilere beşinci $errcontext argümanını artık göndermiyor. Bu parametre zorunluysa her uyarıda işleyici ArgumentCountError fırlatır.",
            "PHP 8.0 no longer passes the fifth $errcontext argument to error handlers. If that parameter is required, the handler throws ArgumentCountError on every warning.",
        ),
    };
    pub const EXCEPTION_HANDLER_TYPE: PatternRule = PatternRule {
        id: "removed.exception_handler_type",
        title: Text::new("Exception tipli istisna işleyici", "Exception handler typed Exception"),
        severity: Severity::Breaking,
        since: (7, 0),
        replacement: code("function (Throwable $e)"),
        fix: FixKind::Manual,
        note: Text::new(
            "PHP 7'den beri yakalanmayan Error nesneleri de istisna işleyicisine gelir. Parametre Exception tipliyse bir Error geldiğinde işleyici TypeError fırlatır ve asıl hata kaybolur. Tipi Throwable yapın.",
            "Since PHP 7, uncaught Error objects reach the exception handler too. With an Exception type, an Error makes the handler throw a TypeError and the original error is lost. Type it as Throwable.",
        ),
    };
    pub const ERROR_REPORTING_ZERO: PatternRule = PatternRule {
        id: "behavior.error_reporting_zero",
        title: Text::new("@ için error_reporting() == 0 kontrolü", "error_reporting() == 0 check for @"),
        severity: Severity::Behavior,
        since: (8, 0),
        replacement: code("!(error_reporting() & $errno)"),
        fix: FixKind::Manual,
        note: Text::new(
            "PHP 8'de @ operatörü error_reporting() değerini 0 yapmıyor, ölümcül hataları açık bırakıyor. Bu kontrol artık hiç tutmaz ve @ ile susturulan uyarılar da işlenir.",
            "In PHP 8 the @ operator no longer sets error_reporting() to 0; it keeps fatal errors on. This check never matches anymore, so warnings silenced with @ get handled too.",
        ),
    };
    pub const PARENT_WITHOUT_PARENT: PatternRule = PatternRule {
        id: "removed.parent_without_parent",
        title: Text::new("Üst sınıfı olmayan sınıfta parent::", "parent:: in a class without a parent"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: tx("parent:: çağrısını silin ya da sınıfa extends ekleyin", "remove the parent:: call or add extends to the class"),
        fix: FixKind::Manual,
        note: Text::new(
            "Sınıfın üst sınıfı yokken parent:: kullanmak PHP 8.0'da derleme hatasıdır; dosya hiç yüklenmez.",
            "Using parent:: in a class without a parent is a compile error in PHP 8.0, so the file doesn't load.",
        ),
    };
    pub const BUILTIN_ARITY: PatternRule = PatternRule {
        id: "removed.builtin_arity",
        title: Text::new("Yerleşik fonksiyona yanlış sayıda argüman", "Wrong number of arguments to a built-in"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: None,
        fix: FixKind::Manual,
        note: Text::new(
            "PHP 7'de yerleşik fonksiyonlar eksik ya da fazla argümanla yalnızca uyarı verip null döndürüyordu. PHP 8'de çağrı ArgumentCountError fırlatır. Örneğin mktime() artık saati, crypt() de salt değerini zorunlu istiyor.",
            "In PHP 7 a built-in function called with too few or too many arguments only warned and returned null. PHP 8 throws ArgumentCountError. For example, mktime() now requires the hour and crypt() the salt.",
        ),
    };
    pub const PARSE_STR_ONE_ARG: PatternRule = PatternRule {
        id: "removed.parse_str_one_arg",
        title: Text::new("Tek parametreli parse_str()", "parse_str() with one argument"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: code("parse_str($s, $result)"),
        fix: FixKind::Likely,
        note: Text::new(
            "parse_str() artık sonucu yerel değişkenlere yazmıyor; ikinci parametre zorunlu. Düzeltmeden sonra bu değişkenleri kullanan yerleri $result['...'] olarak değiştirin.",
            "parse_str() no longer writes into local variables; the second argument is required. After the fix, change code that used those variables to $result['...'].",
        ),
    };
    pub const ASSERT_STRING: PatternRule = PatternRule {
        id: "removed.assert_string",
        title: Text::new("Metin olarak assert()", "assert() with a string"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: code("assert($condition)"),
        fix: FixKind::Likely,
        note: Text::new(
            "assert()'a verilen metin PHP 8.0'dan itibaren kod olarak çalıştırılmıyor; doğrudan koşulu yazın.",
            "Since PHP 8.0 a string passed to assert() is no longer evaluated as code; pass the condition itself.",
        ),
    };
    pub const BAREWORD_KEY: PatternRule = PatternRule {
        id: "removed.bareword_key",
        title: Text::new("Tırnaksız dizi anahtarı", "Unquoted array key"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: code("$row['id']"),
        fix: FixKind::Likely,
        note: Text::new(
            "Tanımsız sabit artık metne çevrilmiyor; $row[id] PHP 8.0'da Error fırlatır. Anahtarı tırnak içine alın.",
            "Undefined constants are no longer turned into strings; $row[id] throws an Error in PHP 8.0. Quote the key.",
        ),
    };
    pub const CSV_ESCAPE: PatternRule = PatternRule {
        id: "deprecated.csv_escape",
        title: Text::new("escape parametresiz CSV fonksiyonu", "CSV function without an escape argument"),
        severity: Severity::Deprecated,
        since: (8, 4),
        replacement: code("fputcsv($f, $row, escape: '')"),
        fix: FixKind::Auto,
        note: Text::new(
            "fputcsv(), fgetcsv() ve str_getcsv() escape parametresi verilmeden çağrılınca 8.4'te uyarı veriyor.",
            "fputcsv(), fgetcsv() and str_getcsv() raise a deprecation in 8.4 when the escape argument isn't given.",
        ),
    };
    pub const MB_ENCODING: PatternRule = PatternRule {
        id: "deprecated.mb_encoding",
        title: Text::new("Kullanımdan kalkan mbstring kodlaması", "Deprecated mbstring encoding"),
        severity: Severity::Deprecated,
        since: (8, 2),
        replacement: tx(
            "htmlspecialchars(), base64_encode() ya da quoted_printable_encode()",
            "htmlspecialchars(), base64_encode() or quoted_printable_encode()",
        ),
        fix: FixKind::Manual,
        note: Text::new(
            "HTML-ENTITIES, BASE64, UUENCODE ve QPrint kodlamaları mbstring'de 8.2'de kullanımdan kalktı.",
            "The HTML-ENTITIES, BASE64, UUENCODE and QPrint encodings are deprecated in mbstring as of 8.2.",
        ),
    };
    pub const RETURN_TYPE_WILL_CHANGE: PatternRule = PatternRule {
        id: "deprecated.tentative_return_type",
        title: Text::new("Dönüş tipi eksik ya da uyumsuz yerleşik metot", "Built-in method with a missing or incompatible return type"),
        severity: Severity::Deprecated,
        since: (8, 1),
        replacement: code("#[\\ReturnTypeWillChange]"),
        fix: FixKind::Manual,
        note: Text::new(
            "ArrayAccess, Iterator, Countable ya da PDO gibi yerleşik sınıf ve arayüzlerin metotları PHP 8.1'den itibaren uyumlu bir dönüş tipi bekliyor. Yerleşik metodunkiyle uyumlu bir dönüş tipi yazın ya da #[\\ReturnTypeWillChange] ekleyin.",
            "Since PHP 8.1, methods of built-in classes and interfaces such as ArrayAccess, Iterator, Countable or PDO expect a compatible return type. Declare one that fits the built-in method or add #[\\ReturnTypeWillChange].",
        ),
    };
    pub const ASSIGN_NEW_BY_REF: PatternRule = PatternRule {
        id: "removed.assign_new_by_ref",
        title: Text::new("= &new", "= &new"),
        severity: Severity::Breaking,
        since: (7, 0),
        replacement: code("= new"),
        fix: FixKind::Auto,
        note: Text::new(
            "new ile oluşturulan nesneyi referansla atamak PHP 7.0'da kaldırıldı ve ayrıştırma hatası verir. Nesneler zaten referans gibi davrandığı için & işaretini silmek yeterli.",
            "Assigning the result of new by reference was removed in PHP 7.0 and is a parse error. Objects already behave like references, so dropping the & is enough.",
        ),
    };
    pub const RESERVED_ALIAS: PatternRule = PatternRule {
        id: "removed.reserved_alias",
        title: Text::new("Ayrılmış ad takma ad olarak kullanılıyor", "Reserved name used as an import alias"),
        severity: Severity::Breaking,
        since: (7, 0),
        replacement: None,
        fix: FixKind::Manual,
        note: Text::new(
            "Bu use satırı sınıfı object, int ya da string gibi yeni PHP sürümlerinde ayrılmış bir adla içe aktarıyor. PHP dosyayı derlerken ölümcül hata verir. Sınıfa farklı bir takma ad verin ve dosyadaki kullanımlarını da bu adla değiştirin.",
            "This use statement imports the class under a name that newer PHP versions reserve, such as object, int or string. PHP throws a fatal error when it compiles the file. Give the class a different alias and update its uses in the file.",
        ),
    };
    pub const GLOBALS_WRITE: PatternRule = PatternRule {
        id: "removed.globals_write",
        title: Text::new("$GLOBALS dizisinin tamamına yazılıyor", "Writing to the whole $GLOBALS array"),
        severity: Severity::Breaking,
        since: (8, 1),
        replacement: code("$GLOBALS['name'] = $value"),
        fix: FixKind::Manual,
        note: Text::new(
            "PHP 8.1'den itibaren $GLOBALS dizisinin tamamına değer atamak, onu unset etmek, referansını almak ya da sonuna eleman eklemek derleme hatası verir ve dosya hiç çalışmaz. Tek tek değişkenleri $GLOBALS['ad'] = $deger biçiminde değiştirin ya da $GLOBALS'ın bir kopyasıyla çalışın.",
            "Since PHP 8.1, assigning to the whole $GLOBALS array, unsetting it, taking a reference to it or appending to it is a compile-time error and the file never runs. Change single variables with $GLOBALS['name'] = $value, or work on a copy of $GLOBALS.",
        ),
    };
    pub const BUILTIN_RESULT_BY_REF: PatternRule = PatternRule {
        id: "removed.builtin_result_by_ref",
        title: Text::new("Yerleşik fonksiyon sonucu referansla atanıyor", "Built-in function result assigned by reference"),
        severity: Severity::Breaking,
        since: (7, 0),
        replacement: None,
        fix: FixKind::Auto,
        note: Text::new(
            "func_get_args(), strlen(), count() gibi fonksiyonları PHP derleme sırasında özel olarak işler; sonuçlarını referansla atamak ölümcül hataya yol açar ve dosya hiç çalışmaz. Sonuç zaten yeni bir değer olduğu için & işaretini silmek yeterli.",
            "PHP compiles functions such as func_get_args(), strlen() and count() specially, so assigning their result by reference is a fatal error and the file never runs. The result is a fresh value anyway, so dropping the & is enough.",
        ),
    };
    pub const UNSET_THIS: PatternRule = PatternRule {
        id: "removed.unset_this",
        title: Text::new("unset($this)", "unset($this)"),
        severity: Severity::Breaking,
        since: (7, 1),
        replacement: tx("çağrıyı silin", "remove the call"),
        fix: FixKind::Manual,
        note: Text::new(
            "$this'i unset etmek PHP 7.1'den beri ölümcül hata verir ve dosya derlenmez.",
            "Unsetting $this has been a fatal error since PHP 7.1, so the file doesn't compile.",
        ),
    };
    pub const BREAK_OUTSIDE_LOOP: PatternRule = PatternRule {
        id: "removed.break_outside_loop",
        title: Text::new("Döngü dışında break/continue", "break/continue outside a loop"),
        severity: Severity::Breaking,
        since: (7, 0),
        replacement: tx("return ya da koşul", "return or a condition"),
        fix: FixKind::Manual,
        note: Text::new(
            "Döngü ya da switch dışındaki break ve continue PHP 7.0'dan beri ölümcül hata verir ve dosya derlenmez.",
            "break and continue outside a loop or switch have been a fatal error since PHP 7.0, so the file doesn't compile.",
        ),
    };
    pub const INCOMPATIBLE_OVERRIDE: PatternRule = PatternRule {
        id: "removed.incompatible_override",
        title: Text::new("Yerleşik metotla uyumsuz imza", "Signature incompatible with a built-in method"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: None,
        fix: FixKind::Manual,
        note: Text::new(
            "Bu metot yerleşik bir sınıfın ya da arayüzün metodunu eziyor ama imzası PHP 8'deki hâliyle uyumlu değil: parametre sayısı, tipleri, referansla geçiş, static olması ya da dönüş tipi farklı. PHP sınıfı yüklerken ölümcül hata verir. İmzayı aşağıdakine uyacak şekilde düzenleyin.",
            "This method overrides a built-in class or interface method, but its signature doesn't match the PHP 8 one: the parameter count, types, by-reference passing, static modifier or return type differ. PHP throws a fatal error when it loads the class. Make the signature compatible with the one below.",
        ),
    };
    pub const INCOMPATIBLE_USER_OVERRIDE: PatternRule = PatternRule {
        id: "removed.incompatible_user_override",
        title: Text::new("Üst sınıfla uyumsuz imza", "Signature incompatible with the parent method"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: None,
        fix: FixKind::Manual,
        note: Text::new(
            "Bu metot projedeki bir üst sınıfın, trait'in ya da arayüzün metodunu eziyor ama imzası onunla uyumlu değil: parametre sayısı, tipleri, referansla geçiş, static olması ya da dönüş tipi farklı. PHP 7 üst sınıf metotlarında bunu yalnızca uyarıyla geçiyordu (arayüz ve soyut metotlarda zaten hataydı); PHP 8 her durumda sınıfı yüklerken ölümcül hata verir. İmzayı aşağıdakine uyacak şekilde düzenleyin.",
            "This method overrides a method of a parent class, trait or interface in the project, but its signature doesn't match it: the parameter count, types, by-reference passing, static modifier or return type differ. PHP 7 only warned for parent class methods (interface and abstract methods were already fatal); PHP 8 throws a fatal error in every case when it loads the class. Make the signature compatible with the one below.",
        ),
    };
    pub const HTTP_RESPONSE_HEADER: PatternRule = PatternRule {
        id: "deprecated.http_response_header",
        title: Text::new("$http_response_header", "$http_response_header"),
        severity: Severity::Deprecated,
        since: (8, 5),
        replacement: code("http_get_last_response_headers()"),
        fix: FixKind::Manual,
        note: Text::new(
            "Kendiliğinden oluşan $http_response_header değişkeni 8.5'te kullanımdan kalktı.",
            "The magic $http_response_header variable is deprecated in 8.5.",
        ),
    };
    pub const IMPLODE_ORDER: PatternRule = PatternRule {
        id: "removed.implode_order",
        title: Text::new("Ters sıralı implode()", "implode() with reversed arguments"),
        severity: Severity::Breaking,
        since: (8, 0),
        replacement: code("implode(', ', $items)"),
        fix: FixKind::Likely,
        note: Text::new(
            "implode()'a önce diziyi sonra ayracı vermek PHP 8.0'da TypeError fırlatır.",
            "Passing the array before the separator to implode() throws a TypeError in PHP 8.0.",
        ),
    };
    pub const EMPTY_CALL: PatternRule = PatternRule {
        id: "deprecated.implicit_argument",
        title: Text::new("Parametresiz çağrı", "Call without its argument"),
        severity: Severity::Deprecated,
        since: (8, 1),
        replacement: tx("argümanı açıkça verin", "pass the argument explicitly"),
        fix: FixKind::Manual,
        note: Text::new(
            "Bu fonksiyonu varsayılan değere güvenerek parametresiz çağırmak kullanımdan kalktı.",
            "Calling this function without its argument, relying on the implicit default, is deprecated.",
        ),
    };
    pub const SESSION_HANDLER_ARGS: PatternRule = PatternRule {
        id: "deprecated.session_set_save_handler",
        title: Text::new("Fonksiyonlarla session_set_save_handler()", "session_set_save_handler() with callables"),
        severity: Severity::Deprecated,
        since: (8, 4),
        replacement: code("session_set_save_handler(new MyHandler())"),
        fix: FixKind::Manual,
        note: Text::new(
            "session_set_save_handler()'a ayrı fonksiyonlar vermek 8.4'te kullanımdan kalktı; SessionHandlerInterface uygulayan bir nesne verin.",
            "Passing separate callables to session_set_save_handler() is deprecated in 8.4; pass an object implementing SessionHandlerInterface.",
        ),
    };
    pub const INI_SETTING: PatternRule = PatternRule {
        id: "deprecated.ini_setting",
        title: Text::new("Kullanımdan kalkan ini ayarı", "Deprecated ini setting"),
        severity: Severity::Deprecated,
        since: (8, 0),
        replacement: tx("ayarı silin", "remove the setting"),
        fix: FixKind::Manual,
        note: Text::new(
            "Bu ini ayarı yeni PHP sürümlerinde kaldırıldı ya da kullanımdan kalktı.",
            "This ini setting was removed or deprecated in newer PHP versions.",
        ),
    };
    pub const NULL_OFFSET: PatternRule = PatternRule {
        id: "deprecated.null_offset",
        title: Text::new("Dizi anahtarı olarak null", "null as an array key"),
        severity: Severity::Deprecated,
        since: (8, 5),
        replacement: code("$a['']"),
        fix: FixKind::Manual,
        note: Text::new(
            "null'ı dizi anahtarı olarak kullanmak 8.5'te kullanımdan kalktı; null zaten boş metne dönüşüyordu.",
            "Using null as an array key is deprecated in 8.5; it was already turned into an empty string.",
        ),
    };
    pub const CASE_SEMICOLON: PatternRule = PatternRule {
        id: "deprecated.case_semicolon",
        title: Text::new("Noktalı virgülle biten case", "case ending with a semicolon"),
        severity: Severity::Deprecated,
        since: (8, 5),
        replacement: code("case 1:"),
        fix: FixKind::Auto,
        note: Text::new(
            "switch içinde case'i iki nokta yerine noktalı virgülle bitirmek 8.5'te kullanımdan kalktı.",
            "Ending a switch case with a semicolon instead of a colon is deprecated in 8.5.",
        ),
    };
    pub const CONCAT_PRECEDENCE: PatternRule = PatternRule {
        id: "behavior.concat_precedence",
        title: Text::new("Parantezsiz . ile + / -", "Unparenthesized . with + or -"),
        severity: Severity::Behavior,
        since: (8, 0),
        replacement: code("\"Toplam: \" . ($a + $b)"),
        fix: FixKind::Manual,
        note: Text::new(
            "PHP 8.0'da + ve - artık . işleminden önce yapılıyor; \"Toplam: \" . $a + $b artık farklı sonuç verir. Niyetinizi parantezle belirtin.",
            "Since PHP 8.0, + and - are evaluated before .; \"Total: \" . $a + $b now gives a different result. Add parentheses to say what you mean.",
        ),
    };
    pub const SERIALIZABLE: PatternRule = PatternRule {
        id: "deprecated.serializable",
        title: Text::new("__serialize olmadan Serializable", "Serializable without __serialize"),
        severity: Severity::Deprecated,
        since: (8, 1),
        replacement: code("__serialize() / __unserialize()"),
        fix: FixKind::Manual,
        note: Text::new(
            "Serializable arayüzünü __serialize() ve __unserialize() olmadan uygulamak 8.1'de kullanımdan kalktı.",
            "Implementing Serializable without __serialize() and __unserialize() is deprecated in 8.1.",
        ),
    };
    pub const REMOVED_VARIABLE: PatternRule = PatternRule {
        id: "removed.magic_variable",
        title: Text::new("Kaldırılan otomatik değişken", "Removed magic variable"),
        severity: Severity::Breaking,
        since: (7, 0),
        replacement: tx("error_get_last() ya da file_get_contents('php://input')", "error_get_last() or file_get_contents('php://input')"),
        fix: FixKind::Manual,
        note: Text::new(
            "$HTTP_RAW_POST_DATA ve $php_errormsg artık oluşturulmuyor.",
            "$HTTP_RAW_POST_DATA and $php_errormsg are no longer created.",
        ),
    };
    pub const DYNAMIC_PROPERTY: PatternRule = PatternRule {
        id: "deprecated.dynamic_property",
        title: Text::new("Dinamik property", "Dynamic property"),
        severity: Severity::Deprecated,
        since: (8, 2),
        replacement: tx("property'yi sınıfta tanımlayın", "declare the property on the class"),
        fix: FixKind::Manual,
        note: Text::new(
            "Sınıfta tanımlanmamış bir property'ye değer atamak 8.2'de kullanımdan kalktı. Property'yi tanımlayın ya da sınıfa #[\\AllowDynamicProperties] ekleyin.",
            "Assigning to a property that isn't declared on the class is deprecated in 8.2. Declare it, or add #[\\AllowDynamicProperties] to the class.",
        ),
    };
    pub const RESERVED_NAME: PatternRule = PatternRule {
        id: "removed.reserved_name",
        title: Text::new("Ayrılmış ad", "Reserved name"),
        severity: Severity::Breaking,
        since: (7, 0),
        replacement: tx("sınıfı yeniden adlandırın", "rename the class"),
        fix: FixKind::Manual,
        note: Text::new(
            "Bu ad yeni PHP sürümlerinde ayrılmış bir kelime; sınıf ya da fonksiyon adı olarak kullanılamaz.",
            "This name is reserved in newer PHP versions and can't be used as a class or function name.",
        ),
    };
    pub const OPTIONAL_BEFORE_REQUIRED: PatternRule = PatternRule {
        id: "deprecated.optional_before_required",
        title: Text::new("Zorunludan önce isteğe bağlı parametre", "Optional parameter before a required one"),
        severity: Severity::Deprecated,
        since: (8, 0),
        replacement: tx("varsayılan değeri kaldırın ya da parametreyi sona taşıyın", "drop the default or move the parameter last"),
        fix: FixKind::Likely,
        note: Text::new(
            "Zorunlu bir parametreden önce gelen varsayılan değer hiçbir zaman kullanılamaz; PHP 8.0 bunu kullanımdan kaldırdı ve parametreyi zorunlu sayar.",
            "A default before a required parameter can never be used; PHP 8.0 deprecates it and treats the parameter as required.",
        ),
    };
    pub const NULL_TO_INTERNAL: PatternRule = PatternRule {
        id: "deprecated.null_to_internal",
        title: Text::new("Dahili fonksiyona null", "null passed to a built-in function"),
        severity: Severity::Deprecated,
        since: (8, 1),
        replacement: tx("'' ya da uygun bir değer", "'' or a suitable value"),
        fix: FixKind::Manual,
        note: Text::new(
            "Dahili fonksiyonların nullable olmayan parametrelerine null vermek 8.1'de kullanımdan kalktı ve PHP 9'da TypeError olacak.",
            "Passing null to non-nullable parameters of built-in functions is deprecated in 8.1 and will be a TypeError in PHP 9.",
        ),
    };
    pub const PARTIAL_CALLABLE: PatternRule = PatternRule {
        id: "deprecated.partial_callable",
        title: Text::new("Kısmen desteklenen callable", "Partially supported callable"),
        severity: Severity::Deprecated,
        since: (8, 2),
        replacement: code("[self::class, 'm'] / self::m(...)"),
        fix: FixKind::Manual,
        note: Text::new(
            "\"self::m\", \"parent::m\", \"static::m\" ve ['self', 'm'] gibi callable'lar call_user_func() ile çalışır ama $callable() ile çalışmaz; 8.2'de kullanımdan kalktı.",
            "Callables like \"self::m\", \"parent::m\", \"static::m\" and ['self', 'm'] work with call_user_func() but not $callable(); they are deprecated in 8.2.",
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
