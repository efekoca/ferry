//! Ferry's own fixes for findings Rector doesn't handle. Each one rewrites only the tokens of a
//! finding it recognises, so a finding counts as automatic exactly when one of these takes it.

use crate::analyzer::Finding;
use crate::lexer::{Token, TokenKind};
use crate::rules::FixKind;
use crate::version::PhpVersion;
use std::collections::HashSet;

pub const CONSTANT_VALUE: &str = "Ferry\\ConstantToValue";
pub const CONSTANT_RENAME: &str = "Ferry\\RenameConstant";
pub const OCI_ALIAS: &str = "Ferry\\Php80\\RenameOciAlias";
pub const NO_OP_CALL: &str = "Ferry\\RemoveNoOpCall";
pub const NO_OP_FLAG: &str = "Ferry\\DropNoOpFlag";
pub const UNSET_THIS: &str = "Ferry\\Php71\\RemoveUnsetThis";
pub const MHASH: &str = "Ferry\\Php81\\MhashToHash";
pub const KILL: &str = "Ferry\\Php84\\KillWithQuery";
pub const REFRESH: &str = "Ferry\\Php84\\RefreshWithFlush";
pub const INTL_METHODS: &str = "Ferry\\Php84\\IntlCalendarMethods";
pub const LCG_VALUE: &str = "Ferry\\Php84\\LcgValueToRandomizer";
pub const MAGIC_QUOTES: &str = "Ferry\\Php80\\MagicQuotesOff";
pub const UNUSED_ARGUMENT: &str = "Ferry\\Php84\\RemoveUnusedArgument";
pub const FUNCTION_RENAME: &str = "Ferry\\RenameFunction";
pub const REFLECTION_EXPORT: &str = "Ferry\\Php80\\ReflectionExportToString";
pub const LDAP_PAGING: &str = "Ferry\\Php80\\LdapPagedControl";
pub const ASSERT_DEFAULT: &str = "Ferry\\Php83\\RemoveDefaultAssertOption";
pub const ZIP_LEVEL: &str = "Ferry\\Php83\\DropRecompressLevel";
pub const GETSS: &str = "Ferry\\Php80\\StripTagsAfterRead";
pub const RECODE: &str = "Ferry\\Php74\\RecodeToIconv";
pub const EZMLM: &str = "Ferry\\Php80\\InlineEzmlmHash";
pub const REGCASE: &str = "Ferry\\Php70\\SqlRegcaseToCallback";
pub const NULL_ARGUMENT: &str = "Ferry\\Php81\\NullToEmptyArgument";
pub const WILL_CHANGE: &str = "Ferry\\Php81\\AddReturnTypeWillChange";
pub const AI_IDN_FLAG: &str = "Ferry\\Php80\\DropAiIdnFlag";
pub const EMBEDDED_SERVER: &str = "Ferry\\Php74\\RemoveEmbeddedServerCall";
pub const BREAK_RETURN: &str = "Ferry\\Php70\\StrayBreakToReturn";
pub const IDNA_UTS46: &str = "Ferry\\Php80\\IdnaVariantUts46";
pub const SUBSTR_EMPTY: &str = "Ferry\\Php70\\SubstrFalseToEmptyString";
pub const VARIABLE_ORDER: &str = "Ferry\\Php70\\KeepVariableVariableOrder";
pub const VARIABLE_BRACES: &str = "Ferry\\Php70\\BraceVariableVariable";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub class: &'static str,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// (rule, constant, value): constants PHP never read or no longer has, whose value keeps the code the same.
const CONSTANT_VALUES: &[(&str, &str, &str)] = &[
    ("deprecated.dom_php_err", "DOM_PHP_ERR", "0"),
    ("deprecated.mysqli_data_truncated", "MYSQLI_DATA_TRUNCATED", "101"),
    ("deprecated.mysqli_is_mariadb", "MYSQLI_IS_MARIADB", "false"),
    ("deprecated.mysqli_no_data", "MYSQLI_NO_DATA", "100"),
    ("deprecated.mysqli_server_ps_out_params", "MYSQLI_SERVER_PS_OUT_PARAMS", "4096"),
    ("deprecated.mysqli_server_query", "MYSQLI_SERVER_QUERY_NO_GOOD_INDEX_USED", "16"),
    ("deprecated.mysqli_server_query", "MYSQLI_SERVER_QUERY_NO_INDEX_USED", "32"),
    ("deprecated.mysqli_server_query", "MYSQLI_SERVER_QUERY_WAS_SLOW", "2048"),
    ("deprecated.mysqli_update_max_length", "MYSQLI_STMT_ATTR_UPDATE_MAX_LENGTH", "0"),
    ("removed.mb_overload", "MB_OVERLOAD_MAIL", "1"),
    ("removed.mb_overload", "MB_OVERLOAD_REGEX", "4"),
    ("removed.mb_overload", "MB_OVERLOAD_STRING", "2"),
];

/// (rule, old name, new name): the new constant holds the same value, or does the same job.
const RENAMED_CONSTANTS: &[(&str, &str, &str)] = &[
    ("deprecated.pgsql_libpq_version_str", "PGSQL_LIBPQ_VERSION_STR", "PGSQL_LIBPQ_VERSION"),
    ("deprecated.u_multiple_decimal_seperators", "U_MULTIPLE_DECIMAL_SEPERATORS", "U_MULTIPLE_DECIMAL_SEPARATORS"),
    ("removed.filter_sanitize_magic_quotes", "FILTER_SANITIZE_MAGIC_QUOTES", "FILTER_SANITIZE_ADD_SLASHES"),
];

const TRANSACTION_STATES: [(&str, &str); 5] = [("IDLE", "0"), ("ACTIVE", "1"), ("INTRANS", "2"), ("INERROR", "3"), ("UNKNOWN", "4")];

/// The @alias tags of the oci8 stub: each old name is the same C function as the new one.
/// ocifetchinto() has no alias and stays manual.
const OCI_ALIASES: [(&str, &str); 45] = [
    ("ocibindbyname", "oci_bind_by_name"),
    ("ocicancel", "oci_cancel"),
    ("ocicollappend", "oci_collection_append"),
    ("ocicollassignelem", "oci_collection_element_assign"),
    ("ocicollgetelem", "oci_collection_element_get"),
    ("ocicollmax", "oci_collection_max"),
    ("ocicollsize", "oci_collection_size"),
    ("ocicolltrim", "oci_collection_trim"),
    ("ocicolumnisnull", "oci_field_is_null"),
    ("ocicolumnname", "oci_field_name"),
    ("ocicolumnprecision", "oci_field_precision"),
    ("ocicolumnscale", "oci_field_scale"),
    ("ocicolumnsize", "oci_field_size"),
    ("ocicolumntype", "oci_field_type"),
    ("ocicolumntyperaw", "oci_field_type_raw"),
    ("ocicommit", "oci_commit"),
    ("ocidefinebyname", "oci_define_by_name"),
    ("ocierror", "oci_error"),
    ("ociexecute", "oci_execute"),
    ("ocifetch", "oci_fetch"),
    ("ocifetchstatement", "oci_fetch_all"),
    ("ocifreecollection", "oci_free_collection"),
    ("ocifreecursor", "oci_free_statement"),
    ("ocifreedesc", "oci_free_descriptor"),
    ("ocifreestatement", "oci_free_statement"),
    ("ociloadlob", "oci_lob_load"),
    ("ocilogoff", "oci_close"),
    ("ocilogon", "oci_connect"),
    ("ocinewcollection", "oci_new_collection"),
    ("ocinewcursor", "oci_new_cursor"),
    ("ocinewdescriptor", "oci_new_descriptor"),
    ("ocinlogon", "oci_new_connect"),
    ("ocinumcols", "oci_num_fields"),
    ("ociparse", "oci_parse"),
    ("ocipasswordchange", "oci_password_change"),
    ("ociplogon", "oci_pconnect"),
    ("ociresult", "oci_result"),
    ("ocirollback", "oci_rollback"),
    ("ocirowcount", "oci_num_rows"),
    ("ocisavelob", "oci_lob_save"),
    ("ocisavelobfile", "oci_lob_import"),
    ("ociserverversion", "oci_server_version"),
    ("ocisetprefetch", "oci_set_prefetch"),
    ("ocistatementtype", "oci_statement_type"),
    ("ociwritelobtofile", "oci_lob_export"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Args {
    Plain,
    PlainOrLiteral,
    Falsy,
}

/// (rule, function, its arguments): calls that do nothing on the versions that report them. They go
/// only when they stand alone as a statement and their arguments can't run anything.
const NO_OP_CALLS: &[(&str, &str, Args)] = &[
    ("deprecated.openssl_free_key", "openssl_free_key", Args::Plain),
    ("deprecated.openssl_pkey_free", "openssl_pkey_free", Args::Plain),
    ("deprecated.openssl_x509_free", "openssl_x509_free", Args::Plain),
    ("deprecated.shmop_close", "shmop_close", Args::Plain),
    ("removed.magic_quotes_runtime", "magic_quotes_runtime", Args::Falsy),
    ("removed.oci_internal_debug", "oci_internal_debug", Args::PlainOrLiteral),
    ("removed.oci_internal_debug", "ociinternaldebug", Args::PlainOrLiteral),
    ("removed.set_magic_quotes_runtime", "set_magic_quotes_runtime", Args::Falsy),
];

/// What a flag becomes where it is the whole value.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Alone {
    /// Its literal value, which is exact anywhere.
    Value(&'static str),
    /// The argument goes, or `'flags' => 0` in an options array.
    Argument,
    /// Another constant, only where it is a value being passed or stored.
    Constant(&'static str),
}

/// (rule, constant, lone use): flags that change nothing, dropped from `|` masks.
const NO_OP_FLAGS: &[(&str, &str, Alone)] = &[
    ("deprecated.curlpipe_http1", "CURLPIPE_HTTP1", Alone::Constant("CURLPIPE_NOTHING")),
    ("deprecated.e_strict", "E_STRICT", Alone::Value("2048")),
    ("deprecated.file_binary", "FILE_BINARY", Alone::Value("0")),
    ("deprecated.file_text", "FILE_TEXT", Alone::Value("0")),
    ("removed.filter_flag_host_required", "FILTER_FLAG_HOST_REQUIRED", Alone::Argument),
    ("removed.filter_flag_scheme_required", "FILTER_FLAG_SCHEME_REQUIRED", Alone::Argument),
    ("removed.ai_idn_constant", "AI_IDN_ALLOW_UNASSIGNED", Alone::Constant("0")),
    ("removed.ai_idn_constant", "AI_IDN_USE_STD3_ASCII_RULES", Alone::Constant("0")),
];

/// Rules whose fix reads the code around the finding, and for three of them the version the project
/// runs on today.
const SITE_RULES: [&str; 8] = [
    "deprecated.null_to_internal",
    "deprecated.tentative_return_type",
    "removed.mysqli_embedded_server",
    "removed.break_outside_loop",
    "removed.intl_idna_variant_2003",
    "behavior.substr_false",
    "behavior.indirect_access_order",
    "review.variable_variable",
];

/// (MHASH_ name, value, hash() algorithm, has an HMAC, mhash_get_hash_name(), mhash_get_block_size()),
/// read from PHP 8.5: mhash() output equals hash() for each, and keyed output equals hash_hmac().
const MHASH_ALGOS: [(&str, u32, &str, bool, &str, u32); 39] = [
    ("CRC32", 0, "crc32", false, "CRC32", 4),
    ("MD5", 1, "md5", true, "MD5", 16),
    ("SHA1", 2, "sha1", true, "SHA1", 20),
    ("HAVAL256", 3, "haval256,3", true, "HAVAL256", 32),
    ("RIPEMD160", 5, "ripemd160", true, "RIPEMD160", 20),
    ("TIGER", 7, "tiger192,3", true, "TIGER", 24),
    ("GOST", 8, "gost", true, "GOST", 32),
    ("CRC32B", 9, "crc32b", false, "CRC32B", 4),
    ("HAVAL224", 10, "haval224,3", true, "HAVAL224", 28),
    ("HAVAL192", 11, "haval192,3", true, "HAVAL192", 24),
    ("HAVAL160", 12, "haval160,3", true, "HAVAL160", 20),
    ("HAVAL128", 13, "haval128,3", true, "HAVAL128", 16),
    ("TIGER128", 14, "tiger128,3", true, "TIGER128", 16),
    ("TIGER160", 15, "tiger160,3", true, "TIGER160", 20),
    ("MD4", 16, "md4", true, "MD4", 16),
    ("SHA256", 17, "sha256", true, "SHA256", 32),
    ("ADLER32", 18, "adler32", false, "ADLER32", 4),
    ("SHA224", 19, "sha224", true, "SHA224", 28),
    ("SHA512", 20, "sha512", true, "SHA512", 64),
    ("SHA384", 21, "sha384", true, "SHA384", 48),
    ("WHIRLPOOL", 22, "whirlpool", true, "WHIRLPOOL", 64),
    ("RIPEMD128", 23, "ripemd128", true, "RIPEMD128", 16),
    ("RIPEMD256", 24, "ripemd256", true, "RIPEMD256", 32),
    ("RIPEMD320", 25, "ripemd320", true, "RIPEMD320", 40),
    ("SNEFRU256", 27, "snefru", true, "SNEFRU256", 32),
    ("MD2", 28, "md2", true, "MD2", 16),
    ("FNV132", 29, "fnv132", false, "FNV132", 4),
    ("FNV1A32", 30, "fnv1a32", false, "FNV1A32", 4),
    ("FNV164", 31, "fnv164", false, "FNV164", 8),
    ("FNV1A64", 32, "fnv1a64", false, "FNV1A64", 8),
    ("JOAAT", 33, "joaat", false, "JOAAT", 4),
    ("CRC32C", 34, "crc32c", false, "CRC32C", 4),
    ("MURMUR3A", 35, "murmur3a", false, "MURMUR3A", 4),
    ("MURMUR3C", 36, "murmur3c", false, "MURMUR3C", 16),
    ("MURMUR3F", 37, "murmur3f", false, "MURMUR3F", 16),
    ("XXH32", 38, "xxh32", false, "XXH32", 4),
    ("XXH64", 39, "xxh64", false, "XXH64", 8),
    ("XXH3", 40, "xxh3", false, "XXH3", 8),
    ("XXH128", 41, "xxh128", false, "XXH128", 16),
];

/// MYSQLI_REFRESH_* flags and the FLUSH option that does the same. The rest stay manual.
const FLUSH_OPTIONS: [(&str, &str); 5] =
    [("GRANT", "PRIVILEGES"), ("LOG", "LOGS"), ("TABLES", "TABLES"), ("HOSTS", "HOSTS"), ("STATUS", "STATUS")];

const REFRESH_VALUES: [(&str, &str); 10] = [
    ("GRANT", "1"),
    ("LOG", "2"),
    ("TABLES", "4"),
    ("HOSTS", "8"),
    ("STATUS", "16"),
    ("THREADS", "32"),
    ("REPLICA", "64"),
    ("SLAVE", "64"),
    ("MASTER", "128"),
    ("BACKUP_LOG", "2097152"),
];

/// Reflection classes with export(), and how many arguments come before its `$return` flag.
const REFLECTION_EXPORTS: [(&str, usize); 9] = [
    ("reflectionclass", 1),
    ("reflectionobject", 1),
    ("reflectionfunction", 1),
    ("reflectionextension", 1),
    ("reflectionzendextension", 1),
    ("reflectionmethod", 2),
    ("reflectionproperty", 2),
    ("reflectionparameter", 2),
    ("reflectionclassconstant", 2),
];

/// (ASSERT_ name, value on PHP 8, literals that set its default).
const ASSERT_SETTINGS: [(&str, &str, &[&str]); 5] = [
    ("ACTIVE", "1", &["1", "true", "'1'", "\"1\"", "'on'", "\"on\""]),
    ("CALLBACK", "2", &["null", "''", "\"\""]),
    ("BAIL", "3", &["0", "false", "'0'", "\"0\"", "'off'", "\"off\""]),
    ("WARNING", "4", &["1", "true", "'1'", "\"1\"", "'on'", "\"on\""]),
    ("EXCEPTION", "5", &["1", "true", "'1'", "\"1\"", "'on'", "\"on\""]),
];

/// recode charset names and their iconv names, for the pairs compared against PHP 5.6.
const RECODE_CHARSETS: [(&str, &str); 8] = [
    ("utf8", "UTF-8"),
    ("utf-8", "UTF-8"),
    ("latin1", "ISO-8859-1"),
    ("iso-8859-1", "ISO-8859-1"),
    ("iso8859-1", "ISO-8859-1"),
    ("latin5", "ISO-8859-9"),
    ("iso-8859-9", "ISO-8859-9"),
    ("iso8859-9", "ISO-8859-9"),
];

const EZMLM_PORT: &str = "(static function (string $address): int { $hash = 5381; for ($i = 0, $n = strlen($address); $i < $n; $i++) { $hash = (($hash + ($hash << 5)) ^ ord(strtolower($address[$i]))) & 0xFFFFFFFF; } return $hash % 53; })";

const CALL_RULES: [&str; 27] = [
    "deprecated.mhash",
    "deprecated.mhash_constant",
    "deprecated.mysqli_kill",
    "deprecated.mysqli_refresh",
    "deprecated.mysqli_refresh_constant",
    "deprecated.intlcal_set",
    "deprecated.intlgregcal_create_instance",
    "deprecated.lcg_value",
    "removed.get_magic_quotes_gpc",
    "removed.get_magic_quotes_runtime",
    "deprecated.pdo_fetch_serialize",
    "deprecated.mysqli_store_result_mode",
    "deprecated.mysqli_store_result_copy_data",
    "removed.datefmt_set_timezone_id",
    "removed.reflection_export",
    "removed.ldap_control_paged_result",
    "removed.ldap_control_paged_result_response",
    "deprecated.assert_options",
    "deprecated.assert_constants",
    "removed.assert_quiet_eval",
    "deprecated.zip_fl_recompress",
    "removed.fgetss",
    "removed.gzgetss",
    "removed.recode",
    "removed.ezmlm_hash",
    "removed.sql_regcase",
    "deprecated.curlopt_binarytransfer",
];

fn handles(rule: &str) -> bool {
    CALL_RULES.contains(&rule)
        || rule == "deprecated.pgsql_transaction_constant"
        || rule == "deprecated.oci8_alias"
        || rule == "deprecated.curlopt_binarytransfer"
        || rule == "removed.unset_this"
        || CONSTANT_VALUES.iter().chain(RENAMED_CONSTANTS).any(|(r, ..)| *r == rule)
        || NO_OP_CALLS.iter().any(|(r, ..)| *r == rule)
        || NO_OP_FLAGS.iter().any(|(r, ..)| *r == rule)
        || SITE_RULES.contains(&rule)
}

/// Whether the text names anything a fix could take, checked before the file is analyzed again.
pub(crate) fn mentions(src: &str) -> bool {
    let names =
        CONSTANT_VALUES.iter().chain(RENAMED_CONSTANTS).map(|(_, name, _)| *name).chain(NO_OP_FLAGS.iter().map(|(_, name, _)| *name));
    if names.chain(["TRANSACTION_", "CURLOPT_BINARYTRANSFER"]).any(|name| src.contains(name)) {
        return true;
    }
    if ["MHASH_", "MYSQLI_REFRESH_", "MYSQLI_STORE_RESULT_COPY_DATA", "FETCH_SERIALIZE", "INTL_IDNA_VARIANT_2003", "$$"]
        .iter()
        .any(|name| src.contains(name))
    {
        return true;
    }
    let lower = src.to_ascii_lowercase();
    let calls = [
        "mhash",
        "kill",
        "refresh",
        "intlcal_set",
        "intlgregcal_create_instance",
        "lcg_value",
        "get_magic_quotes",
        "store_result",
        "settimezoneid",
        "ldap_control_paged_result",
        "assert_options",
        "assert_",
        "fl_recompress",
        "fgetss",
        "gzgetss",
        "recode",
        "ezmlm_hash",
        "sql_regcase",
        "mysqli_embedded_server",
        "substr",
        "null",
        "break",
        "continue",
        "extends",
        "implements",
    ];
    OCI_ALIASES.iter().map(|(old, _)| *old).chain(NO_OP_CALLS.iter().map(|(_, f, _)| *f)).chain(calls).any(|name| lower.contains(name))
        || (lower.contains("unset") && src.contains("$this"))
        || (lower.contains("export") && lower.contains("reflection"))
}

/// A file as the fixes see it: its text, its tokens, which tokens hold a no-op flag finding, the
/// version the project runs on today and whether the file declares strict types.
pub(crate) struct Source<'a> {
    pub(crate) src: &'a str,
    pub(crate) tokens: &'a [Token],
    pub(crate) flagged: HashSet<usize>,
    current: Option<PhpVersion>,
    strict: bool,
}

impl<'a> Source<'a> {
    fn new(
        src: &'a str,
        tokens: &'a [Token],
        findings: &[Finding],
        at_position: &std::collections::HashMap<(usize, usize), usize>,
        current: Option<PhpVersion>,
    ) -> Self {
        let flagged = findings
            .iter()
            .filter(|f| NO_OP_FLAGS.iter().any(|(r, ..)| *r == f.rule_id))
            .filter_map(|f| at_position.get(&(f.line, f.column)).copied())
            .collect();
        let strict = tokens.windows(3).any(|w| w[0].is_ident_ci("strict_types") && w[1].is_punct("=") && w[2].text == "1");
        Source { src, tokens, flagged, current, strict }
    }

    fn text(&self, first: usize, last: usize) -> &'a str {
        &self.src[self.tokens[first].start..self.tokens[last].end]
    }

    fn between(&self, before: usize, after: usize) -> &'a str {
        &self.src[self.tokens[before].end..self.tokens[after].start]
    }
}

