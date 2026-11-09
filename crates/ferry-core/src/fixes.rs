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
];

fn handles(rule: &str) -> bool {
    rule == "deprecated.pgsql_transaction_constant"
        || rule == "deprecated.oci8_alias"
        || rule == "deprecated.curlopt_binarytransfer"
        || rule == "removed.unset_this"
        || CONSTANT_VALUES.iter().chain(RENAMED_CONSTANTS).any(|(r, ..)| *r == rule)
        || NO_OP_CALLS.iter().any(|(r, ..)| *r == rule)
        || NO_OP_FLAGS.iter().any(|(r, ..)| *r == rule)
}

/// Whether the text names anything a fix could take, checked before the file is analyzed again.
pub(crate) fn mentions(src: &str) -> bool {
    let names =
        CONSTANT_VALUES.iter().chain(RENAMED_CONSTANTS).map(|(_, name, _)| *name).chain(NO_OP_FLAGS.iter().map(|(_, name, _)| *name));
    if names.chain(["TRANSACTION_", "CURLOPT_BINARYTRANSFER"]).any(|name| src.contains(name)) {
        return true;
    }
    let lower = src.to_ascii_lowercase();
    OCI_ALIASES.iter().map(|(old, _)| *old).chain(NO_OP_CALLS.iter().map(|(_, f, _)| *f)).any(|name| lower.contains(name))
        || (lower.contains("unset") && src.contains("$this"))
}

/// A file as the fixes see it: its text, its tokens, and which tokens hold a no-op flag finding.
pub(crate) struct Source<'a> {
    pub(crate) src: &'a str,
    pub(crate) tokens: &'a [Token],
    pub(crate) flagged: HashSet<usize>,
}

impl<'a> Source<'a> {
    fn new(
        src: &'a str,
        tokens: &'a [Token],
        findings: &[Finding],
        at_position: &std::collections::HashMap<(usize, usize), usize>,
    ) -> Self {
        let flagged = findings
            .iter()
            .filter(|f| NO_OP_FLAGS.iter().any(|(r, ..)| *r == f.rule_id))
            .filter_map(|f| at_position.get(&(f.line, f.column)).copied())
            .collect();
        Source { src, tokens, flagged }
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
    if let Some(fix) = statement_fix(file, k, rule) {
        return Some(fix);
    }
    if NO_OP_FLAGS.iter().any(|(r, ..)| *r == rule) {
        return flag_fix(file, k);
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
fn flag_fix(file: &Source, k: usize) -> Option<Fix> {
    let tokens = file.tokens;
    let (operands, operators) = mask(tokens, k)?;
    let (first, last) = (operands[0].first, operands.last()?.atom);
    let class = NO_OP_FLAG;
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

fn by_position(tokens: &[Token]) -> std::collections::HashMap<(usize, usize), usize> {
    tokens.iter().enumerate().map(|(k, t)| ((t.line, t.column), k)).collect()
}

/// Marks the findings a fix takes as automatic.
pub(crate) fn mark(src: &str, tokens: &[Token], findings: &mut [Finding]) {
    if !findings.iter().any(|f| f.fix == FixKind::Manual && handles(&f.rule_id)) {
        return;
    }
    let at_position = by_position(tokens);
    let file = Source::new(src, tokens, findings, &at_position);
    let kinds: Vec<Option<FixKind>> = findings
        .iter()
        .map(|f| {
            (f.fix == FixKind::Manual && handles(&f.rule_id))
                .then(|| at_position.get(&(f.line, f.column)).and_then(|&k| at(&file, k, &f.rule_id)))
                .flatten()
                .map(|fix| crate::rector::describe_rule(fix.class).fix)
        })
        .collect();
    for (f, kind) in findings.iter_mut().zip(kinds) {
        if let Some(kind) = kind {
            f.fix = kind;
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
    let text = if classic_mac { src.replace('\r', "\n") } else { src.to_string() };
    let tokens = crate::lexer::tokenize(&text);
    let at_position = by_position(&tokens);
    let file = Source::new(&text, &tokens, &findings, &at_position);
    let mut fixes: Vec<Fix> = findings
        .iter()
        .filter(|f| handles(&f.rule_id))
        .filter_map(|f| at_position.get(&(f.line, f.column)).and_then(|&k| at(&file, k, &f.rule_id)))
        .collect();
    fixes.sort_by_key(|f| std::cmp::Reverse(f.start));
    fixes.dedup_by(|a, b| a.start == b.start);
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
        assert_eq!(
            fixed(curl).unwrap().0,
            "<?php\n$ok = curl_setopt($c, CURLOPT_BINARYTRANSFER, true);\ncurl_setopt_array($c, [CURLOPT_BINARYTRANSFER => true]);\n"
        );
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
