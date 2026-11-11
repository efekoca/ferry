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
pub const FALSE_ARRAY: &str = "Ferry\\Php81\\FalseToEmptyArray";
pub const GMDATE: &str = "Ferry\\Php81\\GmstrftimeToGmdate";
pub const LAST_HEADERS: &str = "Ferry\\Php85\\LastResponseHeaders";
pub const NULL_KEY: &str = "Ferry\\Php85\\NullOffsetToEmptyString";
pub const FIRST_CLASS_CALLABLE: &str = "Ferry\\Php82\\FirstClassCallable";
pub const DIRECTORY_HANDLE: &str = "Ferry\\Php85\\PassDirectoryHandle";
pub const ORIGINAL_ARGS: &str = "Ferry\\Php70\\KeepOriginalArguments";
pub const WAKEUP_CALL: &str = "Ferry\\Php84\\RemoveSplFixedArrayWakeup";
pub const REFLECTION_TYPE_CHECK: &str = "Ferry\\Php80\\ReflectionParameterTypeCheck";
pub const DISABLED_FALSE: &str = "Ferry\\Php80\\IsDisabledToFalse";
pub const XML_CALLABLE: &str = "Ferry\\Php84\\XmlHandlerCallable";
pub const XML_OBJECT: &str = "Ferry\\Php84\\RemoveXmlSetObject";
pub const SESSION_HANDLER: &str = "Ferry\\Php84\\SessionHandlerObject";
pub const AUTOLOAD_REGISTER: &str = "Ferry\\Php80\\AutoloadToSplRegister";
pub const WBMP_PORT: &str = "Ferry\\Php80\\InlineWbmpConversion";
pub const WIDEN_OVERRIDE: &str = "Ferry\\Php80\\WidenBuiltinOverride";
pub const RFC_LITERAL: &str = "Ferry\\Php85\\Rfc7231Literal";
pub const RFC_UTC: &str = "Ferry\\Php85\\Rfc7231InUtc";
pub const DATE_WAKEUP_DROP: &str = "Ferry\\Php85\\RemoveDateWakeup";
pub const BAD_STATE_DROP: &str = "Ferry\\Php82\\RemoveBadStateCall";
pub const KEEP_ERROR: &str = "Ferry\\Php82\\ThrowSameError";
pub const USER_ERROR_EXCEPTION: &str = "Ferry\\Php84\\UserErrorToException";
pub const USER_ERROR_WARNING: &str = "Ferry\\Php84\\UserErrorToWarningAndExit";
pub const INPUT_OLD: &str = "Ferry\\Php80\\InputTypeOldResult";
pub const INPUT_GLOBAL: &str = "Ferry\\Php80\\InputTypeSuperglobal";
pub const SOAP_ALL: &str = "Ferry\\Php84\\SoapDefinedFunctions";
pub const SOAP_USER: &str = "Ferry\\Php84\\SoapUserFunctions";
pub const CONCAT_GROUPS: &str = "Ferry\\Php80\\ParenthesizeConcatOperands";
pub const HTML_OLD: &str = "Ferry\\Php81\\KeepHtmlFlags";
pub const HTML_NEW: &str = "Ferry\\Php81\\ExplicitHtmlFlags";
pub const MYSQLI_REPORT_OFF: &str = "Ferry\\Php81\\MysqliReportOff";
pub const MYSQLI_CATCH: &str = "Ferry\\Php81\\CatchMysqliException";
pub const PING_QUERY: &str = "Ferry\\Php84\\PingWithQuery";
pub const PING_DROP: &str = "Ferry\\Php84\\DropPingCheck";
pub const CURRENCY_METHOD: &str = "Ferry\\Php80\\CurrencyMethods";
pub const CURRENCY_FALSE: &str = "Ferry\\Php80\\CurrencyCallOldResult";
pub const MAGIC_NULL_EXACT: &str = "Ferry\\Php80\\MagicVariableNeverSet";
pub const MAGIC_NULL: &str = "Ferry\\Php80\\MagicVariableOldValue";
pub const MAGIC_READ: &str = "Ferry\\Php80\\MagicVariableSource";
pub const ENTITY_DROP: &str = "Ferry\\Php80\\DropEntityLoaderCall";
pub const ENTITY_LOADER: &str = "Ferry\\Php80\\NullEntityLoader";

/// A finding the user asked to fix with one option: its line, its rule, which of that rule's
/// findings on the line it is, and the option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chosen {
    pub line: usize,
    pub rule: String,
    pub ordinal: usize,
    pub option: String,
}

/// One of several ways to fix a finding, when they keep different things.
struct Choice {
    id: &'static str,
    fix: Fix,
    recommended: bool,
}

/// Rules whose findings can come with options.
const OPTION_RULES: [&str; 12] = [
    "behavior.html_default_flags",
    "behavior.mysqli_default_exceptions",
    "deprecated.mysqli_ping",
    "removed.numberformatter_type_currency",
    "removed.magic_variable",
    "deprecated.libxml_disable_entity_loader",
    "deprecated.date_rfc7231",
    "deprecated.splfileinfo_bad_state_ex",
    "deprecated.e_user_error",
    "removed.input_request",
    "removed.input_session",
    "deprecated.soap_functions_all",
];

/// (rule, option, title, what it changes).
const OPTION_TEXTS: &[(&str, &str, crate::i18n::Text, crate::i18n::Text)] = &[
    (
        "behavior.html_default_flags",
        "old",
        crate::i18n::Text::new("Eski çıktıyı koru", "Keep the old output"),
        crate::i18n::Text::new(
            "Bu dosyadaki bayraksız çağrılara ENT_COMPAT | ENT_HTML401 eklenir; çıktı PHP 8.1 öncesiyle bayt bayt aynı kalır. Tek tırnaklı HTML özniteliklerindeki eski XSS açığı da kalır.",
            "Adds ENT_COMPAT | ENT_HTML401 to the calls without flags in this file; the output stays byte for byte as before PHP 8.1. So does the old XSS gap inside single-quoted HTML attributes.",
        ),
    ),
    (
        "behavior.html_default_flags",
        "new",
        crate::i18n::Text::new("PHP 8.1 bayraklarını yaz", "Write the PHP 8.1 flags"),
        crate::i18n::Text::new(
            "ENT_QUOTES | ENT_SUBSTITUTE | ENT_HTML401 açıkça yazılır. PHP 8.1 ve sonrasında çıktı değişmez, ama eskisinden farklıdır: tek tırnak kodlanır, geçersiz UTF-8 boş sonuç yerine � olur.",
            "Writes ENT_QUOTES | ENT_SUBSTITUTE | ENT_HTML401 out. Nothing changes on PHP 8.1 and later, but the output differs from before: single quotes are encoded and invalid UTF-8 becomes � instead of an empty result.",
        ),
    ),
    (
        "behavior.mysqli_default_exceptions",
        "report_off",
        crate::i18n::Text::new("Eski hata davranışı", "Old error handling"),
        crate::i18n::Text::new(
            "Çağrıdan önce mysqli_report(MYSQLI_REPORT_OFF) yazılır; hatalar yine uyarıyla false döndürür ve bu kontrol çalışır. Ayar istek boyunca bütün mysqli çağrılarını etkiler.",
            "Writes mysqli_report(MYSQLI_REPORT_OFF) before the call, so failures return false with a warning again and this check runs. The setting affects every mysqli call in the request.",
        ),
    ),
    (
        "behavior.mysqli_default_exceptions",
        "catch",
        crate::i18n::Text::new("İstisnayı yakala", "Catch the exception"),
        crate::i18n::Text::new(
            "Çağrı try içine alınır, mysqli_sql_exception gelince bu dal çalışır. Başka yerdeki denetlenmeyen mysqli hataları istisna fırlatmaya devam eder.",
            "Wraps the call in try and runs this branch on mysqli_sql_exception. Unchecked mysqli failures elsewhere keep throwing.",
        ),
    ),
    (
        "deprecated.mysqli_ping",
        "query",
        crate::i18n::Text::new("Sunucuya sor", "Ask the server"),
        crate::i18n::Text::new(
            "Aynı tek gidiş-dönüşü 'DO 1' sorgusuyla yapar: bağlantı canlıysa true. ping()'in aksine affected_rows, insert_id ve info değerlerini sıfırlar.",
            "Makes the same single round trip with 'DO 1': true while the connection lives. Unlike ping(), it resets affected_rows, insert_id and info.",
        ),
    ),
    (
        "deprecated.mysqli_ping",
        "drop",
        crate::i18n::Text::new("Kontrolü kaldır", "Drop the check"),
        crate::i18n::Text::new(
            "Kopan bağlantı PHP 8.2'den beri kendiliğinden yeniden kurulmuyor; bir sonraki sorgu hata verir. Gerekirse bağlantıyı bir catch içinde yeniden kurun.",
            "Dropped connections haven't reconnected on their own since PHP 8.2; the next query fails instead. Reconnect in a catch if needed.",
        ),
    ),
    (
        "removed.numberformatter_type_currency",
        "currency",
        crate::i18n::Text::new("Para birimi metodu", "Currency method"),
        crate::i18n::Text::new(
            "Kastedilen buydu: formatCurrency() ya da parseCurrency() biçimlendiricinin para birimiyle çalışır.",
            "What was meant: formatCurrency() or parseCurrency() with the formatter's currency.",
        ),
    ),
    (
        "removed.numberformatter_type_currency",
        "false",
        crate::i18n::Text::new("Eski sonuç", "Old result"),
        crate::i18n::Text::new(
            "PHP 7 bu çağrıda uyarı verip false döndürüyordu; aynı false yazılır.",
            "PHP 7 warned and returned false for this call; writes that false.",
        ),
    ),
    (
        "removed.magic_variable",
        "old",
        crate::i18n::Text::new("Eski değer", "Old value"),
        crate::i18n::Text::new(
            "Kodun gördüğü değer olan null yazılır; tanımsız değişken uyarısı kalkar.",
            "Writes the null the code saw; the undefined variable warning goes.",
        ),
    ),
    (
        "removed.magic_variable",
        "read",
        crate::i18n::Text::new("Asıl veriyi oku", "Read the actual data"),
        crate::i18n::Text::new(
            "İstek gövdesi php://input'tan, son hata error_get_last()'tan okunur. Bu kod şimdiye kadar null görmüş olabilir; error_get_last() isteğin herhangi bir yerindeki son hatayı verir.",
            "Reads the request body from php://input and the last error from error_get_last(). This code may have only ever seen null; error_get_last() gives the last error anywhere in the request.",
        ),
    ),
    (
        "deprecated.libxml_disable_entity_loader",
        "drop",
        crate::i18n::Text::new("Çağrıyı sil", "Delete the call"),
        crate::i18n::Text::new(
            "PHP 8'in varsayılanı geçerli olur: dış varlıklar yalnızca LIBXML_NOENT verildiğinde yüklenir ve dosya yüklemeleri çalışmaya başlar.",
            "PHP 8's default applies: external entities load only when LIBXML_NOENT is passed, and file loads start to work.",
        ),
    ),
    (
        "deprecated.libxml_disable_entity_loader",
        "loader",
        crate::i18n::Text::new("Dış varlıkları engelle", "Keep blocking external entities"),
        crate::i18n::Text::new(
            "Dış varlık yükleyicisi null döndüren bir fonksiyonla değiştirilir; LIBXML_NOENT kullanan kod da dış varlık yükleyemez. Eski ayarı geri yükleyen çağrı yükleyiciyi sıfırlar.",
            "Replaces the external entity loader with one that returns null, so code that passes LIBXML_NOENT can't load external entities either. The call that restored the old setting resets the loader.",
        ),
    ),
    (
        "deprecated.date_rfc7231",
        "literal",
        crate::i18n::Text::new("Aynı çıktı", "Same output"),
        crate::i18n::Text::new(
            "Sabitin tuttuğu biçim yazılır; yerel saat yine GMT etiketiyle çıkar.",
            "Writes the format the constant held; local time is still labelled GMT.",
        ),
    ),
    (
        "deprecated.date_rfc7231",
        "utc",
        crate::i18n::Text::new("Gerçek GMT saati", "Real GMT time"),
        crate::i18n::Text::new(
            "Saat önce UTC'ye çevrilir. Varsayılan saat dilimi ya da nesnenin saat dilimi UTC değilse çıktı değişir.",
            "The time is converted to UTC first. The output changes when the default or the object's time zone isn't UTC.",
        ),
    ),
    (
        "deprecated.splfileinfo_bad_state_ex",
        "remove",
        crate::i18n::Text::new("Çağrıyı sil", "Delete the call"),
        crate::i18n::Text::new(
            "Büyük olasılıkla kastedilen budur; çağrıdan sonraki kod artık çalışır.",
            "Likely what was meant; the code after the call now runs.",
        ),
    ),
    (
        "deprecated.splfileinfo_bad_state_ex",
        "throw",
        crate::i18n::Text::new("Hatayı koru", "Keep the error"),
        crate::i18n::Text::new(
            "Çağrının PHP 8'de her zaman fırlattığı Error açıkça fırlatılır.",
            "Throws the Error the call always threw on PHP 8.",
        ),
    ),
    (
        "deprecated.e_user_error",
        "exception",
        crate::i18n::Text::new("ErrorException fırlat", "Throw ErrorException"),
        crate::i18n::Text::new(
            "PHP'nin önerisi. Yakalanmazsa yine ölümcül hata yazar ve 255 ile çıkar; ama set_error_handler() artık görmez, catch blokları yakalar.",
            "PHP's suggestion. Uncaught, it still prints a fatal error and exits with 255, but set_error_handler() no longer sees it and catch blocks do.",
        ),
    ),
    (
        "deprecated.e_user_error",
        "warning",
        crate::i18n::Text::new("Uyarı ver ve çık", "Warn and exit"),
        crate::i18n::Text::new(
            "Hata işleyicileri ve kayıtlar mesajı uyarı olarak görmeye devam eder; betik yine 255 ile durur.",
            "Error handlers and logs still get the message, as a warning, and the script still stops with 255.",
        ),
    ),
    (
        "removed.input_request",
        "old",
        crate::i18n::Text::new("Eski sonuç", "Old result"),
        crate::i18n::Text::new(
            "Çağrının her zaman döndürdüğü değer yazılır; yalnızca uyarı kalkar.",
            "Writes what the call always returned; only the warning goes.",
        ),
    ),
    (
        "removed.input_request",
        "superglobal",
        crate::i18n::Text::new("$_REQUEST'i oku", "Read $_REQUEST"),
        crate::i18n::Text::new(
            "Bu kod şimdiye kadar hep boş değer gördü; artık GET, POST ve çerez verisi gelir. Güvenlik açısından gözden geçirin.",
            "This code has only ever seen an empty value; GET, POST and cookie data now flow in. Review it for security.",
        ),
    ),
    (
        "removed.input_session",
        "old",
        crate::i18n::Text::new("Eski sonuç", "Old result"),
        crate::i18n::Text::new(
            "Çağrının her zaman döndürdüğü değer yazılır; yalnızca uyarı kalkar.",
            "Writes what the call always returned; only the warning goes.",
        ),
    ),
    (
        "removed.input_session",
        "superglobal",
        crate::i18n::Text::new("$_SESSION'ı oku", "Read $_SESSION"),
        crate::i18n::Text::new(
            "Bu kod şimdiye kadar hep boş değer gördü; artık oturum verisi gelir ve bunun için oturum başlamış olmalı. Güvenlik açısından gözden geçirin.",
            "This code has only ever seen an empty value; session data now flows in, which needs a started session. Review it for security.",
        ),
    ),
    (
        "deprecated.soap_functions_all",
        "all",
        crate::i18n::Text::new("Tanımlı tüm fonksiyonlar", "Every defined function"),
        crate::i18n::Text::new(
            "PHP'nin önerisi: o an tanımlı bütün fonksiyonlar açılır; sonradan tanımlananlar çağrılamaz.",
            "PHP's suggestion: every function defined at that point is exposed; ones defined later can't be called.",
        ),
    ),
    (
        "deprecated.soap_functions_all",
        "user",
        crate::i18n::Text::new("Yalnızca projenin fonksiyonları", "Only the project's functions"),
        crate::i18n::Text::new(
            "Daha güvenli: PHP'nin kendi fonksiyonları dışarı açılmaz. Servis onlara güveniyorsa çalışmaz.",
            "Safer: PHP's own functions aren't exposed. A service that relied on them stops working.",
        ),
    ),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub class: &'static str,
    pub start: usize,
    pub end: usize,
    pub text: String,
    /// More edits the same fix needs elsewhere in the file, each `(start, end, text)`.
    pub extra: Vec<(usize, usize, String)>,
}

impl Fix {
    /// Every edit of the fix, the main one first.
    fn edits(&self) -> impl Iterator<Item = (usize, usize, &str)> {
        std::iter::once((self.start, self.end, self.text.as_str())).chain(self.extra.iter().map(|(s, e, t)| (*s, *e, t.as_str())))
    }
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
const SITE_RULES: [&str; 38] = [
    "behavior.concat_precedence",
    "behavior.html_default_flags",
    "behavior.mysqli_default_exceptions",
    "deprecated.mysqli_ping",
    "removed.numberformatter_type_currency",
    "removed.magic_variable",
    "deprecated.libxml_disable_entity_loader",
    "deprecated.date_rfc7231",
    "deprecated.splfileinfo_bad_state_ex",
    "deprecated.e_user_error",
    "removed.input_request",
    "removed.input_session",
    "deprecated.soap_functions_all",
    "deprecated.date_wakeup",
    "removed.incompatible_override",
    "removed.png2wbmp",
    "removed.jpeg2wbmp",
    "deprecated.splfixedarray_wakeup",
    "deprecated.reflection_method",
    "deprecated.xml_handler_string",
    "deprecated.xml_set_object",
    "deprecated.session_set_save_handler",
    "removed.__autoload",
    "deprecated.false_to_array",
    "deprecated.gmstrftime",
    "deprecated.http_response_header",
    "deprecated.null_offset",
    "deprecated.partial_callable",
    "deprecated.implicit_argument",
    "behavior.func_get_args_current_value",
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
        "false",
        "gmstrftime",
        "http_response_header",
        "self",
        "parent",
        "static",
        "readdir",
        "rewinddir",
        "closedir",
        "func_get_arg",
        "__wakeup",
        "isarray",
        "iscallable",
        "isdisabled",
        "xml_set_",
        "session_set_save_handler",
        "__autoload",
        "2wbmp",
        "e_user_error",
        "input_request",
        "input_session",
        "soap_functions_all",
        "rfc7231",
        "_bad_state_ex",
        "htmlspecialchars",
        "htmlentities",
        "html_entity_decode",
        "mysqli",
        "ping",
        "type_currency",
        "http_raw_post_data",
        "php_errormsg",
        "libxml_disable_entity_loader",
    ];
    OCI_ALIASES.iter().map(|(old, _)| *old).chain(NO_OP_CALLS.iter().map(|(_, f, _)| *f)).chain(calls).any(|name| lower.contains(name))
        || (lower.contains("unset") && src.contains("$this"))
        || (lower.contains("export") && lower.contains("reflection"))
        || src.contains('.') && ["+", "-", "<<", ">>"].iter().any(|op| src.contains(op))
}