/// The fix for the finding of `rule` at token `k`, if Ferry has one for it.
pub(crate) fn at(file: &Source, k: usize, rule: &str) -> Option<Fix> {
    let tokens = file.tokens;
    if CALL_RULES.contains(&rule) {
        return call_fix(file, k, rule);
    }
    if let Some(fix) = statement_fix(file, k, rule) {
        return Some(fix);
    }
    if NO_OP_FLAGS.iter().any(|(r, ..)| *r == rule) {
        return flag_fix(file, k, rule);
    }
    if SITE_RULES.contains(&rule) {
        return site_fix(file, k, rule);
    }
    let tok = tokens.get(k).filter(|t| t.kind == TokenKind::Ident)?;
    let bare = tok.text.trim_start_matches('\\');
    let lead = &tok.text[..tok.text.len() - bare.len()];
    let fix = |class, start, text: String| Some(Fix { class, start, end: tok.end, text });
    if rule == "deprecated.pgsql_transaction_constant" {
        let class = tokens.get(k.checked_sub(2)?).filter(|c| c.kind == TokenKind::Ident && tokens[k - 1].is_punct("::"))?;
        let short = class.text.rsplit('\\').next().unwrap_or_default();
        let upper = bare.to_ascii_uppercase();
        let state = match short.to_ascii_lowercase().as_str() {
            "pdo" => upper.strip_prefix("PGSQL_TRANSACTION_"),
            "pgsql" => upper.strip_prefix("TRANSACTION_"),
            _ => None,
        }?;
        let value = TRANSACTION_STATES.iter().find(|(s, _)| *s == state)?.1;
        return fix(CONSTANT_VALUE, class.start, value.into());
    }
    if rule == "deprecated.oci8_alias" {
        let (_, new) = OCI_ALIASES.iter().find(|(old, _)| bare.eq_ignore_ascii_case(old))?;
        let call = tokens.get(k + 1).is_some_and(|n| n.is_punct("("));
        let member = k.checked_sub(1).is_some_and(|p| tokens[p].is_punct("->") || tokens[p].is_punct("::"));
        return (call && !member).then(|| Fix { class: OCI_ALIAS, start: tok.start, end: tok.end, text: format!("{lead}{new}") });
    }
    if let Some((_, _, value)) = CONSTANT_VALUES.iter().find(|(r, name, _)| *r == rule && *name == bare) {
        return fix(CONSTANT_VALUE, tok.start, value.to_string());
    }
    let (_, _, new) = RENAMED_CONSTANTS.iter().find(|(r, name, _)| *r == rule && *name == bare)?;
    fix(CONSTANT_RENAME, tok.start, format!("{lead}{new}"))
}

fn is_atom(tokens: &[Token], k: usize) -> bool {
    let Some(t) = tokens.get(k) else { return false };
    let before = k.checked_sub(1).map(|p| &tokens[p]);
    match t.kind {
        TokenKind::Number => true,
        TokenKind::Ident => {
            !tokens.get(k + 1).is_some_and(|n| n.is_punct("(") || n.is_punct("::"))
                && !before.is_some_and(|p| p.is_punct("->") || p.is_punct("?->") || p.is_punct("::"))
        }
        _ => false,
    }
}

/// One operand of a mask: an atom, possibly negated with `~`.
#[derive(Clone, Copy)]
struct Operand {
    first: usize,
    atom: usize,
}

impl Operand {
    fn negated(&self) -> bool {
        self.first != self.atom
    }
}

fn operand_ending(tokens: &[Token], end: usize) -> Option<Operand> {
    if !is_atom(tokens, end) {
        return None;
    }
    let negated = end > 0 && tokens[end - 1].is_punct("~");
    Some(Operand { first: if negated { end - 1 } else { end }, atom: end })
}

fn operand_starting(tokens: &[Token], start: usize) -> Option<Operand> {
    let negated = tokens.get(start).is_some_and(|t| t.is_punct("~"));
    let atom = if negated { start + 1 } else { start };
    is_atom(tokens, atom).then_some(Operand { first: start, atom })
}

const LOOSE_BEFORE: &[&str] = &["(", ",", "[", "=", "=>", "?", ":", "{", ";", "&&", "||", "??", "|=", "&="];
const LOOSE_AFTER: &[&str] = &[")", ",", "]", ";", "?", ":", "}", "&&", "||", "??", "=>"];
const LOOSE_WORDS: &[&str] = &["return", "case", "echo", "print", "and", "or", "xor"];

fn loose(tok: Option<&Token>, puncts: &[&str]) -> bool {
    tok.is_none_or(|t| {
        t.kind == TokenKind::CloseTag || puncts.iter().any(|p| t.is_punct(p)) || LOOSE_WORDS.iter().any(|w| t.is_ident_ci(w))
    })
}

/// The operands and operators of the `|`/`&` mask around token `k`, built only from atoms.
fn mask(tokens: &[Token], k: usize) -> Option<(Vec<Operand>, Vec<usize>)> {
    let here = operand_ending(tokens, k)?;
    let mut operands = vec![here];
    let mut operators = Vec::new();
    while let Some(op) = operands[0].first.checked_sub(1).filter(|&o| tokens[o].is_punct("|") || tokens[o].is_punct("&")) {
        let Some(left) = op.checked_sub(1).and_then(|e| operand_ending(tokens, e)) else { break };
        operands.insert(0, left);
        operators.insert(0, op);
    }
    while let Some(op) = Some(operands.last()?.atom + 1).filter(|&o| tokens.get(o).is_some_and(|t| t.is_punct("|") || t.is_punct("&"))) {
        let Some(right) = operand_starting(tokens, op + 1) else { break };
        operands.push(right);
        operators.push(op);
    }
    Some((operands, operators))
}

fn flag(file: &Source, operand: &Operand) -> Option<Alone> {
    let name = file.tokens[operand.atom].text.trim_start_matches('\\');
    file.flagged.contains(&operand.atom).then(|| NO_OP_FLAGS.iter().find(|(_, n, _)| *n == name).map(|&(.., alone)| alone)).flatten()
}

