//! Ferry's own fixes for findings Rector doesn't handle. Each one rewrites only the tokens of a
//! finding it recognises, so a finding counts as automatic exactly when one of these takes it.

use crate::analyzer::Finding;
use crate::lexer::{Token, TokenKind};
use crate::rules::FixKind;
use crate::version::PhpVersion;

pub const CONSTANT_VALUE: &str = "Ferry\\ConstantToValue";
pub const CONSTANT_RENAME: &str = "Ferry\\RenameConstant";
pub const OCI_ALIAS: &str = "Ferry\\Php80\\RenameOciAlias";

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

fn handles(rule: &str) -> bool {
    rule == "deprecated.pgsql_transaction_constant"
        || rule == "deprecated.oci8_alias"
        || CONSTANT_VALUES.iter().chain(RENAMED_CONSTANTS).any(|(r, ..)| *r == rule)
}

/// Whether the text names anything a fix could take, checked before the file is analyzed again.
pub(crate) fn mentions(src: &str) -> bool {
    if CONSTANT_VALUES.iter().chain(RENAMED_CONSTANTS).any(|(_, name, _)| src.contains(name)) || src.contains("TRANSACTION_") {
        return true;
    }
    let lower = src.to_ascii_lowercase();
    OCI_ALIASES.iter().any(|(old, _)| lower.contains(old))
}

/// The fix for the finding of `rule` at token `k`, if Ferry has one for it.
pub fn at(tokens: &[Token], k: usize, rule: &str) -> Option<Fix> {
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

fn by_position(tokens: &[Token]) -> std::collections::HashMap<(usize, usize), usize> {
    tokens.iter().enumerate().map(|(k, t)| ((t.line, t.column), k)).collect()
}

/// Marks the findings a fix takes as automatic.
pub(crate) fn mark(tokens: &[Token], findings: &mut [Finding]) {
    if !findings.iter().any(|f| f.fix == FixKind::Manual && handles(&f.rule_id)) {
        return;
    }
    let at_position = by_position(tokens);
    for f in findings.iter_mut().filter(|f| f.fix == FixKind::Manual && handles(&f.rule_id)) {
        if let Some(fix) = at_position.get(&(f.line, f.column)).and_then(|&k| at(tokens, k, &f.rule_id)) {
            f.fix = crate::rector::describe_rule(fix.class).fix;
        }
    }
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
    let tokens = crate::lexer::tokenize(&if classic_mac { src.replace('\r', "\n") } else { src.to_string() });
    let at_position = by_position(&tokens);
    let mut fixes: Vec<Fix> = findings
        .iter()
        .filter(|f| handles(&f.rule_id))
        .filter_map(|f| at_position.get(&(f.line, f.column)).and_then(|&k| at(&tokens, k, &f.rule_id)))
        .collect();
    fixes.sort_by_key(|f| std::cmp::Reverse(f.start));
    fixes.dedup_by(|a, b| a.start == b.start);
    let mut out = src.to_string();
    let mut classes = Vec::new();
    let mut floor = usize::MAX;
    for fix in fixes {
        if fix.end > floor {
            continue;
        }
        out.replace_range(fix.start..fix.end, &fix.text);
        floor = fix.start;
        if !classes.contains(&fix.class) {
            classes.push(fix.class);
        }
    }
    classes.reverse();
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
}