/// A file as the fixes see it: its text, its tokens, which tokens hold a no-op flag finding, the
/// version the project runs on today and whether the file declares strict types.
pub(crate) struct Source<'a> {
    pub(crate) src: &'a str,
    pub(crate) tokens: &'a [Token],
    pub(crate) flagged: HashSet<usize>,
    current: Option<PhpVersion>,
    strict: bool,
    index: &'a crate::index::ProjectIndex,
    target: PhpVersion,
}

impl<'a> Source<'a> {
    fn new(
        src: &'a str,
        tokens: &'a [Token],
        findings: &[Finding],
        at_position: &std::collections::HashMap<(usize, usize), usize>,
        index: &'a crate::index::ProjectIndex,
        target: PhpVersion,
    ) -> Self {
        let flagged = findings
            .iter()
            .filter(|f| NO_OP_FLAGS.iter().any(|(r, ..)| *r == f.rule_id))
            .filter_map(|f| at_position.get(&(f.line, f.column)).copied())
            .collect();
        let strict = tokens.windows(3).any(|w| w[0].is_ident_ci("strict_types") && w[1].is_punct("=") && w[2].text == "1");
        Source { src, tokens, flagged, current: index.current(), strict, index, target }
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
    let fix = |class, start, text: String| Some(Fix { extra: Vec::new(), class, start, end: tok.end, text });
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
        return (call && !member).then(|| Fix {
            extra: Vec::new(),
            class: OCI_ALIAS,
            start: tok.start,
            end: tok.end,
            text: format!("{lead}{new}"),
        });
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
    let replace =
        |text: &str| Some(Fix { extra: Vec::new(), class, start: tokens[first].start, end: tokens[last].end, text: text.to_string() });
    match alone {
        Alone::Value(_) if tokens[first..=last].iter().any(|t| t.is_ident_ci("E_STRICT")) => None,
        Alone::Value(value) => replace(value),
        Alone::Constant(name) => {
            let stored = before.is_some_and(|b| [",", "=>", "=", "("].iter().any(|p| b.is_punct(p)));
            let ends = after.is_some_and(|a| [")", ",", "]", ";"].iter().any(|p| a.is_punct(p)));
            (stored && ends).then(|| Fix {
                extra: Vec::new(),
                class,
                start: tokens[first].start,
                end: tokens[last].end,
                text: name.to_string(),
            })
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
                extra: Vec::new(),
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
                extra: Vec::new(),
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
    (text != original).then(|| Fix { extra: Vec::new(), class, start: tokens[first].start, end: tokens[last].end, text })
}

fn site_fix(file: &Source, k: usize, rule: &str) -> Option<Fix> {
    let tokens = file.tokens;
    let tok = tokens.get(k)?;
    let php5 = file.current.is_some_and(|v| v < PhpVersion::new(7, 0));
    let replace = |class, text: String| Some(Fix { extra: Vec::new(), class, start: tok.start, end: tok.end, text });
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
                extra: Vec::new(),
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
                extra: Vec::new(),
                class: crate::analyzer::INDIRECT_ACCESS_FIX,
                start: tok.start,
                end: tokens[end].end,
                text: format!("{{{}}}", file.text(k, end)),
            })
        }
        "review.variable_variable" => variable_variable(file, k),
        "deprecated.false_to_array" => false_to_array(file, k),
        "deprecated.gmstrftime" => gmstrftime_fix(file, k),
        "deprecated.http_response_header" => last_headers(file, k),
        "deprecated.null_offset" => null_key(file, k),
        "deprecated.partial_callable" => first_class_callable(file, k),
        "deprecated.implicit_argument" => directory_handle(file, k),
        "behavior.func_get_args_current_value" => original_args(file, k).filter(|_| php5),
        "deprecated.splfixedarray_wakeup" => wakeup_call(file, k),
        "deprecated.reflection_method" => reflection_check(file, k),
        "deprecated.xml_handler_string" => xml_handler(file, k),
        "deprecated.xml_set_object" => xml_object(file, k),
        "deprecated.session_set_save_handler" => session_handler(file, k),
        "removed.__autoload" => autoload_register(file, k),
        "removed.png2wbmp" => wbmp_port(file, k, "png2wbmp", "imagecreatefrompng", "PNG"),
        "removed.jpeg2wbmp" => wbmp_port(file, k, "jpeg2wbmp", "imagecreatefromjpeg", "JPEG"),
        "removed.incompatible_override" => widen_override(file, k),
        "deprecated.date_wakeup" => date_wakeup(file, k),
        "behavior.concat_precedence" => concat_groups(file, k),
        _ if OPTION_RULES.contains(&rule) => choices(file, k, rule).into_iter().find(|c| c.recommended).map(|c| c.fix),
        _ => None,
    }
}

/// `$date->__wakeup();` on a live date object, when no class of the project extends one.
fn date_wakeup(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let call = tokens[k].text.eq_ignore_ascii_case("__wakeup")
        && tokens.get(k + 1).is_some_and(|t| t.is_punct("("))
        && tokens.get(k + 2).is_some_and(|t| t.is_punct(")"))
        && tokens.get(k + 3).is_some_and(|t| t.is_punct(";"))
        && k.checked_sub(1).is_some_and(|p| tokens[p].is_punct("->"));
    let first = receiver_start(tokens, k.checked_sub(1)?).filter(|_| call)?;
    let extended =
        ["datetime", "datetimeimmutable", "datetimezone", "dateinterval", "dateperiod"].iter().any(|c| file.index.extends_directly(c));
    (starts_statement(tokens, first) && !extended).then(|| removal(file, first, k + 3, DATE_WAKEUP_DROP))
}

/// The ways Ferry can fix the finding of an option rule at `k`. One entry means there is nothing to
/// choose there.
fn choices(file: &Source, k: usize, rule: &str) -> Vec<Choice> {
    match rule {
        "deprecated.date_rfc7231" => rfc7231_choices(file, k),
        "deprecated.splfileinfo_bad_state_ex" => bad_state_choices(file, k),
        "deprecated.e_user_error" => user_error_choices(file, k),
        "removed.input_request" => input_choices(file, k, "INPUT_REQUEST", "$_REQUEST"),
        "removed.input_session" => input_choices(file, k, "INPUT_SESSION", "$_SESSION"),
        "deprecated.soap_functions_all" => soap_choices(file, k),
        "behavior.html_default_flags" => html_choices(file, k),
        "behavior.mysqli_default_exceptions" => mysqli_report_choices(file, k),
        "deprecated.mysqli_ping" => ping_choices(file, k),
        "removed.numberformatter_type_currency" => currency_choices(file, k),
        "removed.magic_variable" => magic_choices(file, k),
        "deprecated.libxml_disable_entity_loader" => entity_choices(file, k),
        _ => [None, None],
    }
    .into_iter()
    .flatten()
    .collect()
}

const RFC7231_FORMAT: &str = "'D, d M Y H:i:s \\G\\M\\T'";

/// DATE_RFC7231 or Date*::RFC7231: its literal value, or, where it formats a date, the same format
/// in UTC.
fn rfc7231_choices(file: &Source, k: usize) -> [Option<Choice>; 2] {
    let tokens = file.tokens;
    let tok = &tokens[k];
    let first = if tok.text.trim_start_matches('\\') == "DATE_RFC7231" && is_atom(tokens, k) {
        k
    } else if tok.text == "RFC7231" && k >= 2 && tokens[k - 1].is_punct("::") {
        let class = tokens[k - 2].text.trim_start_matches('\\').rsplit('\\').next().unwrap_or_default().to_ascii_lowercase();
        if !["datetime", "datetimeinterface", "datetimeimmutable"].contains(&class.as_str()) {
            return [None, None];
        }
        k - 2
    } else {
        return [None, None];
    };
    let literal = Choice {
        id: "literal",
        fix: Fix { extra: Vec::new(), class: RFC_LITERAL, start: tokens[first].start, end: tok.end, text: RFC7231_FORMAT.into() },
        recommended: true,
    };
    let utc = argument_of(tokens, first).filter(|&(_, index, arg)| index == 0 && arg == (first, k)).and_then(|(call, ..)| {
        if let Some((close, args)) = function_call(tokens, call, &["date"]).filter(|(_, a)| a.len() <= 2) {
            let rest = &file.src[tok.end..tokens[close].end];
            let text = format!("{}gmdate{}{RFC7231_FORMAT}{rest}", lead(&tokens[call]), file.between(call, first));
            return (args.len() <= 2).then(|| Fix {
                extra: Vec::new(),
                class: RFC_UTC,
                start: tokens[call].start,
                end: tokens[close].end,
                text,
            });
        }
        let (close, args) = method_call(tokens, call, &["format"]).filter(|(_, a)| a.len() == 1)?;
        let receiver = receiver_start(tokens, call - 1)?;
        let object = file.text(receiver, call - 2);
        let text =
            format!("\\DateTimeImmutable::createFromInterface({object})->setTimezone(new \\DateTimeZone('UTC'))->format({RFC7231_FORMAT})");
        (args.len() == 1).then(|| Fix { extra: Vec::new(), class: RFC_UTC, start: tokens[receiver].start, end: tokens[close].end, text })
    });
    [Some(literal), utc.map(|fix| Choice { id: "utc", fix, recommended: false })]
}

/// `$file->_bad_state_ex();` always threw: delete it, or throw that error. Neither is the default.
fn bad_state_choices(file: &Source, k: usize) -> [Option<Choice>; 2] {
    let tokens = file.tokens;
    let call = tokens[k].text.eq_ignore_ascii_case("_bad_state_ex")
        && tokens.get(k + 1).is_some_and(|t| t.is_punct("("))
        && tokens.get(k + 2).is_some_and(|t| t.is_punct(")"))
        && tokens.get(k + 3).is_some_and(|t| t.is_punct(";"))
        && k.checked_sub(1).is_some_and(|p| tokens[p].is_punct("->"));
    let Some(first) = receiver_start(tokens, k.saturating_sub(1)).filter(|&f| call && starts_statement(tokens, f)) else {
        return [None, None];
    };
    let message = "throw new \\Error('The parent constructor was not called: the object is in an invalid state');";
    [
        Some(Choice { id: "remove", fix: removal(file, first, k + 3, BAD_STATE_DROP), recommended: false }),
        Some(Choice {
            id: "throw",
            fix: Fix { extra: Vec::new(), class: KEEP_ERROR, start: tokens[first].start, end: tokens[k + 3].end, text: message.into() },
            recommended: false,
        }),
    ]
}

/// `trigger_error($msg, E_USER_ERROR);` → an ErrorException, or a warning followed by exit(255).
/// The exception is the default unless the project installs its own error handler.
fn user_error_choices(file: &Source, k: usize) -> [Option<Choice>; 2] {
    let tokens = file.tokens;
    let found = enclosing_call(tokens, k).and_then(|call| {
        let (close, args) = function_call(tokens, call, &["trigger_error", "user_error"])?;
        let [message, level] = args[..] else { return None };
        (level == (k, k) && tokens[k].text.trim_start_matches('\\') == "E_USER_ERROR" && is_statement(tokens, call, close))
            .then_some((call, close, message))
    });
    let Some((call, close, message)) = found else { return [None, None] };
    let (start, end) = (tokens[call].start, tokens[close + 1].end);
    let text = file.text(message.0, message.1);
    let handler = file.index.sets_error_handler();
    [
        Some(Choice {
            id: "exception",
            fix: Fix {
                extra: Vec::new(),
                class: USER_ERROR_EXCEPTION,
                start,
                end,
                text: format!("throw new \\ErrorException({text}, 0, E_USER_ERROR);"),
            },
            recommended: !handler,
        }),
        Some(Choice {
            id: "warning",
            fix: Fix {
                extra: Vec::new(),
                class: USER_ERROR_WARNING,
                start,
                end,
                text: format!("{}({text}, E_USER_WARNING); exit(255);", tokens[call].text),
            },
            recommended: handler,
        }),
    ]
}

/// filter_input(), filter_has_var() and filter_input_array() with INPUT_REQUEST or INPUT_SESSION:
/// what they always returned, or a read of the superglobal they never read.
fn input_choices(file: &Source, k: usize, constant: &str, global: &str) -> [Option<Choice>; 2] {
    let tokens = file.tokens;
    let found = enclosing_call(tokens, k).and_then(|call| {
        let (close, args) = function_call(tokens, call, &["filter_input", "filter_has_var", "filter_input_array"])?;
        (args.first() == Some(&(k, k)) && tokens[k].text.trim_start_matches('\\') == constant).then_some((call, close, args))
    });
    let Some((call, close, args)) = found else { return [None, None] };
    let name = tokens[call].text.trim_start_matches('\\').to_ascii_lowercase();
    let arg = |i: usize| args.get(i).map(|&(f, l)| file.text(f, l).to_string());
    let simple = |i: usize| args.get(i).is_some_and(|&a| plain(tokens, a) || literal(tokens, a));
    let filter = args.get(2).map(|&(f, l)| {
        let bare = tokens[f].text.trim_start_matches('\\');
        match RENAMED_CONSTANTS.iter().find(|(_, old, _)| f == l && *old == bare) {
            Some((_, _, new)) => new.to_string(),
            None => file.text(f, l).to_string(),
        }
    });
    let (old, read) = match name.as_str() {
        "filter_input" if (2..=3).contains(&args.len()) && simple(1) => {
            let key = arg(1).unwrap_or_default();
            let filtered = filter.map_or(String::new(), |f| format!(", {f}"));
            ("null", format!("isset({global}[{key}]) ? filter_var({global}[{key}]{filtered}) : null"))
        }
        "filter_has_var" if args.len() == 2 && simple(1) => {
            ("false", format!("array_key_exists({}, {global} ?? [])", arg(1).unwrap_or_default()))
        }
        "filter_input_array" if args.len() <= 3 => {
            let rest: String = (1..args.len()).filter_map(arg).map(|a| format!(", {a}")).collect();
            ("null", format!("isset({global}) ? filter_var_array({global}{rest}) : null"))
        }
        _ => return [None, None],
    };
    let (start, end) = (tokens[call].start, tokens[close].end);
    [
        Some(Choice { id: "old", fix: Fix { extra: Vec::new(), class: INPUT_OLD, start, end, text: old.into() }, recommended: true }),
        Some(Choice { id: "superglobal", fix: Fix { extra: Vec::new(), class: INPUT_GLOBAL, start, end, text: read }, recommended: false }),
    ]
}

/// SOAP_FUNCTIONS_ALL passed to addFunction(): every defined function, as PHP suggests, or only the
/// project's. Anywhere else it is just 999.
fn soap_choices(file: &Source, k: usize) -> [Option<Choice>; 2] {
    let tokens = file.tokens;
    let tok = &tokens[k];
    if tok.text.trim_start_matches('\\') != "SOAP_FUNCTIONS_ALL" || !is_atom(tokens, k) {
        return [None, None];
    }
    let span = |class, text: &str| Fix { extra: Vec::new(), class, start: tok.start, end: tok.end, text: text.to_string() };
    let added = argument_of(tokens, k).is_some_and(|(call, index, arg)| {
        index == 0 && arg == (k, k) && method_call(tokens, call, &["addFunction"]).is_some_and(|(_, a)| a.len() == 1)
    });
    if !added {
        return [Some(Choice { id: "value", fix: span(CONSTANT_VALUE, "999"), recommended: true }), None];
    }
    [
        Some(Choice { id: "all", fix: span(SOAP_ALL, "array_merge(...array_values(get_defined_functions()))"), recommended: true }),
        Some(Choice { id: "user", fix: span(SOAP_USER, "get_defined_functions()['user']"), recommended: false }),
    ]
}

/// The body of the innermost function around `k`, or the whole file outside functions.
fn scope_of(tokens: &[Token], k: usize) -> (usize, usize) {
    enclosing_function(tokens, k).and_then(|f| f.body).unwrap_or((0, tokens.len()))
}

fn newline(src: &str) -> &'static str {
    if src.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// The statement around `k`: its first token, unless it is a branch of an `if` chain or a loop header.
fn statement_of(tokens: &[Token], k: usize) -> Option<usize> {
    let mut depth = 0isize;
    let mut start = 0;
    for j in (0..k).rev() {
        let t = &tokens[j];
        if t.is_punct(")") || t.is_punct("]") {
            depth += 1;
        } else if t.is_punct("(") || t.is_punct("[") {
            depth -= 1;
        } else if depth <= 0 && (t.is_punct(";") || t.is_punct("{") || t.is_punct("}")) {
            start = j + 1;
            break;
        }
    }
    let t = &tokens[start];
    let condition = (t.is_ident_ci("if") || t.is_ident_ci("while"))
        && tokens.get(start + 1).is_some_and(|o| o.is_punct("("))
        && crate::declarations::close_of(tokens, start + 1).is_some_and(|c| c > k);
    let plain = (t.kind == TokenKind::Variable || t.is_ident_ci("return") || start == k) && depth == 0;
    (condition || plain).then_some(start)
}

/// A new statement written on its own line right before the one that starts at `start`.
fn statement_before(file: &Source, start: usize, statement: &str) -> Fix {
    let src = file.src;
    let at = file.tokens[start].start;
    let line = &src[src[..at].rfind('\n').map_or(0, |i| i + 1)..at];
    let text =
        if line.chars().all(|c| c == ' ' || c == '\t') { format!("{statement}{}{line}", newline(src)) } else { format!("{statement} ") };
    Fix { extra: Vec::new(), class: MYSQLI_REPORT_OFF, start: at, end: at, text }
}