fn enclosing_call(tokens: &[Token], k: usize) -> Option<usize> {
    let mut depth = 0usize;
    for j in (0..k).rev() {
        let t = &tokens[j];
        if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            depth += 1;
        } else if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") {
            if depth == 0 {
                return (t.is_punct("(") && j > 0 && tokens[j - 1].kind == TokenKind::Ident).then(|| j - 1);
            }
            depth -= 1;
        } else if depth == 0 && t.is_punct(";") {
            return None;
        }
    }
    None
}

fn call_named(tokens: &[Token], name: usize, names: &[&str]) -> bool {
    let bare = tokens[name].text.trim_start_matches('\\');
    names.iter().any(|n| bare.eq_ignore_ascii_case(n))
        && !name.checked_sub(1).is_some_and(|p| tokens[p].is_punct("->") || tokens[p].is_punct("::") || tokens[p].is_ident_ci("function"))
}

/// What the whole mask from `first` to `last` becomes once its flags are gone.
fn emptied(file: &Source, first: usize, last: usize, alone: Alone, class: &'static str) -> Option<Fix> {
    let tokens = file.tokens;
    let before = first.checked_sub(1).map(|p| &tokens[p]);
    let after = tokens.get(last + 1);
    let replace = |text: &str| Some(Fix { class, start: tokens[first].start, end: tokens[last].end, text: text.to_string() });
    match alone {
        Alone::Value(_) if tokens[first..=last].iter().any(|t| t.is_ident_ci("E_STRICT")) => None,
        Alone::Value(value) => replace(value),
        Alone::Constant(name) => {
            let stored = before.is_some_and(|b| [",", "=>", "=", "("].iter().any(|p| b.is_punct(p)));
            let ends = after.is_some_and(|a| [")", ",", "]", ";"].iter().any(|p| a.is_punct(p)));
            (stored && ends).then(|| Fix { class, start: tokens[first].start, end: tokens[last].end, text: name.to_string() })
        }
        Alone::Argument => {
            let keyed = before.is_some_and(|b| b.is_punct("=>"))
                && first >= 2
                && matches!(tokens[first - 2].kind, TokenKind::Str { interpolates: false, .. })
                && tokens[first - 2].text.trim_matches(|c| c == '\'' || c == '"') == "flags"
                && after.is_some_and(|a| a.is_punct(",") || a.is_punct("]") || a.is_punct(")"));
            if keyed {
                return replace("0");
            }
            let call = enclosing_call(tokens, first)?;
            let last_argument = before.is_some_and(|b| b.is_punct(",")) && after.is_some_and(|a| a.is_punct(")"));
            let position = if call_named(tokens, call, &["filter_var"]) {
                2
            } else if call_named(tokens, call, &["filter_input"]) {
                3
            } else {
                return None;
            };
            let commas = (call + 2..first).filter(|&j| tokens[j].is_punct(",") && enclosing_call(tokens, j) == Some(call)).count();
            (last_argument && commas == position).then(|| Fix {
                class,
                start: tokens[first - 2].end,
                end: tokens[last].end,
                text: String::new(),
            })
        }
    }
}

/// Drops flags that change nothing from the mask around `k`. Lone flags take their value or go.
fn flag_fix(file: &Source, k: usize, rule: &str) -> Option<Fix> {
    let tokens = file.tokens;
    let (operands, operators) = mask(tokens, k)?;
    let (first, last) = (operands[0].first, operands.last()?.atom);
    let class = if rule == "removed.ai_idn_constant" { AI_IDN_FLAG } else { NO_OP_FLAG };
    if operands.len() == 1 {
        let alone = flag(file, &operands[0])?;
        if let Alone::Value(value) = alone {
            let error_level = enclosing_call(tokens, first).is_some_and(|c| call_named(tokens, c, &["error_reporting", "ini_set"]));
            if tokens[k].text.trim_start_matches('\\') == "E_STRICT" && error_level {
                return None;
            }
            return Some(Fix {
                class: CONSTANT_VALUE,
                start: tokens[operands[0].atom].start,
                end: tokens[operands[0].atom].end,
                text: value.to_string(),
            });
        }
        return if operands[0].negated() { None } else { emptied(file, first, last, alone, class) };
    }
    if !loose(first.checked_sub(1).map(|p| &tokens[p]), LOOSE_BEFORE) || !loose(tokens.get(last + 1), LOOSE_AFTER) {
        return None;
    }
    let mut terms: Vec<Vec<usize>> = vec![vec![0]];
    for (i, &op) in operators.iter().enumerate() {
        if tokens[op].is_punct("|") {
            terms.push(vec![i + 1]);
        } else {
            terms.last_mut()?.push(i + 1);
        }
    }
    let mut kept_terms: Vec<(usize, Vec<usize>)> = Vec::new();
    let mut lone = None;
    for term in terms {
        let opening = term[0];
        if let [only] = term[..] {
            if let Some(alone) = flag(file, &operands[only]).filter(|_| !operands[only].negated()) {
                lone.get_or_insert(alone);
                continue;
            }
        }
        let mut kept = Vec::new();
        for &f in &term {
            let strict = tokens[operands[f].atom].text.trim_start_matches('\\') == "E_STRICT";
            match flag(file, &operands[f]) {
                Some(_) if strict && operands[f].negated() && term.len() > 1 => {}
                Some(Alone::Value(_)) if !operands[f].negated() || !strict => kept.push(f),
                Some(_) => return None,
                None => kept.push(f),
            }
        }
        if kept.is_empty() {
            return None;
        }
        kept_terms.push((opening, kept));
    }
    if kept_terms.is_empty() {
        return emptied(file, first, last, lone?, class);
    }
    let factor = |f: usize| -> String {
        let o = &operands[f];
        match flag(file, o) {
            Some(Alone::Value(value)) => {
                format!("{}{value}", file.text(o.first, o.atom).strip_suffix(tokens[o.atom].text.as_str()).unwrap_or_default())
            }
            _ => file.text(o.first, o.atom).to_string(),
        }
    };
    let separator = |i: usize| -> String {
        let op = operators[i - 1];
        format!("{}{}{}", file.between(operands[i - 1].atom, op), tokens[op].text, file.between(op, operands[i].first))
    };
    let join = |items: &[usize]| -> String {
        let mut out = factor(items[0]);
        for &i in &items[1..] {
            out.push_str(&separator(i));
            out.push_str(&factor(i));
        }
        out
    };
    let mut text = String::new();
    for (n, (opening, term)) in kept_terms.iter().enumerate() {
        if n > 0 {
            text.push_str(&separator(*opening));
        }
        text.push_str(&join(term));
    }
    let original = file.text(first, last);
    (text != original).then(|| Fix { class, start: tokens[first].start, end: tokens[last].end, text })
}

fn site_fix(file: &Source, k: usize, rule: &str) -> Option<Fix> {
    let tokens = file.tokens;
    let tok = tokens.get(k)?;
    let php5 = file.current.is_some_and(|v| v < PhpVersion::new(7, 0));
    let replace = |class, text: String| Some(Fix { class, start: tok.start, end: tok.end, text });
    match rule {
        "deprecated.null_to_internal" => {
            if !tok.is_ident_ci("null") || file.strict {
                return None;
            }
            let call = enclosing_call(tokens, k)?;
            replace(NULL_ARGUMENT, if call_named(tokens, call, &["number_format"]) { "0" } else { "''" }.into())
        }
        "deprecated.tentative_return_type" => will_change(file, k),
        "removed.break_outside_loop" => {
            let plain = (tok.is_ident_ci("break") || tok.is_ident_ci("continue")) && tokens.get(k + 1).is_some_and(|t| t.is_punct(";"));
            let function = enclosing_function(tokens, k).filter(|_| plain)?;
            let void = function.returns.as_ref().is_none_or(|r| r.len() == 1 && r[0] == "void");
            if !void || function.by_ref {
                return None;
            }
            replace(BREAK_RETURN, "return".into())
        }
        "removed.intl_idna_variant_2003" => {
            let constant = tok.text.trim_start_matches('\\') == "INTL_IDNA_VARIANT_2003" && is_atom(tokens, k);
            constant.then(|| Fix {
                class: IDNA_UTS46,
                start: tok.start,
                end: tok.end,
                text: format!("{}INTL_IDNA_VARIANT_UTS46", lead(tok)),
            })
        }
        "behavior.substr_false" => substr_false(file, k).filter(|_| php5),
        "behavior.indirect_access_order" => {
            if !php5 || !crate::calls::indirect_access(tokens, k) {
                return None;
            }
            let end = dimensions_end(tokens, k)?;
            Some(Fix {
                class: crate::analyzer::INDIRECT_ACCESS_FIX,
                start: tok.start,
                end: tokens[end].end,
                text: format!("{{{}}}", file.text(k, end)),
            })
        }
        "review.variable_variable" => variable_variable(file, k),
        _ => None,
    }
}

/// The last `]` of the `[...]` that follow token `k`; `None` when one is empty or unclosed.
fn dimensions_end(tokens: &[Token], k: usize) -> Option<usize> {
    let mut end = k;
    while tokens.get(end + 1).is_some_and(|n| n.is_punct("[")) {
        let close = crate::declarations::close_of(tokens, end + 1)?;
        if close == end + 2 {
            return None;
        }
        end = close;
    }
    (end > k).then_some(end)
}

/// `#[\ReturnTypeWillChange]` on its own line above the method named at `k`, after its docblock.
fn will_change(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let keyword = match k.checked_sub(1).map(|p| &tokens[p]) {
        Some(p) if p.is_ident_ci("function") => k - 1,
        Some(p) if p.is_punct("&") && k >= 2 && tokens[k - 2].is_ident_ci("function") => k - 2,
        _ => return None,
    };
    let modifiers = ["public", "protected", "private", "static", "final", "abstract"];
    let mut first = keyword;
    while first > 0 && modifiers.iter().any(|m| tokens[first - 1].is_ident_ci(m)) {
        first -= 1;
    }
    let at = tokens[first].start;
    let line = &file.src[file.src[..at].rfind('\n').map_or(0, |i| i + 1)..at];
    let indent: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
    let newline = if file.src.contains("\r\n") { "\r\n" } else { "\n" };
    Some(Fix { class: WILL_CHANGE, start: at, end: at, text: format!("#[\\ReturnTypeWillChange]{newline}{indent}") })
}

/// The innermost function or method whose body holds token `k`.
fn enclosing_function(tokens: &[Token], k: usize) -> Option<crate::declarations::Function> {
    (0..k)
        .filter(|&j| tokens[j].is_ident_ci("function"))
        .filter_map(|j| crate::declarations::function_at(tokens, j))
        .filter(|f| f.body.is_some_and(|(open, close)| open < k && k < close))
        .max_by_key(|f| f.body.map(|(open, _)| open))
}

/// `substr(A, B) === false` → `=== ''`: with two arguments PHP 5 returned false exactly where PHP 7.0+
/// returns an empty string.
fn substr_false(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let (close, args) = function_call(tokens, k, &["substr"])?;
    if args.len() != 2 {
        return None;
    }
    let strict = |j: usize| tokens.get(j).is_some_and(|o| o.is_punct("===") || o.is_punct("!=="));
    let falsy = |j: usize| tokens.get(j).is_some_and(|t| t.is_ident_ci("false"));
    let before = |j: usize| j.checked_sub(1).map(|p| &tokens[p]);
    let first = if k > 0 && tokens[k - 1].is_punct("@") { k - 1 } else { k };
    let at = if strict(close + 1) && falsy(close + 2) && loose(tokens.get(close + 3), LOOSE_AFTER) && loose(before(first), LOOSE_BEFORE) {
        close + 2
    } else if first >= 2
        && strict(first - 1)
        && falsy(first - 2)
        && loose(before(first - 2), LOOSE_BEFORE)
        && loose(tokens.get(close + 1), LOOSE_AFTER)
    {
        first - 2
    } else {
        return None;
    };
    Some(Fix { class: SUBSTR_EMPTY, start: tokens[at].start, end: tokens[at].end, text: "''".into() })
}

/// `$$name[...]` with its PHP 5 meaning `${$name[...]}` written out, or `${$name}[...]` on a project
/// that already runs PHP 7.
fn variable_variable(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let current = file.current?;
    let name = tokens.get(k + 1).filter(|t| t.kind == TokenKind::Variable)?;
    let before = k.checked_sub(1).map(|p| &tokens[p]);
    let nested = before.is_some_and(|p| ["$", "->", "?->", "::"].iter().any(|s| p.is_punct(s)) || p.is_ident_ci("global"));
    if !tokens[k].is_punct("$") || nested {
        return None;
    }
    if current >= PhpVersion::new(7, 0) {
        return Some(Fix {
            class: VARIABLE_BRACES,
            start: tokens[k].start,
            end: name.end,
            text: format!("${{{}}}", file.text(k + 1, k + 1)),
        });
    }
    let end = dimensions_end(tokens, k + 1)?;
    Some(Fix { class: VARIABLE_ORDER, start: tokens[k].start, end: tokens[end].end, text: format!("${{{}}}", file.text(k + 1, end)) })
}

/// Token ranges of the arguments of the call whose `(` is at `open`, and the `)` that closes it.
fn arguments(tokens: &[Token], open: usize) -> Option<(usize, Vec<(usize, usize)>)> {
    let mut depth = 0usize;
    let mut start = open + 1;
    let mut out = Vec::new();
    for (j, t) in tokens.iter().enumerate().skip(open + 1) {
        if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") || t.is_punct("#[") {
            depth += 1;
        } else if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            if depth == 0 {
                if j > start {
                    out.push((start, j - 1));
                }
                return t.is_punct(")").then_some((j, out));
            }
            depth -= 1;
        } else if depth == 0 && t.is_punct(",") {
            out.push((start, j.checked_sub(1)?));
            start = j + 1;
        }
    }
    None
}

fn plain(tokens: &[Token], (first, last): (usize, usize)) -> bool {
    tokens[first].kind == TokenKind::Variable
        && tokens[first + 1..=last].chunks(2).all(|c| c.len() == 2 && c[0].is_punct("->") && c[1].kind == TokenKind::Ident)
}