/// `'Done: ' . $done + 1 . ' of ' . $total`: the `+`/`-` (or shift) operands between the
/// concatenations in parentheses, which is how PHP 8 reads them.
fn concat_groups(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let stops = [";", ",", "=>", "=", "?", ":", "??", "&&", "||", "==", "===", "!=", "!==", "<>", "<", ">", "<=", ">=", "<=>", "{", "}"];
    let words = ["and", "or", "xor", "return", "echo", "print", "case", "yield", "throw"];
    let shift = |j: usize| {
        let pair = |a: &Token, b: &Token| a.end == b.start && (a.is_punct("<") && b.is_punct("<") || a.is_punct(">") && b.is_punct(">"));
        tokens.get(j + 1).is_some_and(|n| pair(&tokens[j], n)) || j > 0 && pair(&tokens[j - 1], &tokens[j])
    };
    let stop = |j: usize| {
        let t = &tokens[j];
        !shift(j)
            && (stops.iter().any(|p| t.is_punct(p))
                || words.iter().any(|w| t.is_ident_ci(w))
                || t.kind == TokenKind::CloseTag
                || t.text.ends_with('=') && t.kind == TokenKind::Punct)
    };
    let walk = |range: &mut dyn Iterator<Item = usize>, open: &str, close: &str| {
        let mut depth = 0usize;
        let mut edge = k;
        for j in range {
            let t = &tokens[j];
            if t.is_punct(close) || t.is_punct(if close == ")" { "]" } else { ")" }) {
                depth += 1;
            } else if t.is_punct(open) || t.is_punct(if open == "(" { "[" } else { "(" }) {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            } else if depth == 0 && stop(j) {
                break;
            }
            edge = j;
        }
        edge
    };
    let first = walk(&mut (0..k).rev(), "(", ")");
    let last = walk(&mut (k + 1..tokens.len()), ")", "(");
    let allowed = [".", "+", "-", "*", "/", "%", "**", "<<", ">>", "->", "?->", "::", "!", "@", "~", "$", "\\"];
    let mut depth = 0usize;
    let mut segments = vec![(first, None::<usize>, false)];
    for j in first..=last {
        let t = &tokens[j];
        if t.is_punct("(") || t.is_punct("[") {
            depth += 1;
        } else if t.is_punct(")") || t.is_punct("]") {
            depth = depth.checked_sub(1)?;
        } else if depth == 0 && t.kind == TokenKind::Punct && shift(j) {
            let binary = j > first && crate::analyzer::expression_end(&tokens[j - 1]);
            if binary {
                segments.last_mut()?.2 = true;
            }
        } else if depth == 0 && t.kind == TokenKind::Punct {
            if !allowed.contains(&t.text.as_str()) {
                return None;
            }
            let binary = j > first && crate::analyzer::expression_end(&tokens[j - 1]);
            if t.is_punct(".") && binary {
                segments.last_mut()?.1 = Some(j - 1);
                segments.push((j + 1, None, false));
                continue;
            }
            if binary && ["+", "-", "<<", ">>"].iter().any(|p| t.is_punct(p)) {
                segments.last_mut()?.2 = true;
            }
        } else if depth == 0 && ["instanceof", "new", "clone", "include", "require", "print"].iter().any(|w| t.is_ident_ci(w)) {
            return None;
        }
    }
    segments.last_mut()?.1 = Some(last);
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for (a, b, mixed) in segments {
        let b = b?;
        if mixed && a <= b {
            edits.push((tokens[a].start, tokens[a].start, "(".into()));
            edits.push((tokens[b].end, tokens[b].end, ")".into()));
        }
    }
    let (start, end, text) = edits.first()?.clone();
    Some(Fix { class: CONCAT_GROUPS, start, end, text, extra: edits[1..].to_vec() })
}

const HTML_CALLS: [&str; 4] = ["htmlspecialchars", "htmlentities", "htmlspecialchars_decode", "html_entity_decode"];

/// The first HTML encoding call without flags stands for all of them in the file: each gets the
/// old flags, or the PHP 8.1 ones written out.
fn html_choices(file: &Source, k: usize) -> [Option<Choice>; 2] {
    let tokens = file.tokens;
    let calls: Vec<usize> = (0..tokens.len())
        .filter_map(|j| function_call(tokens, j, &HTML_CALLS).filter(|(_, args)| args.len() == 1).map(|(close, _)| close))
        .collect();
    let Some((close, _)) = function_call(tokens, k, &HTML_CALLS).filter(|(_, args)| args.len() == 1) else { return [None, None] };
    let fix = |class, flags: &str| {
        let insert = |c: usize| (tokens[c].start, tokens[c].start, format!(", {flags}"));
        let (start, end, text) = insert(close);
        Fix { class, start, end, text, extra: calls.iter().filter(|&&c| c != close).map(|&c| insert(c)).collect() }
    };
    [
        Some(Choice { id: "old", fix: fix(HTML_OLD, "ENT_COMPAT | ENT_HTML401"), recommended: true }),
        Some(Choice { id: "new", fix: fix(HTML_NEW, "ENT_QUOTES | ENT_SUBSTITUTE | ENT_HTML401"), recommended: false }),
    ]
}

/// A checked mysqli call that PHP 8.1 makes throw first: mysqli_report(MYSQLI_REPORT_OFF) before
/// it, or for `X or die(...)`, a try that dies in its catch.
fn mysqli_report_choices(file: &Source, k: usize) -> [Option<Choice>; 2] {
    let tokens = file.tokens;
    let Some((close, _)) = function_call(tokens, k, &["mysqli_connect", "mysqli_query", "mysqli_real_connect"]) else {
        return [None, None];
    };
    let Some(start) = statement_of(tokens, k) else { return [None, None] };
    let old = file.current.is_some_and(|v| v < PhpVersion::new(8, 1));
    let off = Choice { id: "report_off", fix: statement_before(file, start, "mysqli_report(MYSQLI_REPORT_OFF);"), recommended: old };
    let dies = tokens.get(close + 1).is_some_and(|t| t.is_ident_ci("or"))
        && tokens.get(close + 2).is_some_and(|t| t.is_ident_ci("die") || t.is_ident_ci("exit"));
    let catch = dies.then(|| {
        let exit_close = if tokens.get(close + 3).is_some_and(|t| t.is_punct("(")) {
            crate::declarations::close_of(tokens, close + 3)?
        } else {
            close + 2
        };
        let semi = exit_close + 1;
        let assigned = start == k
            || tokens[start].kind == TokenKind::Variable && tokens.get(start + 1).is_some_and(|t| t.is_punct("=")) && start + 2 == k;
        if !tokens.get(semi).is_some_and(|t| t.is_punct(";")) || !assigned {
            return None;
        }
        let text =
            format!("try {{ {}; }} catch (\\mysqli_sql_exception $e) {{ {}; }}", file.text(start, close), file.text(close + 2, exit_close));
        Some(Fix { extra: Vec::new(), class: MYSQLI_CATCH, start: tokens[start].start, end: tokens[semi].end, text })
    });
    [Some(off), catch.flatten().map(|fix| Choice { id: "catch", fix, recommended: false })]
}

/// mysqli_ping() and ->ping(): the same round trip as a query, or no check at all.
fn ping_choices(file: &Source, k: usize) -> [Option<Choice>; 2] {
    let tokens = file.tokens;
    let (first, close, query) = if let Some((close, args)) = function_call(tokens, k, &["mysqli_ping"]).filter(|(_, a)| a.len() == 1) {
        (k, close, format!("(bool) {}mysqli_query({}, 'DO 1')", lead(&tokens[k]), file.text(args[0].0, args[0].1)))
    } else if let Some((close, _)) = method_call(tokens, k, &["ping"]).filter(|(_, a)| a.is_empty()) {
        let Some(receiver) = receiver_start(tokens, k - 1) else { return [None, None] };
        (receiver, close, format!("(bool) {}->query('DO 1')", file.text(receiver, k - 2)))
    } else {
        return [None, None];
    };
    let query = Fix { extra: Vec::new(), class: PING_QUERY, start: tokens[first].start, end: tokens[close].end, text: query };
    let statement = starts_statement(tokens, first) && tokens.get(close + 1).is_some_and(|t| t.is_punct(";"));
    let guarded = first >= 3
        && tokens[first - 1].is_punct("!")
        && tokens[first - 2].is_punct("(")
        && tokens[first - 3].is_ident_ci("if")
        && starts_statement(tokens, first - 3)
        && tokens.get(close + 1).is_some_and(|t| t.is_punct(")"))
        && tokens.get(close + 2).is_some_and(|t| t.is_punct("{"));
    let drop = if statement {
        Some(removal(file, first, close + 1, PING_DROP))
    } else if guarded {
        crate::declarations::close_of(tokens, close + 2)
            .filter(|&end| !tokens.get(end + 1).is_some_and(|t| t.is_ident_ci("else") || t.is_ident_ci("elseif")))
            .map(|end| removal(file, first - 3, end, PING_DROP))
    } else {
        None
    };
    [Some(Choice { id: "query", fix: query, recommended: true }), drop.map(|fix| Choice { id: "drop", fix, recommended: false })]
}

/// NumberFormatter::TYPE_CURRENCY as the type of format() or parse(): the currency method, or the
/// false PHP 7 returned. Keeping false is the default on a project that still runs PHP 7.
fn currency_choices(file: &Source, k: usize) -> [Option<Choice>; 2] {
    let tokens = file.tokens;
    let Some(class) = k.checked_sub(2).filter(|&c| tokens[c].kind == TokenKind::Ident && tokens[c + 1].is_punct("::")) else {
        return [None, None];
    };
    let Some((call, index, _)) = argument_of(tokens, class).filter(|&(_, _, arg)| arg == (class, k)) else { return [None, None] };
    let code = format!("{}::CURRENCY_CODE", tokens[class].text);
    let arg = |a: (usize, usize)| file.text(a.0, a.1).to_string();
    let function = scope_of(tokens, call);
    let unused = |name: &str| !tokens[function.0..function.1].iter().any(|t| t.kind == TokenKind::Variable && t.text == name);
    let currency =
        std::iter::once("currency".to_string()).chain((2..).map(|n| format!("currency{n}"))).find(|n| unused(n)).unwrap_or_default();
    let (first, close, text) = if let Some((close, args)) = method_call(tokens, call, &["format", "parse"]).filter(|_| index == 1) {
        let Some(receiver) = receiver_start(tokens, call - 1) else { return [None, None] };
        let object = file.text(receiver, call - 2);
        let text = match (tokens[call].text.to_ascii_lowercase().as_str(), &args[..]) {
            ("format", [n, _]) => format!("{object}->formatCurrency({}, {object}->getTextAttribute({code}))", arg(*n)),
            ("parse", [s, _]) => format!("{object}->parseCurrency({}, ${currency})", arg(*s)),
            ("parse", [s, _, at]) => format!("{object}->parseCurrency({}, ${currency}, {})", arg(*s), arg(*at)),
            _ => return [None, None],
        };
        (receiver, close, text)
    } else if let Some((close, args)) = function_call(tokens, call, &["numfmt_format", "numfmt_parse"]).filter(|_| index == 2) {
        if !plain(tokens, args[0]) {
            return [None, None];
        }
        let f = arg(args[0]);
        let text = match (tokens[call].text.trim_start_matches('\\').to_ascii_lowercase().as_str(), &args[..]) {
            ("numfmt_format", [_, n, _]) => format!("numfmt_format_currency({f}, {}, numfmt_get_text_attribute({f}, {code}))", arg(*n)),
            ("numfmt_parse", [_, s, _]) => format!("numfmt_parse_currency({f}, {}, ${currency})", arg(*s)),
            ("numfmt_parse", [_, s, _, at]) => format!("numfmt_parse_currency({f}, {}, ${currency}, {})", arg(*s), arg(*at)),
            _ => return [None, None],
        };
        (call, close, format!("{}{text}", lead(&tokens[call])))
    } else {
        return [None, None];
    };
    let span = |class, text: String| Fix { extra: Vec::new(), class, start: tokens[first].start, end: tokens[close].end, text };
    let php7 = file.current.is_some_and(|v| v < PhpVersion::new(8, 0));
    [
        Some(Choice { id: "currency", fix: span(CURRENCY_METHOD, text), recommended: !php7 }),
        Some(Choice { id: "false", fix: span(CURRENCY_FALSE, "false".into()), recommended: php7 }),
    ]
}

/// $HTTP_RAW_POST_DATA and $php_errormsg: the null the code saw, or the data they used to carry.
/// Inside a function without `global $HTTP_RAW_POST_DATA` the old value was certainly null.
fn magic_choices(file: &Source, k: usize) -> [Option<Choice>; 2] {
    let tokens = file.tokens;
    let tok = &tokens[k];
    let raw = tok.text == "HTTP_RAW_POST_DATA";
    if tok.kind != TokenKind::Variable || !raw && tok.text != "php_errormsg" {
        return [None, None];
    }
    let after = tokens.get(k + 1);
    let wrapped = enclosing_call(tokens, k).is_some_and(|c| call_named(tokens, c, &["isset", "empty", "unset"]));
    if wrapped || after.is_some_and(|t| t.is_punct("[") || t.is_punct("->") || t.is_punct("=")) || k > 0 && tokens[k - 1].is_punct("&") {
        return [None, None];
    }
    let scope = scope_of(tokens, k);
    let global = tokens[scope.0..scope.1]
        .windows(2)
        .any(|w| w[0].is_ident_ci("global") && w[1].kind == TokenKind::Variable && w[1].text == tok.text);
    let never_set = raw && scope != (0, tokens.len()) && !global;
    let span = |class, text: &str| Fix { extra: Vec::new(), class, start: tok.start, end: tok.end, text: text.to_string() };
    let read = if raw { "file_get_contents('php://input')" } else { "(error_get_last()['message'] ?? null)" };
    [
        Some(Choice { id: "old", fix: span(if never_set { MAGIC_NULL_EXACT } else { MAGIC_NULL }, "null"), recommended: never_set }),
        Some(Choice { id: "read", fix: span(MAGIC_READ, read), recommended: false }),
    ]
}

/// libxml_disable_entity_loader() statements, alone or as a `$old = …(true); … …($old);` pair:
/// gone, or an external entity loader that loads nothing. Deleting is the default only when the
/// project never asks libxml for entities.
fn entity_choices(file: &Source, k: usize) -> [Option<Choice>; 2] {
    let tokens = file.tokens;
    let Some((close, args)) = function_call(tokens, k, &["libxml_disable_entity_loader"]) else { return [None, None] };
    let value = |a: Option<&(usize, usize)>| match a {
        None => Some(true),
        Some(&(f, l)) if f == l && tokens[f].is_ident_ci("true") => Some(true),
        Some(&(f, l)) if f == l && tokens[f].is_ident_ci("false") => Some(false),
        _ => None,
    };
    let block = "libxml_set_external_entity_loader(static fn ($public, $system, $context) => null);";
    let reset = "libxml_set_external_entity_loader(null);";
    let scope = scope_of(tokens, k);
    let loader = |first: usize, semi: usize, text: &str| (tokens[first].start, tokens[semi].end, text.to_string());
    let single = (is_statement(tokens, k, close) && args.len() <= 1).then(|| value(args.first())).flatten();
    let (drop, keep) = if let Some(on) = single {
        let (s, e, t) = loader(k, close + 1, if on { block } else { reset });
        (removal(file, k, close + 1, ENTITY_DROP), Fix { extra: Vec::new(), class: ENTITY_LOADER, start: s, end: e, text: t })
    } else {
        let saved = |at: usize| -> Option<(usize, usize, usize, String)> {
            let (c, a) = function_call(tokens, at, &["libxml_disable_entity_loader"])?;
            let var = at
                .checked_sub(2)
                .filter(|&v| tokens[v].kind == TokenKind::Variable && tokens[v + 1].is_punct("=") && starts_statement(tokens, v))?;
            (value(a.first()) == Some(true) && tokens.get(c + 1).is_some_and(|t| t.is_punct(";")))
                .then(|| (var, at, c + 1, tokens[var].text.clone()))
        };
        let restore = |at: usize, name: &str| -> Option<(usize, usize)> {
            let (c, a) = function_call(tokens, at, &["libxml_disable_entity_loader"])?;
            let [(f, l)] = a[..] else { return None };
            (f == l && is_var(tokens, f, name) && is_statement(tokens, at, c)).then_some((at, c + 1))
        };
        let pair = match saved(k) {
            Some((var, _, semi, name)) => (semi..scope.1).find_map(|j| restore(j, &name)).map(|back| ((var, semi), back, name)),
            None => {
                (scope.0..k).rev().find_map(saved).and_then(|(var, _, semi, name)| restore(k, &name).map(|back| ((var, semi), back, name)))
            }
        };
        let Some(((var, semi), (back, back_semi), name)) = pair else { return [None, None] };
        let uses = (scope.0..scope.1).filter(|&j| is_var(tokens, j, &name)).count();
        if uses != 2 {
            return [None, None];
        }
        let second = removal(file, back, back_semi, ENTITY_DROP);
        let mut drop = removal(file, var, semi, ENTITY_DROP);
        drop.extra.push((second.start, second.end, second.text));
        let (s, e, t) = loader(var, semi, block);
        let keep = Fix { class: ENTITY_LOADER, start: s, end: e, text: t, extra: vec![loader(back, back_semi, reset)] };
        (drop, keep)
    };
    let drop_first = !file.index.loads_entities();
    [Some(Choice { id: "drop", fix: drop, recommended: drop_first }), Some(Choice { id: "loader", fix: keep, recommended: !drop_first })]
}

/// The lines a fix touches, as it leaves them, without their common indentation.
fn after(src: &str, fix: &Fix) -> String {
    let start = src[..fix.start].rfind('\n').map_or(0, |i| i + 1);
    let last = if fix.end > fix.start { fix.end - 1 } else { fix.end };
    let end = src[last..].find('\n').map_or(src.len(), |i| last + i);
    let tail = if fix.end <= end { &src[fix.end..end] } else { "" };
    let text = format!("{}{}{tail}", &src[start..fix.start], fix.text);
    let lines: Vec<&str> = text.lines().collect();
    let indent = lines.iter().filter(|l| !l.trim().is_empty()).map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
    lines.iter().map(|l| l.get(indent..).unwrap_or_else(|| l.trim_start())).collect::<Vec<_>>().join("\n").trim().to_string()
}

const COERCED: [&str; 4] = ["int", "float", "string", "bool"];

/// The child's type widened so it accepts the parent's: the union of both, or no type for a `mixed`
/// parent. `None` where that would change how a value the child already took is coerced, or where
/// PHP would reject the union.
fn widened(shape: &crate::analyzer::ParamShape, parent: &str) -> Option<String> {
    let lower: Vec<String> = parent.trim_start_matches('?').split('|').map(|p| p.to_ascii_lowercase()).collect();
    let mut parts: Vec<(String, String)> =
        parent.trim_start_matches('?').split('|').map(|p| (p.to_ascii_lowercase(), p.to_string())).collect();
    if parent.starts_with('?') {
        parts.push(("null".into(), "null".into()));
    }
    let scalar = shape.ty.iter().any(|t| COERCED.contains(&t.as_str()));
    if parent.is_empty() || lower.iter().any(|p| p == "mixed") {
        return (!scalar).then(String::new);
    }
    let mut union: Vec<(String, String)> = shape.ty.iter().cloned().zip(shape.shown.iter().cloned()).collect();
    for (low, shown) in parts {
        if crate::analyzer::accepts(&shape.ty, &low) {
            continue;
        }
        if scalar && COERCED.contains(&low.as_str()) {
            return None;
        }
        let builtin = ["int", "float", "string", "bool", "array", "null", "callable", "iterable", "false", "true", "object", "void"];
        let written = if builtin.contains(&low.as_str()) { low.clone() } else { format!("\\{shown}") };
        if !union.iter().any(|(l, _)| *l == low) {
            union.push((low, written));
        }
    }
    let has = |t: &str| union.iter().any(|(l, _)| l == t);
    let class = union.iter().any(|(l, _)| {
        !COERCED.contains(&l.as_str()) && !["array", "null", "callable", "iterable", "false", "true", "object"].contains(&l.as_str())
    });
    let redundant = has("bool") && (has("false") || has("true"))
        || has("true") && has("false")
        || has("iterable") && (has("array") || has("traversable"))
        || has("object") && class;
    if redundant {
        return None;
    }
    let (nulls, rest): (Vec<_>, Vec<_>) = union.into_iter().partition(|(l, _)| l == "null");
    Some(rest.into_iter().chain(nulls).map(|(_, s)| s).collect::<Vec<_>>().join("|"))
}

/// A method that overrides a built-in one with a signature PHP 8 rejects: its parameter types widen
/// to accept the parent's, a parameter the parent treats as optional gets a default, missing
/// parameters and a variadic are added, and a tentative return type gets #[\ReturnTypeWillChange].
/// The result has to pass the same check the scan made.
fn widen_override(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let over = crate::analyzer::builtin_overrides(tokens, file.index, file.target).into_iter().find(|o| o.name == k)?;
    let crate::analyzer::Override::Incompatible(method) = over.problem else { return None };
    let &(_, parent_static, required, params, variadic, returns, tentative, ..) = method;
    let keyword = match tokens.get(k.checked_sub(1)?) {
        Some(t) if t.is_ident_ci("function") => k - 1,
        Some(t) if t.is_punct("&") && k >= 2 && tokens[k - 2].is_ident_ci("function") => k - 2,
        _ => return None,
    };
    let modifiers = ["public", "protected", "private", "static", "final", "abstract"];
    let mut first = keyword;
    while first > 0 && modifiers.iter().any(|m| tokens[first - 1].is_ident_ci(m)) {
        first -= 1;
    }
    let is_static = tokens[first..keyword].iter().any(|t| t.is_ident_ci("static"));
    let open = k + 1;
    let close = crate::declarations::close_of(tokens, open).filter(|_| tokens[open].is_punct("("))?;
    let child_return = crate::analyzer::return_type(tokens, close);
    let return_fits = child_return.as_deref().is_some_and(|r| crate::analyzer::covariant(r, returns));
    if is_static != parent_static || !tentative && !returns.is_empty() && !return_fits {
        return None;
    }
    let shapes = crate::analyzer::param_shapes(tokens, open, close);
    let (_, ranges) = arguments(tokens, open)?;
    if ranges.len() != shapes.len() {
        return None;
    }
    let child_variadic = shapes.iter().any(|s| s.variadic);
    let src = file.src;
    let mut texts = Vec::new();
    for (i, (shape, &(f, l))) in shapes.iter().zip(&ranges).enumerate() {
        let mut text = file.text(f, l).to_string();
        if shape.variadic {
            texts.push(text);
            continue;
        }
        if let Some(&(by_ref, ty)) = params.get(i) {
            if shape.by_ref != by_ref {
                return None;
            }
            if !crate::analyzer::accepts(&shape.ty, ty) {
                let var = (f..=l).find(|&j| tokens[j].kind == TokenKind::Variable)?;
                let compound = (f..var)
                    .any(|j| tokens[j].is_punct("(") || tokens[j].is_punct("&") && !(tokens[j + 1].is_punct("...") || j + 1 == var));
                if compound {
                    return None;
                }
                let new = widened(shape, ty)?;
                let typed: Vec<usize> = (f..=l).filter(|&j| tokens[j].is_punct("?") || shape.ty_at.contains(&j)).collect();
                let (&from, &to) = (typed.first()?, typed.last()?);
                let rest = src[tokens[to].end..tokens[l].end].trim_start();
                text = if new.is_empty() {
                    format!("{}{rest}", &src[tokens[f].start..tokens[from].start])
                } else {
                    format!("{}{new} {rest}", &src[tokens[f].start..tokens[from].start])
                };
            }
        }
        if i >= required && !shape.optional {
            if !shape.ty.is_empty() {
                return None;
            }
            text.push_str(" = null");
        }
        texts.push(text);
    }
    let names: HashSet<&str> = shapes.iter().map(|s| s.name.as_str()).collect();
    let free =
        |base: &str| std::iter::once(base.to_string()).chain((2..).map(|n| format!("{base}{n}"))).find(|n| !names.contains(n.as_str()));
    if !child_variadic {
        for i in shapes.len()..params.len() {
            texts.push(format!("${} = null", free(&format!("arg{}", i + 1))?));
        }
        if variadic {
            texts.push(format!("...${}", free("rest")?));
        }
    }
    let header = format!("{}{})", &src[tokens[first].start..tokens[open].end], texts.join(", "));
    let marked =
        tokens[first.saturating_sub(8)..first].iter().any(|t| t.text.trim_start_matches('\\').eq_ignore_ascii_case("ReturnTypeWillChange"));
    let returned = &src[tokens[close].end..return_end(tokens, close)];
    let check = crate::lexer::tokenize(&format!("<?php class C {{ {header}{returned} {{}} }}"));
    let name_at = check.iter().position(|t| t.text.eq_ignore_ascii_case(&tokens[k].text) && t.kind == TokenKind::Ident)?;
    let check_close = crate::declarations::close_of(&check, name_at + 1)?;
    let new_shapes = crate::analyzer::param_shapes(&check, name_at + 1, check_close);
    if !crate::analyzer::compatible(&new_shapes, is_static, crate::analyzer::return_type(&check, check_close).as_deref(), method) {
        return None;
    }
    let attribute = if tentative && !return_fits && !marked {
        let line = &src[src[..tokens[first].start].rfind('\n').map_or(0, |i| i + 1)..tokens[first].start];
        let indent: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
        let nl = if src.contains("\r\n") { "\r\n" } else { "\n" };
        format!("#[\\ReturnTypeWillChange]{nl}{indent}")
    } else {
        String::new()
    };
    Some(Fix {
        extra: Vec::new(),
        class: WIDEN_OVERRIDE,
        start: tokens[first].start,
        end: tokens[close].end,
        text: format!("{attribute}{header}"),
    })
}

/// Where the return type after the parameter list closing at `close` ends.
fn return_end(tokens: &[Token], close: usize) -> usize {
    if !tokens.get(close + 1).is_some_and(|t| t.is_punct(":")) {
        return tokens[close].end;
    }
    (close + 2..tokens.len()).find(|&j| tokens[j].is_punct("{") || tokens[j].is_punct(";")).map_or(tokens[close].end, |j| tokens[j - 1].end)
}

/// What png2wbmp() and jpeg2wbmp() did in php-src (`_php_image_convert`), step for step: the
/// destination opened before the source is read, the scale worked out in 32-bit floats, and each
/// pixel black below `threshold * 32`.
const WBMP_CONVERSION: &str = "(static function ($source, $destination, $height, $width, $threshold) {
    $threshold = (int) $threshold;
    if ($threshold < 0 || $threshold > 8) {
        trigger_error(\"NAME(): Invalid threshold value '$threshold'\", E_USER_WARNING);
        return false;
    }
    $in = @fopen($source, 'rb');
    if ($in === false) {
        trigger_error(\"NAME(): Unable to open '$source' for reading\", E_USER_WARNING);
        return false;
    }
    fclose($in);
    $out = @fopen($destination, 'wb');
    if ($out === false) {
        trigger_error(\"NAME(): Unable to open '$destination' for writing\", E_USER_WARNING);
        return false;
    }
    $image = @LOADER($source);
    if ($image === false) {
        fclose($out);
        trigger_error(\"NAME(): Unable to open '$destination' Not a valid KIND file\", E_USER_WARNING);
        return false;
    }
    $float = static fn (float $v): float => unpack('g', pack('g', $v))[1];
    $int = static fn (float $v): int => is_finite($v) ? (int) $v : 0;
    $orgWidth = imagesx($image);
    $orgHeight = imagesy($image);
    $width = (int) $width;
    $height = (int) $height;
    $x = $float(fdiv($orgWidth, $width));
    $y = $float(fdiv($orgHeight, $height));
    if ($x > 1 && $y > 1) {
        $x = $y = max($x, $y);
        $width = $int($float(fdiv($orgWidth, $x)));
        $height = $int($float(fdiv($orgHeight, $y)));
    } else {
        $x = $y = min($float(fdiv($width, $orgWidth)), $float(fdiv($height, $orgHeight)));
        $width = $int($float($orgWidth * $x));
        $height = $int($float($orgHeight * $y));
    }
    $resized = $width > 0 && $height > 0 ? @imagecreate($width, $height) : false;
    if ($resized === false) {
        fclose($out);
        trigger_error(\"NAME(): Unable to allocate temporary buffer\", E_USER_WARNING);
        return false;
    }
    imagecopyresized($resized, $image, 0, 0, 0, 0, $width, $height, $orgWidth, $orgHeight);
    $result = imagecreate($width, $height);
    $white = imagecolorallocate($result, 255, 255, 255);
    $black = imagecolorallocate($result, 0, 0, 0);
    for ($row = 0; $row < $height; $row++) {
        for ($col = 0; $col < $width; $col++) {
            $rgb = imagecolorsforindex($resized, imagecolorat($resized, $col, $row));
            imagesetpixel($result, $col, $row, intdiv($rgb['red'] + $rgb['green'] + $rgb['blue'], 3) < $threshold * 32 ? $black : $white);
        }
    }
    imagewbmp($result, $out, $black);
    fclose($out);
    return true;
})";

/// `png2wbmp($src, $dest, $height, $width, $threshold)` → the same conversion as a closure called in
/// place.
fn wbmp_port(file: &Source, k: usize, name: &str, loader: &str, kind: &str) -> Option<Fix> {
    let tokens = file.tokens;
    let (close, args) = function_call(tokens, k, &[name])?;
    if args.len() != 5 {
        return None;
    }
    let src = file.src;
    let line_start = src[..tokens[k].start].rfind('\n').map_or(0, |i| i + 1);
    let indent: String = src[line_start..tokens[k].start].chars().take_while(|c| *c == ' ' || *c == '\t').collect();
    let nl = if src.contains("\r\n") { "\r\n" } else { "\n" };
    let closure = WBMP_CONVERSION.replace("NAME", name).replace("LOADER", loader).replace("KIND", kind);
    let body =
        closure.lines().enumerate().map(|(i, l)| if i == 0 { l.to_string() } else { format!("{indent}{l}") }).collect::<Vec<_>>().join(nl);
    let text = format!("{body}({})", file.text(args[0].0, args[4].1));
    Some(Fix { extra: Vec::new(), class: WBMP_PORT, start: tokens[k].start, end: tokens[close].end, text })
}

/// The first token of the plain variable or property chain that ends right before the `->` at `arrow`.
fn receiver_start(tokens: &[Token], arrow: usize) -> Option<usize> {
    let mut j = arrow.checked_sub(1)?;
    while tokens[j].kind == TokenKind::Ident && j >= 2 && tokens[j - 1].is_punct("->") {
        j -= 2;
    }
    let before = j.checked_sub(1).map(|p| &tokens[p]);
    (tokens[j].kind == TokenKind::Variable && !before.is_some_and(|p| ["->", "?->", "::", "$"].iter().any(|s| p.is_punct(s)))).then_some(j)
}

/// `$x->__wakeup();` on an SplFixedArray, when no class of the project extends it.
fn wakeup_call(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let call = tokens[k].text.eq_ignore_ascii_case("__wakeup")
        && tokens.get(k + 1).is_some_and(|t| t.is_punct("("))
        && tokens.get(k + 2).is_some_and(|t| t.is_punct(")"))
        && tokens.get(k + 3).is_some_and(|t| t.is_punct(";"))
        && k.checked_sub(1).is_some_and(|p| tokens[p].is_punct("->"));
    let first = receiver_start(tokens, k.checked_sub(1)?).filter(|_| call)?;
    (starts_statement(tokens, first) && !file.index.extends_directly("splfixedarray")).then(|| removal(file, first, k + 3, WAKEUP_CALL))
}

/// `$param->isArray()` / `->isCallable()` → the same check written with getType(), and
/// `$function->isDisabled()` → `false`.
fn reflection_check(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let empty = tokens.get(k + 1).is_some_and(|t| t.is_punct("(")) && tokens.get(k + 2).is_some_and(|t| t.is_punct(")"));
    let arrow = k.checked_sub(1).filter(|&p| tokens[p].is_punct("->") && empty)?;
    let first = receiver_start(tokens, arrow)?;
    let (start, end) = (tokens[first].start, tokens[k + 2].end);
    let ty = match tokens[k].text.to_ascii_lowercase().as_str() {
        "isdisabled" => return Some(Fix { extra: Vec::new(), class: DISABLED_FALSE, start, end, text: "false".into() }),
        "isarray" => "array",
        "iscallable" => "callable",
        _ => return None,
    };
    let check = format!(
        "(static function (\\ReflectionParameter $p): bool {{ $t = $p->getType(); if ($t === null) {{ return false; }} $b = []; foreach ($t instanceof \\ReflectionUnionType ? $t->getTypes() : [$t] as $x) {{ if ($x instanceof \\ReflectionNamedType && $x->isBuiltin() && $x->getName() !== 'null') {{ $b[] = $x->getName(); }} }} return $b === ['{ty}']; }})({})",
        file.text(first, arrow - 1)
    );
    Some(Fix { extra: Vec::new(), class: REFLECTION_TYPE_CHECK, start, end, text: check })
}

/// Whether the variable `name` gets another value, a reference or an unset anywhere from `from` to `to`.
fn rebinds(tokens: &[Token], from: usize, to: usize, name: &str, except: usize) -> bool {
    (from..to).filter(|&j| j != except && is_var(tokens, j, name)).any(|j| {
        let prev = &tokens[j - 1];
        flow_modifies(tokens, j)
            || prev.is_punct("&")
            || prev.is_ident_ci("as")
            || prev.is_ident_ci("global")
            || prev.is_ident_ci("static")
            || enclosing_call(tokens, j).is_some_and(|c| call_named(tokens, c, &["unset", "list"]))
    })
}

const XML_HANDLER_CALLS: &[&str] = &[
    "xml_set_character_data_handler",
    "xml_set_default_handler",
    "xml_set_element_handler",
    "xml_set_end_namespace_decl_handler",
    "xml_set_external_entity_ref_handler",
    "xml_set_notation_decl_handler",
    "xml_set_processing_instruction_handler",
    "xml_set_start_namespace_decl_handler",
    "xml_set_unparsed_entity_decl_handler",
];

/// The parser and object of the one `xml_set_object($p, $o)` for `parser` in the body `(open, end)`.
fn xml_object_of(file: &Source, open: usize, end: usize, parser: &str) -> Result<Option<(usize, usize, String)>, ()> {
    let tokens = file.tokens;
    let nested = nested_bodies(tokens, open, end);
    let mut found = Vec::new();
    for j in (open + 1..end).filter(|&j| !nested.iter().any(|&(a, b)| a <= j && j <= b)) {
        let Some((close, args)) = function_call(tokens, j, &["xml_set_object"]) else { continue };
        let [p, o] = args[..] else { return Err(()) };
        if file.text(p.0, p.1) != parser {
            continue;
        }
        let base = &tokens[o.0];
        if !plain(tokens, o) || base.text != "this" && rebinds(tokens, open + 1, end, &base.text, o.0) {
            return Err(());
        }
        found.push((j, close, file.text(o.0, o.1).to_string()));
    }
    match found.len() {
        0 => Ok(None),
        1 => Ok(found.pop()),
        _ => Err(()),
    }
}

/// The string handlers of the xml_set_*_handler() call at `k` as callables: `''` → `null`, a
/// method name → `[$object, 'name']`. `None` when one of them can't be written so.
fn xml_handler_edits(file: &Source, k: usize, object: Option<&str>) -> Option<Vec<(usize, String)>> {
    let tokens = file.tokens;
    let (_, args) = function_call(tokens, k, XML_HANDLER_CALLS)?;
    let mut edits = Vec::new();
    for &(f, l) in args.iter().skip(1) {
        if f != l || !matches!(tokens[f].kind, TokenKind::Str { .. }) {
            continue;
        }
        let value = literal_text(&tokens[f])?;
        let named =
            !value.is_empty() && value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') && !value.as_bytes()[0].is_ascii_digit();
        let lower = value.to_ascii_lowercase();
        let global = crate::arity::ARITY.binary_search_by(|(n, _, _)| (*n).cmp(lower.as_str())).is_ok()
            || file.index.declares_function("", &lower, true)
            || tokens.windows(2).any(|w| w[0].is_ident_ci("function") && w[1].text.eq_ignore_ascii_case(&value));
        let text = match object {
            _ if value.is_empty() => "null".to_string(),
            Some(o) if named && !global => format!("[{o}, {}]", quoted(&value)),
            _ => return None,
        };
        edits.push((f, text));
    }
    Some(edits)
}

fn xml_handler(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let (_, args) = function_call(tokens, k, XML_HANDLER_CALLS)?;
    let &(pf, pl) = args.first()?;
    let function = enclosing_function(tokens, k)?;
    let (open, end) = function.body?;
    let object = xml_object_of(file, open, end, file.text(pf, pl)).ok()?;
    let edits = xml_handler_edits(file, k, object.as_ref().map(|o| o.2.as_str()))?;
    let (&(first, _), &(last, _)) = (edits.first()?, edits.last()?);
    let mut text = String::new();
    let mut at = tokens[first].start;
    for (f, new) in &edits {
        text.push_str(&file.src[at..tokens[*f].start]);
        text.push_str(new);
        at = tokens[*f].end;
    }
    Some(Fix { extra: Vec::new(), class: XML_CALLABLE, start: tokens[first].start, end: tokens[last].end, text })
}

/// `xml_set_object($p, $o);` once every handler of `$p` in the file is set in the same function and
/// none of them is left a string.
fn xml_object(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let (close, args) = function_call(tokens, k, &["xml_set_object"])?;
    let [(pf, pl), _] = args[..] else { return None };
    let parser = file.text(pf, pl);
    let function = enclosing_function(tokens, k)?;
    let (open, end) = function.body?;
    let (_, _, object) = xml_object_of(file, open, end, parser).ok()??;
    if !is_statement(tokens, k, close) {
        return None;
    }
    let range = if parser.starts_with("$this->") { 0..tokens.len() } else { open + 1..end };
    for j in range {
        let Some((_, handler_args)) = function_call(tokens, j, XML_HANDLER_CALLS) else { continue };
        if handler_args.first().is_none_or(|&(f, l)| file.text(f, l) != parser) {
            continue;
        }
        if j < open || j > end || enclosing_function(tokens, j).map(|f| f.keyword) != Some(function.keyword) {
            return None;
        }
        xml_handler_edits(file, j, Some(&object))?;
    }
    Some(removal(file, k, close + 1, XML_OBJECT))
}

const SESSION_METHODS: [(&str, &str); 6] =
    [("open", "$path, $name"), ("close", ""), ("read", "$id"), ("write", "$id, $data"), ("destroy", "$id"), ("gc", "$max")];

/// `session_set_save_handler($open, …, $gc)` → the same callables behind a SessionHandlerInterface
/// object, with `false` so that, as before, no session_write_close() is registered for shutdown.
fn session_handler(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let (close, args) = function_call(tokens, k, &["session_set_save_handler"])?;
    if args.len() != SESSION_METHODS.len() {
        return None;
    }
    let class = crate::index::class_decls(tokens).into_iter().filter(|d| d.open < k && k < d.close).max_by_key(|d| d.open);
    let public_method = |name: &str| {
        class.as_ref().is_some_and(|d| {
            (d.open..d.close).any(|j| {
                tokens[j].is_ident_ci("function")
                    && tokens.get(j + 1).is_some_and(|n| n.text.eq_ignore_ascii_case(name))
                    && !(d.open..j)
                        .rev()
                        .take_while(|&m| tokens[m].kind == TokenKind::Ident)
                        .any(|m| tokens[m].is_ident_ci("private") || tokens[m].is_ident_ci("protected"))
            })
        })
    };
    let callable = |&(f, l): &(usize, usize)| {
        let t = &tokens[f];
        let closure = t.is_ident_ci("function")
            || t.is_ident_ci("fn")
            || t.is_ident_ci("static") && tokens.get(f + 1).is_some_and(|n| n.is_ident_ci("function") || n.is_ident_ci("fn"));
        let name = f == l
            && literal_text(t).is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'\\'));
        let pair = match &tokens[f..=l] {
            [o, this, comma, m, c] if o.is_punct("[") && c.is_punct("]") => Some((this, comma, m)),
            [a, o, this, comma, m, c] if a.is_ident_ci("array") && o.is_punct("(") && c.is_punct(")") => Some((this, comma, m)),
            _ => None,
        };
        let own = pair.is_some_and(|(this, comma, m)| {
            this.kind == TokenKind::Variable
                && this.text == "this"
                && comma.is_punct(",")
                && literal_text(m).is_some_and(|m| public_method(&m))
        });
        closure || name || own
    };
    if !args.iter().all(callable) {
        return None;
    }
    let src = file.src;
    let line_start = src[..tokens[k].start].rfind('\n').map_or(0, |i| i + 1);
    let indent: String = src[line_start..tokens[k].start].chars().take_while(|c| *c == ' ' || *c == '\t').collect();
    let nl = if src.contains("\r\n") { "\r\n" } else { "\n" };
    let inner = format!("{indent}    ");
    let callables = file.text(args[0].0, args[args.len() - 1].1);
    let mut lines = vec![
        format!("{}session_set_save_handler(new class({callables}) implements \\SessionHandlerInterface {{", lead(&tokens[k])),
        format!("{inner}private $c;"),
        String::new(),
        format!("{inner}public function __construct(...$c)"),
        format!("{inner}{{"),
        format!("{inner}    $this->c = $c;"),
        format!("{inner}}}"),
    ];
    for (i, (method, params)) in SESSION_METHODS.iter().enumerate() {
        lines.push(String::new());
        lines.push(format!("{inner}#[\\ReturnTypeWillChange]"));
        lines.push(format!("{inner}public function {method}({params})"));
        lines.push(format!("{inner}{{"));
        lines.push(format!("{inner}    return ($this->c[{i}])({params});"));
        lines.push(format!("{inner}}}"));
    }
    lines.push(format!("{indent}}}, false)"));
    let text = lines.join(nl);
    Some(Fix { extra: Vec::new(), class: SESSION_HANDLER, start: tokens[k].start, end: tokens[close].end, text })
}

/// `function __autoload($class) { … }` → `spl_autoload_register(function ($class) { … });`, when
/// nothing else in the project registers or names an autoloader and no code above it needed it.
fn autoload_register(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let keyword = (0..k).rev().find(|&j| !tokens[j].is_punct("&"))?;
    if !tokens[keyword].is_ident_ci("function") || !tokens[k].text.eq_ignore_ascii_case("__autoload") || file.index.hooks_autoload() {
        return None;
    }
    let (_, close) = crate::declarations::function_at(tokens, keyword)?.body?;
    let mut depth = 0isize;
    for t in &tokens[..keyword] {
        if t.is_punct("{") {
            depth += 1;
        } else if t.is_punct("}") {
            depth -= 1;
        }
    }
    let uses_classes = tokens[..keyword].iter().any(|t| {
        ["new", "instanceof", "extends", "implements", "class_exists", "interface_exists"].iter().any(|w| t.is_ident_ci(w))
            || t.is_punct("::")
    });
    let names_itself = tokens[k + 1..close].iter().any(|t| t.is_ident_ci("__FUNCTION__") || t.is_ident_ci("__autoload"));
    if depth != 0 || uses_classes || names_itself {
        return None;
    }
    let by_ref = if keyword + 1 < k { "&" } else { "" };
    let text = format!("spl_autoload_register(function {by_ref}{});", &file.src[tokens[k].end..tokens[close].end]);
    Some(Fix { extra: Vec::new(), class: AUTOLOAD_REGISTER, start: tokens[keyword].start, end: tokens[close].end, text })
}

const CONTROL_WORDS: &[&str] = &[
    "if", "else", "elseif", "for", "foreach", "while", "do", "switch", "case", "default", "try", "catch", "finally", "return", "break",
    "continue", "goto", "yield", "throw", "declare", "match",
];
const SCOPE_WORDS: &[&str] =
    &["compact", "extract", "get_defined_vars", "eval", "include", "include_once", "require", "require_once", "global"];
const LOOP_WORDS: &[&str] = &["for", "foreach", "while", "do"];

/// Whether a variable named `name` appears at `j`.
fn is_var(tokens: &[Token], j: usize, name: &str) -> bool {
    tokens[j].kind == TokenKind::Variable && tokens[j].text == name
}

/// Whether the tokens from `from` to `to` can read or rebind a variable without naming it.
fn reaches_scope(tokens: &[Token], from: usize, to: usize) -> bool {
    tokens[from..to].iter().any(|t| t.is_punct("$") || SCOPE_WORDS.iter().any(|w| t.is_ident_ci(w)))
}

/// The loops whose span holds `k`: from the keyword to the end of the body.
fn loops_around(tokens: &[Token], from: usize, k: usize) -> Vec<(usize, usize)> {
    (from..k)
        .filter(|&j| LOOP_WORDS.iter().any(|w| tokens[j].is_ident_ci(w)))
        .filter_map(|j| {
            let end = if tokens[j].is_ident_ci("do") {
                let body = tokens.get(j + 1).filter(|t| t.is_punct("{")).map(|_| j + 1)?;
                let close = crate::declarations::close_of(tokens, body)?;
                let head = tokens.get(close + 2).filter(|t| t.is_punct("(")).map(|_| close + 2)?;
                crate::declarations::close_of(tokens, head)?
            } else {
                let head = tokens.get(j + 1).filter(|t| t.is_punct("(")).map(|_| j + 1)?;
                let close = crate::declarations::close_of(tokens, head)?;
                if tokens.get(close + 1).is_some_and(|t| t.is_punct("{")) {
                    crate::declarations::close_of(tokens, close + 1)?
                } else {
                    (close + 1..tokens.len()).find(|&e| tokens[e].is_punct(";"))?
                }
            };
            (k < end).then_some((j, end))
        })
        .collect()
}

/// `$v = false;` that a later `$v[...] = ...` in the same straight run of statements turns into an
/// array: the initializer becomes `[]`.
fn false_to_array(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let name = tokens.get(k).filter(|t| t.kind == TokenKind::Variable)?.text.as_str();
    if !starts_statement(tokens, k) || !tokens.get(k + 1).is_some_and(|t| t.is_punct("[")) {
        return None;
    }
    let mut end = crate::declarations::close_of(tokens, k + 1)?;
    while tokens.get(end + 1).is_some_and(|t| t.is_punct("[")) {
        end = crate::declarations::close_of(tokens, end + 1)?;
    }
    if !tokens.get(end + 1).is_some_and(|t| t.is_punct("=")) {
        return None;
    }
    let semi = (end + 2..tokens.len()).find(|&j| tokens[j].is_punct(";") && enclosing_call(tokens, j).is_none())?;
    if (k + 1..semi).any(|j| is_var(tokens, j, name)) || reaches_scope(tokens, k, semi) {
        return None;
    }
    let init = (0..k).rev().find(|&j| is_var(tokens, j, name))?;
    let initializer = starts_statement(tokens, init)
        && tokens.get(init + 1).is_some_and(|t| t.is_punct("="))
        && tokens.get(init + 2).is_some_and(|t| t.is_ident_ci("false"))
        && tokens.get(init + 3).is_some_and(|t| t.is_punct(";"));
    let straight = (init + 4..k).all(|j| {
        let t = &tokens[j];
        !(t.is_punct("{") || t.is_punct("}") || t.is_punct(":") || t.is_punct("?") || t.kind == TokenKind::CloseTag)
            && !CONTROL_WORDS.iter().any(|w| t.is_ident_ci(w))
    });
    (initializer && straight && !reaches_scope(tokens, init, k)).then(|| Fix {
        extra: Vec::new(),
        class: FALSE_ARRAY,
        start: tokens[init + 2].start,
        end: tokens[init + 2].end,
        text: "[]".into(),
    })
}

/// The text of a quoted literal without interpolation.
fn literal_text(tok: &Token) -> Option<String> {
    let inner = tok.text.get(1..tok.text.len().checked_sub(1)?)?;
    match tok.kind {
        TokenKind::Str { interpolates: false, backtick: false } if tok.text.starts_with('\'') => {
            let mut out = String::new();
            let mut chars = inner.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '\\' && chars.peek().is_some_and(|n| *n == '\\' || *n == '\'') {
                    out.push(chars.next()?);
                } else {
                    out.push(c);
                }
            }
            Some(out)
        }
        TokenKind::Str { backtick: false, .. } if !inner.contains(['\\', '$']) => Some(inner.to_string()),
        _ => None,
    }
}