fn literal(tokens: &[Token], (first, last): (usize, usize)) -> bool {
    let t = &tokens[first];
    first == last
        && (t.kind == TokenKind::Number
            || matches!(t.kind, TokenKind::Str { interpolates: false, backtick: false })
            || ["true", "false", "null"].iter().any(|w| t.is_ident_ci(w)))
}

/// `[...]` or `array(...)` holding only literals, keys included.
fn literal_array(tokens: &[Token], (first, last): (usize, usize)) -> bool {
    let open = if tokens[first].is_punct("[") {
        first
    } else if tokens[first].is_ident_ci("array") && tokens.get(first + 1).is_some_and(|t| t.is_punct("(")) {
        first + 1
    } else {
        return false;
    };
    crate::declarations::close_of(tokens, open) == Some(last)
        && (open + 1..last).all(|j| tokens[j].is_punct(",") || tokens[j].is_punct("=>") || literal(tokens, (j, j)))
}

fn falsy(tokens: &[Token], (first, last): (usize, usize)) -> bool {
    let t = &tokens[first];
    first == last
        && (t.text == "0" || ["''", "\"\"", "'0'", "\"0\""].contains(&t.text.as_str()) || t.is_ident_ci("false") || t.is_ident_ci("null"))
}

fn starts_statement(tokens: &[Token], k: usize) -> bool {
    k.checked_sub(1).is_none_or(|p| tokens[p].is_punct(";") || tokens[p].is_punct("{") || tokens[p].is_punct("}"))
}

/// The text a statement from token `first` to its `;` at `semi` leaves behind: its whole line when
/// nothing else is on it.
fn removal(file: &Source, first: usize, semi: usize, class: &'static str) -> Fix {
    let src = file.src;
    let (start, end) = (file.tokens[first].start, file.tokens[semi].end);
    let line_start = src[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = src[end..].find('\n').map_or(src.len(), |i| end + i + 1);
    let blank = |s: &str| s.chars().all(char::is_whitespace);
    if blank(&src[line_start..start]) && blank(&src[end..line_end]) {
        return Fix { class, start: line_start, end: line_end, text: String::new() };
    }
    let trailing = src[end..].len() - src[end..].trim_start_matches([' ', '\t']).len();
    Fix { class, start, end: end + trailing, text: String::new() }
}

/// The innermost function whose body holds token `k`: its closing brace.
fn body_end(tokens: &[Token], k: usize) -> Option<usize> {
    (0..k)
        .filter(|&j| tokens[j].is_ident_ci("function") || tokens[j].is_ident_ci("fn"))
        .filter_map(|j| crate::declarations::function_at(tokens, j)?.body)
        .filter(|&(open, close)| open < k && k < close)
        .max_by_key(|&(open, _)| open)
        .map(|(_, close)| close)
}

fn statement_fix(file: &Source, k: usize, rule: &str) -> Option<Fix> {
    let tokens = file.tokens;
    if let Some(&(_, _, args)) = NO_OP_CALLS
        .iter()
        .find(|(r, name, _)| *r == rule && tokens.get(k).is_some_and(|t| t.text.trim_start_matches('\\').eq_ignore_ascii_case(name)))
    {
        if !tokens.get(k + 1).is_some_and(|t| t.is_punct("("))
            || !starts_statement(tokens, k)
            || !call_named(tokens, k, &[tokens[k].text.trim_start_matches('\\')])
        {
            return None;
        }
        let (close, given) = arguments(tokens, k + 1)?;
        let fits = |a: &(usize, usize)| match args {
            Args::Plain => plain(tokens, *a),
            Args::PlainOrLiteral => plain(tokens, *a) || literal(tokens, *a),
            Args::Falsy => falsy(tokens, *a),
        };
        let expected = if args == Args::Plain { 1 } else { given.len().min(1) };
        let semi = close + 1;
        return (given.len() == expected && given.iter().all(fits) && tokens.get(semi).is_some_and(|t| t.is_punct(";")))
            .then(|| removal(file, k, semi, NO_OP_CALL));
    }
    match rule {
        "deprecated.curlopt_binarytransfer" => {
            let call = enclosing_call(tokens, k)?;
            if !call_named(tokens, call, &["curl_setopt"]) || !starts_statement(tokens, call) {
                return None;
            }
            let (close, given) = arguments(tokens, call + 1)?;
            let simple = |a: &(usize, usize)| plain(tokens, *a) || literal(tokens, *a);
            let fits = given.len() == 3 && given[1] == (k, k) && simple(&given[0]) && simple(&given[2]);
            (fits && tokens.get(close + 1).is_some_and(|t| t.is_punct(";"))).then(|| removal(file, call, close + 1, NO_OP_CALL))
        }
        "removed.mysqli_embedded_server" => {
            let (close, given) = function_call(tokens, k, &["mysqli_embedded_server_start", "mysqli_embedded_server_end"])?;
            let inert = |a: &(usize, usize)| plain(tokens, *a) || literal(tokens, *a) || literal_array(tokens, *a);
            (is_statement(tokens, k, close) && given.iter().all(inert)).then(|| removal(file, k, close + 1, EMBEDDED_SERVER))
        }
        "removed.unset_this" => {
            let (close, given) = arguments(tokens, k + 1)?;
            let this = given.iter().position(|&(f, l)| f == l && tokens[f].kind == TokenKind::Variable && tokens[f].text == "this")?;
            let end = body_end(tokens, k)?;
            if (close + 1..end).any(|j| tokens[j].kind == TokenKind::Variable && tokens[j].text == "this") {
                return None;
            }
            if given.len() == 1 {
                let statement = starts_statement(tokens, k) && tokens.get(close + 1).is_some_and(|t| t.is_punct(";"));
                return statement.then(|| removal(file, k, close + 1, UNSET_THIS));
            }
            let (first, last) = given[this];
            let (start, stop) =
                if this == 0 { (tokens[first].start, tokens[given[1].0].start) } else { (tokens[given[this - 1].1].end, tokens[last].end) };
            Some(Fix { class: UNSET_THIS, start, end: stop, text: String::new() })
        }
        _ => None,
    }
}

/// A function call `name(...)` at `k`, not a method or a declaration: its `)` and arguments.
fn function_call(tokens: &[Token], k: usize, names: &[&str]) -> Option<(usize, Vec<(usize, usize)>)> {
    if tokens[k].kind != TokenKind::Ident || !tokens.get(k + 1).is_some_and(|t| t.is_punct("(")) || !call_named(tokens, k, names) {
        return None;
    }
    let (close, args) = arguments(tokens, k + 1)?;
    let positional = args
        .iter()
        .all(|&(f, l)| !tokens[f].is_punct("...") && !(f < l && tokens[f + 1].is_punct(":") && tokens[f].kind == TokenKind::Ident));
    positional.then_some((close, args))
}

/// A method call `->name(...)` at `k`: its `)` and arguments.
fn method_call(tokens: &[Token], k: usize, names: &[&str]) -> Option<(usize, Vec<(usize, usize)>)> {
    let arrow = k.checked_sub(1).is_some_and(|p| tokens[p].is_punct("->") || tokens[p].is_punct("?->"));
    let named = names.iter().any(|n| tokens[k].text.eq_ignore_ascii_case(n));
    if !arrow || !named || !tokens.get(k + 1).is_some_and(|t| t.is_punct("(")) {
        return None;
    }
    let (close, args) = arguments(tokens, k + 1)?;
    args.iter().all(|&(f, _)| !tokens[f].is_punct("...")).then_some((close, args))
}

fn lead(tok: &Token) -> &str {
    &tok.text[..tok.text.len() - tok.text.trim_start_matches('\\').len()]
}

fn is_statement(tokens: &[Token], first: usize, close: usize) -> bool {
    starts_statement(tokens, first) && tokens.get(close + 1).is_some_and(|t| t.is_punct(";"))
}

/// `(int) X`, with parentheses when X is more than one variable, property or number.
fn as_int(file: &Source, arg: (usize, usize)) -> String {
    let text = file.text(arg.0, arg.1);
    if plain(file.tokens, arg) || literal(file.tokens, arg) {
        format!("(int) {text}")
    } else {
        format!("(int) ({text})")
    }
}

fn mhash_algo(
    tokens: &[Token],
    (first, last): (usize, usize),
) -> Option<&'static (&'static str, u32, &'static str, bool, &'static str, u32)> {
    let name = tokens[first].text.trim_start_matches('\\').strip_prefix("MHASH_")?;
    (first == last && tokens[first].kind == TokenKind::Ident).then(|| MHASH_ALGOS.iter().find(|a| a.0 == name)).flatten()
}

/// mhash(), mhash_get_hash_name() and mhash_get_block_size() with a constant algorithm.
fn mhash_fix(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let names = ["mhash", "mhash_get_hash_name", "mhash_get_block_size"];
    let (close, args) = function_call(tokens, k, &names)?;
    let algo = mhash_algo(tokens, *args.first()?)?;
    let bare = tokens[k].text.trim_start_matches('\\').to_ascii_lowercase();
    let fix = |start: usize, end: usize, text: String| Some(Fix { class: MHASH, start, end, text });
    match (bare.as_str(), args.len()) {
        ("mhash_get_hash_name", 1) => fix(tokens[k].start, tokens[close].end, format!("'{}'", algo.4)),
        ("mhash_get_block_size", 1) => fix(tokens[k].start, tokens[close].end, algo.5.to_string()),
        ("mhash", 2 | 3) => {
            if args.len() == 3 && !algo.3 {
                return None;
            }
            let data = file.text(args[1].0, args[1].1);
            let call = match args.get(2) {
                Some(key) => format!("{}hash_hmac('{}', {data}, {}", lead(&tokens[k]), algo.2, file.text(key.0, key.1)),
                None => format!("{}hash('{}', {data}", lead(&tokens[k]), algo.2),
            };
            let hex = enclosing_call(tokens, k).filter(|&c| call_named(tokens, c, &["bin2hex"])).and_then(|c| {
                let (outer, inner) = function_call(tokens, c, &["bin2hex"])?;
                (inner == [(k, close)]).then_some((c, outer))
            });
            match hex {
                Some((c, outer)) => fix(tokens[c].start, tokens[outer].end, format!("{call})")),
                None => fix(tokens[k].start, tokens[close].end, format!("{call}, true)")),
            }
        }
        _ => None,
    }
}

/// The FLUSH statement for a `|` mask of MYSQLI_REFRESH_* constants, if every flag has one.
fn flush_statement(tokens: &[Token], (first, last): (usize, usize)) -> Option<String> {
    let mut options = Vec::new();
    for (i, t) in tokens[first..=last].iter().enumerate() {
        if i % 2 == 1 {
            if !t.is_punct("|") {
                return None;
            }
            continue;
        }
        let flag = t.text.trim_start_matches('\\').strip_prefix("MYSQLI_REFRESH_").filter(|_| t.kind == TokenKind::Ident)?;
        let option = FLUSH_OPTIONS.iter().find(|(f, _)| *f == flag)?.1;
        if !options.contains(&option) {
            options.push(option);
        }
    }
    (!options.is_empty() && (last - first) % 2 == 0).then(|| format!("'FLUSH {}'", options.join(", ")))
}

/// mysqli_refresh($link, FLAGS) and $link->refresh(FLAGS).
fn refresh_fix(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    if let Some((close, args)) = function_call(tokens, k, &["mysqli_refresh"]) {
        let [link, flags] = args[..] else { return None };
        let sql = flush_statement(tokens, flags)?;
        let text = format!("{}mysqli_query({}, {sql})", lead(&tokens[k]), file.text(link.0, link.1));
        return Some(Fix { class: REFRESH, start: tokens[k].start, end: tokens[close].end, text });
    }
    let (close, args) = method_call(tokens, k, &["refresh"])?;
    let [flags] = args[..] else { return None };
    let sql = flush_statement(tokens, flags)?;
    Some(Fix { class: REFRESH, start: tokens[k].start, end: tokens[close].end, text: format!("query({sql})") })
}

/// The mode argument of mysqli_store_result($link, MODE) or ->store_result(MODE) goes.
fn store_result_fix(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let simple = |a: (usize, usize)| {
        plain(tokens, a) || literal(tokens, a) || (a.0 == a.1 && is_atom(tokens, a.0)) || (a.1 == a.0 + 2 && tokens[a.0 + 1].is_punct("::"))
    };
    let fix = |start: usize, end: usize| Some(Fix { class: UNUSED_ARGUMENT, start, end, text: String::new() });
    if let Some((_, args)) = function_call(tokens, k, &["mysqli_store_result"]) {
        let [link, mode] = args[..] else { return None };
        return if simple(mode) { fix(tokens[link.1].end, tokens[mode.1].end) } else { None };
    }
    let (_, args) = method_call(tokens, k, &["store_result"])?;
    let [mode] = args[..] else { return None };
    if simple(mode) {
        fix(tokens[mode.0].start, tokens[mode.1].end)
    } else {
        None
    }
}

/// The call whose argument holds token `k`, as (call name, argument index), when that argument is
/// the token alone or the `|` mask around it.
fn argument_of(tokens: &[Token], k: usize) -> Option<(usize, usize, (usize, usize))> {
    let call = enclosing_call(tokens, k)?;
    let (_, args) = arguments(tokens, call + 1)?;
    let index = args.iter().position(|&(f, l)| f <= k && k <= l)?;
    Some((call, index, args[index]))
}

/// assert_options(ASSERT_X, VALUE); as a statement: ASSERT_QUIET_EVAL always goes, the others when
/// VALUE is the setting's default.
fn assert_options_fix(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let (close, args) = function_call(tokens, k, &["assert_options"])?;
    if !is_statement(tokens, k, close) {
        return None;
    }
    let (first, last) = *args.first()?;
    let name = tokens[first].text.trim_start_matches('\\').strip_prefix("ASSERT_").filter(|_| first == last)?;
    if name == "QUIET_EVAL" && args.len() <= 2 {
        return Some(removal(file, k, close + 1, NO_OP_CALL));
    }
    let (_, _, defaults) = ASSERT_SETTINGS.iter().find(|(n, ..)| *n == name)?;
    let value = args.get(1).filter(|&&(f, l)| f == l && args.len() == 2)?;
    let literal = tokens[value.0].text.to_ascii_lowercase();
    defaults.contains(&literal.as_str()).then(|| removal(file, k, close + 1, ASSERT_DEFAULT))
}