/// A single-quoted PHP literal holding `value`.
fn quoted(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut out = String::from("'");
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '\'' => out.push_str("\\'"),
            '\\' if chars.get(i + 1).is_none_or(|n| *n == '\\' || *n == '\'') => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

/// (strftime, date): specifiers that print the same in UTC whatever the locale.
const STRFTIME_DATE: [(char, &str); 17] = [
    ('Y', "Y"),
    ('y', "y"),
    ('m', "m"),
    ('d', "d"),
    ('H', "H"),
    ('I', "h"),
    ('M', "i"),
    ('S', "s"),
    ('u', "N"),
    ('w', "w"),
    ('G', "o"),
    ('V', "W"),
    ('F', "Y-m-d"),
    ('T', "H:i:s"),
    ('R', "H:i"),
    ('D', "m/d/y"),
    ('%', "%"),
];

/// `gmstrftime('<format>'[, $ts])` → `gmdate('<format>'[, $ts])`, when every specifier has a
/// date() letter that prints the same.
fn gmstrftime_fix(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let (_, args) = function_call(tokens, k, &["gmstrftime"])?;
    let &(first, last) = args.first().filter(|_| args.len() <= 2)?;
    if first != last {
        return None;
    }
    let format = literal_text(&tokens[first])?;
    let mut out = String::new();
    let mut chars = format.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let spec = chars.next()?;
            out.push_str(STRFTIME_DATE.iter().find(|(s, _)| *s == spec)?.1);
        } else if c.is_ascii_alphabetic() || c == '\\' {
            out.push('\\');
            out.push(c);
        } else {
            out.push(c);
        }
    }
    let between = &file.src[tokens[k].end..tokens[first].start];
    Some(Fix {
        extra: Vec::new(),
        class: GMDATE,
        start: tokens[k].start,
        end: tokens[first].end,
        text: format!("{}gmdate{between}{}", lead(&tokens[k]), quoted(&out)),
    })
}

/// Calls that fill `$http_response_header` through a stream; any of them earlier in the scope could
/// leave headers that a failed request doesn't clear.
const STREAM_CALLS: &[&str] = &[
    "file_get_contents",
    "file",
    "fopen",
    "readfile",
    "copy",
    "get_headers",
    "file_put_contents",
    "getimagesize",
    "md5_file",
    "sha1_file",
    "hash_file",
    "simplexml_load_file",
    "parse_ini_file",
    "fsockopen",
];
const HEADER_CALLS: &[&str] = &["file_get_contents", "file", "fopen", "readfile", "copy"];
const NOT_CALLS: &[&str] = &[
    "if", "elseif", "while", "for", "foreach", "switch", "match", "catch", "declare", "array", "isset", "empty", "list", "unset", "and",
    "or", "xor", "return", "echo", "print", "exit", "die", "function", "fn", "use",
];

/// Whether `j` starts a call that runs code: a function, a method, a closure or a constructor.
fn starts_call(tokens: &[Token], j: usize) -> bool {
    let t = &tokens[j];
    let open = tokens.get(j + 1).is_some_and(|n| n.is_punct("("));
    (open
        && t.kind == TokenKind::Ident
        && !NOT_CALLS.iter().any(|w| t.is_ident_ci(w))
        && !tokens[j.saturating_sub(1)].is_ident_ci("function"))
        || (open && (t.kind == TokenKind::Variable || t.is_punct(")") || t.is_punct("]") || t.is_punct("}")))
        || ["new", "include", "include_once", "require", "require_once", "eval", "clone"].iter().any(|w| t.is_ident_ci(w))
        || t.is_punct("->")
        || t.is_punct("?->")
        || t.is_punct("::")
        || matches!(t.kind, TokenKind::Str { backtick: true, .. })
}

/// Bodies of the functions nested inside the body `(open, close)`.
fn nested_bodies(tokens: &[Token], open: usize, close: usize) -> Vec<(usize, usize)> {
    (open + 1..close)
        .filter(|&j| tokens[j].is_ident_ci("function") || tokens[j].is_ident_ci("fn"))
        .filter_map(|j| {
            let f = crate::declarations::function_at(tokens, j)?;
            f.body.or_else(|| {
                let arrow = (j..close).find(|&a| tokens[a].is_punct("=>"))?;
                Some((j, (arrow..close).find(|&e| tokens[e].is_punct(";") || tokens[e].is_punct(")"))?))
            })
        })
        .collect()
}

/// A read of `$http_response_header` right after the one request of its scope that set it, with
/// nothing in between that could make another: `http_get_last_response_headers()`.
fn last_headers(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let function = enclosing_function(tokens, k)?;
    let (open, _) = function.body?;
    let nested = nested_bodies(tokens, open, k);
    let own = |j: usize| !nested.iter().any(|&(a, b)| a <= j && j <= b);
    let call = (open + 1..k).rev().find(|&j| own(j) && function_call(tokens, j, STREAM_CALLS).is_some())?;
    if !call_named(tokens, call, HEADER_CALLS) || (open + 1..call).any(|j| own(j) && function_call(tokens, j, STREAM_CALLS).is_some()) {
        return None;
    }
    if !loops_around(tokens, open + 1, call).is_empty() {
        return None;
    }
    let mut depth = 0usize;
    for j in (open + 1..call).rev() {
        if tokens[j].is_punct(")") {
            depth += 1;
        } else if tokens[j].is_punct("(") {
            if depth == 0 && j > 0 && starts_call(tokens, j - 1) {
                return None;
            }
            depth = depth.saturating_sub(1);
        } else if depth == 0 && (tokens[j].is_punct(";") || tokens[j].is_punct("{") || tokens[j].is_punct("}")) {
            break;
        }
    }
    let close = arguments(tokens, call + 1)?.0;
    for j in close + 1..k {
        let t = &tokens[j];
        if t.is_punct("}") || LOOP_WORDS.iter().any(|w| t.is_ident_ci(w)) {
            return None;
        }
        if starts_call(tokens, j) {
            let encloses = tokens
                .get(j + 1)
                .filter(|n| n.is_punct("("))
                .and_then(|_| crate::declarations::close_of(tokens, j + 1))
                .is_some_and(|c| c > k);
            if !encloses || t.is_punct("->") || t.is_punct("::") || t.is_ident_ci("new") {
                return None;
            }
        }
    }
    let isset =
        k >= 2 && tokens[k - 1].is_punct("(") && tokens[k - 2].is_ident_ci("isset") && tokens.get(k + 1).is_some_and(|t| t.is_punct(")"));
    let argument = k.checked_sub(1).is_some_and(|p| tokens[p].is_punct("(") || tokens[p].is_punct(","))
        && tokens.get(k + 1).is_some_and(|n| n.is_punct(")") || n.is_punct(","));
    if let Some(outer) = enclosing_call(tokens, k).filter(|_| !isset) {
        let name = tokens[outer].text.trim_start_matches('\\').to_ascii_lowercase();
        let by_value = inert_builtin(tokens, outer) && !crate::calls::BY_REF_ARGUMENT.contains(&name.as_str());
        if name == "isset" || argument && !by_value && !["empty", "array", "list"].contains(&name.as_str()) {
            return None;
        }
    }
    if isset {
        return Some(Fix {
            extra: Vec::new(),
            class: LAST_HEADERS,
            start: tokens[k - 2].start,
            end: tokens[k + 1].end,
            text: "http_get_last_response_headers() !== null".into(),
        });
    }
    Some(Fix {
        extra: Vec::new(),
        class: LAST_HEADERS,
        start: tokens[k].start,
        end: tokens[k].end,
        text: "http_get_last_response_headers()".into(),
    })
}

/// `$a[null]` → `$a['']` where `$a` was just given an array literal and only its elements were
/// touched since.
fn null_key(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let base = k.checked_sub(2)?;
    let tok = &tokens[k];
    let plain = tok.is_ident_ci("null") && tokens[k - 1].is_punct("[") && tokens.get(k + 1).is_some_and(|t| t.is_punct("]"));
    let var = tokens[base].kind == TokenKind::Variable
        && !base.checked_sub(1).is_some_and(|p| ["->", "?->", "::", "$", "&"].iter().any(|s| tokens[p].is_punct(s)));
    if !plain || !var {
        return None;
    }
    let name = tokens[base].text.as_str();
    let dim = |j: usize| tokens.get(j + 1).is_some_and(|t| t.is_punct("[")) && !tokens[j.saturating_sub(1)].is_punct("&");
    let mut init = base;
    loop {
        init = (0..init).rev().find(|&j| is_var(tokens, j, name))?;
        if !dim(init) {
            break;
        }
    }
    let literal_end = match tokens.get(init + 2) {
        Some(t) if t.is_punct("[") => crate::declarations::close_of(tokens, init + 2),
        Some(t) if t.is_ident_ci("array") && tokens.get(init + 3).is_some_and(|o| o.is_punct("(")) => {
            crate::declarations::close_of(tokens, init + 3)
        }
        _ => None,
    }?;
    let initializer = starts_statement(tokens, init)
        && tokens.get(init + 1).is_some_and(|t| t.is_punct("="))
        && tokens.get(literal_end + 1).is_some_and(|t| t.is_punct(";"));
    if !initializer || reaches_scope(tokens, init, base) {
        return None;
    }
    let mut depth = 0isize;
    for t in &tokens[init..base] {
        if t.is_punct("{") {
            depth += 1;
        } else if t.is_punct("}") {
            depth -= 1;
            if depth < 0 {
                return None;
            }
        }
    }
    for (from, to) in loops_around(tokens, init, base) {
        if (from..to).any(|j| is_var(tokens, j, name) && !dim(j)) || reaches_scope(tokens, from, to) {
            return None;
        }
    }
    Some(Fix { extra: Vec::new(), class: NULL_KEY, start: tok.start, end: tok.end, text: "''".into() })
}