/// ldap_control_paged_result($link, $size[, $critical[, $cookie]]) sets the same control the
/// connection-wide way ldap_set_option() does.
fn paged_result_fix(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let (close, args) = function_call(tokens, k, &["ldap_control_paged_result"])?;
    if !(2..=4).contains(&args.len()) || !args.iter().all(|&a| plain(tokens, a) || literal(tokens, a)) {
        return None;
    }
    let text = |a: (usize, usize)| file.text(a.0, a.1).to_string();
    let critical = match args.get(2) {
        None => "false".to_string(),
        Some(&a) if ["true", "false"].iter().any(|w| tokens[a.0].is_ident_ci(w)) => text(a).to_ascii_lowercase(),
        Some(&a) => format!("(bool) {}", text(a)),
    };
    let cookie = args.get(3).map_or_else(|| "''".to_string(), |&a| text(a));
    let control = format!(
        "[['oid' => LDAP_CONTROL_PAGEDRESULTS, 'iscritical' => {critical}, 'value' => ['size' => {}, 'cookie' => {cookie}]]]",
        text(args[1])
    );
    let call = format!("{}ldap_set_option({}, LDAP_OPT_SERVER_CONTROLS, {control})", lead(&tokens[k]), text(args[0]));
    Some(Fix { class: LDAP_PAGING, start: tokens[k].start, end: tokens[close].end, text: call })
}

/// The innermost bracket around token `k` that opens an array literal: `[` or `array(`.
fn array_around(tokens: &[Token], k: usize) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    for j in (0..k).rev() {
        let t = &tokens[j];
        if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            depth += 1;
        } else if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") {
            if depth > 0 {
                depth -= 1;
                continue;
            }
            let first = if t.is_punct("[") {
                j
            } else if t.is_punct("(") && j > 0 && tokens[j - 1].is_ident_ci("array") {
                j - 1
            } else {
                return None;
            };
            return Some((first, crate::declarations::close_of(tokens, j)?));
        }
    }
    None
}

/// `CURLOPT_BINARYTRANSFER => VALUE` in an array passed straight to curl_setopt_array().
fn curl_option_item(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    if !tokens.get(k + 1).is_some_and(|t| t.is_punct("=>")) {
        return None;
    }
    let value = (k + 2, k + 2);
    if !(plain(tokens, value) || literal(tokens, value)) {
        return None;
    }
    let (first, close) = array_around(tokens, k)?;
    let (call, index, arg) = argument_of(tokens, first)?;
    if !call_named(tokens, call, &["curl_setopt_array"]) || index != 1 || arg != (first, close) {
        return None;
    }
    let before = &tokens[k - 1];
    let after = tokens.get(value.1 + 1)?;
    let (start, end) = if after.is_punct(",") {
        (tokens[k].start, tokens.get(value.1 + 2).map_or(after.end, |n| n.start))
    } else if before.is_punct(",") {
        (tokens[k - 2].end, tokens[value.1].end)
    } else {
        (tokens[k].start, tokens[value.1].end)
    };
    Some(Fix { class: NO_OP_FLAG, start, end, text: String::new() })
}

fn call_fix(file: &Source, k: usize, rule: &str) -> Option<Fix> {
    let tokens = file.tokens;
    let tok = &tokens[k];
    let bare = tok.text.trim_start_matches('\\');
    let value = |text: &str, class| Some(Fix { class, start: tok.start, end: tok.end, text: text.to_string() });
    match rule {
        "deprecated.mhash" => mhash_fix(file, k),
        "deprecated.mhash_constant" => {
            let algo = mhash_algo(tokens, (k, k))?;
            match argument_of(tokens, k) {
                Some((call, 0, _)) if call_named(tokens, call, &["mhash", "mhash_get_hash_name", "mhash_get_block_size"]) => {
                    mhash_fix(file, call)
                }
                Some((call, ..)) if tokens[call].text.trim_start_matches('\\').to_ascii_lowercase().starts_with("mhash") => None,
                _ => value(&algo.1.to_string(), CONSTANT_VALUE),
            }
        }
        "deprecated.mysqli_kill" => {
            if let Some((close, args)) = function_call(tokens, k, &["mysqli_kill"]) {
                let [link, id] = args[..] else { return None };
                let text = format!("{}mysqli_query({}, 'KILL CONNECTION ' . {})", lead(tok), file.text(link.0, link.1), as_int(file, id));
                return Some(Fix { class: KILL, start: tok.start, end: tokens[close].end, text });
            }
            let (close, args) = method_call(tokens, k, &["kill"])?;
            let [id] = args[..] else { return None };
            Some(Fix {
                class: KILL,
                start: tok.start,
                end: tokens[close].end,
                text: format!("query('KILL CONNECTION ' . {})", as_int(file, id)),
            })
        }
        "deprecated.mysqli_refresh" => refresh_fix(file, k),
        "deprecated.mysqli_refresh_constant" => {
            let flag = bare.strip_prefix("MYSQLI_REFRESH_")?;
            match argument_of(tokens, k) {
                Some((call, index, _))
                    if call_named(tokens, call, &["mysqli_refresh"])
                        || (index == 0 && method_call(tokens, call, &["refresh"]).is_some()) =>
                {
                    refresh_fix(file, call)
                }
                _ => value(REFRESH_VALUES.iter().find(|(f, _)| *f == flag)?.1, CONSTANT_VALUE),
            }
        }
        "deprecated.intlcal_set" => {
            let (close, args) = function_call(tokens, k, &["intlcal_set"])?;
            let (cal, values) = args.split_first()?;
            let method = match values.len() {
                2 => "set",
                3 if is_statement(tokens, k, close) => "setDate",
                5 | 6 if is_statement(tokens, k, close) => "setDateTime",
                _ => return None,
            };
            let receiver = if plain(tokens, *cal) { file.text(cal.0, cal.1).to_string() } else { format!("({})", file.text(cal.0, cal.1)) };
            let rest = file.text(values[0].0, values.last()?.1);
            Some(Fix { class: INTL_METHODS, start: tok.start, end: tokens[close].end, text: format!("{receiver}->{method}({rest})") })
        }
        "deprecated.intlgregcal_create_instance" => {
            let (close, args) = function_call(tokens, k, &["intlgregcal_create_instance"])?;
            let inner = args.first().map_or("", |a| file.text(a.0, args.last().map_or(a.1, |l| l.1)));
            let text = match args.len() {
                0..=2 => format!("new \\IntlGregorianCalendar({inner})"),
                3 => format!("\\IntlGregorianCalendar::createFromDate({inner})"),
                5 | 6 => format!("\\IntlGregorianCalendar::createFromDateTime({inner})"),
                _ => return None,
            };
            Some(Fix { class: INTL_METHODS, start: tok.start, end: tokens[close].end, text })
        }
        "deprecated.lcg_value" => {
            let (close, args) = function_call(tokens, k, &["lcg_value"])?;
            let text = "(new \\Random\\Randomizer())->getFloat(0, 1, \\Random\\IntervalBoundary::OpenOpen)".to_string();
            args.is_empty().then(|| Fix { class: LCG_VALUE, start: tok.start, end: tokens[close].end, text })
        }
        "removed.get_magic_quotes_gpc" | "removed.get_magic_quotes_runtime" => {
            let (close, args) = function_call(tokens, k, &["get_magic_quotes_gpc", "get_magic_quotes_runtime"])?;
            if !args.is_empty() {
                return None;
            }
            let guard = k >= 2
                && tokens[k - 1].is_punct("(")
                && tokens[k - 2].is_ident_ci("if")
                && tokens.get(close + 1).is_some_and(|t| t.is_punct(")"));
            let block = close + 2;
            if guard && starts_statement(tokens, k - 2) && tokens.get(block).is_some_and(|t| t.is_punct("{")) {
                let end = crate::declarations::close_of(tokens, block)?;
                let chained = tokens.get(end + 1).is_some_and(|t| t.is_ident_ci("else") || t.is_ident_ci("elseif"));
                if !chained {
                    return Some(removal(file, k - 2, end, MAGIC_QUOTES));
                }
            }
            if k > 0 && tokens[k - 1].is_punct("!") {
                return Some(Fix { class: MAGIC_QUOTES, start: tokens[k - 1].start, end: tokens[close].end, text: "true".into() });
            }
            Some(Fix { class: MAGIC_QUOTES, start: tok.start, end: tokens[close].end, text: "false".into() })
        }
        "deprecated.pdo_fetch_serialize" => {
            let class = tokens.get(k.checked_sub(2)?).filter(|c| c.kind == TokenKind::Ident && tokens[k - 1].is_punct("::"))?;
            if !class.text.trim_start_matches('\\').eq_ignore_ascii_case("pdo") {
                return None;
            }
            let (call, index, arg) = argument_of(tokens, k)?;
            let slot = match tokens[call].text.to_ascii_lowercase().as_str() {
                "fetch" | "fetchall" | "setfetchmode" => 0,
                "query" => 1,
                _ => return None,
            };
            let method = call > 0 && (tokens[call - 1].is_punct("->") || tokens[call - 1].is_punct("?->"));
            (method && index == slot && arg == (k - 2, k)).then(|| Fix {
                class: CONSTANT_RENAME,
                start: tok.start,
                end: tok.end,
                text: "FETCH_DEFAULT".into(),
            })
        }
        "deprecated.mysqli_store_result_mode" => store_result_fix(file, k),
        "deprecated.mysqli_store_result_copy_data" => match argument_of(tokens, k) {
            Some((call, index, arg)) if arg == (k, k) => {
                let mode = if call_named(tokens, call, &["mysqli_store_result"]) {
                    1
                } else if method_call(tokens, call, &["store_result"]).is_some() {
                    0
                } else {
                    return value("16", CONSTANT_VALUE);
                };
                if index == mode {
                    store_result_fix(file, call)
                } else {
                    None
                }
            }
            Some((call, ..)) if tokens[call].text.to_ascii_lowercase().ends_with("store_result") => None,
            _ => value("16", CONSTANT_VALUE),
        },
        "removed.datefmt_set_timezone_id" => {
            if function_call(tokens, k, &["datefmt_set_timezone_id"]).is_some() {
                return value(&format!("{}datefmt_set_timezone", lead(tok)), FUNCTION_RENAME);
            }
            method_call(tokens, k, &["settimezoneid"]).and_then(|_| value("setTimeZone", FUNCTION_RENAME))
        }
        "removed.ldap_control_paged_result" => paged_result_fix(file, k),
        "removed.ldap_control_paged_result_response" => {
            let (close, args) = function_call(tokens, k, &["ldap_control_paged_result_response"])?;
            let fits = args.len() == 2 && args.iter().all(|&a| plain(tokens, a)) && is_statement(tokens, k, close);
            fits.then(|| removal(file, k, close + 1, NO_OP_CALL))
        }
        "deprecated.assert_options" => assert_options_fix(file, k),
        "deprecated.assert_constants" | "removed.assert_quiet_eval" => {
            let name = bare.strip_prefix("ASSERT_")?;
            match argument_of(tokens, k) {
                Some((call, 0, arg)) if arg == (k, k) && call_named(tokens, call, &["assert_options"]) => assert_options_fix(file, call),
                Some((call, ..)) if call_named(tokens, call, &["assert_options"]) => None,
                _ if rule == "removed.assert_quiet_eval" => None,
                _ => value(ASSERT_SETTINGS.iter().find(|(n, ..)| *n == name)?.1, CONSTANT_VALUE),
            }
        }
        "deprecated.zip_fl_recompress" => {
            let class = tokens.get(k.checked_sub(2)?).filter(|c| c.kind == TokenKind::Ident && tokens[k - 1].is_punct("::"))?;
            if !class.text.trim_start_matches('\\').eq_ignore_ascii_case("ziparchive") {
                return None;
            }
            let (call, index, arg) = argument_of(tokens, k)?;
            let (_, args) = method_call(tokens, call, &["setcompressionname", "setcompressionindex"])?;
            (index == 2 && args.len() == 3 && arg == (k - 2, k)).then(|| Fix {
                class: ZIP_LEVEL,
                start: tokens[args[1].1].end,
                end: tok.end,
                text: String::new(),
            })
        }
        "removed.fgetss" | "removed.gzgetss" => {
            let (name, read, reads) = if rule == "removed.fgetss" { ("fgetss", "fgets", 1) } else { ("gzgetss", "gzgets", 2) };
            let (close, args) = function_call(tokens, k, &[name])?;
            if args.len() < reads || args.len() > 3 {
                return None;
            }
            let allowed = match args.get(2) {
                Some(&a) if a.0 == a.1 && matches!(tokens[a.0].kind, TokenKind::Str { interpolates: false, .. }) => {
                    format!(", {}", file.text(a.0, a.1))
                }
                Some(_) => return None,
                None => String::new(),
            };
            let read_args = file.text(args[0].0, args[args.len().min(2) - 1].1);
            let text =
                format!("(static fn ($line) => $line === false ? false : strip_tags($line{allowed}))({}{read}({read_args}))", lead(tok));
            Some(Fix { class: GETSS, start: tok.start, end: tokens[close].end, text })
        }
        "removed.recode" => {
            let (close, args) = function_call(tokens, k, &["recode_string"])?;
            let [request, text] = args[..] else { return None };
            if request.0 != request.1 || !matches!(tokens[request.0].kind, TokenKind::Str { interpolates: false, backtick: false }) {
                return None;
            }
            let raw = tokens[request.0].text.trim_matches(|c| c == '\'' || c == '"').to_ascii_lowercase();
            let (from, to) = raw.split_once("..")?;
            let name = |n: &str| RECODE_CHARSETS.iter().find(|(r, _)| *r == n).map(|(_, i)| *i);
            let (from, to) = (name(from)?, name(to)?);
            let call = format!("{}iconv('{from}', '{to}//TRANSLIT//IGNORE', {})", lead(tok), file.text(text.0, text.1));
            Some(Fix { class: RECODE, start: tok.start, end: tokens[close].end, text: call })
        }
        "removed.ezmlm_hash" => {
            let (close, args) = function_call(tokens, k, &["ezmlm_hash"])?;
            let [address] = args[..] else { return None };
            Some(Fix {
                class: EZMLM,
                start: tok.start,
                end: tokens[close].end,
                text: format!("{EZMLM_PORT}({})", file.text(address.0, address.1)),
            })
        }
        "removed.sql_regcase" => {
            let other = ["LC_MONETARY", "LC_NUMERIC", "LC_TIME", "LC_COLLATE", "LC_MESSAGES"];
            let ctype = (0..tokens.len())
                .filter(|&j| tokens[j].is_ident_ci("setlocale") && tokens.get(j + 1).is_some_and(|t| t.is_punct("(")))
                .any(|j| tokens.get(j + 2).is_none_or(|category| !other.iter().any(|o| category.text.trim_start_matches('\\') == *o)));
            if ctype {
                return None;
            }
            let (close, args) = function_call(tokens, k, &["sql_regcase"])?;
            let [pattern] = args[..] else { return None };
            let text = format!(
                "{}preg_replace_callback('/[a-zA-Z]/', static fn ($m) => '[' . strtoupper($m[0]) . strtolower($m[0]) . ']', {})",
                lead(tok),
                file.text(pattern.0, pattern.1)
            );
            Some(Fix { class: REGCASE, start: tok.start, end: tokens[close].end, text })
        }
        "deprecated.curlopt_binarytransfer" => statement_fix(file, k, rule).or_else(|| curl_option_item(file, k)),
        "removed.reflection_export" => {
            let class = tokens.get(k.checked_sub(2)?).filter(|c| c.kind == TokenKind::Ident && tokens[k - 1].is_punct("::"))?;
            let short = class.text.rsplit('\\').next().unwrap_or_default().to_ascii_lowercase();
            let base = REFLECTION_EXPORTS.iter().find(|(c, _)| *c == short)?.1;
            if !tokens.get(k + 1).is_some_and(|t| t.is_punct("(")) {
                return None;
            }
            let (close, args) = arguments(tokens, k + 1)?;
            if args.len() < base || args.len() > base + 1 || args.iter().any(|&(f, _)| tokens[f].is_punct("...")) {
                return None;
            }
            let flag =
                args.get(base).map(|&(f, l)| (f == l).then(|| tokens[f].text.to_ascii_lowercase())).unwrap_or(Some("false".into()))?;
            let inner = file.text(args[0].0, args[base - 1].1);
            let object = format!("new {}({inner})", class.text);
            let first = k - 2;
            match flag.as_str() {
                "false" if is_statement(tokens, first, close) => Some(Fix {
                    class: REFLECTION_EXPORT,
                    start: tokens[first].start,
                    end: tokens[close].end,
                    text: format!("echo {object}, \"\\n\""),
                }),
                "true" => Some(Fix {
                    class: REFLECTION_EXPORT,
                    start: tokens[first].start,
                    end: tokens[close].end,
                    text: format!("(string) {object}"),
                }),
                _ => None,
            }
        }
        _ => None,
    }
}

fn by_position(tokens: &[Token]) -> std::collections::HashMap<(usize, usize), usize> {
    tokens.iter().enumerate().map(|(k, t)| ((t.line, t.column), k)).collect()
}

/// Marks the findings a fix takes as automatic.
pub(crate) fn mark(src: &str, tokens: &[Token], findings: &mut [Finding], current: Option<PhpVersion>) {
    if !findings.iter().any(|f| f.fix == FixKind::Manual && handles(&f.rule_id)) {
        return;
    }
    let at_position = by_position(tokens);
    let file = Source::new(src, tokens, findings, &at_position, current);
    for (i, fix) in plan(&file, findings, &at_position) {
        if findings[i].fix == FixKind::Manual {
            findings[i].fix = crate::rector::describe_rule(fix.class).fix;
        }
    }
}

/// The fixes the findings allow, by finding. A fix inside another one, or crossing it, is left out
/// and its finding stays manual.
fn plan(file: &Source, findings: &[Finding], at_position: &std::collections::HashMap<(usize, usize), usize>) -> Vec<(usize, Fix)> {
    let all: Vec<(usize, Fix)> = findings
        .iter()
        .enumerate()
        .filter(|(_, f)| handles(&f.rule_id))
        .filter_map(|(i, f)| at_position.get(&(f.line, f.column)).and_then(|&k| at(file, k, &f.rule_id)).map(|fix| (i, fix)))
        .collect();
    let inside = |fix: &Fix| all.iter().any(|(_, o)| o.start <= fix.start && fix.end <= o.end && (o.start, o.end) != (fix.start, fix.end));
    let mut outer: Vec<(usize, Fix)> = all.iter().filter(|(_, f)| !inside(f)).cloned().collect();
    outer.sort_by_key(|(_, f)| (f.start, f.end));
    let mut kept: Vec<(usize, Fix)> = Vec::new();
    for (i, fix) in outer {
        if kept.last().is_some_and(|(_, last)| fix.start < last.end && (fix.start, fix.end) != (last.start, last.end)) {
            continue;
        }
        kept.push((i, fix));
    }
    kept
}