/// (function, callable argument): calls that run the callable before they return and keep nothing of it.
const INVOKING_CALLS: [(&str, usize); 12] = [
    ("call_user_func", 0),
    ("call_user_func_array", 0),
    ("array_map", 0),
    ("array_filter", 1),
    ("array_walk", 1),
    ("array_walk_recursive", 1),
    ("array_reduce", 1),
    ("usort", 1),
    ("uasort", 1),
    ("uksort", 1),
    ("preg_replace_callback", 1),
    ("iterator_apply", 1),
];

/// `'self::m'` or `['parent', 'm']` passed straight to a call that runs it → `self::m(...)`.
fn first_class_callable(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let value = literal_text(&tokens[k])?;
    let (first, last, class, method) = if let Some((class, method)) = value.split_once("::") {
        (k, k, class.to_string(), method.to_string())
    } else {
        let method = literal_text(tokens.get(k + 2)?)?;
        let short = k >= 1 && tokens[k - 1].is_punct("[") && tokens.get(k + 3).is_some_and(|t| t.is_punct("]"));
        let long = k >= 2
            && tokens[k - 1].is_punct("(")
            && tokens[k - 2].is_ident_ci("array")
            && tokens.get(k + 3).is_some_and(|t| t.is_punct(")"));
        if !tokens.get(k + 1).is_some_and(|t| t.is_punct(",")) || !(short || long) {
            return None;
        }
        (if short { k - 1 } else { k - 2 }, k + 3, value, method)
    };
    let class = class.to_ascii_lowercase();
    let named =
        !method.is_empty() && method.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') && !method.as_bytes()[0].is_ascii_digit();
    if !["self", "parent", "static"].contains(&class.as_str()) || !named {
        return None;
    }
    let (call, index, arg) = argument_of(tokens, first)?;
    if arg != (first, last) || !INVOKING_CALLS.iter().any(|&(f, slot)| slot == index && call_named(tokens, call, &[f])) {
        return None;
    }
    let decl = crate::index::class_decls(tokens).into_iter().filter(|d| d.open < k && k < d.close).max_by_key(|d| d.open)?;
    if class == "parent" && decl.info.parent.is_none() {
        return None;
    }
    Some(Fix {
        extra: Vec::new(),
        class: FIRST_CLASS_CALLABLE,
        start: tokens[first].start,
        end: tokens[last].end,
        text: format!("{class}::{method}(...)"),
    })
}

/// Builtins that run a callback, and the ones that open a directory.
const CALLBACK_BUILTINS: &[&str] = &[
    "array_map",
    "array_filter",
    "array_walk",
    "array_walk_recursive",
    "array_reduce",
    "usort",
    "uasort",
    "uksort",
    "call_user_func",
    "call_user_func_array",
    "forward_static_call",
    "forward_static_call_array",
    "preg_replace_callback",
    "preg_replace_callback_array",
    "iterator_apply",
    "register_shutdown_function",
    "spl_autoload_call",
    "class_exists",
    "interface_exists",
    "trait_exists",
    "enum_exists",
    "is_subclass_of",
    "is_a",
    "opendir",
    "dir",
];

/// Whether the call at `j` is a PHP function that can't open a directory or run project code.
fn inert_builtin(tokens: &[Token], j: usize) -> bool {
    let t = &tokens[j];
    let name = t.text.trim_start_matches('\\').to_ascii_lowercase();
    let declared = tokens.windows(2).any(|w| w[0].is_ident_ci("function") && w[1].text.eq_ignore_ascii_case(&name));
    t.kind == TokenKind::Ident
        && !j.checked_sub(1).is_some_and(|p| ["->", "?->", "::"].iter().any(|s| tokens[p].is_punct(s)) || tokens[p].is_ident_ci("new"))
        && crate::arity::ARITY.binary_search_by(|(n, _, _)| (*n).cmp(name.as_str())).is_ok()
        && !CALLBACK_BUILTINS.contains(&name.as_str())
        && !declared
}

/// Whether anything from `from` to `to` could open another directory or run project code.
fn runs_code(tokens: &[Token], from: usize, to: usize) -> bool {
    (from..to).any(|j| starts_call(tokens, j) && !(tokens.get(j + 1).is_some_and(|n| n.is_punct("(")) && inert_builtin(tokens, j)))
}

/// `readdir()` → `readdir($h)` where `$h = opendir(...)` is the only directory its function opens
/// and nothing between them could open another.
fn directory_handle(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let names = ["readdir", "rewinddir", "closedir"];
    let call = if tokens[k].kind == TokenKind::Ident && call_named(tokens, k, &names) { k } else { enclosing_call(tokens, k)? };
    let (close, args) = function_call(tokens, call, &names)?;
    let target = match args[..] {
        [] => None,
        [(f, l)] if f == l && tokens[f].is_ident_ci("null") => Some(f),
        _ => return None,
    };
    let function = enclosing_function(tokens, call)?;
    let (open, end) = function.body?;
    let nested = nested_bodies(tokens, open, end);
    let own = |j: usize| !nested.iter().any(|&(a, b)| a <= j && j <= b);
    let openers: Vec<usize> = (open + 1..end).filter(|&j| own(j) && function_call(tokens, j, &["opendir", "dir"]).is_some()).collect();
    let [opener] = openers[..] else { return None };
    let handle = opener.checked_sub(2).filter(|&h| {
        tokens[h].kind == TokenKind::Variable
            && tokens[h + 1].is_punct("=")
            && call_named(tokens, opener, &["opendir"])
            && starts_statement(tokens, h)
    })?;
    let name = tokens[handle].text.as_str();
    let rebound = (open + 1..end).filter(|&j| j != handle && is_var(tokens, j, name)).any(|j| {
        let prev = &tokens[j - 1];
        flow_modifies(tokens, j)
            || prev.is_punct("&")
            || prev.is_ident_ci("as")
            || prev.is_ident_ci("global")
            || prev.is_ident_ci("static")
            || enclosing_call(tokens, j).is_some_and(|c| call_named(tokens, c, &["unset", "list"]))
    });
    let statement_end = (opener..call).find(|&j| tokens[j].is_punct(";"))?;
    if rebound || opener > call || reaches_scope(tokens, open + 1, end) || runs_code(tokens, statement_end, call) {
        return None;
    }
    for (from, to) in loops_around(tokens, statement_end, call) {
        if runs_code(tokens, from, to) {
            return None;
        }
    }
    match target {
        Some(null) => Some(Fix {
            extra: Vec::new(),
            class: DIRECTORY_HANDLE,
            start: tokens[null].start,
            end: tokens[null].end,
            text: format!("${name}"),
        }),
        None => Some(Fix {
            extra: Vec::new(),
            class: DIRECTORY_HANDLE,
            start: tokens[call + 1].start,
            end: tokens[close].end,
            text: format!("(${name})"),
        }),
    }
}

fn flow_modifies(tokens: &[Token], k: usize) -> bool {
    let next = tokens.get(k + 1);
    let compound = next.is_some_and(|n| ["+", "-", "*", "/", ".", "%", "&", "|", "^", "??"].iter().any(|o| n.is_punct(o)))
        && tokens.get(k + 2).is_some_and(|n| n.is_punct("="));
    next.is_some_and(|n| n.is_punct("=") || n.is_punct("++") || n.is_punct("--") || n.is_punct("["))
        || compound
        || k > 0 && (tokens[k - 1].is_punct("++") || tokens[k - 1].is_punct("--"))
}

/// On a PHP 5 project, `func_get_args()` after a parameter changed returned what was passed:
/// read it once at the top of the function and use that copy everywhere.
fn original_args(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let function = enclosing_function(tokens, k)?;
    let (open, close) = function.body?;
    if function.params.iter().any(|p| tokens[p.var - 1].is_punct("&")) {
        return None;
    }
    let required = function.params.iter().take_while(|p| p.default.is_none() && !tokens[p.var - 1].is_punct("...")).count();
    let nested = nested_bodies(tokens, open, close);
    let own = |j: usize| !nested.iter().any(|&(a, b)| a <= j && j <= b);
    let mut calls = Vec::new();
    for j in (open + 1..close).filter(|&j| own(j)) {
        if let Some((end, args)) = function_call(tokens, j, &["func_get_args"]) {
            if !args.is_empty() {
                return None;
            }
            calls.push((j, end, None));
        } else if let Some((end, args)) = function_call(tokens, j, &["func_get_arg"]) {
            let [(f, l)] = args[..] else { return None };
            let n = tokens[f].text.parse::<usize>().ok().filter(|&n| f == l && tokens[f].kind == TokenKind::Number && n < required)?;
            calls.push((j, end, Some(n)));
        }
    }
    let last = calls.last()?.1;
    let mentioned = |name: &str| (open..close).any(|j| is_var(tokens, j, name));
    let var = std::iter::once("originalArgs".to_string()).chain((2..).map(|n| format!("originalArgs{n}"))).find(|n| !mentioned(n))?;
    let src = file.src;
    let after = &src[tokens[open].end..tokens.get(open + 1).map_or(tokens[open].end, |t| t.start)];
    let lead = after.rfind('\n').map_or(" ", |i| &after[i..]);
    let mut text = format!("{lead}${var} = func_get_args();");
    let mut at = tokens[open].end;
    for &(start, end, index) in &calls {
        text.push_str(&src[at..tokens[start].start]);
        match index {
            Some(n) => text.push_str(&format!("${var}[{n}]")),
            None => text.push_str(&format!("${var}")),
        }
        at = tokens[end].end;
    }
    Some(Fix { extra: Vec::new(), class: ORIGINAL_ARGS, start: tokens[open].end, end: tokens[last].end, text })
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
    Some(Fix { extra: Vec::new(), class: WILL_CHANGE, start: at, end: at, text: format!("#[\\ReturnTypeWillChange]{newline}{indent}") })
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
    Some(Fix { extra: Vec::new(), class: SUBSTR_EMPTY, start: tokens[at].start, end: tokens[at].end, text: "''".into() })
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
            extra: Vec::new(),
            class: VARIABLE_BRACES,
            start: tokens[k].start,
            end: name.end,
            text: format!("${{{}}}", file.text(k + 1, k + 1)),
        });
    }
    let end = dimensions_end(tokens, k + 1)?;
    Some(Fix {
        extra: Vec::new(),
        class: VARIABLE_ORDER,
        start: tokens[k].start,
        end: tokens[end].end,
        text: format!("${{{}}}", file.text(k + 1, end)),
    })
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
        return Fix { extra: Vec::new(), class, start: line_start, end: line_end, text: String::new() };
    }
    let trailing = src[end..].len() - src[end..].trim_start_matches([' ', '\t']).len();
    Fix { extra: Vec::new(), class, start, end: end + trailing, text: String::new() }
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
            Some(Fix { extra: Vec::new(), class: UNSET_THIS, start, end: stop, text: String::new() })
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
    let fix = |start: usize, end: usize, text: String| Some(Fix { extra: Vec::new(), class: MHASH, start, end, text });
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
        return Some(Fix { extra: Vec::new(), class: REFRESH, start: tokens[k].start, end: tokens[close].end, text });
    }
    let (close, args) = method_call(tokens, k, &["refresh"])?;
    let [flags] = args[..] else { return None };
    let sql = flush_statement(tokens, flags)?;
    Some(Fix { extra: Vec::new(), class: REFRESH, start: tokens[k].start, end: tokens[close].end, text: format!("query({sql})") })
}

/// The mode argument of mysqli_store_result($link, MODE) or ->store_result(MODE) goes.
fn store_result_fix(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let simple = |a: (usize, usize)| {
        plain(tokens, a) || literal(tokens, a) || (a.0 == a.1 && is_atom(tokens, a.0)) || (a.1 == a.0 + 2 && tokens[a.0 + 1].is_punct("::"))
    };
    let fix = |start: usize, end: usize| Some(Fix { extra: Vec::new(), class: UNUSED_ARGUMENT, start, end, text: String::new() });
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
    Some(Fix { extra: Vec::new(), class: LDAP_PAGING, start: tokens[k].start, end: tokens[close].end, text: call })
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
    Some(Fix { extra: Vec::new(), class: NO_OP_FLAG, start, end, text: String::new() })
}