/// Applies every fix the file's findings allow. `None` when nothing changes.
pub(crate) fn apply(
    rel: &str,
    src: &str,
    target: PhpVersion,
    index: &std::sync::Arc<crate::index::ProjectIndex>,
) -> Option<(String, Vec<&'static str>)> {
    if !mentions(src) {
        return None;
    }
    let findings = crate::analyzer::findings_for_fixes(rel, src, target, index);
    if !findings.iter().any(|f| handles(&f.rule_id)) {
        return None;
    }
    let classic_mac = src.contains('\r') && !src.contains('\n');
    let text = if classic_mac { src.replace('\r', "\n") } else { src.to_string() };
    let tokens = crate::lexer::tokenize(&text);
    let at_position = by_position(&tokens);
    let file = Source::new(&text, &tokens, &findings, &at_position, index.current());
    let mut fixes: Vec<Fix> = plan(&file, &findings, &at_position).into_iter().map(|(_, fix)| fix).collect();
    fixes.dedup_by(|a, b| (a.start, a.end) == (b.start, b.end));
    fixes.reverse();
    let mut out = src.to_string();
    let mut applied = Vec::new();
    let mut floor = usize::MAX;
    for fix in fixes {
        if fix.end > floor {
            continue;
        }
        let line_start = |at: usize, s: &str| s[..at].rfind('\n').map_or(0, |i| i + 1);
        let whole_line = fix.start == line_start(fix.start, src) && (fix.end == src.len() || src[..fix.end].ends_with('\n'));
        out.replace_range(fix.start..fix.end, &fix.text);
        floor = fix.start;
        if fix.text.is_empty() && !whole_line {
            let start = line_start(fix.start, &out);
            let end = out[fix.start..].find('\n').map_or(out.len(), |i| fix.start + i + 1);
            if out[start..end].chars().all(char::is_whitespace) {
                out.replace_range(start..end, "");
                floor = start;
            }
        }
        applied.push((fix.start, fix.class));
    }
    applied.sort();
    let mut classes = Vec::new();
    for (_, class) in applied {
        if !classes.contains(&class) {
            classes.push(class);
        }
    }
    (!classes.is_empty()).then_some((out, classes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::analyze_source;

    fn fixed(src: &str) -> Option<(String, Vec<&'static str>)> {
        fixed_for(src, PhpVersion::new(8, 5))
    }

    fn fixed_for(src: &str, target: PhpVersion) -> Option<(String, Vec<&'static str>)> {
        let index = std::sync::Arc::new(crate::index::ProjectIndex::from_tokens_for(&crate::lexer::tokenize(src), Some(target)));
        apply("t.php", src, target, &index)
    }

    fn fixes(src: &str) -> Vec<(String, FixKind)> {
        analyze_source("t.php", src, PhpVersion::new(8, 5)).into_iter().map(|f| (f.rule_id, f.fix)).collect()
    }

    #[test]
    fn constants_take_their_value() {
        let src = "<?php\n$f = MYSQLI_SERVER_QUERY_WAS_SLOW | \\MYSQLI_SERVER_PS_OUT_PARAMS;\n$m = [MYSQLI_NO_DATA, MYSQLI_DATA_TRUNCATED, MYSQLI_IS_MARIADB];\nreturn [DOM_PHP_ERR, MB_OVERLOAD_STRING & 2, MYSQLI_STMT_ATTR_UPDATE_MAX_LENGTH];\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\n$f = 2048 | 4096;\n$m = [100, 101, false];\nreturn [0, 2 & 2, 0];\n");
        assert_eq!(classes, [CONSTANT_VALUE]);
        assert!(fixes(src).iter().all(|(_, fix)| *fix == FixKind::Auto), "{:?}", fixes(src));
    }

    #[test]
    fn renamed_constants_keep_their_backslash() {
        let src = "<?php\nnamespace App;\necho \\PGSQL_LIBPQ_VERSION_STR, filter_var($s, FILTER_SANITIZE_MAGIC_QUOTES), \\U_MULTIPLE_DECIMAL_SEPERATORS;\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\nnamespace App;\necho \\PGSQL_LIBPQ_VERSION, filter_var($s, FILTER_SANITIZE_ADD_SLASHES), \\U_MULTIPLE_DECIMAL_SEPARATORS;\n");
        assert_eq!(classes, [CONSTANT_RENAME]);
    }

    #[test]
    fn transaction_states_become_integers() {
        let src = "<?php\n$a = [PDO::PGSQL_TRANSACTION_IDLE, \\PDO::PGSQL_TRANSACTION_INERROR, Pdo\\Pgsql::TRANSACTION_UNKNOWN];\n";
        assert_eq!(fixed(src).unwrap().0, "<?php\n$a = [0, 3, 4];\n");
        let own = "<?php\nclass Db extends PDO { const PGSQL_TRANSACTION_IDLE = 9; }\necho Db::PGSQL_TRANSACTION_IDLE;\n";
        assert_eq!(fixed(own), None);
    }

    #[test]
    fn oci_aliases_become_their_oci_names() {
        let src =
            "<?php\n$c = OCILogon('u', 'p');\n$s = \\ociparse($c, 'x');\nocifetchinto($s, $row);\n$cb = 'ociexecute';\n$o->ocicommit();\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\n$c = oci_connect('u', 'p');\n$s = \\oci_parse($c, 'x');\nocifetchinto($s, $row);\n$cb = 'ociexecute';\n$o->ocicommit();\n");
        assert_eq!(classes, [OCI_ALIAS]);
        let kinds = fixes(src);
        assert!(kinds.contains(&("deprecated.oci8_alias".to_string(), FixKind::Auto)));
        assert!(kinds.contains(&("deprecated.oci8_alias".to_string(), FixKind::Manual)), "ocifetchinto() stays manual: {kinds:?}");
    }

    #[test]
    fn calls_that_do_nothing_lose_their_line() {
        let src = "<?php\nfunction f($k, $o) {\n    openssl_free_key($k);\n    \\openssl_x509_free($o->cert); shmop_close($o->shm);\n    $r = openssl_pkey_free($k);\n    if ($k) openssl_pkey_free($k);\n    openssl_free_key(make_key());\n    ociinternaldebug(true);\n    set_magic_quotes_runtime(0);\n    set_magic_quotes_runtime(1);\n    magic_quotes_runtime(false);\n}\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\nfunction f($k, $o) {\n    $r = openssl_pkey_free($k);\n    if ($k) openssl_pkey_free($k);\n    openssl_free_key(make_key());\n    set_magic_quotes_runtime(1);\n}\n");
        assert_eq!(classes, [NO_OP_CALL]);
        let manual: Vec<_> = fixes(src).into_iter().filter(|(_, f)| *f == FixKind::Manual).map(|(r, _)| r).collect();
        assert_eq!(
            manual,
            [
                "deprecated.openssl_pkey_free",
                "deprecated.openssl_pkey_free",
                "deprecated.openssl_free_key",
                "removed.set_magic_quotes_runtime"
            ]
        );
    }

    #[test]
    fn binarytransfer_and_unset_this_go_when_nothing_reads_them() {
        let curl = "<?php\ncurl_setopt($c, CURLOPT_BINARYTRANSFER, true);\n$ok = curl_setopt($c, CURLOPT_BINARYTRANSFER, true);\ncurl_setopt_array($c, [CURLOPT_BINARYTRANSFER => true]);\n";
        assert_eq!(fixed(curl).unwrap().0, "<?php\n$ok = curl_setopt($c, CURLOPT_BINARYTRANSFER, true);\ncurl_setopt_array($c, []);\n");
        let this = "<?php\nclass A {\n    function a() {\n        unset($this);\n        return 1;\n    }\n    function b($x) {\n        unset($x, $this);\n    }\n    function c() {\n        unset($this);\n        return $this->p;\n    }\n}\n";
        let (out, classes) = fixed_for(this, PhpVersion::new(7, 1)).unwrap();
        assert_eq!(out, "<?php\nclass A {\n    function a() {\n        return 1;\n    }\n    function b($x) {\n        unset($x);\n    }\n    function c() {\n        unset($this);\n        return $this->p;\n    }\n}\n");
        assert_eq!(classes, [UNSET_THIS]);
    }

    #[test]
    fn flags_that_change_nothing_leave_their_mask() {
        let src = "<?php\nfile_put_contents($f, $d, FILE_APPEND | FILE_TEXT);\nfile_put_contents($f, $d, FILE_BINARY);\n$m = FILE_BINARY|FILE_TEXT;\n$ok = filter_var($u, FILTER_VALIDATE_URL, FILTER_FLAG_SCHEME_REQUIRED | FILTER_FLAG_HOST_REQUIRED);\n$p = filter_var($u, FILTER_VALIDATE_URL, FILTER_FLAG_HOST_REQUIRED | FILTER_FLAG_PATH_REQUIRED);\n$o = filter_var($u, FILTER_VALIDATE_URL, ['flags' => FILTER_FLAG_SCHEME_REQUIRED]);\ncurl_multi_setopt($mh, CURLMOPT_PIPELINING, CURLPIPE_HTTP1);\ncurl_multi_setopt($mh, CURLMOPT_PIPELINING, CURLPIPE_HTTP1 | CURLPIPE_MULTIPLEX);\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\nfile_put_contents($f, $d, FILE_APPEND);\nfile_put_contents($f, $d, 0);\n$m = 0;\n$ok = filter_var($u, FILTER_VALIDATE_URL);\n$p = filter_var($u, FILTER_VALIDATE_URL, FILTER_FLAG_PATH_REQUIRED);\n$o = filter_var($u, FILTER_VALIDATE_URL, ['flags' => 0]);\ncurl_multi_setopt($mh, CURLMOPT_PIPELINING, CURLPIPE_NOTHING);\ncurl_multi_setopt($mh, CURLMOPT_PIPELINING, CURLPIPE_MULTIPLEX);\n");
        assert_eq!(classes, [NO_OP_FLAG, CONSTANT_VALUE]);
        assert!(fixes(src).iter().all(|(_, f)| *f == FixKind::Auto), "{:?}", fixes(src));
    }

    #[test]
    fn e_strict_leaves_masks_and_keeps_its_value_elsewhere() {
        let src = "<?php\nerror_reporting(E_ALL | E_STRICT);\nerror_reporting(E_ALL & ~E_NOTICE & ~E_STRICT);\nini_set('error_reporting', E_STRICT | E_ALL);\nif ($errno === E_STRICT) {}\nswitch ($errno) { case E_STRICT: break; }\nerror_reporting(E_STRICT);\nerror_reporting(E_ALL ^ E_STRICT);\n$x = $a + E_STRICT | E_NOTICE;\n";
        let (out, _) = fixed(src).unwrap();
        assert_eq!(out, "<?php\nerror_reporting(E_ALL);\nerror_reporting(E_ALL & ~E_NOTICE);\nini_set('error_reporting', E_ALL);\nif ($errno === 2048) {}\nswitch ($errno) { case 2048: break; }\nerror_reporting(E_STRICT);\nerror_reporting(E_ALL ^ E_STRICT);\n$x = $a + E_STRICT | E_NOTICE;\n");
        let manual = fixes(src).into_iter().filter(|(r, f)| r == "deprecated.e_strict" && *f == FixKind::Manual).count();
        assert_eq!(manual, 3);
    }

    #[test]
    fn bit_tests_on_removed_flags_stay_manual() {
        let src = "<?php\nif ($flags & FILTER_FLAG_HOST_REQUIRED) {}\n$same = $v == CURLPIPE_HTTP1;\nfilter_var_array($a, ['u' => ['filter' => FILTER_VALIDATE_URL, 'options' => FILTER_FLAG_HOST_REQUIRED]]);\n";
        assert_eq!(fixed(src), None);
    }

    fn kinds(src: &str, rule: &str) -> Vec<FixKind> {
        analyze_source("t.php", src, PhpVersion::new(8, 5)).into_iter().filter(|f| f.rule_id == rule).map(|f| f.fix).collect()
    }

    #[test]
    fn mhash_becomes_hash() {
        let src = "<?php\n$a = mhash(MHASH_SHA256, $data);\n$b = bin2hex(mhash(MHASH_MD5, $data));\n$c = \\mhash(MHASH_SHA1, $data, $key);\n$d = mhash(MHASH_CRC32, $data, $key);\n$e = [mhash_get_hash_name(MHASH_TIGER), mhash_get_block_size(MHASH_SHA512), MHASH_WHIRLPOOL];\n$f = mhash($algo, $data);\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\n$a = hash('sha256', $data, true);\n$b = hash('md5', $data);\n$c = \\hash_hmac('sha1', $data, $key, true);\n$d = mhash(MHASH_CRC32, $data, $key);\n$e = ['TIGER', 64, 22];\n$f = mhash($algo, $data);\n");
        assert_eq!(classes, [MHASH, CONSTANT_VALUE]);
        assert_eq!(
            kinds(src, "deprecated.mhash"),
            [FixKind::Auto, FixKind::Auto, FixKind::Auto, FixKind::Manual, FixKind::Auto, FixKind::Auto, FixKind::Manual]
        );
        assert_eq!(kinds(src, "deprecated.mhash_constant").iter().filter(|k| **k == FixKind::Manual).count(), 1);
    }

    #[test]
    fn kill_and_refresh_become_queries() {
        let src = "<?php\nfunction f(mysqli $db, $l, $id) {\n    mysqli_kill($l, $id);\n    $db->kill($id + 1);\n    mysqli_refresh($l, MYSQLI_REFRESH_GRANT | MYSQLI_REFRESH_TABLES);\n    $db->refresh(MYSQLI_REFRESH_LOG);\n    $db->refresh(MYSQLI_REFRESH_MASTER);\n    return MYSQLI_REFRESH_HOSTS;\n}\n";
        let (out, _) = fixed(src).unwrap();
        assert_eq!(out, "<?php\nfunction f(mysqli $db, $l, $id) {\n    mysqli_query($l, 'KILL CONNECTION ' . (int) $id);\n    $db->query('KILL CONNECTION ' . (int) ($id + 1));\n    mysqli_query($l, 'FLUSH PRIVILEGES, TABLES');\n    $db->query('FLUSH LOGS');\n    $db->refresh(MYSQLI_REFRESH_MASTER);\n    return 8;\n}\n");
        assert_eq!(kinds(src, "deprecated.mysqli_kill"), [FixKind::Likely, FixKind::Likely]);
        assert_eq!(kinds(src, "deprecated.mysqli_refresh"), [FixKind::Likely, FixKind::Likely, FixKind::Manual]);
        assert_eq!(
            kinds(src, "deprecated.mysqli_refresh_constant"),
            [FixKind::Likely, FixKind::Likely, FixKind::Likely, FixKind::Manual, FixKind::Auto]
        );
    }

    #[test]
    fn intl_functions_become_methods() {
        let src = "<?php\nintlcal_set($cal, IntlCalendar::FIELD_MONTH, 3);\n$ok = intlcal_set($cal, IntlCalendar::FIELD_YEAR, 2020);\nintlcal_set($this->cal, 2020, 1, 15);\nintlcal_set(make(), 2020, 1, 15, 10, 30);\n$x = intlcal_set($cal, 2020, 1, 15);\nintlcal_set($cal, 2020, 1, 15, 10);\n$g = [intlgregcal_create_instance(), intlgregcal_create_instance('UTC', 'tr_TR'), intlgregcal_create_instance(2020, 1, 15), intlgregcal_create_instance(2020, 1, 15, 10, 30, 45)];\n$r = lcg_value();\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\n$cal->set(IntlCalendar::FIELD_MONTH, 3);\n$ok = $cal->set(IntlCalendar::FIELD_YEAR, 2020);\n$this->cal->setDate(2020, 1, 15);\n(make())->setDateTime(2020, 1, 15, 10, 30);\n$x = intlcal_set($cal, 2020, 1, 15);\nintlcal_set($cal, 2020, 1, 15, 10);\n$g = [new \\IntlGregorianCalendar(), new \\IntlGregorianCalendar('UTC', 'tr_TR'), \\IntlGregorianCalendar::createFromDate(2020, 1, 15), \\IntlGregorianCalendar::createFromDateTime(2020, 1, 15, 10, 30, 45)];\n$r = (new \\Random\\Randomizer())->getFloat(0, 1, \\Random\\IntervalBoundary::OpenOpen);\n");
        assert_eq!(classes, [INTL_METHODS, LCG_VALUE]);
    }

    #[test]
    fn magic_quotes_checks_are_always_off() {
        let src = "<?php\nif (get_magic_quotes_gpc()) {\n    $_POST = array_map('stripslashes', $_POST);\n}\n$raw = !get_magic_quotes_runtime();\n$v = get_magic_quotes_gpc() ? stripslashes($v) : $v;\nif (get_magic_quotes_gpc()) { a(); } else { b(); }\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\n$raw = true;\n$v = false ? stripslashes($v) : $v;\nif (false) { a(); } else { b(); }\n");
        assert_eq!(classes, [MAGIC_QUOTES]);
    }

    #[test]
    fn unused_modes_and_renamed_calls() {
        let src = "<?php\nfunction g(PDO $pdo, mysqli $db, $l, $f) {\n    $rows = $pdo->query('select 1')->fetchAll(PDO::FETCH_SERIALIZE);\n    $s = $pdo->query('select 1', \\PDO::FETCH_SERIALIZE);\n    $m = PDO::FETCH_CLASS | PDO::FETCH_SERIALIZE;\n    mysqli_store_result($l, MYSQLI_STORE_RESULT_COPY_DATA);\n    $db->store_result(0);\n    $copy = MYSQLI_STORE_RESULT_COPY_DATA;\n    datefmt_set_timezone_id($f, 'UTC');\n    $d = new IntlDateFormatter('tr');\n    $d->setTimeZoneID('UTC');\n}\n";
        let (out, _) = fixed(src).unwrap();
        assert_eq!(out, "<?php\nfunction g(PDO $pdo, mysqli $db, $l, $f) {\n    $rows = $pdo->query('select 1')->fetchAll(PDO::FETCH_DEFAULT);\n    $s = $pdo->query('select 1', \\PDO::FETCH_DEFAULT);\n    $m = PDO::FETCH_CLASS | PDO::FETCH_SERIALIZE;\n    mysqli_store_result($l);\n    $db->store_result();\n    $copy = 16;\n    datefmt_set_timezone($f, 'UTC');\n    $d = new IntlDateFormatter('tr');\n    $d->setTimeZone('UTC');\n}\n");
        assert_eq!(kinds(src, "deprecated.pdo_fetch_serialize"), [FixKind::Auto, FixKind::Auto, FixKind::Manual]);
    }

    #[test]
    fn reflection_export_prints_or_returns_the_text() {
        let src = "<?php\nReflectionClass::export('ArrayIterator');\n\\ReflectionMethod::export('ArrayIterator', 'count', false);\n$s = ReflectionFunction::export('strlen', true);\n$t = ReflectionClass::export($c);\nReflectionMethod::export('A::b');\nReflectionProperty::export($c, $p, $return);\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\necho new ReflectionClass('ArrayIterator'), \"\\n\";\necho new \\ReflectionMethod('ArrayIterator', 'count'), \"\\n\";\n$s = (string) new ReflectionFunction('strlen');\n$t = ReflectionClass::export($c);\nReflectionMethod::export('A::b');\nReflectionProperty::export($c, $p, $return);\n");
        assert_eq!(classes, [REFLECTION_EXPORT]);
    }

    #[test]
    fn paged_results_and_assert_options() {
        let src = "<?php\nfunction p($l, $r, $c) {\n    ldap_control_paged_result($l, 100);\n    $ok = ldap_control_paged_result($l, 50, true, $c);\n    ldap_control_paged_result($l, size(), $crit);\n    ldap_control_paged_result_response($l, $r);\n    ldap_control_paged_result_response($l, $r, $c);\n    assert_options(ASSERT_ACTIVE, 1);\n    assert_options(ASSERT_BAIL, false);\n    assert_options(ASSERT_ACTIVE, 0);\n    $old = assert_options(ASSERT_WARNING, 1);\n    assert_options(ASSERT_QUIET_EVAL, 1);\n    return [ASSERT_EXCEPTION, ASSERT_QUIET_EVAL];\n}\n";
        let (out, _) = fixed(src).unwrap();
        assert_eq!(out, "<?php\nfunction p($l, $r, $c) {\n    ldap_set_option($l, LDAP_OPT_SERVER_CONTROLS, [['oid' => LDAP_CONTROL_PAGEDRESULTS, 'iscritical' => false, 'value' => ['size' => 100, 'cookie' => '']]]);\n    $ok = ldap_set_option($l, LDAP_OPT_SERVER_CONTROLS, [['oid' => LDAP_CONTROL_PAGEDRESULTS, 'iscritical' => true, 'value' => ['size' => 50, 'cookie' => $c]]]);\n    ldap_control_paged_result($l, size(), $crit);\n    ldap_control_paged_result_response($l, $r, $c);\n    assert_options(ASSERT_ACTIVE, 0);\n    $old = assert_options(ASSERT_WARNING, 1);\n    return [5, ASSERT_QUIET_EVAL];\n}\n");
        assert_eq!(
            kinds(src, "deprecated.assert_options"),
            [FixKind::Likely, FixKind::Likely, FixKind::Manual, FixKind::Manual, FixKind::Auto]
        );
        assert_eq!(kinds(src, "removed.assert_quiet_eval"), [FixKind::Auto, FixKind::Manual]);
    }

    #[test]
    fn removed_string_functions_get_their_ports() {
        let src = "<?php\nwhile (($l = fgetss($h, 512, '<b>')) !== false) {}\n$z = gzgetss($zp, 1024);\n$x = fgetss($h, 10, $tags);\n$t = recode_string('UTF8..latin1', $s);\n$u = recode_string('utf8..koi8-r', $s);\n$n = ezmlm_hash($address);\n$p = sql_regcase($pattern);\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\nwhile (($l = (static fn ($line) => $line === false ? false : strip_tags($line, '<b>'))(fgets($h, 512))) !== false) {}\n$z = (static fn ($line) => $line === false ? false : strip_tags($line))(gzgets($zp, 1024));\n$x = fgetss($h, 10, $tags);\n$t = iconv('UTF-8', 'ISO-8859-1//TRANSLIT//IGNORE', $s);\n$u = recode_string('utf8..koi8-r', $s);\n$n = (static function (string $address): int { $hash = 5381; for ($i = 0, $n = strlen($address); $i < $n; $i++) { $hash = (($hash + ($hash << 5)) ^ ord(strtolower($address[$i]))) & 0xFFFFFFFF; } return $hash % 53; })($address);\n$p = preg_replace_callback('/[a-zA-Z]/', static fn ($m) => '[' . strtoupper($m[0]) . strtolower($m[0]) . ']', $pattern);\n");
        assert_eq!(classes, [GETSS, RECODE, EZMLM, REGCASE]);
        assert_eq!(fixed("<?php\nsetlocale(LC_ALL, 'tr_TR');\n$p = sql_regcase($pattern);\n"), None);
        assert!(fixed("<?php\nsetlocale(LC_MONETARY, 'tr_TR');\n$p = sql_regcase($pattern);\n").is_some());
    }

    #[test]
    fn recompress_level_and_curl_array_items() {
        let src = "<?php\n$zip->setCompressionName('a.txt', ZipArchive::CM_DEFLATE, ZipArchive::FL_RECOMPRESS);\n$zip->setCompressionIndex(0, ZipArchive::CM_STORE, \\ZipArchive::FL_RECOMPRESS);\n$flags = ZipArchive::FL_RECOMPRESS;\ncurl_setopt_array($c, [CURLOPT_RETURNTRANSFER => true, CURLOPT_BINARYTRANSFER => true]);\ncurl_setopt_array($c, array(CURLOPT_BINARYTRANSFER => 1, CURLOPT_URL => $u));\ncurl_setopt_array($c, [CURLOPT_BINARYTRANSFER => true]);\n$opts = [CURLOPT_BINARYTRANSFER => true];\n";
        let (out, _) = fixed(src).unwrap();
        assert_eq!(out, "<?php\n$zip->setCompressionName('a.txt', ZipArchive::CM_DEFLATE);\n$zip->setCompressionIndex(0, ZipArchive::CM_STORE);\n$flags = ZipArchive::FL_RECOMPRESS;\ncurl_setopt_array($c, [CURLOPT_RETURNTRANSFER => true]);\ncurl_setopt_array($c, array(CURLOPT_URL => $u));\ncurl_setopt_array($c, []);\n$opts = [CURLOPT_BINARYTRANSFER => true];\n");
        assert_eq!(kinds(src, "deprecated.zip_fl_recompress"), [FixKind::Likely, FixKind::Likely, FixKind::Manual]);
    }

    #[test]
    fn a_fix_inside_a_removed_block_is_not_counted() {
        let src = "<?php\nif (get_magic_quotes_gpc()) {\n    $h = mhash(MHASH_MD5, $x);\n}\n";
        assert_eq!(fixed(src).unwrap().0, "<?php\n");
        assert_eq!(kinds(src, "deprecated.mhash"), [FixKind::Manual]);
    }

    #[test]
    fn project_definitions_and_other_versions_are_left_alone() {
        assert_eq!(fixed("<?php\nconst MYSQLI_NO_DATA = 7;\necho MYSQLI_NO_DATA;\n"), None);
        assert_eq!(fixed("<?php\nfunction ocilogon() {}\nocilogon();\n"), None);
        assert_eq!(fixed("<?php\necho 'MYSQLI_NO_DATA', $o->MYSQLI_NO_DATA, Foo::MYSQLI_NO_DATA;\n"), None);
        assert_eq!(fixed_for("<?php\necho MYSQLI_NO_DATA, DOM_PHP_ERR;\n", PhpVersion::new(8, 0)), None);
        assert_eq!(
            fixed_for("<?php\necho MYSQLI_NO_DATA, DOM_PHP_ERR;\n", PhpVersion::new(8, 1)).unwrap().0,
            "<?php\necho 100, DOM_PHP_ERR;\n"
        );
    }

    fn fixed_from(src: &str, current: PhpVersion) -> Option<(String, Vec<&'static str>)> {
        let target = PhpVersion::new(8, 5);
        let index = crate::index::ProjectIndex::from_tokens_for(&crate::lexer::tokenize(src), Some(target)).with_current(current);
        apply("t.php", src, target, &std::sync::Arc::new(index))
    }

    fn fixes_from(src: &str, current: PhpVersion) -> Vec<(String, FixKind)> {
        let target = PhpVersion::new(8, 5);
        let index = crate::index::ProjectIndex::from_tokens_for(&crate::lexer::tokenize(src), Some(target)).with_current(current);
        crate::analyzer::analyze_source_in("t.php", src, target, &index).into_iter().map(|f| (f.rule_id, f.fix)).collect()
    }

    fn kinds_of(found: &[(String, FixKind)], rule: &str) -> Vec<FixKind> {
        found.iter().filter(|(r, _)| r == rule).map(|(_, k)| *k).collect()
    }

    #[test]
    fn null_arguments_take_the_empty_value() {
        let src = "<?php\nstrlen(null);\nstr_replace('a', 'b', null);\nhtmlspecialchars(string: null);\n\\strpos('abc', NULL);\necho number_format(null);\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(
            out,
            "<?php\nstrlen('');\nstr_replace('a', 'b', '');\nhtmlspecialchars(string: '');\n\\strpos('abc', '');\necho number_format(0);\n"
        );
        assert_eq!(classes, [NULL_ARGUMENT]);
        assert_eq!(kinds_of(&fixes(src), "deprecated.null_to_internal"), [FixKind::Auto; 5]);
        let strict = "<?php declare(strict_types=1);\nstrlen(null);\n";
        assert_eq!(fixed(strict), None, "null was already a TypeError there");
        assert_eq!(kinds_of(&fixes(strict), "deprecated.null_to_internal"), [FixKind::Manual]);
    }

    #[test]
    fn tentative_return_types_get_the_attribute() {
        let src = "<?php\nclass C implements \\Countable {\n    /** Items. */\n    public function count() { return 0; }\n}\nclass D implements \\Countable { public function count() { return 1; } }\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(
            out,
            "<?php\nclass C implements \\Countable {\n    /** Items. */\n    #[\\ReturnTypeWillChange]\n    public function count() { return 0; }\n}\nclass D implements \\Countable { #[\\ReturnTypeWillChange]\npublic function count() { return 1; } }\n"
        );
        assert_eq!(classes, [WILL_CHANGE]);
        assert_eq!(kinds_of(&fixes(src), "deprecated.tentative_return_type"), [FixKind::Auto; 2]);
        assert!(kinds_of(&fixes(&out), "deprecated.tentative_return_type").is_empty());
        let crlf = "<?php\r\nclass C implements \\Countable {\r\n\tpublic function count() { return 0; }\r\n}\r\n";
        assert_eq!(
            fixed(crlf).unwrap().0,
            "<?php\r\nclass C implements \\Countable {\r\n\t#[\\ReturnTypeWillChange]\r\n\tpublic function count() { return 0; }\r\n}\r\n"
        );
    }

    #[test]
    fn ai_idn_flags_go() {
        let src = "<?php\n$hints = AI_IDN_ALLOW_UNASSIGNED;\n$h = ['ai_flags' => AI_CANONNAME | AI_IDN_USE_STD3_ASCII_RULES];\n$b = AI_IDN_ALLOW_UNASSIGNED | AI_IDN_USE_STD3_ASCII_RULES;\nif ($f & AI_IDN_ALLOW_UNASSIGNED) {}\nif ($f === AI_IDN_USE_STD3_ASCII_RULES) {}\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(
            out,
            "<?php\n$hints = 0;\n$h = ['ai_flags' => AI_CANONNAME];\n$b = 0;\nif ($f & AI_IDN_ALLOW_UNASSIGNED) {}\nif ($f === AI_IDN_USE_STD3_ASCII_RULES) {}\n"
        );
        assert_eq!(classes, [AI_IDN_FLAG]);
        let kinds = kinds_of(&fixes(src), "removed.ai_idn_constant");
        assert_eq!(kinds, [FixKind::Likely, FixKind::Likely, FixKind::Likely, FixKind::Likely, FixKind::Manual, FixKind::Manual]);
    }

    #[test]
    fn embedded_server_calls_go() {
        let src = "<?php\nmysqli_embedded_server_start(1, ['--datadir=/tmp'], array());\nif (!mysqli_embedded_server_start(true, [], [])) { die(); }\nmysqli_embedded_server_start(1, options(), []);\n\\mysqli_embedded_server_end();\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(
            out,
            "<?php\nif (!mysqli_embedded_server_start(true, [], [])) { die(); }\nmysqli_embedded_server_start(1, options(), []);\n"
        );
        assert_eq!(classes, [EMBEDDED_SERVER]);
        let kinds = kinds_of(&fixes(src), "removed.mysqli_embedded_server");
        assert_eq!(kinds, [FixKind::Likely, FixKind::Manual, FixKind::Manual, FixKind::Likely]);
    }

    #[test]
    fn stray_breaks_return() {
        let src = "<?php\nfunction f($r) {\n    if ($r) { throw new E(); break; }\n}\n$c = function () use ($x): void { if ($x) { continue; } };\nfunction g(): int { if (h()) { break; } return 1; }\nfunction &r() { break; }\nfunction n() { if ($x) { break 2; } }\nif ($x) { break; }\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(
            out,
            "<?php\nfunction f($r) {\n    if ($r) { throw new E(); return; }\n}\n$c = function () use ($x): void { if ($x) { return; } };\nfunction g(): int { if (h()) { break; } return 1; }\nfunction &r() { break; }\nfunction n() { if ($x) { break 2; } }\nif ($x) { break; }\n"
        );
        assert_eq!(classes, [BREAK_RETURN]);
        let kinds = kinds_of(&fixes(src), "removed.break_outside_loop");
        assert_eq!(kinds, [FixKind::Likely, FixKind::Likely, FixKind::Manual, FixKind::Manual, FixKind::Manual, FixKind::Manual]);
    }

    #[test]
    fn idna_2003_becomes_uts46() {
        let src = "<?php\nnamespace App;\necho idn_to_ascii($d, IDNA_DEFAULT, INTL_IDNA_VARIANT_2003), \\INTL_IDNA_VARIANT_2003;\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(
            out,
            "<?php\nnamespace App;\necho idn_to_ascii($d, IDNA_DEFAULT, INTL_IDNA_VARIANT_UTS46), \\INTL_IDNA_VARIANT_UTS46;\n"
        );
        assert_eq!(classes, [IDNA_UTS46]);
        assert_eq!(kinds_of(&fixes(src), "removed.intl_idna_variant_2003"), [FixKind::Likely; 2]);
    }

    #[test]
    fn substr_false_needs_a_php5_project() {
        let src = "<?php\nif (substr($p, strrpos($p, '.') + 1) === false) {}\n$a = false !== substr($s, 3);\n$d = @substr($s, 1) === FALSE;\n$b = substr($s, 1, 2) === false;\n$c = !substr($s, 1) === false;\n";
        let (out, classes) = fixed_from(src, PhpVersion::new(5, 6)).unwrap();
        assert_eq!(
            out,
            "<?php\nif (substr($p, strrpos($p, '.') + 1) === '') {}\n$a = '' !== substr($s, 3);\n$d = @substr($s, 1) === '';\n$b = substr($s, 1, 2) === false;\n$c = !substr($s, 1) === false;\n"
        );
        assert_eq!(classes, [SUBSTR_EMPTY]);
        let kinds = kinds_of(&fixes_from(src, PhpVersion::new(5, 6)), "behavior.substr_false");
        assert_eq!(kinds, [FixKind::Likely, FixKind::Likely, FixKind::Likely, FixKind::Manual, FixKind::Manual]);
        assert_eq!(fixed_from(src, PhpVersion::new(7, 4)), None);
        assert_eq!(fixed(src), None);
        assert!(kinds_of(&fixes(src), "behavior.substr_false").iter().all(|k| *k == FixKind::Manual));
    }

    #[test]
    fn indirect_access_keeps_the_php5_order() {
        let src = "<?php\necho $this->store->$map['column'];\nFoo::$bar['baz']();\n";
        let php5 = PhpVersion::new(5, 6);
        let (out, classes) = crate::analyzer::fix_legacy_syntax_with(src, &HashSet::new(), PhpVersion::new(8, 5), Some(php5)).unwrap();
        assert_eq!(out, "<?php\necho $this->store->{$map['column']};\nFoo::{$bar['baz']}();\n");
        assert_eq!(classes, [crate::analyzer::INDIRECT_ACCESS_FIX]);
        let at_position = by_position(&crate::lexer::tokenize(src));
        let tokens = crate::lexer::tokenize(src);
        let findings = crate::analyzer::analyze_source("t.php", src, PhpVersion::new(8, 5));
        let file = Source::new(src, &tokens, &findings, &at_position, Some(php5));
        let marked: String = plan(&file, &findings, &at_position).into_iter().rev().fold(src.to_string(), |mut text, (_, fix)| {
            text.replace_range(fix.start..fix.end, &fix.text);
            text
        });
        assert_eq!(marked, out, "the fix that marks the finding is the one staging applies");
        assert_eq!(kinds_of(&fixes_from(src, php5), "behavior.indirect_access_order"), [FixKind::Likely; 2]);
        assert_eq!(kinds_of(&fixes_from(src, PhpVersion::new(7, 0)), "behavior.indirect_access_order"), [FixKind::Manual; 2]);
    }

    #[test]
    fn variable_variables_get_braces() {
        let src = "<?php\n$$name['k'] = 1;\necho $$a['b']['c'], $$o['p']->q;\n$$e[] = 1;\n";
        let (php5, classes) = fixed_from(src, PhpVersion::new(5, 6)).unwrap();
        assert_eq!(php5, "<?php\n${$name['k']} = 1;\necho ${$a['b']['c']}, ${$o['p']}->q;\n$$e[] = 1;\n");
        assert_eq!(classes, [VARIABLE_ORDER]);
        let kinds = kinds_of(&fixes_from(src, PhpVersion::new(5, 6)), "review.variable_variable");
        assert_eq!(kinds, [FixKind::Likely, FixKind::Likely, FixKind::Likely, FixKind::Manual]);
        let (php7, classes) = fixed_from(src, PhpVersion::new(7, 4)).unwrap();
        assert_eq!(php7, "<?php\n${$name}['k'] = 1;\necho ${$a}['b']['c'], ${$o}['p']->q;\n${$e}[] = 1;\n");
        assert_eq!(classes, [VARIABLE_BRACES]);
        assert_eq!(kinds_of(&fixes_from(src, PhpVersion::new(7, 4)), "review.variable_variable"), [FixKind::Auto; 4]);
        assert_eq!(fixed(src), None);
    }
}