fn call_fix(file: &Source, k: usize, rule: &str) -> Option<Fix> {
    let tokens = file.tokens;
    let tok = &tokens[k];
    let bare = tok.text.trim_start_matches('\\');
    let value = |text: &str, class| Some(Fix { extra: Vec::new(), class, start: tok.start, end: tok.end, text: text.to_string() });
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
                return Some(Fix { extra: Vec::new(), class: KILL, start: tok.start, end: tokens[close].end, text });
            }
            let (close, args) = method_call(tokens, k, &["kill"])?;
            let [id] = args[..] else { return None };
            Some(Fix {
                extra: Vec::new(),
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
            Some(Fix {
                extra: Vec::new(),
                class: INTL_METHODS,
                start: tok.start,
                end: tokens[close].end,
                text: format!("{receiver}->{method}({rest})"),
            })
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
            Some(Fix { extra: Vec::new(), class: INTL_METHODS, start: tok.start, end: tokens[close].end, text })
        }
        "deprecated.lcg_value" => {
            let (close, args) = function_call(tokens, k, &["lcg_value"])?;
            let text = "(new \\Random\\Randomizer())->getFloat(0, 1, \\Random\\IntervalBoundary::OpenOpen)".to_string();
            args.is_empty().then(|| Fix { extra: Vec::new(), class: LCG_VALUE, start: tok.start, end: tokens[close].end, text })
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
                return Some(Fix {
                    extra: Vec::new(),
                    class: MAGIC_QUOTES,
                    start: tokens[k - 1].start,
                    end: tokens[close].end,
                    text: "true".into(),
                });
            }
            Some(Fix { extra: Vec::new(), class: MAGIC_QUOTES, start: tok.start, end: tokens[close].end, text: "false".into() })
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
                extra: Vec::new(),
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
                extra: Vec::new(),
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
            Some(Fix { extra: Vec::new(), class: GETSS, start: tok.start, end: tokens[close].end, text })
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
            Some(Fix { extra: Vec::new(), class: RECODE, start: tok.start, end: tokens[close].end, text: call })
        }
        "removed.ezmlm_hash" => {
            let (close, args) = function_call(tokens, k, &["ezmlm_hash"])?;
            let [address] = args[..] else { return None };
            Some(Fix {
                extra: Vec::new(),
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
            Some(Fix { extra: Vec::new(), class: REGCASE, start: tok.start, end: tokens[close].end, text })
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
                    extra: Vec::new(),
                    class: REFLECTION_EXPORT,
                    start: tokens[first].start,
                    end: tokens[close].end,
                    text: format!("echo {object}, \"\\n\""),
                }),
                "true" => Some(Fix {
                    extra: Vec::new(),
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
/// `guessed` holds the positions of findings whose receiver type was taken from elsewhere in the
/// class: their fixes would rest on a guess, so they stay manual.
pub(crate) fn mark(
    src: &str,
    tokens: &[Token],
    findings: &mut [Finding],
    index: &crate::index::ProjectIndex,
    target: PhpVersion,
    guessed: &HashSet<(usize, usize)>,
) {
    if !findings.iter().any(|f| f.fix == FixKind::Manual && handles(&f.rule_id)) {
        return;
    }
    let at_position = by_position(tokens);
    let file = Source::new(src, tokens, findings, &at_position, index, target);
    let open = |f: &Finding| f.fix == FixKind::Manual && !guessed.contains(&(f.line, f.column));
    let pick = |_: usize, f: &Finding, k: usize| if open(f) { at(&file, k, &f.rule_id) } else { None };
    for (i, fix) in plan(findings, &at_position, &pick) {
        if findings[i].fix == FixKind::Manual {
            findings[i].fix = crate::rector::describe_rule(fix.class).fix;
        }
    }
    for f in findings.iter_mut().filter(|f| OPTION_RULES.contains(&f.rule_id.as_str()) && !guessed.contains(&(f.line, f.column))) {
        let Some(&k) = at_position.get(&(f.line, f.column)) else { continue };
        let options = choices(&file, k, &f.rule_id);
        if options.len() < 2 && options.iter().all(|c| c.recommended) {
            continue;
        }
        f.options = options
            .into_iter()
            .map(|c| {
                let text = OPTION_TEXTS.iter().find(|(r, id, ..)| *r == f.rule_id && *id == c.id);
                let applied = crate::rector::describe_rule(c.fix.class);
                crate::analyzer::FixOption {
                    id: c.id.to_string(),
                    title: text.map_or_else(|| applied.title.clone(), |t| t.2.into()),
                    effect: text.map_or_else(|| applied.title.clone(), |t| t.3.into()),
                    code: after(src, &c.fix),
                    fix: applied.fix,
                    recommended: c.recommended,
                }
            })
            .collect();
    }
}

/// Which of the findings of its rule on its line each finding is, counted from the left.
fn ordinals(findings: &[Finding]) -> Vec<usize> {
    findings.iter().map(|f| findings.iter().filter(|o| o.rule_id == f.rule_id && o.line == f.line && o.column < f.column).count()).collect()
}

/// The fixes the findings allow, by finding. A fix inside another one, or crossing it, is left out
/// and its finding stays manual.
fn plan(
    findings: &[Finding],
    at_position: &std::collections::HashMap<(usize, usize), usize>,
    pick: &dyn Fn(usize, &Finding, usize) -> Option<Fix>,
) -> Vec<(usize, Fix)> {
    let all: Vec<(usize, Fix)> = findings
        .iter()
        .enumerate()
        .filter(|(_, f)| handles(&f.rule_id))
        .filter_map(|(i, f)| at_position.get(&(f.line, f.column)).and_then(|&k| pick(i, f, k)).map(|fix| (i, fix)))
        .collect();
    let nested = |a: &Fix, b: &Fix| a.edits().any(|(s, e, _)| b.edits().any(|(bs, be, _)| bs <= s && e <= be && (bs, be) != (s, e)));
    let mut outer: Vec<(usize, Fix)> = all.iter().filter(|(_, f)| !all.iter().any(|(_, o)| o != f && nested(f, o))).cloned().collect();
    outer.sort_by_key(|(_, f)| (f.start, f.end));
    let crosses = |a: &Fix, b: &Fix| {
        a != b
            && a.edits().any(|(s, e, _)| b.edits().any(|(bs, be, _)| (s, e) == (bs, be) || s < be && bs < e || s == e && bs < s && s < be))
    };
    let mut kept: Vec<(usize, Fix)> = Vec::new();
    for (i, fix) in outer {
        if kept.iter().any(|(_, k)| crosses(&fix, k)) {
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
    chosen: &[Chosen],
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
    let file = Source::new(&text, &tokens, &findings, &at_position, index, target);
    let ordinal = ordinals(&findings);
    let pick = |i: usize, f: &Finding, k: usize| {
        let choice = chosen.iter().find(|c| c.line == f.line && c.rule == f.rule_id && c.ordinal == ordinal[i]);
        match choice {
            Some(c) if f.options.iter().any(|o| o.id == c.option) => {
                choices(&file, k, &f.rule_id).into_iter().find(|o| o.id == c.option).map(|o| o.fix)
            }
            _ if f.fix != FixKind::Manual => at(&file, k, &f.rule_id),
            _ => None,
        }
    };
    let fixes: Vec<Fix> = plan(&findings, &at_position, &pick).into_iter().map(|(_, fix)| fix).collect();
    let mut edits: Vec<(usize, usize, &str, &'static str)> =
        fixes.iter().flat_map(|f| f.edits().map(move |(s, e, t)| (s, e, t, f.class))).collect();
    edits.sort_by_key(|&(s, e, ..)| (s, e));
    edits.dedup_by(|a, b| (a.0, a.1) == (b.0, b.1));
    edits.reverse();
    let mut out = src.to_string();
    let mut applied = Vec::new();
    let mut floor = usize::MAX;
    for (start, end, text, class) in edits {
        if end > floor {
            continue;
        }
        let line_start = |at: usize, s: &str| s[..at].rfind('\n').map_or(0, |i| i + 1);
        let whole_line = start == line_start(start, src) && (end == src.len() || src[..end].ends_with('\n'));
        out.replace_range(start..end, text);
        floor = start;
        if text.is_empty() && !whole_line {
            let from = line_start(start, &out);
            let to = out[start..].find('\n').map_or(out.len(), |i| start + i + 1);
            if out[from..to].chars().all(char::is_whitespace) {
                out.replace_range(from..to, "");
                floor = from;
            }
        }
        applied.push((start, class));
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
        apply("t.php", src, target, &index, &[])
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
        apply("t.php", src, target, &std::sync::Arc::new(index), &[])
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
        let index = crate::index::ProjectIndex::from_tokens(&tokens).with_current(php5);
        let file = Source::new(src, &tokens, &findings, &at_position, &index, PhpVersion::new(8, 5));
        let marked: String = plan(&findings, &at_position, &|_, f, k| at(&file, k, &f.rule_id)).into_iter().rev().fold(
            src.to_string(),
            |mut text, (_, fix)| {
                text.replace_range(fix.start..fix.end, &fix.text);
                text
            },
        );
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

    #[test]
    fn false_initializers_become_arrays() {
        let src = "<?php\nfunction f($raw) {\n    $tags = false;\n    $tags[] = $raw;\n    return $tags;\n}\nfunction g($c) {\n    $a = false;\n    if ($c) { $a[] = 1; }\n    return $a;\n}\nfunction h() {\n    $b = false;\n    $b['k'] = count($b);\n}\nfunction i() {\n    $d = false;\n    $x = 1;\n    $d['a']['b'] = $x;\n}\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(
            out,
            "<?php\nfunction f($raw) {\n    $tags = [];\n    $tags[] = $raw;\n    return $tags;\n}\nfunction g($c) {\n    $a = false;\n    if ($c) { $a[] = 1; }\n    return $a;\n}\nfunction h() {\n    $b = false;\n    $b['k'] = count($b);\n}\nfunction i() {\n    $d = [];\n    $x = 1;\n    $d['a']['b'] = $x;\n}\n"
        );
        assert_eq!(classes, [FALSE_ARRAY]);
        assert_eq!(kinds_of(&fixes(src), "deprecated.false_to_array"), [FixKind::Auto, FixKind::Manual, FixKind::Manual, FixKind::Auto]);
    }

    #[test]
    fn gmstrftime_becomes_gmdate() {
        let src = "<?php\necho gmstrftime('%Y-%m-%dT%H:%M:%SZ', $t), \\gmstrftime(\"%F %T\"), gmstrftime('%e %B', $t), gmstrftime($f), gmstrftime('100%% at %I:%M');\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(
            out,
            "<?php\necho gmdate('Y-m-d\\TH:i:s\\Z', $t), \\gmdate('Y-m-d H:i:s'), gmstrftime('%e %B', $t), gmstrftime($f), gmdate('100% \\a\\t h:i');\n"
        );
        assert_eq!(classes, [GMDATE]);
        let kinds = kinds_of(&fixes(src), "deprecated.gmstrftime");
        assert_eq!(kinds, [FixKind::Auto, FixKind::Auto, FixKind::Manual, FixKind::Manual, FixKind::Auto]);
        assert_eq!(quoted("a'b\\"), "'a\\'b\\\\'");
    }

    #[test]
    fn response_headers_read_right_after_their_request() {
        let src = "<?php\nfunction a($u) {\n    file_get_contents($u);\n    return $http_response_header;\n}\nfunction b($u) {\n    $body = @file_get_contents($u);\n    if (isset($http_response_header)) { return count($http_response_header); }\n}\nfunction c($u) {\n    file_get_contents($u);\n    log_it();\n    return $http_response_header;\n}\nfunction d($urls) {\n    foreach ($urls as $u) { file_get_contents($u); $h = $http_response_header; }\n}\nfunction e($u) {\n    file_get_contents('a.txt');\n    file_get_contents($u);\n    return $http_response_header;\n}\nfunction g($u) {\n    file_get_contents($u);\n    return end($http_response_header);\n}\nfunction i($u) {\n    file_get_contents($u);\n    return isset($http_response_header[0]);\n}\n";
        let (out, classes) = fixed(src).unwrap();
        assert!(out.contains("    file_get_contents($u);\n    return http_get_last_response_headers();\n}\nfunction b"), "{out}");
        assert!(
            out.contains("if (http_get_last_response_headers() !== null) { return count(http_get_last_response_headers()); }"),
            "{out}"
        );
        assert_eq!(out.matches("http_get_last_response_headers").count(), 3, "{out}");
        assert_eq!(classes, [LAST_HEADERS]);
        let kinds = kinds_of(&fixes(src), "deprecated.http_response_header");
        assert_eq!(kinds[..3], [FixKind::Likely; 3]);
        assert!(kinds[3..].iter().all(|k| *k == FixKind::Manual), "{kinds:?}");
    }

    #[test]
    fn null_keys_of_fresh_arrays() {
        let src = "<?php\nfunction f($rows) {\n    $lookup = array();\n    $lookup['a'] = 1;\n    $lookup[null] = $rows;\n    return isset($lookup[null]);\n}\nfunction g($o) {\n    $o[null] = 1;\n}\nfunction h($c) {\n    $a = [];\n    if ($c) { $a = new ArrayObject(); }\n    $a[null] = 1;\n}\nfunction i($c) {\n    if ($c) { $b = []; }\n    $b[null] = 1;\n}\n";
        let (out, classes) = fixed(src).unwrap();
        assert!(out.contains("    $lookup[''] = $rows;\n    return isset($lookup['']);"), "{out}");
        assert_eq!(out.matches("[null]").count(), 3, "{out}");
        assert_eq!(classes, [NULL_KEY]);
        let kinds = kinds_of(&fixes(src), "deprecated.null_offset");
        assert_eq!(kinds, [FixKind::Auto, FixKind::Auto, FixKind::Manual, FixKind::Manual, FixKind::Manual]);
    }

    #[test]
    fn relative_callables_become_first_class() {
        let src = "<?php\nclass K extends P {\n    public static function a($x) {\n        call_user_func('self::report', $x);\n        array_map(['parent', 'wrap'], $x);\n        usort($x, array('static', 'cmp'));\n        if (is_callable('self::report')) {}\n        set_error_handler('self::handle');\n    }\n}\nclass Solo {\n    public function b() { return call_user_func('parent::b'); }\n}\nfunction outside() { return call_user_func('self::x'); }\n";
        let (out, classes) = fixed(src).unwrap();
        assert!(
            out.contains(
                "call_user_func(self::report(...), $x);\n        array_map(parent::wrap(...), $x);\n        usort($x, static::cmp(...));"
            ),
            "{out}"
        );
        assert!(out.contains("is_callable('self::report')") && out.contains("set_error_handler('self::handle')"), "{out}");
        assert!(out.contains("call_user_func('parent::b')") && out.contains("call_user_func('self::x')"), "{out}");
        assert_eq!(classes, [FIRST_CLASS_CALLABLE]);
        let kinds = kinds_of(&fixes(src), "deprecated.partial_callable");
        assert_eq!(kinds[..3], [FixKind::Auto; 3]);
        assert_eq!(kinds[3..], [FixKind::Manual; 4]);
    }

    #[test]
    fn directory_calls_get_their_handle() {
        let src = "<?php\nfunction a($path) {\n    $dir = opendir($path);\n    while (($entry = readdir()) !== false) {\n        $text[] = strtoupper($entry);\n    }\n    rewinddir(null);\n    closedir();\n}\nfunction b($path) {\n    $dir = opendir($path);\n    while (($entry = readdir()) !== false) {\n        handle($entry);\n    }\n}\nfunction c($p, $q) {\n    $x = opendir($p);\n    $y = opendir($q);\n    readdir();\n}\nfunction d() {\n    readdir();\n}\nfunction e($path, $names) {\n    $dir = opendir($path);\n    foreach ($names as $n) {\n        $seen[] = readdir();\n    }\n}\n";
        let (out, classes) = fixed(src).unwrap();
        assert!(out.contains("while (($entry = readdir($dir)) !== false) {\n        $text[] = strtoupper($entry);\n    }\n    rewinddir($dir);\n    closedir($dir);"), "{out}");
        assert_eq!(out.matches("readdir()").count(), 3, "{out}");
        assert!(out.contains("$seen[] = readdir($dir);"), "{out}");
        assert_eq!(classes, [DIRECTORY_HANDLE]);
        let kinds = kinds_of(&fixes(src), "deprecated.implicit_argument");
        assert_eq!(
            kinds,
            [FixKind::Likely, FixKind::Likely, FixKind::Likely, FixKind::Manual, FixKind::Manual, FixKind::Manual, FixKind::Likely]
        );
    }

    #[test]
    fn original_arguments_on_php5() {
        let src = "<?php\nclass R {\n    public function audit($first, $second) {\n        // keep\n        $first = trim($first);\n        return func_get_args();\n    }\n    public function pick($a, $b = 2) {\n        $a++;\n        return [func_get_arg(0), func_get_arg(1)];\n    }\n    public function byRef(&$a) {\n        $a = 1;\n        return func_get_args();\n    }\n    public function once($x) { $x = 2; return func_get_args(); }\n}\n";
        let php5 = PhpVersion::new(5, 6);
        let (out, classes) = fixed_from(src, php5).unwrap();
        assert!(
            out.contains("public function audit($first, $second) {\n        $originalArgs = func_get_args();\n        // keep\n        $first = trim($first);\n        return $originalArgs;\n    }"),
            "{out}"
        );
        assert!(out.contains("public function once($x) { $originalArgs = func_get_args(); $x = 2; return $originalArgs; }"), "{out}");
        assert!(
            out.contains("return [func_get_arg(0), func_get_arg(1)];") && out.contains("$a = 1;\n        return func_get_args();"),
            "{out}"
        );
        assert_eq!(classes, [ORIGINAL_ARGS]);
        let kinds = kinds_of(&fixes_from(src, php5), "behavior.func_get_args_current_value");
        assert_eq!(kinds, [FixKind::Likely, FixKind::Manual, FixKind::Manual, FixKind::Likely]);
        assert_eq!(fixed_from(src, PhpVersion::new(7, 4)), None);
    }

    #[test]
    fn spl_wakeup_calls_go() {
        let src = "<?php\nfunction f($snapshot) {\n    $items = new SplFixedArray(3);\n    $items->__wakeup();\n    return $items;\n}\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\nfunction f($snapshot) {\n    $items = new SplFixedArray(3);\n    return $items;\n}\n");
        assert_eq!(classes, [WAKEUP_CALL]);
        assert_eq!(kinds_of(&fixes(src), "deprecated.splfixedarray_wakeup"), [FixKind::Likely]);
        let subclass = format!("{src}class Slots extends SplFixedArray {{ public $extra = 1; }}\n");
        assert_eq!(fixed(&subclass), None);
        let guessed = "<?php\nclass Q {\n    public function make() { $items = new SplFixedArray(2); return $items; }\n    public function restore($items) { $items->__wakeup(); }\n}\n";
        let kinds = kinds_of(&fixes(guessed), "deprecated.splfixedarray_wakeup");
        assert!(kinds.iter().all(|k| *k == FixKind::Manual), "{kinds:?}");
        assert_eq!(fixed(guessed), None);
    }

    #[test]
    fn reflection_type_checks() {
        let src = "<?php\nfunction f(ReflectionFunction $rf) {\n    foreach ($rf->getParameters() as $param) {\n        $a = $param->isArray();\n        $c = $param->isCallable();\n        $k = $param->getClass();\n    }\n    return $rf->isDisabled() || (new ReflectionFunction('x'))->isDisabled();\n}\n";
        let (out, classes) = fixed(src).unwrap();
        assert!(out.contains("$a = (static function (\\ReflectionParameter $p): bool { $t = $p->getType();"), "{out}");
        assert!(out.contains("return $b === ['array']; })($param);\n        $c = (static function"), "{out}");
        assert!(out.contains("return $b === ['callable']; })($param);\n        $k = $param->getClass();"), "{out}");
        assert!(out.contains("return false || (new ReflectionFunction('x'))->isDisabled();"), "{out}");
        assert_eq!(classes, [REFLECTION_TYPE_CHECK, DISABLED_FALSE]);
        let kinds = kinds_of(&fixes(src), "deprecated.reflection_method");
        assert_eq!(kinds, [FixKind::Auto, FixKind::Auto, FixKind::Manual, FixKind::Auto, FixKind::Manual]);
    }

    #[test]
    fn xml_handlers_become_callables() {
        let src = "<?php\nclass Feed {\n    public function parse($xml, $handler) {\n        $parser = xml_parser_create();\n        xml_set_object($parser, $handler);\n        xml_set_element_handler($parser, 'openTag', 'closeTag');\n        xml_set_character_data_handler($parser, 'text');\n        xml_set_default_handler($parser, '');\n        xml_parse($parser, $xml);\n    }\n    public function other($xml) {\n        $p = xml_parser_create();\n        xml_set_object($p, $this);\n        xml_set_element_handler($p, 'start', 'end');\n    }\n}\n";
        let (out, classes) = fixed(src).unwrap();
        assert!(
            out.contains("        $parser = xml_parser_create();\n        xml_set_element_handler($parser, [$handler, 'openTag'], [$handler, 'closeTag']);\n        xml_set_character_data_handler($parser, [$handler, 'text']);\n        xml_set_default_handler($parser, null);\n"),
            "{out}"
        );
        assert!(out.contains("xml_set_object($p, $this);\n        xml_set_element_handler($p, 'start', 'end');"), "{out}");
        let shared = src.replace("($p,", "($parser,").replace("$p = ", "$parser = ");
        assert_eq!(shared.matches("$parser").count(), 9, "{shared}");
        let found = fixes(&shared);
        assert_eq!(
            kinds_of(&found, "deprecated.xml_set_object"),
            [FixKind::Auto, FixKind::Manual],
            "a local in another method is another parser"
        );
        let property = shared.replace("$parser", "$this->parser");
        assert_eq!(
            kinds_of(&fixes(&property), "deprecated.xml_set_object"),
            [FixKind::Manual, FixKind::Manual],
            "a property is shared by the methods"
        );
        assert_eq!(classes, [XML_OBJECT, XML_CALLABLE]);
        let found = fixes(src);
        assert_eq!(kinds_of(&found, "deprecated.xml_handler_string"), [FixKind::Auto, FixKind::Auto, FixKind::Auto, FixKind::Manual]);
        assert_eq!(kinds_of(&found, "deprecated.xml_set_object"), [FixKind::Auto, FixKind::Manual]);
    }

    #[test]
    fn session_callables_get_a_handler_object() {
        let src = "<?php\nfunction boot() {\n    session_set_save_handler('m_open', 'm_close', 'm_read', 'm_write', 'm_destroy', 'm_gc');\n}\nclass S {\n    public function start() {\n        session_set_save_handler([$this, 'open'], [$this, 'close'], [$this, 'read'], [$this, 'write'], [$this, 'destroy'], [$this, 'gc']);\n    }\n    public function open($p, $n) {} public function close() {} public function read($i) {} public function write($i, $d) {} public function destroy($i) {} private function gc($m) {}\n}\nsession_set_save_handler($o, $c, $r, $w, $d, $g);\n";
        let (out, classes) = fixed(src).unwrap();
        assert!(
            out.starts_with("<?php\nfunction boot() {\n    session_set_save_handler(new class('m_open', 'm_close', 'm_read', 'm_write', 'm_destroy', 'm_gc') implements \\SessionHandlerInterface {\n        private $c;\n\n        public function __construct(...$c)\n        {\n            $this->c = $c;\n        }\n\n        #[\\ReturnTypeWillChange]\n        public function open($path, $name)\n        {\n            return ($this->c[0])($path, $name);\n        }\n"),
            "{out}"
        );
        assert!(out.contains("            return ($this->c[5])($max);\n        }\n    }, false);\n}\nclass S {"), "{out}");
        assert!(out.contains("session_set_save_handler([$this, 'open'],") && out.contains("session_set_save_handler($o, $c,"), "{out}");
        assert_eq!(classes, [SESSION_HANDLER]);
        assert_eq!(kinds_of(&fixes(src), "deprecated.session_set_save_handler"), [FixKind::Likely, FixKind::Manual, FixKind::Manual]);
    }

    #[test]
    fn autoload_becomes_a_registered_closure() {
        let src = "<?php\nerror_reporting(E_ALL);\n\nfunction __autoload($class)\n{\n    require __DIR__ . '/' . $class . '.php';\n}\n\n$x = new Foo();\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\nerror_reporting(E_ALL);\n\nspl_autoload_register(function ($class)\n{\n    require __DIR__ . '/' . $class . '.php';\n});\n\n$x = new Foo();\n");
        assert_eq!(classes, [AUTOLOAD_REGISTER]);
        assert_eq!(kinds_of(&fixes(src), "removed.__autoload"), [FixKind::Likely]);
        for other in [
            format!("{src}spl_autoload_register('load');\n"),
            format!("{src}require __DIR__ . '/vendor/autoload.php';\n"),
            src.replace("error_reporting(E_ALL);", "$early = new Bar();"),
            src.replace("require __DIR__", "echo __FUNCTION__; require __DIR__"),
        ] {
            assert_eq!(fixed(&other), None, "{other}");
        }
    }

    #[test]
    fn wbmp_conversions_run_in_place() {
        let src = "<?php\nfunction t($png, $jpg, $args) {\n    png2wbmp($png, 'p.wbmp', 32, 32, 7);\n    $ok = jpeg2wbmp($jpg, 'j.wbmp', 20, 30, 4);\n    png2wbmp(...$args);\n}\n";
        let (out, classes) = fixed(src).unwrap();
        assert!(
            out.contains(
                "    (static function ($source, $destination, $height, $width, $threshold) {\n        $threshold = (int) $threshold;\n"
            ),
            "{out}"
        );
        assert!(out.contains("        return true;\n    })($png, 'p.wbmp', 32, 32, 7);\n"), "{out}");
        assert!(out.contains("    $ok = (static function ($source,") && out.contains("$image = @imagecreatefromjpeg($source);"), "{out}");
        assert!(out.contains("trigger_error(\"jpeg2wbmp(): Invalid threshold value '$threshold'\", E_USER_WARNING);"), "{out}");
        assert!(out.contains("    png2wbmp(...$args);\n"), "{out}");
        assert_eq!(classes, [WBMP_PORT]);
        let found = fixes(src);
        assert_eq!(kinds_of(&found, "removed.png2wbmp"), [FixKind::Likely, FixKind::Manual]);
        assert_eq!(kinds_of(&found, "removed.jpeg2wbmp"), [FixKind::Likely]);
    }

    #[test]
    fn built_in_overrides_widen() {
        let src = "<?php\nclass PdoConnection extends PDO {\n    public function query($sql, array $params = array())\n    {\n        return parent::query($sql);\n    }\n}\nclass Items implements Countable {\n    public function count($mode) { return 0; }\n}\nclass Bag extends ArrayObject {\n    public function setFlags(array $flags) {}\n    public function offsetGet(int $key) {}\n    public function append(&$value) {}\n}\nclass Fixed implements Countable {\n    public static function count() { return 0; }\n}\n";
        let (out, classes) = fixed(src).unwrap();
        assert!(
            out.contains(
                "    #[\\ReturnTypeWillChange]\n    public function query($sql, array|int|null $params = array(), ...$rest)\n    {"
            ),
            "{out}"
        );
        assert!(out.contains("    #[\\ReturnTypeWillChange]\n    public function count($mode = null) { return 0; }"), "{out}");
        assert!(out.contains("    #[\\ReturnTypeWillChange]\n    public function setFlags(array|int $flags) {}"), "{out}");
        assert!(out.contains("    public function offsetGet(int $key) {}\n    public function append(&$value) {}"), "{out}");
        assert!(out.contains("    public static function count() { return 0; }"), "{out}");
        assert_eq!(classes, [WIDEN_OVERRIDE]);
        let kinds = kinds_of(&fixes(src), "removed.incompatible_override");
        assert_eq!(kinds, [FixKind::Likely, FixKind::Likely, FixKind::Likely, FixKind::Manual, FixKind::Manual, FixKind::Manual]);
        let again = fixes(&out);
        assert_eq!(kinds_of(&again, "removed.incompatible_override").len(), 3, "only the three left manual remain");
    }

    fn options_of(src: &str, rule: &str) -> Vec<Vec<(String, FixKind, bool, String)>> {
        analyze_source("t.php", src, PhpVersion::new(8, 5))
            .into_iter()
            .filter(|f| f.rule_id == rule)
            .map(|f| f.options.into_iter().map(|o| (o.id, o.fix, o.recommended, o.code)).collect())
            .collect()
    }

    fn fixed_choosing(src: &str, chosen: &[(usize, &str, &str)]) -> Option<(String, Vec<&'static str>)> {
        let target = PhpVersion::new(8, 5);
        let index = std::sync::Arc::new(crate::index::ProjectIndex::from_tokens_for(&crate::lexer::tokenize(src), Some(target)));
        let chosen: Vec<Chosen> =
            chosen.iter().map(|&(line, rule, option)| Chosen { line, rule: rule.into(), ordinal: 0, option: option.into() }).collect();
        apply("t.php", src, target, &index, &chosen)
    }

    #[test]
    fn rfc7231_offers_the_literal_or_utc() {
        let src = "<?php\nheader('Expires: ' . date(DATE_RFC7231, $t));\necho $d->format(DateTimeInterface::RFC7231);\n$formats = [DateTime::RFC7231];\n";
        let rule = "deprecated.date_rfc7231";
        let options = options_of(src, rule);
        assert_eq!(options.len(), 3, "{options:?}");
        assert_eq!(
            options[0][0],
            ("literal".into(), FixKind::Auto, true, "header('Expires: ' . date('D, d M Y H:i:s \\G\\M\\T', $t));".into())
        );
        assert_eq!(
            options[0][1],
            ("utc".into(), FixKind::Likely, false, "header('Expires: ' . gmdate('D, d M Y H:i:s \\G\\M\\T', $t));".into())
        );
        assert_eq!(
            options[1][1].3,
            "echo \\DateTimeImmutable::createFromInterface($d)->setTimezone(new \\DateTimeZone('UTC'))->format('D, d M Y H:i:s \\G\\M\\T');"
        );
        assert!(options[2].is_empty(), "a constant that formats nothing has one fix");
        assert_eq!(kinds_of(&fixes(src), rule), [FixKind::Auto; 3]);
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\nheader('Expires: ' . date('D, d M Y H:i:s \\G\\M\\T', $t));\necho $d->format('D, d M Y H:i:s \\G\\M\\T');\n$formats = ['D, d M Y H:i:s \\G\\M\\T'];\n");
        assert_eq!(classes, [RFC_LITERAL]);
        let (chose, classes) = fixed_choosing(src, &[(2, rule, "utc")]).unwrap();
        assert!(
            chose.starts_with("<?php\nheader('Expires: ' . gmdate('D, d M Y H:i:s \\G\\M\\T', $t));\necho $d->format('D, d M Y"),
            "{chose}"
        );
        assert_eq!(classes, [RFC_UTC, RFC_LITERAL]);
    }

    #[test]
    fn bad_state_calls_wait_for_a_choice() {
        let src = "<?php\nfunction f(SplFileInfo $file) {\n    $file->_bad_state_ex();\n    return 1;\n}\n";
        let rule = "deprecated.splfileinfo_bad_state_ex";
        let options = options_of(src, rule);
        assert_eq!(options[0].iter().map(|o| (o.0.as_str(), o.2)).collect::<Vec<_>>(), [("remove", false), ("throw", false)]);
        assert_eq!(options[0][0].3, "", "deleting leaves no line");
        assert_eq!(kinds_of(&fixes(src), rule), [FixKind::Manual]);
        assert_eq!(fixed(src), None);
        let (out, _) = fixed_choosing(src, &[(3, rule, "throw")]).unwrap();
        assert_eq!(out, "<?php\nfunction f(SplFileInfo $file) {\n    throw new \\Error('The parent constructor was not called: the object is in an invalid state');\n    return 1;\n}\n");
        let (out, _) = fixed_choosing(src, &[(3, rule, "remove")]).unwrap();
        assert_eq!(out, "<?php\nfunction f(SplFileInfo $file) {\n    return 1;\n}\n");
        assert_eq!(fixed_choosing(src, &[(3, rule, "nonsense")]), None, "an unknown option changes nothing");
    }

    #[test]
    fn user_errors_throw_unless_the_project_handles_errors() {
        let src = "<?php\nfunction fail_hard($message) {\n    trigger_error($message, E_USER_ERROR);\n}\n";
        let rule = "deprecated.e_user_error";
        let options = options_of(src, rule);
        assert_eq!(
            options[0][0],
            ("exception".into(), FixKind::Likely, true, "throw new \\ErrorException($message, 0, E_USER_ERROR);".into())
        );
        assert_eq!(options[0][1], ("warning".into(), FixKind::Likely, false, "trigger_error($message, E_USER_WARNING); exit(255);".into()));
        let handled = format!("{src}set_error_handler('handle');\n");
        let options = options_of(&handled, rule);
        assert_eq!(options[0].iter().map(|o| o.2).collect::<Vec<_>>(), [false, true]);
        assert!(fixed(&handled).unwrap().0.contains("    trigger_error($message, E_USER_WARNING); exit(255);\n"));
        assert!(options_of("<?php\nif ($x) trigger_error('m', E_USER_ERROR);\n", rule)[0].is_empty());
    }

    #[test]
    fn request_and_session_input_keep_their_old_result() {
        let src = "<?php\n$a = [filter_input(INPUT_REQUEST, 'id', FILTER_SANITIZE_MAGIC_QUOTES), filter_input(INPUT_SESSION, 'user')];\n$b = filter_has_var(INPUT_REQUEST, 'id');\n$c = filter_input_array(INPUT_SESSION, $definition);\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\n$a = [null, null];\n$b = false;\n$c = null;\n");
        assert_eq!(classes, [INPUT_OLD]);
        let options = options_of(src, "removed.input_request");
        assert_eq!(options[0][1].3, "$a = [isset($_REQUEST['id']) ? filter_var($_REQUEST['id'], FILTER_SANITIZE_ADD_SLASHES) : null, filter_input(INPUT_SESSION, 'user')];");
        assert_eq!(options[1][1].3, "$b = array_key_exists('id', $_REQUEST ?? []);");
        assert_eq!(
            options_of(src, "removed.input_session")[1][1].3,
            "$c = isset($_SESSION) ? filter_var_array($_SESSION, $definition) : null;"
        );
    }

    #[test]
    fn soap_functions_all() {
        let src = "<?php\n$server->addFunction(SOAP_FUNCTIONS_ALL);\n$list = [SOAP_FUNCTIONS_ALL, 1];\n";
        let rule = "deprecated.soap_functions_all";
        let options = options_of(src, rule);
        assert_eq!(options[0][0].3, "$server->addFunction(array_merge(...array_values(get_defined_functions())));");
        assert_eq!(options[0][1].3, "$server->addFunction(get_defined_functions()['user']);");
        assert!(options[1].is_empty());
        let (out, _) = fixed(src).unwrap();
        assert_eq!(out, "<?php\n$server->addFunction(array_merge(...array_values(get_defined_functions())));\n$list = [999, 1];\n");
        assert_eq!(kinds_of(&fixes(src), rule), [FixKind::Likely, FixKind::Auto]);
    }

    #[test]
    fn date_wakeup_calls_go() {
        let src = "<?php\nfunction restore(DateTime $date) {\n    $date->__wakeup();\n    return $date;\n}\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\nfunction restore(DateTime $date) {\n    return $date;\n}\n");
        assert_eq!(classes, [DATE_WAKEUP_DROP]);
        assert_eq!(fixed(&format!("{src}class Stamp extends DateTime {{ public function __wakeup(): void {{}} }}\n")), None);
    }

    fn options_from(src: &str, rule: &str, current: PhpVersion) -> Vec<Vec<(String, FixKind, bool, String)>> {
        let target = PhpVersion::new(8, 5);
        let index = crate::index::ProjectIndex::from_tokens_for(&crate::lexer::tokenize(src), Some(target)).with_current(current);
        crate::analyzer::analyze_source_in("t.php", src, target, &index)
            .into_iter()
            .filter(|f| f.rule_id == rule)
            .map(|f| f.options.into_iter().map(|o| (o.id, o.fix, o.recommended, o.code)).collect())
            .collect()
    }

    #[test]
    fn concatenated_arithmetic_gets_parentheses() {
        let src = "<?php\necho 'Done: ' . $done + 1 . ' of ' . $total;\n$s = \"1\" . 2 << 1;\n$x = $a . $b + $c * 2 - 1 . $d;\n$y = $a << 2 . $b;\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(
            out,
            "<?php\necho 'Done: ' . ($done + 1) . ' of ' . $total;\n$s = \"1\" . (2 << 1);\n$x = $a . ($b + $c * 2 - 1) . $d;\n$y = ($a << 2) . $b;\n"
        );
        assert_eq!(classes, [CONCAT_GROUPS]);
        assert_eq!(kinds_of(&fixes(src), "behavior.concat_precedence"), [FixKind::Likely; 4]);
        assert_eq!(fixed("<?php\n$v = $a . $b + 1 & 3;\n"), None, "a bitwise operator binds looser than the dot");
    }

    #[test]
    fn html_flags_for_every_call_in_the_file() {
        let src = "<?php\necho htmlspecialchars($name);\necho htmlentities($a), html_entity_decode($b, ENT_QUOTES);\n";
        let rule = "behavior.html_default_flags";
        let options = options_of(src, rule);
        assert_eq!(options.len(), 1, "the first call stands for the file");
        assert_eq!(
            options[0].iter().map(|o| (o.0.as_str(), o.1, o.2)).collect::<Vec<_>>(),
            [("old", FixKind::Auto, true), ("new", FixKind::Likely, false)]
        );
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\necho htmlspecialchars($name, ENT_COMPAT | ENT_HTML401);\necho htmlentities($a, ENT_COMPAT | ENT_HTML401), html_entity_decode($b, ENT_QUOTES);\n");
        assert_eq!(classes, [HTML_OLD]);
        let (out, _) = fixed_choosing(src, &[(2, rule, "new")]).unwrap();
        assert!(out.contains("htmlentities($a, ENT_QUOTES | ENT_SUBSTITUTE | ENT_HTML401)"), "{out}");
    }

    #[test]
    fn mysqli_checks_turn_reporting_off_on_older_projects() {
        let src = "<?php\nfunction a($db, $sql) {\n    if (mysqli_query($db, $sql) === false) {\n        return;\n    }\n    $link = mysqli_connect('h', 'u', 'p') or die('no db');\n}\n";
        let rule = "behavior.mysqli_default_exceptions";
        let options = options_of(src, rule);
        assert_eq!(options[0].iter().map(|o| (o.0.as_str(), o.2)).collect::<Vec<_>>(), [("report_off", false), ("catch", false)][..1]);
        assert_eq!(options[1][1].3, "try { $link = mysqli_connect('h', 'u', 'p'); } catch (\\mysqli_sql_exception $e) { die('no db'); }");
        assert_eq!(kinds_of(&fixes(src), rule), [FixKind::Manual; 2], "nothing is assumed when the project's version is unknown");
        let (out, _) = fixed_from(src, PhpVersion::new(7, 4)).unwrap();
        assert_eq!(
            out,
            "<?php\nfunction a($db, $sql) {\n    mysqli_report(MYSQLI_REPORT_OFF);\n    if (mysqli_query($db, $sql) === false) {\n        return;\n    }\n    mysqli_report(MYSQLI_REPORT_OFF);\n    $link = mysqli_connect('h', 'u', 'p') or die('no db');\n}\n"
        );
        let older = options_from(src, rule, PhpVersion::new(7, 4));
        assert!(older[0].is_empty(), "a single recommended fix needs no choice");
        assert_eq!(older[1].iter().map(|o| (o.0.as_str(), o.2)).collect::<Vec<_>>(), [("report_off", true), ("catch", false)]);
    }

    #[test]
    fn ping_becomes_a_query_or_goes() {
        let src = "<?php\nfunction p(mysqli $db, $link) {\n    if (!mysqli_ping($link)) {\n        $link = reconnect();\n    }\n    $ok = $db->ping();\n}\n";
        let rule = "deprecated.mysqli_ping";
        let options = options_of(src, rule);
        assert_eq!(options[0][0].3, "if (!(bool) mysqli_query($link, 'DO 1')) {");
        assert_eq!(options[0][1].3, "");
        assert!(options[1].is_empty(), "an assigned ping can't be dropped, so there is nothing to choose");
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(out, "<?php\nfunction p(mysqli $db, $link) {\n    if (!(bool) mysqli_query($link, 'DO 1')) {\n        $link = reconnect();\n    }\n    $ok = (bool) $db->query('DO 1');\n}\n");
        assert_eq!(classes, [PING_QUERY]);
        let (out, _) = fixed_choosing(src, &[(3, rule, "drop")]).unwrap();
        assert_eq!(out, "<?php\nfunction p(mysqli $db, $link) {\n    $ok = (bool) $db->query('DO 1');\n}\n");
    }

    #[test]
    fn currency_types_use_the_currency_methods() {
        let src = "<?php\nfunction c(NumberFormatter $f, $n, $s) {\n    return [$f->format($n, NumberFormatter::TYPE_CURRENCY), $f->parse($s, \\NumberFormatter::TYPE_CURRENCY), numfmt_format($f, $n, NumberFormatter::TYPE_CURRENCY)];\n}\n";
        let rule = "removed.numberformatter_type_currency";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(
            out,
            "<?php\nfunction c(NumberFormatter $f, $n, $s) {\n    return [$f->formatCurrency($n, $f->getTextAttribute(NumberFormatter::CURRENCY_CODE)), $f->parseCurrency($s, $currency), numfmt_format_currency($f, $n, numfmt_get_text_attribute($f, NumberFormatter::CURRENCY_CODE))];\n}\n"
        );
        assert_eq!(classes, [CURRENCY_METHOD]);
        let (out, _) = fixed_from(src, PhpVersion::new(7, 4)).unwrap();
        assert_eq!(out, "<?php\nfunction c(NumberFormatter $f, $n, $s) {\n    return [false, false, false];\n}\n");
        assert_eq!(kinds_of(&fixes_from(src, PhpVersion::new(7, 4)), rule), [FixKind::Auto; 3]);
    }

    #[test]
    fn magic_variables_keep_the_null_they_held() {
        let src = "<?php\nclass M {\n    public function body() {\n        return [$HTTP_RAW_POST_DATA, $php_errormsg];\n    }\n}\n$raw = $HTTP_RAW_POST_DATA;\n";
        let rule = "removed.magic_variable";
        let options = options_of(src, rule);
        assert_eq!(
            options.iter().map(|o| o.iter().map(|c| c.2).collect::<Vec<_>>()).collect::<Vec<_>>(),
            [vec![true, false], vec![false, false], vec![false, false]]
        );
        assert_eq!(options[0][1].3, "return [file_get_contents('php://input'), $php_errormsg];");
        assert_eq!(options[1][1].3, "return [$HTTP_RAW_POST_DATA, (error_get_last()['message'] ?? null)];");
        assert_eq!(kinds_of(&fixes(src), rule), [FixKind::Auto, FixKind::Manual, FixKind::Manual]);
        assert_eq!(fixed(src).unwrap().0, src.replacen("[$HTTP_RAW_POST_DATA,", "[null,", 1));
    }

    #[test]
    fn entity_loader_calls_go_unless_the_project_loads_entities() {
        let src = "<?php\nfunction load($xml) {\n    libxml_disable_entity_loader(true);\n    $doc = simplexml_load_string($xml);\n    libxml_disable_entity_loader(false);\n    return $doc;\n}\nfunction safe($xml) {\n    $old = libxml_disable_entity_loader(true);\n    $doc = simplexml_load_string($xml);\n    libxml_disable_entity_loader($old);\n    return $doc;\n}\n";
        let (out, classes) = fixed(src).unwrap();
        assert_eq!(
            out,
            "<?php\nfunction load($xml) {\n    $doc = simplexml_load_string($xml);\n    return $doc;\n}\nfunction safe($xml) {\n    $doc = simplexml_load_string($xml);\n    return $doc;\n}\n"
        );
        assert_eq!(classes, [ENTITY_DROP]);
        assert_eq!(kinds_of(&fixes(src), "deprecated.libxml_disable_entity_loader"), [FixKind::Likely; 4]);
        let noent = format!("{src}simplexml_load_string($x, null, LIBXML_NOENT);\n");
        let (out, classes) = fixed(&noent).unwrap();
        assert!(out.contains("    libxml_set_external_entity_loader(static fn ($public, $system, $context) => null);\n    $doc = simplexml_load_string($xml);\n    libxml_set_external_entity_loader(null);\n    return $doc;\n}\nfunction safe"), "{out}");
        assert!(out.contains("function safe($xml) {\n    libxml_set_external_entity_loader(static fn ($public, $system, $context) => null);\n    $doc = simplexml_load_string($xml);\n    libxml_set_external_entity_loader(null);\n"), "{out}");
        assert_eq!(classes, [ENTITY_LOADER]);
    }
}
