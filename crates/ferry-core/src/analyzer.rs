use crate::i18n::{Localized, Text};
use crate::lexer::{lex, tokenize, Token, TokenKind};
use crate::rules::{patterns, FixKind, MemberKind, PatternRule, Severity, SymbolRule, CONSTANTS, FUNCTIONS, MEMBERS};
use crate::version::PhpVersion;
use serde::Serialize;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub rule_id: String,
    pub title: Localized,
    pub message: Localized,
    pub since: PhpVersion,
    pub severity: Severity,
    pub fix: FixKind,
    pub replacement: Option<Localized>,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub target: PhpVersion,
    pub files_scanned: usize,
    pub lines_scanned: usize,
    pub findings: Vec<Finding>,
    pub errors: Vec<FileError>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileError {
    pub file: String,
    pub message: String,
}

pub fn relative_path(root: &Path, path: &Path) -> String {
    let lexical =
        path.strip_prefix(root).ok().filter(|rel| rel.components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir)));
    let rel = match lexical {
        Some(rel) => rel.to_path_buf(),
        None => match (root.canonicalize(), path.canonicalize()) {
            (Ok(root), Ok(full)) => full.strip_prefix(&root).map(Path::to_path_buf).unwrap_or_else(|_| path.to_path_buf()),
            _ => path.to_path_buf(),
        },
    };
    rel.to_string_lossy().replace('\\', "/")
}

pub fn analyze_files(root: &Path, files: &[PathBuf], target: PhpVersion, mut progress: impl FnMut(usize, usize, &str)) -> ScanResult {
    let started = Instant::now();
    let index = crate::index::ProjectIndex::build(root, files);
    let mut findings = Vec::new();
    let mut errors = Vec::new();
    let mut lines = 0;
    for (i, path) in files.iter().enumerate() {
        let rel = relative_path(root, path);
        progress(i, files.len(), &rel);
        if path.is_dir() {
            let message = match std::fs::read_dir(path) {
                Err(e) => e.to_string(),
                Ok(_) => crate::t!("Klasör okunamadı.", "Couldn't read the folder."),
            };
            errors.push(FileError { file: rel, message });
            continue;
        }
        if std::fs::metadata(path).is_ok_and(|m| m.len() > crate::project::MAX_FILE_BYTES) {
            errors.push(FileError {
                file: rel,
                message: crate::t!("Atlandı: dosya çok büyük (2 MB üstü).", "Skipped: file is too large (over 2 MB)."),
            });
            continue;
        }
        match std::fs::read(path) {
            Ok(bytes) => {
                let src = String::from_utf8_lossy(&bytes);
                lines += src.lines().count();
                findings.extend(analyze_source_in(&rel, &src, target, &index));
            }
            Err(e) => errors.push(FileError { file: rel, message: e.to_string() }),
        }
    }
    progress(files.len(), files.len(), "");
    if let Some(framework) = index.framework {
        findings.extend(crate::framework::check(root, framework, target));
    }
    findings.sort_by(|a, b| (a.severity, &a.file, a.line, a.column).cmp(&(b.severity, &b.file, b.line, b.column)));
    ScanResult {
        target,
        files_scanned: files.len(),
        lines_scanned: lines,
        findings,
        errors,
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

fn offset_base(tokens: &[Token], i: usize) -> bool {
    let tok = &tokens[i];
    match tok.kind {
        TokenKind::Variable => {
            let declaration =
                i > 0 && tokens[i - 1].kind == TokenKind::Ident && !EXPR_KEYWORDS.iter().any(|k| tokens[i - 1].is_ident_ci(k));
            !declaration
        }
        TokenKind::Ident => i > 0 && (tokens[i - 1].is_punct("->") || tokens[i - 1].is_punct("?->")),
        TokenKind::Punct if tok.text == "]" => {
            let mut depth = 0;
            let open = (0..=i).rev().find(|&k| {
                if tokens[k].is_punct("]") {
                    depth += 1;
                } else if tokens[k].is_punct("[") {
                    depth -= 1;
                }
                depth == 0
            });
            open.is_some_and(|k| k > 0 && offset_base(tokens, k - 1))
        }
        _ => false,
    }
}

fn curly_offsets(tokens: &[Token]) -> Vec<(usize, Vec<(usize, usize)>)> {
    let mut out = Vec::new();
    for i in 0..tokens.len() {
        if !tokens.get(i + 1).is_some_and(|t| t.is_punct("{")) || !offset_base(tokens, i) {
            continue;
        }
        let mut pairs = Vec::new();
        let mut j = i + 1;
        while tokens.get(j).is_some_and(|t| t.is_punct("{")) {
            let mut depth = 0;
            let Some(close) = (j..tokens.len()).find(|&k| {
                if tokens[k].is_punct("{") {
                    depth += 1;
                } else if tokens[k].is_punct("}") {
                    depth -= 1;
                }
                depth == 0
            }) else {
                break;
            };
            pairs.push((j, close));
            j = close + 1;
        }
        if !pairs.is_empty() {
            out.push((i, pairs));
        }
    }
    out
}

fn class_references(tokens: &[Token]) -> std::collections::HashSet<String> {
    let before = ["new", "instanceof", "extends", "implements", "use", "catch"];
    tokens
        .iter()
        .enumerate()
        .filter(|&(i, t)| {
            t.kind == TokenKind::Ident
                && (i > 0
                    && (before.iter().any(|b| tokens[i - 1].is_ident_ci(b))
                        || tokens[i - 1].is_punct("(") && tokens.get(i + 1).is_some_and(|n| n.kind == TokenKind::Variable))
                    || tokens.get(i + 1).is_some_and(|n| n.is_punct("::") || n.kind == TokenKind::Variable))
        })
        .map(|(_, t)| t.text.rsplit('\\').next().unwrap_or_default().to_ascii_lowercase())
        .collect()
}

fn local_functions(tokens: &[Token], methods: &std::collections::HashSet<usize>) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();
    for i in 0..tokens.len() {
        if tokens[i].is_ident_ci("function") && !methods.contains(&i) {
            let name = tokens[i + 1..].iter().take(2).find(|t| !t.is_punct("&"));
            if let Some(t) = name.filter(|t| t.kind == TokenKind::Ident) {
                names.insert(t.text.to_ascii_lowercase());
            }
        }
        if tokens[i].is_ident_ci("use") && tokens.get(i + 1).is_some_and(|t| t.is_ident_ci("function")) {
            let mut j = i + 2;
            while let Some(t) = tokens.get(j).filter(|t| !t.is_punct(";")) {
                let alias = tokens.get(j + 1).is_some_and(|n| n.is_ident_ci("as"));
                if t.kind == TokenKind::Ident && !t.is_ident_ci("as") && !alias {
                    names.insert(t.text.rsplit('\\').next().unwrap_or_default().to_ascii_lowercase());
                }
                j += 1;
            }
        }
    }
    names
}

pub const CURLY_FIX: &str = "Rector\\Php74\\Rector\\ArrayDimFetch\\CurlyToSquareBracketArrayStringRector";
pub const BAREWORD_FIX: &str = "Ferry\\QuoteBarewordKeys";
pub const NEW_BY_REF_FIX: &str = "Ferry\\Php70\\AssignNewWithoutReference";
pub const DEFAULT_COLON_FIX: &str = "Ferry\\Php85\\DefaultColonAfterSwitch";
pub const BUILTIN_BY_REF_FIX: &str = "Ferry\\Php70\\AssignBuiltinResultWithoutReference";

pub fn fix_legacy_syntax(src: &str) -> Option<(String, Vec<&'static str>)> {
    let tokens = tokenize(src);
    let mut edits: Vec<(usize, usize, &str)> = Vec::new();
    let mut applied = Vec::new();
    let defined = defined_constants(&tokens);
    let mut keys = bareword_keys(&tokens);
    for (_, pairs) in curly_offsets(&tokens) {
        for (open, close) in pairs {
            edits.push((tokens[open].start, tokens[open].end, "["));
            edits.push((tokens[close].start, tokens[close].end, "]"));
            if close == open + 2 && is_bareword(&tokens[open + 1], &defined) {
                keys.push(open + 1);
            }
        }
    }
    if !edits.is_empty() {
        applied.push(CURLY_FIX);
    }
    for &k in &keys {
        edits.push((tokens[k].start, tokens[k].start, "'"));
        edits.push((tokens[k].end, tokens[k].end, "'"));
    }
    if !keys.is_empty() {
        applied.push(BAREWORD_FIX);
    }
    let by_ref: Vec<usize> = (1..tokens.len().saturating_sub(1))
        .filter(|&k| tokens[k].is_punct("&") && tokens[k - 1].is_punct("=") && tokens[k + 1].is_ident_ci("new"))
        .collect();
    for &k in &by_ref {
        edits.push((tokens[k].start, tokens[k].end, ""));
    }
    if !by_ref.is_empty() {
        applied.push(NEW_BY_REF_FIX);
    }
    let builtin_refs = builtin_results_by_ref(&tokens);
    for &(k, _) in &builtin_refs {
        let end = if tokens[k].start > tokens[k - 1].end { tokens[k + 1].start } else { tokens[k].end };
        edits.push((tokens[k].start, end, ""));
    }
    if !builtin_refs.is_empty() {
        applied.push(BUILTIN_BY_REF_FIX);
    }
    let defaults: Vec<usize> = case_semicolons(&tokens).into_iter().filter(|&k| tokens[k].is_ident_ci("default")).collect();
    for &k in &defaults {
        edits.push((tokens[k + 1].start, tokens[k + 1].end, ":"));
    }
    if !defaults.is_empty() {
        applied.push(DEFAULT_COLON_FIX);
    }
    if edits.is_empty() {
        return None;
    }
    edits.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    let mut out = src.to_string();
    for (start, end, text) in edits {
        out.replace_range(start..end, text);
    }
    Some((out, applied))
}

pub fn fix_curly_offsets(src: &str) -> Option<String> {
    let tokens = tokenize(src);
    let braces: Vec<(usize, u8)> = curly_offsets(&tokens)
        .into_iter()
        .flat_map(|(_, pairs)| pairs)
        .flat_map(|(open, close)| [(tokens[open].start, b'['), (tokens[close].start, b']')])
        .collect();
    if braces.is_empty() {
        return None;
    }
    let mut bytes = src.as_bytes().to_vec();
    for (pos, b) in braces {
        bytes[pos] = b;
    }
    String::from_utf8(bytes).ok()
}

pub fn analyze_source(file: &str, src: &str, target: PhpVersion) -> Vec<Finding> {
    let index = crate::index::ProjectIndex::from_tokens(&lex(src).tokens);
    analyze_source_in(file, src, target, &index)
}

pub fn analyze_source_in(file: &str, src: &str, target: PhpVersion, index: &crate::index::ProjectIndex) -> Vec<Finding> {
    let lexed = lex(src);
    let tokens = lexed.tokens;
    let mut line_starts = vec![0];
    line_starts.extend(src.match_indices('\n').map(|(i, _)| i + 1));
    let methods = method_tokens(&tokens);
    let mut ctx = Ctx {
        file,
        src,
        line_starts,
        target,
        local_functions: local_functions(&tokens, &methods),
        methods,
        namespaced: tokens.windows(2).any(|w| w[0].is_ident_ci("namespace") && (w[1].kind == TokenKind::Ident || w[1].is_punct("{"))),
        classes: class_references(&tokens),
        dead: [crate::guards::dead_spans(&tokens, target), crate::guards::dead_expressions(&tokens, target)].concat(),
        index,
        namespaces: crate::index::namespaces(&tokens),
        out: Vec::new(),
    };
    for &(line, column) in &lexed.stray_hash_brackets {
        ctx.pattern_at(&patterns::HASH_BRACKET_COMMENT, line, column);
    }
    for (base, _) in curly_offsets(&tokens) {
        ctx.pattern(&patterns::CURLY_OFFSET, &tokens[base], None);
    }
    for k in nested_ternaries(&tokens) {
        ctx.pattern(&patterns::NESTED_TERNARY, &tokens[k], None);
    }
    for k in php4_constructors(&tokens) {
        ctx.pattern(&patterns::PHP4_CONSTRUCTOR, &tokens[k], None);
    }
    for k in bareword_keys(&tokens) {
        ctx.pattern(&patterns::BAREWORD_KEY, &tokens[k], None);
    }
    for over in builtin_overrides(&tokens, index) {
        match over.problem {
            Override::MissingTentativeType => ctx.pattern(&patterns::RETURN_TYPE_WILL_CHANGE, &tokens[over.name], None),
            Override::Incompatible(method) => {
                let &(_, is_static, required, params, variadic, returns, _) = method;
                let params: Vec<(bool, String, String)> = params
                    .iter()
                    .enumerate()
                    .map(|(i, &(r, t))| (r, if t.is_empty() { "mixed".to_string() } else { t.to_string() }, format!("p{}", i + 1)))
                    .collect();
                let variadic = variadic.then(|| format!("p{}", params.len() + 1));
                let signature = render_signature(&tokens[over.name].text, is_static, required, &params, variadic, returns);
                let parent = tokens[over.parent].text.trim_start_matches('\\');
                ctx.incompatible_override(&patterns::INCOMPATIBLE_OVERRIDE, &tokens[over.name], parent, signature);
            }
            Override::UserIncompatible(parent, sig) => {
                let fixed: Vec<&ParamShape> = sig.params.iter().filter(|p| !p.variadic).collect();
                let params: Vec<(bool, String, String)> = fixed.iter().map(|p| (p.by_ref, display_type(&p.ty), p.name.clone())).collect();
                let required = fixed.iter().rposition(|p| !p.optional).map_or(0, |i| i + 1);
                let returns = sig.returns.as_deref().map(display_type).unwrap_or_default();
                let variadic = sig.params.iter().find(|p| p.variadic).map(|p| p.name.clone());
                let signature = render_signature(&tokens[over.name].text, sig.is_static, required, &params, variadic, &returns);
                ctx.incompatible_override(&patterns::INCOMPATIBLE_USER_OVERRIDE, &tokens[over.name], &parent, signature);
            }
        }
    }
    for k in null_offsets(&tokens) {
        ctx.pattern(&patterns::NULL_OFFSET, &tokens[k], None);
    }
    for k in case_semicolons(&tokens) {
        ctx.pattern(&patterns::CASE_SEMICOLON, &tokens[k], None);
    }
    for k in 1..tokens.len().saturating_sub(1) {
        if tokens[k].is_punct("&") && tokens[k - 1].is_punct("=") && tokens[k + 1].is_ident_ci("new") {
            ctx.pattern(&patterns::ASSIGN_NEW_BY_REF, &tokens[k], None);
        }
    }
    for k in unset_this(&tokens) {
        ctx.pattern(&patterns::UNSET_THIS, &tokens[k], None);
    }
    for (k, since) in builtin_results_by_ref(&tokens) {
        ctx.pattern_since(&patterns::BUILTIN_RESULT_BY_REF, tokens[k].line, tokens[k].column, since);
    }
    for k in stray_breaks(&tokens) {
        ctx.pattern(&patterns::BREAK_OUTSIDE_LOOP, &tokens[k], None);
    }
    for k in concat_with_arithmetic(&tokens) {
        ctx.pattern(&patterns::CONCAT_PRECEDENCE, &tokens[k], None);
    }
    for k in serializable_without_magic(&tokens) {
        ctx.pattern(&patterns::SERIALIZABLE, &tokens[k], None);
    }
    for k in dynamic_properties(&tokens, index) {
        ctx.pattern(&patterns::DYNAMIC_PROPERTY, &tokens[k], None);
    }
    let mut i = 0;
    while i < tokens.len() {
        let tok = &tokens[i];
        i += 1;
        if tok.is_punct("#[") {
            let mut depth = 1;
            while i < tokens.len() && depth > 0 {
                if tokens[i].is_punct("[") || tokens[i].is_punct("#[") {
                    depth += 1;
                } else if tokens[i].is_punct("]") {
                    depth -= 1;
                }
                i += 1;
            }
            continue;
        }
        let i = i - 1;
        match &tok.kind {
            TokenKind::Ident => ctx.ident(&tokens, i),
            TokenKind::Punct if tok.text == "$" => {
                if let Some(next) = tokens.get(i + 1) {
                    if next.start == tok.end && (next.kind == TokenKind::Variable || next.is_punct("{")) {
                        ctx.pattern(&patterns::VARIABLE_VARIABLE, tok, None);
                    }
                }
            }
            TokenKind::Str { backtick: true, interpolates } => {
                ctx.pattern(&patterns::BACKTICK, tok, None);
                if *interpolates {
                    ctx.dollar_brace(tok);
                }
            }
            TokenKind::Variable if tok.text == "http_response_header" => {
                let local = tokens.iter().enumerate().any(|(k, t)| {
                    t.kind == TokenKind::Variable
                        && t.text == "http_response_header"
                        && (tokens.get(k + 1).is_some_and(|n| n.is_punct("=") || n.is_punct(",") || n.is_punct("]"))
                            || inside_call(&tokens, k, "list"))
                });
                if !local {
                    ctx.pattern(&patterns::HTTP_RESPONSE_HEADER, tok, None);
                }
            }
            TokenKind::Variable if tok.text == "HTTP_RAW_POST_DATA" || tok.text == "php_errormsg" => {
                let since = if tok.text == "php_errormsg" { PhpVersion::new(8, 0) } else { PhpVersion::new(7, 0) };
                ctx.pattern_since(&patterns::REMOVED_VARIABLE, tok.line, tok.column, since);
            }
            TokenKind::Str { interpolates, .. } => {
                if *interpolates {
                    ctx.dollar_brace(tok);
                }
                ctx.mb_encoding(&tokens, i);
            }
            _ => {}
        }
    }
    ctx.out
}

struct Hit<'r> {
    rule_id: &'r str,
    title: Localized,
    message: Text,
    since: PhpVersion,
    severity: Severity,
    fix: FixKind,
    replacement: Option<Localized>,
}

struct Ctx<'a> {
    file: &'a str,
    src: &'a str,
    line_starts: Vec<usize>,
    target: PhpVersion,
    local_functions: std::collections::HashSet<String>,
    methods: std::collections::HashSet<usize>,
    classes: std::collections::HashSet<String>,
    namespaced: bool,
    dead: Vec<crate::guards::Span>,
    index: &'a crate::index::ProjectIndex,
    namespaces: Vec<(usize, String)>,
    out: Vec<Finding>,
}

const EXPR_KEYWORDS: [&str; 17] = [
    "return",
    "echo",
    "print",
    "yield",
    "from",
    "case",
    "else",
    "and",
    "or",
    "xor",
    "throw",
    "include",
    "include_once",
    "require",
    "require_once",
    "clone",
    "default",
];

const DECL_KEYWORDS: [&str; 13] =
    ["function", "fn", "new", "const", "class", "interface", "trait", "enum", "use", "goto", "instanceof", "extends", "implements"];

impl<'a> Ctx<'a> {
    fn applies(&self, since: PhpVersion) -> bool {
        since <= self.target
    }

    fn snippet(&self, line: usize) -> String {
        let Some(&start) = self.line_starts.get(line - 1) else {
            return String::new();
        };
        let end = self.line_starts.get(line).map_or(self.src.len(), |&e| e - 1);
        let text = self.src[start..end].trim_start_matches('\u{FEFF}').trim();
        match text.char_indices().nth(200) {
            Some((cut, _)) => format!("{}…", &text[..cut]),
            None => text.to_string(),
        }
    }

    fn push(&mut self, hit: Hit<'_>, line: usize, column: usize) {
        let compiled = crate::rules::COMPILE_TIME.contains(&hit.rule_id);
        if !compiled && self.dead.iter().any(|&(start, end)| (line, column) >= start && (line, column) <= end) {
            return;
        }
        self.out.push(Finding {
            id: format!("{}:{}:{}:{}", self.file, line, column, hit.rule_id),
            file: self.file.to_string(),
            line,
            column,
            rule_id: hit.rule_id.to_string(),
            title: hit.title,
            message: hit.message.into(),
            since: hit.since,
            severity: hit.severity,
            fix: hit.fix,
            replacement: hit.replacement,
            snippet: self.snippet(line),
        });
    }

    fn symbol(&mut self, rule: &SymbolRule, tok: &Token, is_call: bool) {
        if !self.applies(rule.since()) {
            return;
        }
        let name = tok.text.trim_start_matches('\\');
        let title = if is_call { format!("{name}()") } else { name.to_string() };
        let hit = Hit {
            rule_id: rule.id,
            title: Localized::same(title),
            message: rule.note,
            since: rule.since(),
            severity: rule.severity(),
            fix: rule.fix,
            replacement: rule.replacement.map(Into::into),
        };
        self.push(hit, tok.line, tok.column);
    }

    fn pattern(&mut self, rule: &PatternRule, tok: &Token, at: Option<(usize, usize)>) {
        let (line, column) = at.unwrap_or((tok.line, tok.column));
        self.pattern_at(rule, line, column);
    }

    fn pattern_at(&mut self, rule: &PatternRule, line: usize, column: usize) {
        self.pattern_since(rule, line, column, rule.since());
    }

    fn incompatible_override(&mut self, rule: &PatternRule, name: &Token, parent: &str, signature: String) {
        if !self.applies(rule.since()) {
            return;
        }
        let hit = Hit {
            rule_id: rule.id,
            title: Localized::same(format!("{parent}::{}()", name.text)),
            message: rule.note,
            since: rule.since(),
            severity: rule.severity,
            fix: rule.fix,
            replacement: Some(Localized::same(signature)),
        };
        self.push(hit, name.line, name.column);
    }

    fn pattern_since(&mut self, rule: &PatternRule, line: usize, column: usize, since: PhpVersion) {
        if !self.applies(since) {
            return;
        }
        let hit = Hit {
            rule_id: rule.id,
            title: rule.title.into(),
            message: rule.note,
            since,
            severity: rule.severity,
            fix: rule.fix,
            replacement: rule.replacement.map(Into::into),
        };
        self.push(hit, line, column);
    }

    fn ident(&mut self, tokens: &[Token], i: usize) {
        let tok = &tokens[i];
        let bare = tok.text.trim_start_matches('\\');
        if bare.contains('\\') {
            return;
        }
        let lower = bare.to_ascii_lowercase();
        let prev = i.checked_sub(1).map(|p| &tokens[p]);
        let next = tokens.get(i + 1);
        let by_ref_fn =
            prev.is_some_and(|p| p.is_punct("&")) && i >= 2 && (tokens[i - 2].is_ident_ci("function") || tokens[i - 2].is_ident_ci("fn"));
        let decl_idx = if by_ref_fn { i - 2 } else { i.saturating_sub(1) };
        let prev = if by_ref_fn { Some(&tokens[i - 2]) } else { prev };

        let is_member = !by_ref_fn && prev.is_some_and(|p| p.is_punct("->") || p.is_punct("?->") || p.is_punct("::"));
        let after_decl = prev.is_some_and(|p| p.kind == TokenKind::Ident && DECL_KEYWORDS.iter().any(|k| p.text.eq_ignore_ascii_case(k)));

        if lower == "function" || lower == "fn" {
            self.function_params(tokens, i);
        }

        if prev.is_some_and(|p| p.is_punct("(")) && next.is_some_and(|n| n.is_punct(")")) {
            let before_paren = i.checked_sub(2).map(|p| &tokens[p]);
            let paren_is_call = before_paren.is_some_and(|b| {
                b.kind == TokenKind::Variable
                    || (b.kind == TokenKind::Ident && !EXPR_KEYWORDS.iter().any(|k| b.is_ident_ci(k)))
                    || b.is_punct(")")
                    || b.is_punct("]")
            });
            let open = &tokens[i - 1];
            let tight = |a: usize, b: usize| self.src.get(a..b).is_some_and(|gap| gap.bytes().all(|c| c == b' ' || c == b'\t'));
            if !paren_is_call && tight(open.end, tok.start) && tight(tok.end, next.map_or(tok.end, |n| n.start)) {
                match lower.as_str() {
                    "real" => return self.pattern(&patterns::REAL_CAST, open, None),
                    "unset" => return self.pattern(&patterns::UNSET_CAST, open, None),
                    "boolean" | "integer" | "double" | "binary" => return self.pattern(&patterns::NON_CANONICAL_CAST, open, None),
                    _ => {}
                }
            }
        }

        if is_member {
            self.member(tokens, i);
            return;
        }

        if after_decl {
            let is_function_decl = prev.is_some_and(|p| p.is_ident_ci("function"));
            let is_type_decl = prev.is_some_and(|p| ["class", "interface", "trait", "enum"].iter().any(|k| p.is_ident_ci(k)));
            let reserved = if is_type_decl {
                RESERVED_CLASS_NAMES.iter().find(|(n, _, _)| *n == lower).map(|&(_, a, b)| PhpVersion::new(a, b))
            } else if is_function_decl && !self.methods.contains(&decl_idx) {
                match lower.as_str() {
                    "match" => Some(PhpVersion::new(8, 0)),
                    "fn" => Some(PhpVersion::new(7, 4)),
                    _ => None,
                }
            } else {
                None
            };
            if let Some(since) = reserved {
                self.pattern_since(&patterns::RESERVED_NAME, tok.line, tok.column, since);
            }
            if is_function_decl && lower == "__autoload" && !self.namespaced && !self.methods.contains(&decl_idx) {
                self.pattern(&patterns::AUTOLOAD, tok, None);
            }
            return;
        }

        let is_call = next.is_some_and(|n| n.is_punct("("));
        if is_call {
            match lower.as_str() {
                "eval" => return self.pattern(&patterns::EVAL, tok, None),
                "extract" => return self.pattern(&patterns::EXTRACT, tok, None),
                "get_class" | "get_parent_class" => {
                    if tokens.get(i + 2).is_some_and(|t| t.is_punct(")")) {
                        self.pattern(&patterns::GET_CLASS_NO_ARGS, tok, None);
                    }
                    return;
                }
                _ => {}
            }
            let rooted = tok.text.starts_with('\\');
            if !rooted && self.local_functions.contains(&lower) {
                return;
            }
            if self.index.declares_function(crate::index::namespace_at(&self.namespaces, i), &lower, rooted) {
                return;
            }
            if self.index.framework == Some(crate::project::Framework::Laravel) && crate::rules::LARAVEL_HELPERS.contains(&lower.as_str()) {
                return self.symbol(&crate::rules::LARAVEL_HELPER, tok, true);
            }
            let args = call_args(tokens, i + 1);
            let single = |n: usize| args.get(n).filter(|a| a.len() == 1).map(|a| &a[0]);
            let named = args.iter().any(|a| a.len() > 1 && a[1].is_punct(":") && a[0].kind == TokenKind::Ident);
            let spread = args.iter().any(|a| a.first().is_some_and(|t| t.is_punct("...")));
            let callable_slot = CALLABLE_ARGS.iter().chain(&[("is_callable", 0)]).find(|(f, _)| *f == lower).map(|&(_, n)| n);
            if let Some(slot) = callable_slot {
                if let Some(arg) = single(slot).filter(|_| lower != "is_callable") {
                    self.callable_string(arg, i);
                }
                if let Some(arg) = args.get(slot).filter(|_| !named && !spread) {
                    if let Some(at) = partial_callable(arg) {
                        self.pattern(&patterns::PARTIAL_CALLABLE, at, None);
                    }
                }
            }
            if !named && !spread {
                if let Some((_, slots)) = crate::rules::NULL_STRING_ARGS.iter().find(|(f, _)| *f == lower) {
                    for &slot in *slots {
                        if let Some(arg) = single(slot).filter(|t| t.is_ident_ci("null")) {
                            self.pattern(&patterns::NULL_TO_INTERNAL, arg, None);
                        }
                    }
                }
            }
            match lower.as_str() {
                "define" if single(2).is_some_and(|t| t.is_ident_ci("true")) => {
                    return self.pattern(&patterns::CASE_INSENSITIVE_CONSTANT, tok, None)
                }
                "parse_str" if args.len() == 1 && !spread => return self.pattern(&patterns::PARSE_STR_ONE_ARG, tok, None),
                "assert" if single(0).is_some_and(|t| matches!(t.kind, TokenKind::Str { .. })) => {
                    return self.pattern(&patterns::ASSERT_STRING, tok, None)
                }
                "implode" | "join"
                    if args.len() == 2
                        && single(1).is_some_and(|t| matches!(t.kind, TokenKind::Str { .. }))
                        && !single(0).is_some_and(|t| matches!(t.kind, TokenKind::Str { .. })) =>
                {
                    return self.pattern(&patterns::IMPLODE_ORDER, tok, None)
                }
                "mb_check_encoding" if args.is_empty() => {
                    return self.pattern_since(&patterns::EMPTY_CALL, tok.line, tok.column, PhpVersion::new(8, 1))
                }
                "readdir" | "closedir" | "rewinddir" if args.is_empty() => {
                    return self.pattern_since(&patterns::EMPTY_CALL, tok.line, tok.column, PhpVersion::new(8, 5))
                }
                "session_set_save_handler" if args.len() > 2 => return self.pattern(&patterns::SESSION_HANDLER_ARGS, tok, None),
                "array_key_exists" if single(0).is_some_and(|t| t.is_ident_ci("null")) => {
                    return self.pattern(&patterns::NULL_OFFSET, tok, None)
                }
                "ini_set" | "ini_alter" => {
                    if let Some(key) = single(0).filter(|t| matches!(t.kind, TokenKind::Str { .. })) {
                        let name = key.text.get(1..key.text.len().saturating_sub(1)).unwrap_or_default().to_ascii_lowercase();
                        if let Some(&(_, a, b)) = crate::rules::DEPRECATED_INI.iter().find(|(k, _, _)| *k == name) {
                            self.pattern_since(&patterns::INI_SETTING, key.line, key.column, PhpVersion::new(a, b));
                        }
                    }
                    return;
                }
                "fputcsv" | "fgetcsv" | "str_getcsv" => {
                    let needed = if lower == "str_getcsv" { 4 } else { 5 };
                    let has_escape = args
                        .iter()
                        .any(|a| a.first().is_some_and(|t| t.is_ident_ci("escape")) && a.get(1).is_some_and(|t| t.is_punct(":")));
                    if !has_escape && !spread && (named || args.len() < needed) {
                        self.pattern(&patterns::CSV_ESCAPE, tok, None);
                    }
                    return;
                }
                _ => {}
            }
            if let Some(rule) = FUNCTIONS.iter().find(|r| r.matches(&lower)) {
                self.symbol(rule, tok, true);
            }
            return;
        }

        if bare.bytes().any(|b| b.is_ascii_lowercase()) {
            return;
        }
        if let Some(rule) = CONSTANTS.iter().find(|r| r.matches(&lower)) {
            if rule.id == "deprecated.e_user_error" && !inside_call(tokens, i, "trigger_error") && !inside_call(tokens, i, "user_error") {
                return;
            }
            self.symbol(rule, tok, false);
        }
    }

    fn member(&mut self, tokens: &[Token], i: usize) {
        let tok = &tokens[i];
        let lower = tok.text.to_ascii_lowercase();
        let is_call = tokens.get(i + 1).is_some_and(|n| n.is_punct("("));
        let (kind, class) = if tokens[i - 1].is_punct("::") {
            let Some(class) = i.checked_sub(2).map(|k| &tokens[k]).filter(|t| t.kind == TokenKind::Ident) else { return };
            let name = class.text.rsplit('\\').next().unwrap_or_default();
            if is_call && name.eq_ignore_ascii_case("Input") && self.index.framework == Some(crate::project::Framework::Laravel) {
                if self.applies(crate::rules::LARAVEL_INPUT.since()) {
                    let hit = Hit {
                        rule_id: crate::rules::LARAVEL_INPUT.id,
                        title: Localized::same(format!("Input::{}()", tok.text)),
                        message: crate::rules::LARAVEL_INPUT.note,
                        since: crate::rules::LARAVEL_INPUT.since(),
                        severity: crate::rules::LARAVEL_INPUT.severity(),
                        fix: FixKind::Manual,
                        replacement: crate::rules::LARAVEL_INPUT.replacement.map(Into::into),
                    };
                    self.push(hit, tok.line, tok.column);
                }
                return;
            }
            if is_call && name.eq_ignore_ascii_case("Closure") && lower == "fromcallable" {
                if let Some(arg) = call_args(tokens, i + 1).first().filter(|a| a.len() == 1) {
                    self.callable_string(&arg[0], i);
                }
                return;
            }
            (if is_call { MemberKind::StaticCall } else { MemberKind::Constant }, Some(name))
        } else if is_call {
            (MemberKind::Method, None)
        } else {
            return;
        };
        let Some(m) = MEMBERS.iter().find(|m| {
            m.kind == kind
                && m.rule.matches(&lower)
                && match class {
                    Some(c) => c.to_ascii_lowercase().starts_with(m.class),
                    None => self.classes.iter().any(|c| c.starts_with(m.class)),
                }
        }) else {
            return;
        };
        if !self.applies(m.rule.since()) {
            return;
        }
        let title = match (kind, class) {
            (MemberKind::Method, _) => format!("->{}()", tok.text),
            (MemberKind::StaticCall, Some(c)) => format!("{c}::{}()", tok.text),
            (_, c) => format!("{}::{}", c.unwrap_or_default(), tok.text),
        };
        let hit = Hit {
            rule_id: m.rule.id,
            title: Localized::same(title),
            message: m.rule.note,
            since: m.rule.since(),
            severity: m.rule.severity(),
            fix: m.rule.fix,
            replacement: m.rule.replacement.map(Into::into),
        };
        self.push(hit, tok.line, tok.column);
    }

    fn callable_string(&mut self, arg: &Token, call: usize) {
        let TokenKind::Str { interpolates, .. } = arg.kind else { return };
        let inner = arg.text.get(1..arg.text.len().saturating_sub(1)).unwrap_or_default();
        if interpolates && inner.contains('$') {
            return;
        }
        let name = inner.trim_start_matches('\\').to_ascii_lowercase();
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return;
        }
        if self.local_functions.contains(&name)
            || self.index.declares_function(crate::index::namespace_at(&self.namespaces, call), &name, true)
        {
            return;
        }
        let Some(rule) = FUNCTIONS.iter().find(|r| r.matches(&name)) else { return };
        if !self.applies(rule.since()) {
            return;
        }
        let hit = Hit {
            rule_id: rule.id,
            title: Localized::same(format!("'{}'", inner.trim_start_matches('\\'))),
            message: rule.note,
            since: rule.since(),
            severity: rule.severity(),
            fix: FixKind::Manual,
            replacement: rule.replacement.map(Into::into),
        };
        self.push(hit, arg.line, arg.column);
    }

    fn mb_encoding(&mut self, tokens: &[Token], i: usize) {
        let text = &tokens[i].text;
        let inner = text.get(1..text.len().saturating_sub(1)).unwrap_or_default();
        let deprecated = ["html-entities", "html", "base64", "uuencode", "qprint", "quoted-printable"];
        if deprecated.iter().any(|e| inner.eq_ignore_ascii_case(e))
            && argument_position(tokens, i)
                .is_some_and(|(f, n)| MB_ENCODING_ARGS.iter().any(|(name, slots)| f.eq_ignore_ascii_case(name) && slots.contains(&n)))
        {
            self.pattern(&patterns::MB_ENCODING, &tokens[i], None);
        }
    }

    fn function_params(&mut self, tokens: &[Token], fn_idx: usize) {
        let open = (fn_idx + 1..(fn_idx + 4).min(tokens.len())).find(|&j| tokens[j].is_punct("("));
        let Some(open) = open else { return };
        let mut depth = 0usize;
        let mut params: Vec<Vec<&Token>> = vec![Vec::new()];
        let mut j = open;
        while j < tokens.len() {
            let t = &tokens[j];
            if t.is_punct("(") || t.is_punct("[") || t.is_punct("#[") || t.is_punct("{") {
                depth += 1;
                if depth > 1 {
                    params.last_mut().unwrap().push(t);
                }
            } else if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                params.last_mut().unwrap().push(t);
            } else if depth == 1 && t.is_punct(",") {
                params.push(Vec::new());
            } else {
                params.last_mut().unwrap().push(t);
            }
            j += 1;
        }

        let mut optional: Option<&Token> = None;
        let mut reported = false;
        for param in &params {
            let Some(var_pos) = param.iter().position(|t| t.kind == TokenKind::Variable) else {
                continue;
            };
            let has_default = param[var_pos + 1..].first().is_some_and(|t| t.is_punct("="));
            let variadic = param[..var_pos].iter().any(|t| t.is_punct("..."));
            let typed_null = has_default
                && param[var_pos + 1..].len() == 2
                && param[var_pos + 2].is_ident_ci("null")
                && param[..var_pos].iter().any(|t| {
                    t.kind == TokenKind::Ident && !["public", "protected", "private", "readonly"].iter().any(|m| t.is_ident_ci(m))
                });
            if has_default {
                if !typed_null && optional.is_none() {
                    optional = Some(param[var_pos]);
                }
            } else if !variadic {
                if let Some(opt) = optional.filter(|_| !reported) {
                    self.pattern(&patterns::OPTIONAL_BEFORE_REQUIRED, opt, None);
                    reported = true;
                }
            }
        }

        for param in params {
            let Some(var_pos) = param.iter().position(|t| t.kind == TokenKind::Variable) else {
                continue;
            };
            let mut type_tokens = Vec::new();
            let mut attr_depth = 0;
            for t in &param[..var_pos] {
                if t.is_punct("#[") {
                    attr_depth += 1;
                    continue;
                }
                if attr_depth > 0 {
                    if t.is_punct("[") {
                        attr_depth += 1;
                    } else if t.is_punct("]") {
                        attr_depth -= 1;
                    }
                    continue;
                }
                let modifier = t.kind == TokenKind::Ident
                    && ["public", "protected", "private", "readonly"].iter().any(|m| t.text.eq_ignore_ascii_case(m));
                if modifier || t.is_punct("&") || t.is_punct("...") {
                    continue;
                }
                type_tokens.push(*t);
            }
            if type_tokens.is_empty() {
                continue;
            }
            let already_nullable = type_tokens.iter().any(|t| t.is_punct("?") || t.is_ident_ci("null") || t.is_ident_ci("mixed"));
            if already_nullable {
                continue;
            }
            let rest = &param[var_pos + 1..];
            let defaults_to_null = rest.len() == 2 && rest[0].is_punct("=") && rest[1].is_ident_ci("null");
            if defaults_to_null {
                self.pattern(&patterns::IMPLICIT_NULLABLE, type_tokens[0], None);
            }
        }
    }

    fn dollar_brace(&mut self, tok: &Token) {
        let bytes = tok.text.as_bytes();
        let (mut line, mut column) = (tok.line, tok.column);
        let mut counted = 0;
        let mut k = 0;
        while k + 1 < bytes.len() {
            if bytes[k] == b'\\' {
                k += 2;
                continue;
            }
            if bytes[k] == b'$' && bytes[k + 1] == b'{' && (k == 0 || bytes[k - 1] != b'{') {
                for c in tok.text[counted..k].chars() {
                    if c == '\n' {
                        line += 1;
                        column = 1;
                    } else {
                        column += 1;
                    }
                }
                counted = k;
                self.pattern(&patterns::DOLLAR_BRACE_INTERPOLATION, tok, Some((line, column)));
            }
            k += 1;
        }
    }
}

pub(crate) fn method_tokens(tokens: &[Token]) -> std::collections::HashSet<usize> {
    let modifiers = ["public", "protected", "private", "static", "abstract", "final"];
    let mut methods = std::collections::HashSet::new();
    let mut stack: Vec<bool> = Vec::new();
    let mut parens = 0usize;
    let mut pending: Option<usize> = None;
    for (k, t) in tokens.iter().enumerate() {
        if t.is_punct("(") {
            parens += 1;
        } else if t.is_punct(")") {
            parens = parens.saturating_sub(1);
        } else if t.is_punct(";") && pending == Some(parens) {
            pending = None;
        } else if ["class", "trait", "interface", "enum"].iter().any(|c| t.is_ident_ci(c)) && !(k > 0 && tokens[k - 1].is_punct("::")) {
            pending = Some(parens);
        } else if t.is_punct("{") {
            stack.push(pending == Some(parens));
            if pending == Some(parens) {
                pending = None;
            }
        } else if t.is_punct("}") {
            stack.pop();
        } else if t.is_ident_ci("function") {
            let modified = k > 0 && modifiers.iter().any(|m| tokens[k - 1].is_ident_ci(m));
            if modified || stack.last() == Some(&true) {
                methods.insert(k);
            }
        }
    }
    methods
}

fn inside_call(tokens: &[Token], i: usize, func: &str) -> bool {
    enclosing_call(tokens, i).is_some_and(|name| name.eq_ignore_ascii_case(func))
}

fn enclosing_call(tokens: &[Token], i: usize) -> Option<&str> {
    let mut depth: isize = 0;
    for k in (0..i).rev() {
        let t = &tokens[k];
        if t.is_punct(";") || t.is_punct("{") || t.is_punct("}") {
            return None;
        }
        if t.is_punct(")") {
            depth += 1;
        } else if t.is_punct("(") {
            if depth == 0 {
                return (k > 0 && tokens[k - 1].kind == TokenKind::Ident).then(|| tokens[k - 1].text.trim_start_matches('\\'));
            }
            depth -= 1;
        }
    }
    None
}

const CALLABLE_ARGS: [(&str, usize); 23] = [
    ("array_map", 0),
    ("array_filter", 1),
    ("array_walk", 1),
    ("array_walk_recursive", 1),
    ("array_reduce", 1),
    ("usort", 1),
    ("uasort", 1),
    ("uksort", 1),
    ("call_user_func", 0),
    ("call_user_func_array", 0),
    ("forward_static_call", 0),
    ("forward_static_call_array", 0),
    ("register_shutdown_function", 0),
    ("spl_autoload_register", 0),
    ("set_error_handler", 0),
    ("set_exception_handler", 0),
    ("preg_replace_callback", 1),
    ("iterator_apply", 1),
    ("ob_start", 0),
    ("header_register_callback", 0),
    ("register_tick_function", 0),
    ("array_udiff", 2),
    ("array_uintersect", 2),
];

const MB_ENCODING_ARGS: [(&str, &[usize]); 34] = [
    ("mb_convert_encoding", &[1, 2]),
    ("mb_check_encoding", &[1]),
    ("mb_strlen", &[1]),
    ("mb_substr", &[3]),
    ("mb_strcut", &[3]),
    ("mb_strtolower", &[1]),
    ("mb_strtoupper", &[1]),
    ("mb_convert_case", &[2]),
    ("mb_internal_encoding", &[0]),
    ("mb_strpos", &[3]),
    ("mb_stripos", &[3]),
    ("mb_strrpos", &[3]),
    ("mb_strripos", &[3]),
    ("mb_strstr", &[3]),
    ("mb_stristr", &[3]),
    ("mb_strrchr", &[3]),
    ("mb_strrichr", &[3]),
    ("mb_substr_count", &[2]),
    ("mb_strwidth", &[1]),
    ("mb_strimwidth", &[4]),
    ("mb_detect_encoding", &[1]),
    ("mb_encode_numericentity", &[2]),
    ("mb_decode_numericentity", &[2]),
    ("mb_preferred_mime_name", &[0]),
    ("mb_regex_encoding", &[0]),
    ("mb_http_output", &[0]),
    ("mb_str_split", &[2]),
    ("mb_ord", &[1]),
    ("mb_chr", &[1]),
    ("mb_scrub", &[1]),
    ("mb_convert_variables", &[0, 1]),
    ("mb_encode_mimeheader", &[1]),
    ("mb_str_pad", &[4]),
    ("mb_trim", &[2]),
];

fn partial_callable(arg: &[Token]) -> Option<&Token> {
    fn literal(t: &Token) -> Option<&str> {
        match t.kind {
            TokenKind::Str { interpolates, .. } if !(interpolates && t.text.contains('$')) => t.text.get(1..t.text.len().saturating_sub(1)),
            _ => None,
        }
    }
    let relative = |s: &str| ["self", "parent", "static"].iter().any(|k| s.eq_ignore_ascii_case(k));
    let is_name = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
    if let [t] = arg {
        let (class, method) = literal(t)?.split_once("::")?;
        return (relative(class) && is_name(method)).then_some(t);
    }
    let inner = match arg {
        [open, inner @ .., close] if open.is_punct("[") && close.is_punct("]") => inner,
        [kw, open, inner @ .., close] if kw.is_ident_ci("array") && open.is_punct("(") && close.is_punct(")") => inner,
        _ => return None,
    };
    let [class, comma, method] = inner else { return None };
    (comma.is_punct(",") && literal(class).is_some_and(relative) && literal(method).is_some_and(is_name)).then_some(class)
}

fn argument_position(tokens: &[Token], i: usize) -> Option<(&str, usize)> {
    let mut depth: isize = 0;
    let mut commas = 0;
    for k in (0..i).rev() {
        let t = &tokens[k];
        if t.is_punct(";") || t.is_punct("{") || t.is_punct("}") {
            return None;
        }
        if t.is_punct(")") || t.is_punct("]") {
            depth += 1;
        } else if t.is_punct("(") || t.is_punct("[") {
            if depth == 0 {
                return (t.is_punct("(") && k > 0 && tokens[k - 1].kind == TokenKind::Ident)
                    .then(|| (tokens[k - 1].text.trim_start_matches('\\'), commas));
            }
            depth -= 1;
        } else if depth == 0 && t.is_punct(",") {
            commas += 1;
        }
    }
    None
}

fn call_args(tokens: &[Token], open: usize) -> Vec<&[Token]> {
    let mut args = Vec::new();
    let mut depth = 0usize;
    let mut start = open + 1;
    for k in open..tokens.len() {
        let t = &tokens[k];
        if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") || t.is_punct("#[") {
            depth += 1;
        } else if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            depth -= 1;
            if depth == 0 {
                if k > start {
                    args.push(&tokens[start..k]);
                }
                break;
            }
        } else if depth == 1 && t.is_punct(",") {
            args.push(&tokens[start..k]);
            start = k + 1;
        }
    }
    args
}

const RESERVED_CLASS_NAMES: [(&str, u32, u32); 12] = [
    ("int", 7, 0),
    ("float", 7, 0),
    ("bool", 7, 0),
    ("string", 7, 0),
    ("null", 7, 0),
    ("true", 7, 0),
    ("false", 7, 0),
    ("void", 7, 1),
    ("iterable", 7, 1),
    ("object", 7, 2),
    ("mixed", 8, 0),
    ("never", 8, 1),
];

fn expression_end(t: &Token) -> bool {
    let modifiers = ["public", "protected", "private", "static", "readonly", "var", "const", "function", "fn"];
    matches!(t.kind, TokenKind::Variable | TokenKind::Number | TokenKind::Str { .. })
        || (t.kind == TokenKind::Ident && !EXPR_KEYWORDS.iter().any(|k| t.is_ident_ci(k)) && !modifiers.iter().any(|k| t.is_ident_ci(k)))
        || t.is_punct(")")
        || t.is_punct("]")
}

#[derive(Clone, Copy, Default)]
struct TernaryLevel {
    open: usize,
    done: bool,
    done_short: bool,
}

fn nested_ternaries(tokens: &[Token]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut stack = vec![TernaryLevel::default()];
    for (k, t) in tokens.iter().enumerate() {
        if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") || t.is_punct("#[") {
            stack.push(TernaryLevel::default());
            continue;
        }
        if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            if stack.len() > 1 {
                stack.pop();
            }
            continue;
        }
        let level = stack.last_mut().unwrap();
        let keyword = t.kind == TokenKind::Ident
            && ["and", "or", "xor", "return", "echo", "print", "case", "yield", "throw"].iter().any(|k| t.is_ident_ci(k));
        let label = t.is_punct(":") && level.open == 0 && !tokens[k - 1].is_punct("?");
        if t.is_punct(";") || t.is_punct(",") || t.is_punct("=>") || t.kind == TokenKind::CloseTag || keyword || label {
            *level = TernaryLevel::default();
        } else if t.is_punct("?") && k > 0 && expression_end(&tokens[k - 1]) {
            let short = tokens.get(k + 1).is_some_and(|n| n.is_punct(":"));
            if level.open == 0 {
                if level.done && !(short && level.done_short) {
                    out.push(k);
                }
                if short {
                    level.done_short = !level.done || level.done_short;
                    level.done = true;
                }
            }
            if !short {
                level.open += 1;
            }
        } else if t.is_punct(":") && level.open > 0 && !tokens[k - 1].is_punct("?") {
            level.open -= 1;
            if level.open == 0 {
                level.done = true;
                level.done_short = false;
            }
        }
    }
    out
}

fn php4_constructors(tokens: &[Token]) -> Vec<usize> {
    if tokens.iter().any(|t| t.is_ident_ci("namespace")) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 0..tokens.len() {
        if !tokens[i].is_ident_ci("class") || i > 0 && (tokens[i - 1].is_punct("::") || tokens[i - 1].is_ident_ci("new")) {
            continue;
        }
        let Some(name) = tokens.get(i + 1).filter(|t| t.kind == TokenKind::Ident) else { continue };
        let Some(open) = (i + 2..tokens.len()).find(|&k| tokens[k].is_punct("{")) else { continue };
        let mut depth = 0;
        let mut found = None;
        let mut has_construct = false;
        for k in open..tokens.len() {
            if tokens[k].is_punct("{") {
                depth += 1;
            } else if tokens[k].is_punct("}") {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            } else if depth == 1 && tokens[k].is_ident_ci("function") {
                let m = tokens[k + 1..].iter().position(|t| !t.is_punct("&")).map(|p| k + 1 + p);
                if let Some(m) = m.filter(|&m| tokens[m].kind == TokenKind::Ident) {
                    if tokens[m].text.eq_ignore_ascii_case("__construct") {
                        has_construct = true;
                    } else if tokens[m].text.eq_ignore_ascii_case(&name.text) {
                        found = Some(m);
                    }
                }
            }
        }
        if let (Some(m), false) = (found, has_construct) {
            out.push(m);
        }
    }
    out
}

fn builtin_method(class: &str, name: &str) -> Option<&'static crate::builtin::Method> {
    let i = crate::builtin::CLASSES.binary_search_by(|(c, _, _)| (*c).cmp(class)).ok()?;
    let (_, parents, methods) = &crate::builtin::CLASSES[i];
    if let Ok(m) = methods.binary_search_by(|m| m.0.cmp(name)) {
        return Some(&methods[m]);
    }
    parents.iter().find_map(|p| builtin_method(p, name))
}

enum Override {
    MissingTentativeType,
    Incompatible(&'static crate::builtin::Method),
    UserIncompatible(String, crate::index::MethodSig),
}

struct BuiltinOverride {
    name: usize,
    parent: usize,
    problem: Override,
}

#[derive(Debug, Clone)]
pub(crate) struct ParamShape {
    pub by_ref: bool,
    pub variadic: bool,
    pub optional: bool,
    pub ty: Vec<String>,
    pub name: String,
}

pub(crate) fn param_shapes(tokens: &[Token], open: usize, close: usize) -> Vec<ParamShape> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut start = open + 1;
    for k in open + 1..=close {
        let t = &tokens[k];
        if k < close && (t.is_punct("(") || t.is_punct("[") || t.is_punct("#[")) {
            depth += 1;
            continue;
        }
        if k < close && (t.is_punct(")") || t.is_punct("]")) {
            depth -= 1;
            continue;
        }
        if !(k == close || depth == 0 && t.is_punct(",")) {
            continue;
        }
        let param = &tokens[start..k];
        start = k + 1;
        let Some(var) = param.iter().position(|t| t.kind == TokenKind::Variable) else { continue };
        let mut ty = Vec::new();
        let mut attr = 0;
        for t in &param[..var] {
            if t.is_punct("#[") || attr > 0 && t.is_punct("[") {
                attr += 1;
            } else if attr > 0 && t.is_punct("]") {
                attr -= 1;
            } else if attr == 0 && t.kind == TokenKind::Ident {
                let name = t.text.trim_start_matches('\\').to_ascii_lowercase();
                if !["public", "protected", "private", "readonly"].contains(&name.as_str()) {
                    ty.push(name);
                }
            } else if attr == 0 && t.is_punct("?") {
                ty.push("null".into());
            }
        }
        let null_default = param.get(var + 1).is_some_and(|t| t.is_punct("=")) && param.get(var + 2).is_some_and(|t| t.is_ident_ci("null"));
        if null_default && !ty.is_empty() && !ty.iter().any(|t| t == "null" || t == "mixed") {
            ty.push("null".into());
        }
        out.push(ParamShape {
            by_ref: param[..var].iter().rev().take_while(|t| t.is_punct("&") || t.is_punct("...")).any(|t| t.is_punct("&")),
            variadic: param[..var].iter().any(|t| t.is_punct("...")),
            optional: param.get(var + 1).is_some_and(|t| t.is_punct("=")),
            ty,
            name: param[var].text.clone(),
        });
    }
    out
}

const SCALAR_TYPES: [&str; 11] = ["int", "float", "string", "bool", "array", "null", "callable", "iterable", "false", "true", "void"];

fn accepts(child: &[String], parent: &str) -> bool {
    if child.is_empty() || child.iter().any(|t| t == "mixed") {
        return true;
    }
    let parent = parent.to_ascii_lowercase();
    let parent = parent.as_str();
    if parent.is_empty() || parent == "mixed" {
        return false;
    }
    parent.split('|').all(|p| {
        child.iter().any(|c| c == p)
            || p == "array" && child.iter().any(|c| c == "iterable")
            || (p == "true" || p == "false") && child.iter().any(|c| c == "bool")
            || !SCALAR_TYPES.contains(&p) && child.iter().any(|c| c == "object" || !SCALAR_TYPES.contains(&c.as_str()))
    })
}

fn covariant(child: &[String], parent: &str) -> bool {
    if parent.is_empty() || child.iter().any(|c| c.starts_with('(')) {
        return true;
    }
    let parent = parent.to_ascii_lowercase();
    let p: Vec<&str> = parent.split('|').collect();
    if p.contains(&"mixed") {
        return child != ["void"];
    }
    let class_like = |t: &str| !SCALAR_TYPES.contains(&t) && !["mixed", "object", "never", "static", "self"].contains(&t);
    child.iter().all(|c| {
        let c = c.as_str();
        p.contains(&c)
            || c == "never"
            || (c == "true" || c == "false") && p.contains(&"bool")
            || c == "array" && p.contains(&"iterable")
            || (c == "static" || c == "self") && p.iter().any(|t| *t == "static" || *t == "self" || *t == "object" || class_like(t))
            || class_like(c) && p.iter().any(|t| *t == "object" || *t == "iterable" || *t == "callable" || *t == "self" || class_like(t))
    })
}

fn compatible(child: &[ParamShape], is_static: bool, child_return: Option<&[String]>, parent: &crate::builtin::Method) -> bool {
    let &(name, parent_static, required, params, variadic, returns, tentative) = parent;
    let params: Vec<(bool, String)> = params.iter().map(|&(by_ref, ty)| (by_ref, ty.to_string())).collect();
    let returns = if tentative { "" } else { returns };
    fits(child, is_static, child_return, &ParentShape { name, is_static: parent_static, required, params: &params, variadic, returns })
}

struct ParentShape<'a> {
    name: &'a str,
    is_static: bool,
    required: usize,
    params: &'a [(bool, String)],
    variadic: bool,
    returns: &'a str,
}

fn fits(child: &[ParamShape], is_static: bool, child_return: Option<&[String]>, parent: &ParentShape) -> bool {
    let &ParentShape { name, is_static: parent_static, required, params, variadic, returns } = parent;
    let has_return = child_return.is_some();
    let returns = if name == "__tostring" && !has_return { "" } else { returns };
    let child_variadic = child.iter().any(|p| p.variadic);
    let fixed: Vec<&ParamShape> = child.iter().filter(|p| !p.variadic).collect();
    let child_required = fixed.iter().rposition(|p| !p.optional).map_or(0, |i| i + 1);
    is_static == parent_static
        && (returns.is_empty() || has_return && child_return.is_some_and(|r| covariant(r, returns)))
        && child_required <= required
        && (!variadic || child_variadic)
        && (child_variadic || fixed.len() >= params.len())
        && fixed.iter().zip(params.iter()).all(|(c, (by_ref, ty))| c.by_ref == *by_ref && accepts(&c.ty, ty))
}

fn user_parent_fits(
    child: &[ParamShape],
    is_static: bool,
    child_return: Option<&[String]>,
    name: &str,
    parent: &crate::index::MethodSig,
) -> bool {
    let fixed: Vec<&ParamShape> = parent.params.iter().filter(|p| !p.variadic).collect();
    let params: Vec<(bool, String)> = fixed.iter().map(|p| (p.by_ref, p.ty.join("|"))).collect();
    let returns = parent.returns.as_ref().map(|r| r.join("|")).unwrap_or_default();
    let shape = ParentShape {
        name,
        is_static: parent.is_static,
        required: fixed.iter().rposition(|p| !p.optional).map_or(0, |i| i + 1),
        params: &params,
        variadic: parent.params.iter().any(|p| p.variadic),
        returns: &returns,
    };
    fits(child, is_static, child_return, &shape)
}

fn user_declaration(
    fqn: &str,
    method: &str,
    index: &crate::index::ProjectIndex,
    depth: usize,
) -> Option<(String, crate::index::MethodSig)> {
    if depth > 8 {
        return None;
    }
    if let Some(class) = index.type_by_fqn(fqn).filter(|t| !t.is_trait) {
        if let Some(sig) = class.signatures.get(method) {
            return (!sig.is_private).then(|| (class.name.clone(), sig.clone()));
        }
        for t in &class.trait_fqns {
            if let Some((name, sig)) = index.type_by_fqn(t).and_then(|t| t.signatures.get(method).map(|s| (t.name.clone(), s.clone()))) {
                return Some((name, sig));
            }
        }
        return class.parent_fqn.iter().chain(&class.related_fqns).find_map(|p| user_declaration(p, method, index, depth + 1));
    }
    let (name, signatures, parents) = index.interface_by_fqn(fqn)?;
    if let Some(sig) = signatures.get(method) {
        return Some((name.to_string(), sig.clone()));
    }
    parents.iter().find_map(|p| user_declaration(p, method, index, depth + 1))
}

fn builtin_ancestors(name: &str, index: &crate::index::ProjectIndex, depth: usize, out: &mut Vec<String>) {
    if crate::builtin::CLASSES.binary_search_by(|(c, _, _)| (*c).cmp(name)).is_ok() {
        out.push(name.to_string());
    } else if depth < 8 {
        for parent in index.marker_interface_parents(name).unwrap_or_default() {
            builtin_ancestors(parent, index, depth + 1, out);
        }
    }
}

fn inherited_builtins(name: &str, method: &str, index: &crate::index::ProjectIndex, depth: usize, out: &mut Vec<String>) {
    if depth > 8 {
        return;
    }
    let Some(class) = index.class(name) else {
        if crate::builtin::CLASSES.binary_search_by(|(c, _, _)| (*c).cmp(name)).is_ok() {
            out.push(name.to_string());
        } else {
            builtin_ancestors(name, index, depth, out);
        }
        return;
    };
    if class.methods.contains(method) || class.traits.iter().any(|t| index.class_or_trait(t).is_none_or(|t| t.methods.contains(method))) {
        return;
    }
    for parent in class.parent.iter().chain(&class.interfaces) {
        inherited_builtins(parent, method, index, depth + 1, out);
    }
}

pub(crate) fn return_type(tokens: &[Token], close: usize) -> Option<Vec<String>> {
    if !tokens.get(close + 1)?.is_punct(":") {
        return None;
    }
    let mut out = Vec::new();
    for t in &tokens[close + 2..] {
        if t.is_punct("{") || t.is_punct(";") {
            break;
        } else if t.is_punct("(") {
            out.push("(".to_string());
        } else if t.is_punct("?") {
            out.push("null".to_string());
        } else if t.kind == TokenKind::Ident {
            out.push(t.text.trim_start_matches('\\').to_ascii_lowercase());
        }
    }
    Some(out)
}

fn display_type(ty: &[String]) -> String {
    match ty {
        [one, null] | [null, one] if null == "null" && one != "null" => format!("?{one}"),
        _ => ty.join("|"),
    }
}

fn render_signature(
    name: &str,
    is_static: bool,
    required: usize,
    params: &[(bool, String, String)],
    variadic: Option<String>,
    returns: &str,
) -> String {
    let mut args: Vec<String> = params
        .iter()
        .enumerate()
        .map(|(i, (by_ref, ty, var))| {
            let ty = if ty.is_empty() { String::new() } else { format!("{ty} ") };
            format!("{ty}{}${var}{}", if *by_ref { "&" } else { "" }, if i >= required { " = …" } else { "" })
        })
        .collect();
    if let Some(var) = variadic {
        args.push(format!("...${var}"));
    }
    let ret = if returns.is_empty() { String::new() } else { format!(": {returns}") };
    format!("{}function {name}({}){ret}", if is_static { "static " } else { "" }, args.join(", "))
}

fn builtin_overrides(tokens: &[Token], index: &crate::index::ProjectIndex) -> Vec<BuiltinOverride> {
    let mut out = Vec::new();
    let names = crate::index::Names::new(tokens);
    for i in 0..tokens.len() {
        if !(tokens[i].is_ident_ci("class") || tokens[i].is_ident_ci("enum")) || i > 0 && tokens[i - 1].is_punct("::") {
            continue;
        }
        let Some(open) = (i + 1..tokens.len()).find(|&k| tokens[k].is_punct("{") || tokens[k].is_punct(";")) else { continue };
        if !tokens[open].is_punct("{") {
            continue;
        }
        let Some(clause) = (i + 1..open).find(|&k| tokens[k].is_ident_ci("implements") || tokens[k].is_ident_ci("extends")) else {
            continue;
        };
        let mut parents: Vec<(usize, String)> = Vec::new();
        let mut user_parents: Vec<(usize, String)> = Vec::new();
        let mut user_fqns: Vec<String> = Vec::new();
        for k in clause + 1..open {
            let t = &tokens[k];
            if t.kind != TokenKind::Ident || t.is_ident_ci("implements") {
                continue;
            }
            let full = t.text.trim_start_matches('\\').to_ascii_lowercase();
            if refers_to_global(tokens, t) && crate::builtin::CLASSES.binary_search_by(|(c, _, _)| (*c).cmp(full.as_str())).is_ok() {
                parents.push((k, full));
                continue;
            }
            let short = full.rsplit('\\').next().unwrap_or_default().to_string();
            let mut found = Vec::new();
            for parent in index.marker_interface_parents(&short).unwrap_or_default() {
                builtin_ancestors(parent, index, 1, &mut found);
            }
            let fqn = names.resolve(k, &t.text);
            if index.type_by_fqn(&fqn).is_some() || index.interface_by_fqn(&fqn).is_some() {
                user_fqns.push(fqn);
            }
            if index.class(&short).is_some() {
                user_parents.push((k, short));
            }
            parents.extend(found.into_iter().map(|b| (k, b)));
        }
        if parents.is_empty() && user_parents.is_empty() && user_fqns.is_empty() {
            continue;
        }
        let mut depth = 0;
        for k in open..tokens.len() {
            if tokens[k].is_punct("{") {
                depth += 1;
            } else if tokens[k].is_punct("}") {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            } else if depth == 1 && tokens[k].is_ident_ci("function") {
                let name_at = if tokens.get(k + 1).is_some_and(|t| t.is_punct("&")) { k + 2 } else { k + 1 };
                let Some(name) = tokens.get(name_at).filter(|t| t.kind == TokenKind::Ident) else { continue };
                let lower = name.text.to_ascii_lowercase();
                let Some(popen) = tokens.get(name_at + 1).filter(|t| t.is_punct("(")).map(|_| name_at + 1) else { continue };
                let mut d = 0;
                let Some(close) = (popen..tokens.len()).find(|&j| {
                    if tokens[j].is_punct("(") {
                        d += 1;
                    } else if tokens[j].is_punct(")") {
                        d -= 1;
                    }
                    d == 0
                }) else {
                    continue;
                };
                let child_return = return_type(tokens, close);
                let start =
                    (0..k).rev().find(|&j| tokens[j].is_punct(";") || tokens[j].is_punct("{") || tokens[j].is_punct("}")).unwrap_or(0);
                let is_static = tokens[start..k].iter().any(|t| t.is_ident_ci("static"));
                if let Some((parent_name, sig)) = user_fqns.iter().find_map(|u| user_declaration(u, &lower, index, 0)) {
                    if (lower != "__construct" || sig.is_abstract)
                        && !user_parent_fits(&param_shapes(tokens, popen, close), is_static, child_return.as_deref(), &lower, &sig)
                    {
                        out.push(BuiltinOverride { name: name_at, parent: name_at, problem: Override::UserIncompatible(parent_name, sig) });
                        continue;
                    }
                }
                if lower == "__construct" {
                    continue;
                }
                let inherited = || {
                    user_parents.iter().find_map(|(at, u)| {
                        let mut found = Vec::new();
                        inherited_builtins(u, &lower, index, 0, &mut found);
                        found.iter().find_map(|b| builtin_method(b, &lower)).map(|m| (*at, m))
                    })
                };
                let direct = parents.iter().find_map(|(at, p)| builtin_method(p, &lower).map(|m| (*at, m)));
                let Some((parent, method)) = direct.or_else(inherited) else {
                    continue;
                };
                let marked = tokens[start..k].iter().any(|t| t.text.trim_start_matches('\\').eq_ignore_ascii_case("ReturnTypeWillChange"));
                let tentative_mismatch = method.6 && child_return.as_deref().is_none_or(|r| !covariant(r, method.5));
                if !compatible(&param_shapes(tokens, popen, close), is_static, child_return.as_deref(), method) {
                    out.push(BuiltinOverride { name: name_at, parent, problem: Override::Incompatible(method) });
                } else if tentative_mismatch && !marked {
                    out.push(BuiltinOverride { name: name_at, parent, problem: Override::MissingTentativeType });
                }
            }
        }
    }
    out
}

fn refers_to_global(tokens: &[Token], name: &Token) -> bool {
    if name.text.starts_with('\\') {
        return true;
    }
    let short = name.text.to_ascii_lowercase();
    let namespaced = tokens.windows(2).any(|w| w[0].is_ident_ci("namespace") && w[1].kind == TokenKind::Ident);
    let declared_here = tokens
        .windows(2)
        .any(|w| ["interface", "class", "trait", "enum"].iter().any(|k| w[0].is_ident_ci(k)) && w[1].text.eq_ignore_ascii_case(&short));
    let imported = tokens.windows(2).any(|w| w[0].is_ident_ci("use") && w[1].text.trim_start_matches('\\').eq_ignore_ascii_case(&short));
    !declared_here && (!namespaced || imported)
}

fn null_offsets(tokens: &[Token]) -> Vec<usize> {
    (2..tokens.len().saturating_sub(1))
        .filter(|&i| {
            tokens[i].is_ident_ci("null") && tokens[i - 1].is_punct("[") && tokens[i + 1].is_punct("]") && offset_base(tokens, i - 2)
        })
        .collect()
}

fn block_owner(tokens: &[Token], open: usize) -> Option<&Token> {
    if open == 0 || !tokens[open - 1].is_punct(")") {
        return None;
    }
    let mut depth = 0;
    let start = (0..open).rev().find(|&k| {
        if tokens[k].is_punct(")") {
            depth += 1;
        } else if tokens[k].is_punct("(") {
            depth -= 1;
        }
        depth == 0
    })?;
    start.checked_sub(1).map(|k| &tokens[k])
}

fn case_semicolons(tokens: &[Token]) -> Vec<usize> {
    let mut out = Vec::new();
    for i in 0..tokens.len() {
        let default = tokens[i].is_ident_ci("default") && tokens.get(i + 1).is_some_and(|t| t.is_punct(";"));
        if !tokens[i].is_ident_ci("case") && !default || i > 0 && (tokens[i - 1].is_punct("->") || tokens[i - 1].is_punct("::")) {
            continue;
        }
        let mut depth = 0;
        let open = (0..i).rev().find(|&k| {
            if tokens[k].is_punct("}") {
                depth += 1;
            } else if tokens[k].is_punct("{") {
                if depth == 0 {
                    return true;
                }
                depth -= 1;
            }
            false
        });
        if !open.and_then(|o| block_owner(tokens, o)).is_some_and(|t| t.is_ident_ci("switch")) {
            continue;
        }
        let mut d = 0usize;
        for t in &tokens[i + 1..] {
            if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") {
                d += 1;
            } else if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
                if d == 0 {
                    break;
                }
                d -= 1;
            } else if d == 0 && (t.is_punct(":") || t.is_punct("?")) {
                break;
            } else if d == 0 && t.is_punct(";") {
                out.push(i);
                break;
            }
        }
    }
    out
}

const BUILTIN_RESULT_ONE_ARG: &[&str] = &[
    "strlen",
    "is_null",
    "is_bool",
    "is_int",
    "is_integer",
    "is_long",
    "is_float",
    "is_double",
    "is_string",
    "is_array",
    "is_object",
    "is_resource",
    "is_scalar",
    "boolval",
    "intval",
    "floatval",
    "doubleval",
    "strval",
    "defined",
    "count",
    "sizeof",
    "gettype",
];

fn compiled_sprintf(format: &Token, args: usize) -> bool {
    let quoted = matches!(format.kind, TokenKind::Str { backtick: false, .. }) && format.text.len() >= 2;
    if !quoted || format.text.starts_with('"') && format.text.contains('$') {
        return false;
    }
    let body = &format.text[1..format.text.len() - 1];
    let mut chars = body.chars();
    let mut placeholders = 0;
    while let Some(c) = chars.next() {
        if c == '%' {
            match chars.next() {
                Some('s' | 'd') => placeholders += 1,
                Some('%') => {}
                _ => return false,
            }
        }
    }
    placeholders + 1 == args
}

fn builtin_results_by_ref(tokens: &[Token]) -> Vec<(usize, PhpVersion)> {
    let spaces = crate::index::namespaces(tokens);
    let mut out = Vec::new();
    for k in 2..tokens.len().saturating_sub(2) {
        if !(tokens[k].is_punct("&") && tokens[k - 1].is_punct("=") && tokens[k - 2].kind == TokenKind::Variable) {
            continue;
        }
        if k >= 3
            && (tokens[k - 3].is_punct("->")
                || tokens[k - 3].is_punct("?->")
                || tokens[k - 3].is_punct("::")
                || tokens[k - 3].is_punct("$"))
        {
            continue;
        }
        let name = &tokens[k + 1];
        if name.kind != TokenKind::Ident || !tokens[k + 2].is_punct("(") {
            continue;
        }
        let rooted = name.text.starts_with('\\');
        let bare = name.text.trim_start_matches('\\').to_ascii_lowercase();
        if bare.contains('\\') || !rooted && !crate::index::namespace_at(&spaces, k).is_empty() {
            continue;
        }
        let args = call_args(tokens, k + 2);
        let plain = args
            .iter()
            .all(|a| !a.iter().any(|t| t.is_punct("...")) && !(a.len() >= 2 && a[0].kind == TokenKind::Ident && a[1].is_punct(":")));
        if !plain {
            continue;
        }
        let n = args.len();
        let since = match bare.as_str() {
            b if BUILTIN_RESULT_ONE_ARG.contains(&b) && n == 1 => PhpVersion::new(7, 0),
            "func_num_args" | "func_get_args" | "get_called_class" if n == 0 => PhpVersion::new(7, 0),
            "get_class" if n <= 1 => PhpVersion::new(7, 0),
            "array_key_exists" if n == 2 => PhpVersion::new(7, 0),
            "array_slice" if n == 2 && args[0].len() == 3 && args[0][0].is_ident_ci("func_get_args") && args[0][1].is_punct("(") => {
                PhpVersion::new(7, 0)
            }
            "sprintf" if n >= 1 && args[0].len() == 1 && compiled_sprintf(&args[0][0], n) => PhpVersion::new(8, 4),
            _ => continue,
        };
        out.push((k, since));
    }
    out
}

fn unset_this(tokens: &[Token]) -> Vec<usize> {
    (0..tokens.len())
        .filter(|&i| tokens[i].is_ident_ci("unset") && tokens.get(i + 1).is_some_and(|t| t.is_punct("(")))
        .filter(|&i| !(i > 0 && (tokens[i - 1].is_punct("->") || tokens[i - 1].is_punct("::") || tokens[i - 1].is_ident_ci("function"))))
        .filter(|&i| call_args(tokens, i + 1).iter().any(|a| a.len() == 1 && a[0].kind == TokenKind::Variable && a[0].text == "this"))
        .collect()
}

const LOOP_KEYWORDS: [&str; 5] = ["for", "foreach", "while", "do", "switch"];
const PASS_THROUGH_BLOCKS: [&str; 9] = ["if", "elseif", "else", "try", "catch", "finally", "declare", "case", "default"];

fn stray_breaks(tokens: &[Token]) -> Vec<usize> {
    let alternative = tokens.iter().any(|t| ["endwhile", "endfor", "endforeach", "endswitch"].iter().any(|k| t.is_ident_ci(k)));
    if alternative {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 0..tokens.len() {
        if !(tokens[i].is_ident_ci("break") || tokens[i].is_ident_ci("continue")) {
            continue;
        }
        if i > 0 && (tokens[i - 1].is_punct("->") || tokens[i - 1].is_punct("::") || tokens[i - 1].is_punct("$")) {
            continue;
        }
        if !tokens.get(i + 1).is_some_and(|t| t.is_punct(";") || t.kind == TokenKind::Number || t.kind == TokenKind::CloseTag) {
            continue;
        }
        let mut depth = 0usize;
        let mut verdict = None;
        for k in (0..i).rev() {
            let t = &tokens[k];
            if depth == 0 && LOOP_KEYWORDS.iter().any(|w| t.is_ident_ci(w)) {
                verdict = Some(false);
                break;
            }
            if t.is_punct("}") {
                depth += 1;
            } else if t.is_punct("{") {
                if depth > 0 {
                    depth -= 1;
                    continue;
                }
                let owner = block_owner(tokens, k).or_else(|| k.checked_sub(1).map(|j| &tokens[j]));
                if owner.is_some_and(|o| LOOP_KEYWORDS.iter().any(|w| o.is_ident_ci(w))) {
                    verdict = Some(false);
                    break;
                }
                if owner.is_some_and(|o| PASS_THROUGH_BLOCKS.iter().any(|w| o.is_ident_ci(w))) {
                    continue;
                }
                let statement = (0..k)
                    .rev()
                    .find(|&j| tokens[j].is_punct(";") || tokens[j].is_punct("{") || tokens[j].is_punct("}"))
                    .map_or(0, |j| j + 1);
                let function = tokens[statement..k].iter().any(|t| t.is_ident_ci("function"));
                verdict = Some(function);
                break;
            }
        }
        if verdict.unwrap_or(true) {
            out.push(i);
        }
    }
    out
}

fn concat_with_arithmetic(tokens: &[Token]) -> Vec<usize> {
    let resets = [
        ";", ",", "=>", "=", "?", ":", "??", "&&", "||", "==", "===", "!=", "!==", "<>", "<", ">", "<=", ">=", "<=>", "and", "or", "xor",
        "??=", "**=", "<<=", ">>=",
    ];
    let mut out = Vec::new();
    let mut stack = vec![(false, false, None::<usize>)];
    for (k, t) in tokens.iter().enumerate() {
        if t.is_punct("(") || t.is_punct("[") || t.is_punct("{") || t.is_punct("#[") {
            stack.push((false, false, None));
            continue;
        }
        if t.is_punct(")") || t.is_punct("]") || t.is_punct("}") {
            if stack.len() > 1 {
                stack.pop();
            }
            continue;
        }
        let level = stack.last_mut().unwrap();
        let binary = k > 0 && expression_end(&tokens[k - 1]);
        let compound = tokens.get(k + 1).is_some_and(|n| n.is_punct("=") && n.start == t.end);
        if t.kind == TokenKind::CloseTag
            || (t.kind == TokenKind::Punct && resets.contains(&t.text.as_str()))
            || (t.kind == TokenKind::Ident && ["and", "or", "xor", "return", "echo", "print"].iter().any(|r| t.is_ident_ci(r)))
            || compound
        {
            *level = (false, false, None);
        } else if t.is_punct(".") && binary {
            level.0 = true;
            level.2.get_or_insert(k);
        } else if (t.is_punct("+") || t.is_punct("-")) && binary && level.0 {
            level.1 = true;
        }
        if level.0 && level.1 {
            out.push(level.2.unwrap());
            *level = (false, false, None);
        }
    }
    out
}

fn class_bodies(tokens: &[Token]) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    for i in 0..tokens.len() {
        if !tokens[i].is_ident_ci("class") || i > 0 && (tokens[i - 1].is_punct("::") || tokens[i - 1].is_ident_ci("new")) {
            continue;
        }
        let Some(open) = (i + 1..tokens.len()).find(|&k| tokens[k].is_punct("{") || tokens[k].is_punct(";")) else { continue };
        if !tokens[open].is_punct("{") {
            continue;
        }
        let mut depth = 0;
        let close = (open..tokens.len()).find(|&k| {
            if tokens[k].is_punct("{") {
                depth += 1;
            } else if tokens[k].is_punct("}") {
                depth -= 1;
            }
            depth == 0
        });
        if let Some(close) = close {
            out.push((i, open, close));
        }
    }
    out
}

fn declares_method(tokens: &[Token], open: usize, close: usize, name: &str) -> bool {
    (open..close).any(|k| tokens[k].is_ident_ci("function") && tokens.get(k + 1).is_some_and(|t| t.is_ident_ci(name)))
}

fn serializable_without_magic(tokens: &[Token]) -> Vec<usize> {
    class_bodies(tokens)
        .into_iter()
        .filter(|&(i, open, close)| {
            let implements = tokens[i..open].iter().any(|t| t.text.trim_start_matches('\\').eq_ignore_ascii_case("Serializable"));
            let extends = tokens[i..open].iter().any(|t| t.is_ident_ci("extends")) || i > 0 && tokens[i - 1].is_ident_ci("abstract");
            implements && !extends && !declares_method(tokens, open, close, "__serialize")
        })
        .filter_map(|(i, _, _)| tokens.get(i + 1).filter(|t| t.kind == TokenKind::Ident).map(|_| i + 1))
        .collect()
}

fn dynamic_properties(tokens: &[Token], index: &crate::index::ProjectIndex) -> Vec<usize> {
    let mut out = Vec::new();
    for decl in crate::index::class_decls(tokens) {
        let (open, close) = (decl.open, decl.close);
        let Some(declared) = index.properties(&decl.info) else { continue };
        let nested: Vec<(usize, usize)> = (open..close)
            .filter(|&k| tokens[k].is_ident_ci("class") && k > 0 && tokens[k - 1].is_ident_ci("new"))
            .filter_map(|k| {
                let body = (k..close).find(|&j| tokens[j].is_punct("{"))?;
                let mut d = 0;
                let end = (body..close).find(|&j| {
                    if tokens[j].is_punct("{") {
                        d += 1;
                    } else if tokens[j].is_punct("}") {
                        d -= 1;
                    }
                    d == 0
                })?;
                Some((body, end))
            })
            .collect();
        let mut seen = std::collections::HashSet::new();
        for k in open..close.saturating_sub(2) {
            if !(tokens[k].kind == TokenKind::Variable && tokens[k].text == "this" && tokens[k + 1].is_punct("->")) {
                continue;
            }
            if nested.iter().any(|&(a, b)| k > a && k < b) {
                continue;
            }
            let name = &tokens[k + 2];
            if name.kind != TokenKind::Ident || declared.contains(&name.text.to_ascii_lowercase()) {
                continue;
            }
            let mut j = k + 3;
            while tokens.get(j).is_some_and(|t| t.is_punct("[")) {
                let mut d = 0;
                match (j..close).find(|&m| {
                    if tokens[m].is_punct("[") {
                        d += 1;
                    } else if tokens[m].is_punct("]") {
                        d -= 1;
                    }
                    d == 0
                }) {
                    Some(end) => j = end + 1,
                    None => break,
                }
            }
            let Some(next) = tokens.get(j) else { continue };
            let assigns = next.is_punct("=")
                || next.is_punct("??=")
                || ([".", "+", "-", "*", "/", "%", "|", "&", "^"].iter().any(|op| next.is_punct(op))
                    && tokens.get(j + 1).is_some_and(|n| n.is_punct("=") && n.start == next.end));
            if assigns && seen.insert(name.text.to_ascii_lowercase()) {
                out.push(k + 2);
            }
        }
    }
    out
}

fn defined_constants(tokens: &[Token]) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();
    for (i, t) in tokens.iter().enumerate() {
        if t.is_ident_ci("const") {
            let mut j = i + 1;
            while let Some(n) = tokens.get(j).filter(|n| n.kind == TokenKind::Ident) {
                if tokens.get(j + 1).is_some_and(|x| x.is_punct("=")) {
                    names.insert(n.text.clone());
                    break;
                }
                if tokens.get(j + 1).is_some_and(|x| x.is_ident_ci("as")) {
                    j += 2;
                    continue;
                }
                names.insert(n.text.rsplit('\\').next().unwrap_or_default().to_string());
                j += 1;
            }
        }
        if t.is_ident_ci("define") && tokens.get(i + 1).is_some_and(|n| n.is_punct("(")) {
            if let Some(name) = tokens.get(i + 2).filter(|n| matches!(n.kind, TokenKind::Str { .. })) {
                names.insert(name.text.get(1..name.text.len().saturating_sub(1)).unwrap_or_default().to_string());
            }
        }
    }
    names
}

fn is_bareword(t: &Token, defined: &std::collections::HashSet<String>) -> bool {
    t.kind == TokenKind::Ident
        && !t.text.contains('\\')
        && t.text.bytes().any(|b| b.is_ascii_lowercase())
        && !["true", "false", "null"].iter().any(|k| t.is_ident_ci(k))
        && !(t.text.starts_with("__") && t.text.ends_with("__"))
        && !defined.contains(&t.text)
}

fn bareword_keys(tokens: &[Token]) -> Vec<usize> {
    let defined = defined_constants(tokens);
    (2..tokens.len().saturating_sub(1))
        .filter(|&i| {
            tokens[i - 1].is_punct("[") && tokens[i + 1].is_punct("]") && is_bareword(&tokens[i], &defined) && offset_base(tokens, i - 2)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule_ids(src: &str) -> Vec<String> {
        analyze_source("a.php", src, PhpVersion::new(8, 4)).into_iter().map(|f| f.rule_id).collect()
    }

    #[test]
    fn curly_offsets_on_properties_and_elements() {
        let src = "<?php\n$this->name{0}; $a['k']{1}; $str {2}; $o?->p{3};\nif ($x) { echo 1; }\n";
        assert_eq!(rule_ids(src).iter().filter(|r| *r == "removed.curly_offset").count(), 4);
        assert_eq!(fix_curly_offsets(src).unwrap(), "<?php\n$this->name[0]; $a['k'][1]; $str [2]; $o?->p[3];\nif ($x) { echo 1; }\n");
        assert_eq!(fix_curly_offsets("<?php foreach ($a['x'] as $v) { } function f($a = []) { }"), None);
    }

    #[test]
    fn legacy_syntax_fix_quotes_bare_keys_and_curly_offsets() {
        let (fixed, applied) = fix_legacy_syntax("<?php echo $row[id] . $s{0} . $a[KEY] . $b['x'][name];").unwrap();
        assert_eq!(fixed, "<?php echo $row['id'] . $s[0] . $a[KEY] . $b['x']['name'];");
        assert_eq!(applied, [CURLY_FIX, BAREWORD_FIX]);
        assert_eq!(fix_legacy_syntax("<?php $a = ['id' => 1];"), None);
    }

    #[test]
    fn nothing_after_halt_compiler_is_code() {
        let src = "<?php\nmysql_query($q);\n__halt_compiler();\nmysql_query(each($x)); $s{0};";
        assert_eq!(rule_ids(src), ["removed.mysql"]);
        assert_eq!(fix_curly_offsets(src), None);
    }

    #[test]
    fn user_functions_with_removed_names_are_not_flagged() {
        assert!(rule_ids("<?php namespace App; function each($a) {} each($x);").is_empty());
        assert!(rule_ids("<?php use function App\\Util\\each; each($x);").is_empty());
        assert_eq!(rule_ids("<?php namespace App; function each($a) {} \\each($x);"), ["removed.each"]);
        assert!(rule_ids("<?php function each($a) {} \\each($x);").is_empty(), "a global polyfill also serves rooted calls");
        assert!(rule_ids("<?php mysql_escape_custom($x);").is_empty());
        assert_eq!(rule_ids("<?php mysql_fetch_assoc($r);"), ["removed.mysql"]);
        assert_eq!(rule_ids("<?php user_error('x', E_USER_ERROR);"), ["deprecated.e_user_error"]);
    }

    fn count(src: &str, id: &str) -> usize {
        rule_ids(src).iter().filter(|r| *r == id).count()
    }

    #[test]
    fn nested_ternaries_need_parentheses() {
        let id = "removed.nested_ternary";
        assert_eq!(count("<?php $x = $a ? 1 : $b ? 2 : 3;", id), 1);
        assert_eq!(count("<?php $x = $a ?: $b ? 2 : 3;", id), 1);
        assert_eq!(count("<?php $x = $a ? 1 : $b ?: 3;", id), 1);
        assert_eq!(count("<?php $x = $a ?: $b ?: $c;", id), 0);
        assert_eq!(count("<?php $x = $a ? ($b ? 1 : 2) : 3; $y = $a ? $b ? 1 : 2 : 3;", id), 0);
        assert_eq!(count("<?php $x = $a ? 1 : ($b ? 2 : 3); f($a ? 1 : 2, $b ? 3 : 4);", id), 0);
        assert_eq!(count("<?php class A { private ?int $a = null; public function f(?int $x): ?int { return $x ? 1 : 2; } }", id), 0);
        assert_eq!(count("<?php switch ($a) { case 1: $b = $c ? 1 : 2; break; default: $d = $e ?: 3; }", id), 0);
        assert_eq!(count("<?php $m = match($a) { 1 => $b ? 1 : 2, default => $c ? 3 : 4 };", id), 0);
    }

    #[test]
    fn php4_constructors_only_outside_namespaces() {
        let id = "removed.php4_constructor";
        assert_eq!(count("<?php class Report { function Report() {} }", id), 1);
        assert_eq!(count("<?php class Report { function __construct() {} function report() {} }", id), 0);
        assert_eq!(count("<?php namespace App; class Report { function Report() {} }", id), 0);
        assert_eq!(count("<?php $o = new class { function x() {} };", id), 0);
    }

    #[test]
    fn call_argument_rules() {
        assert_eq!(count("<?php define('A', 1, true); define('B', 2); define('C', 3, false);", "removed.case_insensitive_constant"), 1);
        assert_eq!(count("<?php parse_str($q); parse_str($q, $out);", "removed.parse_str_one_arg"), 1);
        assert_eq!(count("<?php assert('$a > 0'); assert($a > 0);", "removed.assert_string"), 1);
        let csv = "<?php fputcsv($f, $row); fputcsv($f, $row, ',', '\"', ''); fputcsv($f, $row, escape: ''); str_getcsv($s); str_getcsv($s, ',', '\"', '\\\\'); fgetcsv($f);";
        assert_eq!(count(csv, "deprecated.csv_escape"), 3);
        assert_eq!(
            count(
                "<?php mb_convert_encoding($h, 'HTML-ENTITIES', 'UTF-8'); mb_convert_encoding($h, 'UTF-8'); echo 'html';",
                "deprecated.mb_encoding"
            ),
            1
        );
    }

    #[test]
    fn bareword_keys_and_reserved_names() {
        assert_eq!(
            count("<?php echo $row[id]; echo $a['id'][name]; $b[ID]; $c[true]; $d[Foo\\BAR]; $e = [id];", "removed.bareword_key"),
            2
        );
        let f = analyze_source(
            "a.php",
            "<?php class Object {} class Foo {} function match() {} class M { function match() {} }",
            PhpVersion::new(8, 4),
        );
        let reserved: Vec<_> = f.iter().filter(|f| f.rule_id == "removed.reserved_name").map(|f| f.since.to_string()).collect();
        assert_eq!(reserved, ["7.2", "8.0"]);
    }

    fn at(src: &str, target: (u32, u32)) -> Vec<(String, String)> {
        analyze_source("a.php", src, PhpVersion::new(target.0, target.1))
            .into_iter()
            .map(|f| (f.rule_id, f.title.get(crate::i18n::Lang::En).to_string()))
            .collect()
    }

    #[test]
    fn method_calls_and_class_constants() {
        let src = "<?php\n$db = new mysqli('h'); $db->ping(); $r = new \\ReflectionProperty($o, 'x'); $r->setAccessible(true);\n$s = new SplObjectStorage(); $s->attach($o);\nReflectionClass::export('A'); $pdo->setAttribute(PDO::MYSQL_ATTR_INIT_COMMAND, 'x'); $x = PDO::FETCH_SERIALIZE;";
        let hits = at(src, (8, 5));
        let titles: Vec<&str> = hits.iter().map(|(_, t)| t.as_str()).collect();
        assert_eq!(
            titles,
            [
                "->ping()",
                "->setAccessible()",
                "->attach()",
                "ReflectionClass::export()",
                "PDO::MYSQL_ATTR_INIT_COMMAND",
                "PDO::FETCH_SERIALIZE"
            ]
        );
        assert!(at("<?php $client->ping(); $set->attach($x); $q->refresh();", (8, 5)).is_empty(), "no hint, no finding");
        assert!(at("<?php $r = new ReflectionMethod('A', 'b'); $r->setAccessible(true);", (8, 4)).is_empty(), "below the target");
    }

    #[test]
    fn interface_methods_need_return_types() {
        let src = "<?php class C implements \\Countable, ArrayAccess, JsonSerializable {\n public function count() { return 0; }\n public function offsetGet($o): mixed { return 1; }\n #[\\ReturnTypeWillChange]\n public function offsetSet($o, $v) {}\n public function offsetExists($o) { return true; }\n public function jsonSerialize() { return []; }\n public function other() {}\n}";
        let names: Vec<String> = analyze_source("a.php", src, PhpVersion::new(8, 1))
            .into_iter()
            .filter(|f| f.rule_id == "deprecated.tentative_return_type")
            .map(|f| f.snippet)
            .collect();
        assert_eq!(names.len(), 3, "{names:?}");
        assert_eq!(count("<?php echo $http_response_header[0];", "deprecated.http_response_header"), 0, "8.5 only");
        assert_eq!(at("<?php echo $http_response_header[0];", (8, 5))[0].0, "deprecated.http_response_header");
    }

    fn count85(src: &str, id: &str) -> usize {
        at(src, (8, 5)).iter().filter(|(r, _)| r == id).count()
    }

    #[test]
    fn tentative_return_types_of_extended_built_in_classes() {
        let id = "deprecated.tentative_return_type";
        let src = "<?php
class Db extends PDO {
    public function query($sql, $mode = null, ...$args) {}
    public function commit() {}
    #[\\ReturnTypeWillChange] public function rollBack() {}
    public function beginTransaction(): bool { return true; }
    public function helper() {}
}
class Files extends RecursiveDirectoryIterator { function next() {} function getChildren() {} }
class Handler implements SessionHandlerInterface { function read($id) {} }";
        assert_eq!(lines_of(src, "8.1", id), vec![3, 4, 9, 9, 10]);
        assert!(lines_of(src, "8.0", id).is_empty());
        assert_eq!(count85("<?php namespace App; class PDO {} class Db extends PDO { function commit() {} }", id), 0);
        assert_eq!(count85("<?php namespace App; use PDO; class Db extends PDO { function commit() {} }", id), 1);
    }

    #[test]
    fn compile_time_problems_in_version_guarded_code() {
        let src = "<?php\nif (version_compare(PHP_VERSION, '5.3.0', '<')) {\n    $c = $name{0};\n    srand((double) microtime());\n    mysql_connect();\n}\n";
        let ids: Vec<String> = at(src, (8, 5)).into_iter().map(|(id, _)| id).collect();
        assert_eq!(
            ids,
            ["removed.curly_offset", "deprecated.non_canonical_cast"],
            "runtime findings stay skipped, compile-time ones don't"
        );
    }

    #[test]
    fn default_with_a_semicolon() {
        let id = "deprecated.case_semicolon";
        assert_eq!(count85("<?php switch ($a) { case 1: break; default; echo 1; }", id), 1);
        assert_eq!(count85("<?php $x = match ($a) { default => 1 }; $o->default; Foo::default;", id), 0);
    }

    #[test]
    fn assign_new_by_reference_and_unset_this() {
        assert_eq!(count85("<?php $a = &new Foo; $b =& new Bar(); $c = new Baz; $d = &$e;", "removed.assign_new_by_ref"), 2);
        assert_eq!(count85("<?php unset($this); unset($a, $this); unset($this->x); $o->unset($this);", "removed.unset_this"), 2);
    }

    #[test]
    fn built_in_results_assigned_by_reference() {
        let id = "removed.builtin_result_by_ref";
        let fatal = [
            "function g() { $args = & func_get_args(); }",
            "function g($s) { $n =& STRLEN ($s); }",
            "function g($a) { $n = &count($a); }",
            "function g($o) { $c = &get_class(); }",
            "function g($a) { $e = &array_key_exists(1, $a); }",
            "function g() { $rest = &array_slice(func_get_args(), 1); }",
            "namespace A; function g($s) { $n = &\\strlen($s); }",
        ];
        for src in fatal {
            assert_eq!(count85(&format!("<?php {src}"), id), 1, "{src}");
        }
        let fine = [
            "function g($a) { $n = &count($a, 1); }",
            "function g($a) { $v = &end($a); }",
            "function g($a) { $r = &array_slice($a, 1); }",
            "function g($a) { $n = &strlen(...$a); }",
            "function g($s) { $a[] = &strlen($s); }",
            "function g($s) { $o->p = &strlen($s); }",
            "function g($s) { $n = &$this->strlen($s); }",
            "namespace A; function g($s) { $n = &strlen($s); }",
            "function g($a) { $s = &sprintf(\"%.2f\", $a); }",
            "function g($a) { $s = &sprintf(\"%s\", $a, $a); }",
        ];
        for src in fine {
            assert_eq!(count85(&format!("<?php {src}"), id), 0, "{src}");
        }
        let sprintf = "<?php function g($a) { $s = &sprintf('%s-%d', $a, $a); }";
        assert_eq!(at(sprintf, (8, 4)).iter().filter(|(r, _)| r == id).count(), 1);
        assert_eq!(at(sprintf, (8, 3)).iter().filter(|(r, _)| r == id).count(), 0);
        let (fixed, applied) = fix_legacy_syntax("<?php function g() { $args = & func_get_args(); $v = &end($a); }").unwrap();
        assert_eq!(fixed, "<?php function g() { $args = func_get_args(); $v = &end($a); }");
        assert_eq!(applied, [BUILTIN_BY_REF_FIX]);
    }

    #[test]
    fn legacy_syntax_prefix_handles_new_by_reference_and_default() {
        let src = "<?php\n$a = &new Foo;\n$b =& new Bar();\n$c = &$d;\nswitch ($x) { case 1: break; default; echo 1; }\n";
        let (fixed, applied) = fix_legacy_syntax(src).unwrap();
        assert_eq!(fixed, "<?php\n$a = new Foo;\n$b = new Bar();\n$c = &$d;\nswitch ($x) { case 1: break; default: echo 1; }\n");
        assert_eq!(applied, [NEW_BY_REF_FIX, DEFAULT_COLON_FIX]);
        let rule = crate::rector::describe_rule(NEW_BY_REF_FIX);
        assert_eq!((rule.title.en.as_str(), rule.since), ("= &new → = new", Some(PhpVersion::new(7, 0))));
    }

    #[test]
    fn break_outside_a_loop() {
        let id = "removed.break_outside_loop";
        let stray = [
            "<?php function f($r) { if ($r) { throw new E(); break; } }",
            "<?php class A { function m() { try { x(); } catch (E $e) { continue; } } }",
            "<?php $f = function () use ($x) { if ($x) { break; } };",
            "<?php foreach ($a as $b) { $f = function () { break; }; }",
            "<?php if ($x) { break; }",
        ];
        for src in stray {
            assert_eq!(count85(src, id), 1, "{src}");
        }
        let fine = [
            "<?php foreach ($a as $b) { if ($b) { break; } }",
            "<?php while (true) break;",
            "<?php switch ($a) { case 1: if ($b) { break; } default: break; }",
            "<?php do { if ($x) { continue; } } while ($y);",
            "<?php for ($i = 0; $i < 3; $i++) { try { x(); } catch (E $e) { continue 1; } }",
            "<?php foreach ($a as $b): if ($b) { break; } endforeach;",
            "<?php function f($a) { foreach ($a as $b) { switch ($b) { case 1: break 2; } } }",
            "<?php while ($x) { ?><p><?php break; ?></p><?php }",
            "<?php $o->break(); Foo::continue();",
        ];
        for src in fine {
            assert_eq!(count85(src, id), 0, "{src}");
        }
    }

    #[test]
    fn overrides_incompatible_with_built_in_signatures() {
        let id = "removed.incompatible_override";
        let fatal = [
            "class A extends PDO { public function query($query, array $args = array(), $options = array()) {} }",
            "class A extends DateTime { public function createFromFormat($f, $t, $tz = null) {} }",
            "class A extends DateTime { public function format() {} }",
            "class A extends PDO { public function getAttribute(string $a) {} }",
            "class A extends SplObjectStorage { public function attach(object $o) {} }",
            "class A extends ArrayObject { public function append(&$v) {} }",
            "class A extends ArrayIterator { public function seek(string $p) {} }",
            "class A extends ArrayIterator { public function uasort($c, $extra) {} }",
            "namespace App; class A extends \\PDO { public function query($q) {} }",
            "namespace App; use PDO; class A extends PDO { public function exec($s, $x) {} }",
            "class A extends ArrayIterator { #[\\ReturnTypeWillChange] public function offsetGet(string $k) {} }",
            "class A implements SessionHandlerInterface { function read(&$id) {} }",
        ];
        for src in fatal {
            assert_eq!(count85(&format!("<?php {src}"), id), 1, "{src}");
        }
        let fine = [
            "class A extends ArrayIterator { function offsetSet($k, $v): void {} }",
            "class A implements Stringable { function __toString() { return ''; } }",
            "class A extends PDO { public function prepare($q, array $o = array()) {} }",
            "class A extends PDO { public function query(string $query, ?int $fetchMode = null, mixed ...$fetchModeArgs) {} }",
            "class A extends SplObjectStorage { public function attach($o, $inf = null) {} }",
            "class A extends ArrayIterator { public function seek(int|string $p) {} }",
            "class A extends ArrayIterator { public function offsetSet($k, $v, ...$rest) {} }",
            "class A extends ArrayIterator { public function offsetSet(#[SensitiveParameter] $k, $v = null) {} }",
            "class A extends Exception { public function __construct($m) {} }",
            "namespace App; class PDO {} class A extends PDO { public function query($q) {} }",
        ];
        for src in fine {
            assert_eq!(count85(&format!("<?php {src}"), id), 0, "{src}");
        }
        let found = analyze_source("t.php", "<?php class Db extends PDO { function query($sql, array $p = []) {} }", PhpVersion::new(8, 1));
        let f = found.iter().find(|f| f.rule_id == id).unwrap();
        assert_eq!(f.title.en, "PDO::query()");
        assert_eq!(f.replacement.as_ref().unwrap().en, "function query(string $p1, int|null $p2 = …, ...$p3): PDOStatement|false");
    }

    #[test]
    fn return_types_must_fit_built_in_ones() {
        let dep = "deprecated.tentative_return_type";
        let fatal = "removed.incompatible_override";
        let iterator = |valid: &str, next: &str| {
            format!("<?php class A implements Iterator {{ function current(): mixed {{}} function key(): mixed {{}} function next(): {next} {{}} function rewind(): void {{}} function valid(): {valid} {{}} }}")
        };
        for src in [
            "<?php class A extends DateTime { public function modify(string $m): DateTime|false {} }".to_string(),
            "<?php class A implements Countable { public function count(): string {} }".into(),
            "<?php class A implements Countable { public function count(): float {} }".into(),
            "<?php class A implements IteratorAggregate { public function getIterator(): array {} }".into(),
            "<?php class A implements IteratorAggregate { public function getIterator(): iterable {} }".into(),
            "<?php class A implements JsonSerializable { public function jsonSerialize(): void {} }".into(),
            "<?php class A extends DateTimeZone { public function getName(): ?string {} }".into(),
            "<?php class A extends ArrayIterator { public function getArrayCopy(): iterable {} }".into(),
            iterator("bool", "int"),
            iterator("int", "void"),
        ] {
            assert_eq!(count85(&src, dep), 1, "{src}");
        }
        for src in [
            "<?php class A implements IteratorAggregate { public function getIterator(): ArrayIterator {} }".to_string(),
            "<?php class A implements IteratorAggregate { public function getIterator(): Generator {} }".into(),
            "<?php class A implements JsonSerializable { public function jsonSerialize(): never {} }".into(),
            "<?php class A extends ArrayIterator { public function current(): int {} }".into(),
            "<?php class A extends Exception implements JsonSerializable { function jsonSerialize(): static {} }".into(),
            "<?php class A extends DateTime { public function getTimezone(): DateTimeZone|false {} }".into(),
            "<?php class A implements Stringable { function __toString() { return ''; } }".into(),
            iterator("bool", "void"),
            iterator("true", "void"),
        ] {
            assert_eq!(count85(&src, dep) + count85(&src, fatal), 0, "{src}");
        }
        assert_eq!(count85("<?php class A implements Stringable { function __toString(): int {} }", fatal), 1);
    }

    #[test]
    fn built_in_interfaces_inherited_through_marker_interfaces() {
        let id = "deprecated.tentative_return_type";
        assert_eq!(count85("<?php interface I extends Countable {} class A implements I { public function count() { return 0; } }", id), 1);
        assert_eq!(count85("<?php namespace App; interface I extends \\Countable {} class A implements I { function count() {} }", id), 1);
        assert_eq!(
            count85("<?php interface I extends Countable {} interface J extends I {} class A implements J { function count() {} }", id),
            1
        );
        assert_eq!(
            count85(
                "<?php interface I extends Countable { public function count(): int; } class A implements I { function count() {} }",
                id
            ),
            0
        );
        assert_eq!(count85("<?php interface I {} class A implements I { function count() {} }", id), 0);
        assert_eq!(count85("<?php interface I { function count(); } class A implements I, Countable { function count() {} }", id), 1);
    }

    #[test]
    fn built_in_methods_inherited_through_project_classes() {
        let fatal = "removed.incompatible_override";
        let dep = "deprecated.tentative_return_type";
        let cases: [(&str, &str, usize); 10] = [
            ("class A extends PDO {} class B extends A { public function query($q, array $a = []) {} }", fatal, 1),
            ("class A extends DateTime {} class B extends A { public function format() {} }", fatal, 1),
            ("class A extends ArrayObject {} class B extends A { function offsetGet(string $k) {} }", fatal, 1),
            ("class A extends PDO {} class B extends A {} class C extends B { public function exec($s, $x) {} }", fatal, 1),
            ("class A extends SimpleXMLElement {} class B extends A { public function getChildren($source) {} }", fatal, 1),
            ("abstract class A implements Countable {} class B extends A { public function count() { return 0; } }", dep, 1),
            ("interface I extends Countable {} abstract class A implements I {} class B extends A { function count() {} }", dep, 1),
            ("class A extends PDO { public function query($q, ...$args) {} } class B extends A { public function query($q, array $a = []) {} }", fatal, 0),
            ("trait T { public function count(): int { return 0; } } class A implements Countable { use T; } class B extends A { function count() { return 0; } }", fatal, 0),
            ("namespace App; class PDO {} class A extends PDO {} class B extends A { public function query($q) {} }", fatal, 0),
        ];
        for (src, id, n) in cases {
            assert_eq!(count85(&format!("<?php {src}"), id), n, "{src}");
        }
    }

    #[test]
    fn overrides_incompatible_with_project_signatures() {
        let id = "removed.incompatible_user_override";
        let fatal = [
            "class A { function f($a) {} } class B extends A { function f() {} }",
            "class A { function f($a = 1) {} } class B extends A { function f($a) {} }",
            "class A { function f($a) {} } class B extends A { function f(array $a) {} }",
            "class A { function f(&$a) {} } class B extends A { function f($a) {} }",
            "class A { static function f() {} } class B extends A { function f() {} }",
            "class A { function f(): int {} } class B extends A { function f() {} }",
            "class A { function f(): int {} } class B extends A { function f(): ?int {} }",
            "class A { function f(?int $a) {} } class B extends A { function f(int $a) {} }",
            "abstract class A { abstract function __construct($a); } class B extends A { function __construct() {} }",
            "interface I { function f($a); } interface J extends I {} class B implements J { function f() {} }",
            "trait T { function f($a) {} } class A { use T; } class B extends A { function f() {} }",
            "class A { function f($a) {} } class M extends A { function f($a, $b = 2) {} } class B extends M { function f($a) {} }",
            "namespace Lib { class Base { function f($a) {} } } namespace App { use Lib\\Base as P; class B extends P { function f() {} } }",
        ];
        for src in fatal {
            assert_eq!(count85(&format!("<?php {src}"), id), 1, "{src}");
        }
        let fine = [
            "class A { function f($a) {} } class B extends A { function f($a, $b = 1) {} }",
            "class A { function f(array $a) {} } class B extends A { function f($a) {} }",
            "class A { function f() {} } class B extends A { function f(): int {} }",
            "class A { function f(): ?int {} } class B extends A { function f(): int {} }",
            "class A { private function f($a) {} } class B extends A { function f() {} }",
            "class A { function __construct($a) {} } class B extends A { function __construct() {} }",
            "class A { function f(): self {} } class B extends A { function f(): B {} }",
            "class A { function f(?int $a) {} } class B extends A { function f(int $a = null) {} }",
            "class A { function f($a) {} } class B extends A { function f(...$a) {} }",
            "namespace Lib { class Base { function f($a) {} } } namespace App { class Base {} class B extends Base { function f() {} } }",
            "namespace App; use Vendor\\TestCase; class TestCase { function f($a) {} } class B extends TestCase { function f() {} }",
        ];
        for src in fine {
            assert_eq!(count85(&format!("<?php {src}"), id), 0, "{src}");
        }
        let found = analyze_source(
            "t.php",
            "<?php class A { function f(?int $a, &$b = []) {} } class B extends A { function f() {} }",
            PhpVersion::new(8, 1),
        );
        let f = found.iter().find(|f| f.rule_id == id).unwrap();
        assert_eq!(f.title.en, "A::f()");
        assert_eq!(f.replacement.as_ref().unwrap().en, "function f(?int $a, &$b = …)");
    }

    #[test]
    fn deprecated_resource_frees_and_intl_calendar_functions() {
        let src = "<?php\nopenssl_x509_free($c);\nopenssl_pkey_free($k);\nopenssl_free_key($k);\nshmop_close($s);\nintlcal_set($cal, 2024, 1, 1);\nintlgregcal_create_instance();";
        let ids: Vec<String> = at(src, (8, 4)).into_iter().map(|(id, _)| id).collect();
        assert_eq!(ids.len(), 6, "{ids:?}");
        assert_eq!(at(src, (8, 3)).len(), 4);
    }

    #[test]
    fn more_call_and_constant_rules() {
        assert_eq!(count85("<?php implode($items, ', '); implode(', ', $items); join($a, '-'); implode($a);", "removed.implode_order"), 2);
        assert_eq!(count85("<?php mb_check_encoding(); mb_check_encoding($s); readdir(); readdir($h);", "deprecated.implicit_argument"), 2);
        assert_eq!(
            count85(
                "<?php session_set_save_handler('o', 'c', 'r', 'w', 'd', 'g'); session_set_save_handler($h, true);",
                "deprecated.session_set_save_handler"
            ),
            1
        );
        assert_eq!(
            count85(
                "<?php ini_set('session.sid_length', 32); ini_get('mbstring.func_overload'); ini_set('mbstring.func_overload', 0);",
                "deprecated.ini_setting"
            ),
            2
        );
        assert_eq!(count85("<?php $a[null] = 1; array_key_exists(null, $a); $b = [null]; $c[0];", "deprecated.null_offset"), 2);
        assert_eq!(count85("<?php pg_numrows($r); pg_num_rows($r); pg_query($c, $q);", "deprecated.pg_alias"), 1);
        assert_eq!(
            count85("<?php $x = ASSERT_ACTIVE; $y = ASSERT_QUIET_EVAL; $z = MHASH_MD5; $w = FILE_BINARY;", "deprecated.assert_constants"),
            1
        );
        assert_eq!(count85("<?php $y = ASSERT_QUIET_EVAL;", "removed.assert_quiet_eval"), 1);
        assert_eq!(count85("<?php echo $php_errormsg; echo $HTTP_RAW_POST_DATA; echo $errormsg;", "removed.magic_variable"), 2);
    }

    #[test]
    fn case_semicolons_only_in_switch() {
        let id = "deprecated.case_semicolon";
        assert_eq!(count85("<?php switch ($a) { case 1; echo 1; break; case 2: break; case Foo::BAR; default: }", id), 2);
        assert_eq!(count85("<?php enum Suit { case Hearts; case Spades; }", id), 0);
        assert_eq!(count85("<?php switch ($a) { case $b ? 1 : 2: break; }", id), 0);
    }

    #[test]
    fn concat_before_arithmetic_changes_meaning() {
        let id = "behavior.concat_precedence";
        assert_eq!(count85("<?php echo 'Sum: ' . $a + $b;", id), 1);
        assert_eq!(count85("<?php echo 'Sum: ' . ($a + $b); $x = $a - 1 . 'x'; $y = 'a' . -1; $s .= $a + 1;", id), 0);
        assert_eq!(count85("<?php $u = $base . '/page/' . $n - 1;", id), 1);
        assert_eq!(count85("<?php f('a' . $b, $c + 1); $d = $e == 'x' . $f ? $g + 1 : 0;", id), 0);
    }

    #[test]
    fn dynamic_properties_in_simple_classes() {
        let id = "deprecated.dynamic_property";
        let src = "<?php class A {\n public $declared;\n public function __construct(private $promoted, $plain) { $this->declared = 1; $this->promoted = 2; $this->extra = 3; $this->extra = 4; $this->list[] = 5; $this->total += 1; $x = $this->read; }\n}";
        let found: Vec<_> =
            analyze_source("a.php", src, PhpVersion::new(8, 2)).into_iter().filter(|f| f.rule_id == id).map(|f| f.snippet).collect();
        assert_eq!(found.len(), 3, "{found:?}");
        assert_eq!(count85("<?php class B extends A { function f() { $this->x = 1; } }", id), 0);
        assert_eq!(count85("<?php #[\\AllowDynamicProperties]\nclass C { function f() { $this->x = 1; } }", id), 0);
        assert_eq!(count85("<?php class D { use T; function f() { $this->x = 1; } }", id), 0);
        assert_eq!(count85("<?php class E { function __set($n, $v) {} function f() { $this->x = 1; } }", id), 0);
        assert_eq!(count85("<?php class F { var $x; function f() { $this->x = 1; } }", id), 0);
        assert_eq!(count85("<?php class G { function f() { return new class(1) extends V { private $a; function __construct($a) { $this->a = $a; } }; } }", id), 0);
    }

    #[test]
    fn serializable_needs_magic_methods() {
        let id = "deprecated.serializable";
        assert_eq!(count85("<?php class A implements \\Serializable { function serialize() {} function unserialize($d) {} }", id), 1);
        assert_eq!(count85("<?php class B implements Serializable { function __serialize(): array { return []; } function __unserialize(array $d): void {} function serialize() {} function unserialize($d) {} }", id), 0);
    }

    #[test]
    fn corpus_false_positives_stay_fixed() {
        let hooks = "<?php class U { public string $name { get => $this->name; } public function __construct(public int $n { set => $value; }) {} }\ninterface I { public string $title { get; } }";
        assert!(!rule_ids(hooks).iter().any(|r| r == "removed.curly_offset"));
        assert_eq!(fix_legacy_syntax(hooks), None);

        let keys =
            "<?php const key = 'k'; define('lower', 1); use const Foo\\bar; echo $a[__line__], $a[key], $a[lower], $a[bar], $a[name];";
        assert_eq!(fix_legacy_syntax(keys).unwrap().0, keys.replace("$a[name]", "$a['name']"));
        assert_eq!(fix_legacy_syntax("<?php echo $a{id};").unwrap().0, "<?php echo $a['id'];");

        assert!(rule_ids("<?php function f(#[Attr] (A&B)|null $x = null) { return mysql_query($x); }").iter().eq(["removed.mysql"].iter()));
        assert_eq!(
            count85("<?php $x = $a ? 1 : 2 and $b ? 3 : 4; switch ($x) { case $a ? 1 : 2: return $b ? 3 : 4; }", "removed.nested_ternary"),
            0
        );
        assert!(rule_ids("<?php namespace Foo; function __autoload($c) {}").is_empty());
        assert_eq!(count85("<?php mb_stripos($ct, 'html'); mb_strlen($s, 'BASE64');", "deprecated.mb_encoding"), 1);
        assert!(at("<?php $r = $res->fetch_assoc(MYSQLI_ASSOC); $redis->ping(); $user->refresh();", (8, 5))
            .iter()
            .all(|(r, _)| r != "deprecated.mysqli_method"));
        assert_eq!(count85("<?php function f(mysqli $db) { $db->ping(); }", "deprecated.mysqli_method"), 1);
        assert_eq!(
            count85("<?php [$body, $http_response_header] = fetch($u); echo $http_response_header[0];", "deprecated.http_response_header"),
            0
        );
        assert_eq!(
            count85(
                "<?php list($body, $http_response_header) = fetch($u); echo $http_response_header[0];",
                "deprecated.http_response_header"
            ),
            0
        );
        assert_eq!(count85("<?php abstract class N implements \\Serializable {}", "deprecated.serializable"), 0);
        assert_eq!(count85("<?php namespace App; interface Countable { function count(); } class C implements Countable { function count() { return 1; } }", "deprecated.tentative_return_type"), 0);
        assert_eq!(
            count85(
                "<?php namespace App; class C implements \\Countable { function count() { return 1; } }",
                "deprecated.tentative_return_type"
            ),
            1
        );
        assert_eq!(
            count85(
                "<?php enum E implements JsonSerializable { case A; public function jsonSerialize() { return 1; } }",
                "deprecated.tentative_return_type"
            ),
            1
        );
        assert_eq!(count85("<?php fgetcsv($f, length: 0, separator: ','); fgetcsv($f, escape: '');", "deprecated.csv_escape"), 1);
        assert!(rule_ids("<?php const REAL = 1.0; $x = (\nREAL\n);").iter().all(|r| r != "removed.real_cast"));
        assert_eq!(count85("<?php new class(function () {}) { function __autoload($c) {} }; each($x);", "removed.each"), 1);
    }

    #[test]
    fn many_functions_stay_linear() {
        let src = |n: usize| {
            let mut src = String::from("<?php\n");
            for k in 0..n {
                src.push_str(&format!("function f{k}($a) {{ return $a; }}\n"));
            }
            src
        };
        let (small, large) = (fastest_run(&src(4_000), "8.5"), fastest_run(&src(16_000), "8.5"));
        assert!(large < small * 10.0, "4k: {small:.3}s, 16k: {large:.3}s");
    }

    fn fastest_run(src: &str, target: &str) -> f64 {
        analyze_source("big.php", src, target.parse().unwrap());
        (0..5)
            .map(|_| {
                let started = Instant::now();
                analyze_source("big.php", src, target.parse().unwrap());
                started.elapsed().as_secs_f64()
            })
            .fold(f64::INFINITY, f64::min)
    }

    #[test]
    fn removed_functions_passed_as_strings() {
        let src = "<?php\n$a = array_map('utf8_encode', $rows);\nusort($list, 'strcmp');\ncall_user_func(\"each\", $arr);\n$f = Closure::fromCallable('create_function');\narray_filter($x, 'is_real');\nfunction legacy_helper() {}\narray_map('legacy_helper', $x);\narray_map(\"pre_$name\", $x);\n$s = 'each';";
        let found = at(src, (8, 5));
        let titles: Vec<&str> = found.iter().map(|(_, t)| t.as_str()).collect();
        assert_eq!(titles, ["'utf8_encode'", "'each'", "'create_function'", "'is_real'"]);
        let fixes: Vec<_> = analyze_source("a.php", src, PhpVersion::new(8, 5)).into_iter().map(|f| f.fix).collect();
        assert!(fixes.iter().all(|f| *f == FixKind::Manual), "Rector doesn't rewrite strings");
        assert!(at("<?php function each($a) {} array_map('each', $x);", (8, 5)).is_empty());
    }

    #[test]
    fn dollar_brace_inside_backticks() {
        let ids: Vec<String> =
            analyze_source("a.php", "<?php `ls ${dir}`;", PhpVersion::new(8, 5)).into_iter().map(|f| f.rule_id).collect();
        assert!(
            ids.contains(&"deprecated.backtick".to_string()) && ids.contains(&"deprecated.dollar_brace_interpolation".to_string()),
            "{ids:?}"
        );
    }

    #[test]
    fn relative_path_survives_different_spellings_of_the_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.php"), "<?php").unwrap();
        let canonical = dir.path().canonicalize().unwrap();
        assert_eq!(relative_path(dir.path(), &canonical.join("src/a.php")), "src/a.php");
        assert_eq!(relative_path(&canonical, &dir.path().join("src").join("a.php")), "src/a.php");
        assert_eq!(relative_path(Path::new("/elsewhere"), Path::new("/x/y.php")), "/x/y.php");
        let name = dir.path().file_name().unwrap();
        let detour = dir.path().join("..").join(name).join("src/a.php");
        assert_eq!(relative_path(dir.path(), &detour), "src/a.php");
    }

    fn rules(src: &str, target: &str) -> Vec<String> {
        analyze_source("t.php", src, target.parse().unwrap()).into_iter().map(|f| f.rule_id).collect()
    }

    fn lines_of(src: &str, target: &str, rule: &str) -> Vec<usize> {
        analyze_source("t.php", src, target.parse().unwrap()).into_iter().filter(|f| f.rule_id == rule).map(|f| f.line).collect()
    }

    #[test]
    fn optional_parameters_before_required_ones() {
        let src = "<?php
function a($x = 1, $y) {}
function b(Foo $x = null, $y) {}
function c($x, $y = 1) {}
function d($x = 1, ...$rest) {}
class K { public function m(array $x = [], $y, $z = 2, $w) {} }
class P { public function __construct(public int $x = 1, $y) {} }
$f = function ($x = null, $y) {};
function e(public $x = null, $y) {}";
        let rule = "deprecated.optional_before_required";
        assert_eq!(lines_of(src, "8.0", rule), vec![2, 6, 7, 8, 9]);
        assert!(lines_of(src, "7.4", rule).is_empty());
        let fixes: Vec<FixKind> =
            analyze_source("t.php", src, PhpVersion::new(8, 3)).into_iter().filter(|f| f.rule_id == rule).map(|f| f.fix).collect();
        assert!(fixes.iter().all(|f| *f == FixKind::Likely));
    }

    #[test]
    fn null_passed_to_built_in_string_parameters() {
        let src = "<?php
strlen(null);
str_replace('a', 'b', null);
str_replace(null, 'b', $s);
trim($s ?? null);
htmlspecialchars(string: null);
\\strpos('abc', NULL);
$obj->strlen(null);";
        let rule = "deprecated.null_to_internal";
        assert_eq!(lines_of(src, "8.1", rule), vec![2, 3, 7]);
        assert!(lines_of(src, "8.0", rule).is_empty());
        assert!(lines_of("<?php namespace App; function strlen($s) {} strlen(null);", "8.1", rule).is_empty());
    }

    #[test]
    fn partially_supported_callables() {
        let src = "<?php
call_user_func('self::create');
call_user_func(['parent', 'boot']);
array_map(\"static::wrap\", $items);
is_callable(array('SELF', 'check'));
call_user_func([self::class, 'create']);
call_user_func('Foo::create');
call_user_func(['self', $name]);
$x = 'self::create';";
        let rule = "deprecated.partial_callable";
        assert_eq!(lines_of(src, "8.2", rule), vec![2, 3, 4, 5]);
        assert!(lines_of(src, "8.1", rule).is_empty());
    }

    #[test]
    fn detects_removed_functions() {
        let src = "<?php\n$c = mysql_connect('h');\nwhile (list($k, $v) = each($a)) {}\n$f = create_function('$a', 'return 1;');";
        let found = rules(src, "8.3");
        assert_eq!(found, vec!["removed.mysql", "removed.each", "removed.create_function"]);
    }

    #[test]
    fn respects_target_version() {
        let src = "<?php utf8_encode($s); lcg_value();";
        assert_eq!(rules(src, "8.1"), Vec::<String>::new());
        assert_eq!(rules(src, "8.2"), vec!["deprecated.utf8_encode"]);
        assert_eq!(rules(src, "8.4"), vec!["deprecated.utf8_encode", "deprecated.lcg_value"]);
    }

    #[test]
    fn ignores_methods_strings_comments_and_namespaces() {
        let src = "<?php\n$o->each();\nFoo::split();\n// mysql_query()\n$s = 'ereg(';\nApp\\each($x);\nfunction each() {}\nnew Split();";
        assert!(rules(src, "8.5").is_empty(), "{:?}", rules(src, "8.5"));
    }

    #[test]
    fn root_qualified_calls_are_detected() {
        assert_eq!(rules("<?php \\mysql_query($q);", "8.0"), vec!["removed.mysql"]);
    }

    #[test]
    fn positions_and_snippet() {
        let f = &analyze_source("a/b.php", "<?php\n\n    $x = each($arr);\n", "8.0".parse().unwrap())[0];
        assert_eq!((f.line, f.column), (3, 10));
        assert_eq!(f.snippet, "$x = each($arr);");
        assert_eq!(f.id, "a/b.php:3:10:removed.each");
        assert_eq!(f.severity, Severity::Breaking);
    }

    #[test]
    fn syntax_patterns() {
        let src = "<?php\n$c = $str{0};\n$f = (real) $x;\n$n = (unset) $x;\n$b = (boolean) $x;\n$o = `ls`;\n$p = foo(REAL);";
        assert_eq!(
            rules(src, "8.5"),
            vec!["removed.curly_offset", "removed.real_cast", "removed.unset_cast", "deprecated.non_canonical_cast", "deprecated.backtick"]
        );
    }

    #[test]
    fn rewrites_curly_offsets() {
        let src = "<?php\n$a = $s{0} . $m{$i}{f($x{1})};\nif ($b){ echo \"{$c}\"; }\n";
        assert_eq!(fix_curly_offsets(src).unwrap(), "<?php\n$a = $s[0] . $m[$i][f($x[1])];\nif ($b){ echo \"{$c}\"; }\n");
        assert_eq!(fix_curly_offsets("<?php $a[0]; ${'x'};"), None);
    }

    #[test]
    fn dollar_brace_interpolation() {
        let src = "<?php\n$a = \"Hi ${name}\";\n$b = \"ok {$name}\";\n$c = \"esc \\${x}\";\n$d = 'no ${x}';";
        let f = analyze_source("t.php", src, "8.2".parse().unwrap());
        assert_eq!(f.len(), 1);
        assert_eq!((f[0].line, f[0].column), (2, 10));
    }

    #[test]
    fn implicit_nullable_parameters() {
        let src = "<?php\nfunction a(Foo $x = null, ?Bar $y = null, int|null $z = null, $w = null, string $s = 'x') {}\nclass C { public function __construct(private Baz $b = null, #[A] mixed $m = null) {} }\n$f = fn(array $a = null) => 1;";
        let f = analyze_source("t.php", src, "8.4".parse().unwrap());
        let ids: Vec<_> = f.iter().map(|f| (f.rule_id.as_str(), f.line)).collect();
        assert_eq!(
            ids,
            vec![("deprecated.implicit_nullable", 2), ("deprecated.implicit_nullable", 3), ("deprecated.implicit_nullable", 4)]
        );
        assert!(rules(src, "8.3").is_empty());
    }

    #[test]
    fn get_class_without_args() {
        assert_eq!(rules("<?php get_class(); get_class($x);", "8.3"), vec!["deprecated.get_class_no_args"]);
    }

    #[test]
    fn autoload_function_but_not_method() {
        assert_eq!(rules("<?php function __autoload($c) {}", "8.0"), vec!["removed.__autoload"]);
        assert!(rules("<?php class A { function __autoload($c) {} }", "8.0").is_empty());
    }

    #[test]
    fn constants() {
        let src = "<?php\nfilter_var($s, FILTER_SANITIZE_STRING);\nerror_reporting(E_ALL & ~E_STRICT);\ntrigger_error('x', E_USER_ERROR);\nif ($n === E_USER_ERROR) {}";
        assert_eq!(rules(src, "8.4"), vec!["deprecated.filter_sanitize_string", "deprecated.e_strict", "deprecated.e_user_error"]);
    }

    #[test]
    fn casts_after_keywords() {
        let src = "<?php\nfunction f($x) { return (real) $x; }\necho (double) $y;\nprint (integer) $z;\n$g = yield (boolean) $w;\nif (REAL) {}\nfoo(REAL);";
        assert_eq!(
            rules(src, "8.5"),
            vec!["removed.real_cast", "deprecated.non_canonical_cast", "deprecated.non_canonical_cast", "deprecated.non_canonical_cast"]
        );
    }

    #[test]
    fn by_reference_function_declarations() {
        assert!(rules("<?php function &each($a) {} function &split() {} $f = fn &($x) => $x;", "8.0").is_empty());
        assert_eq!(rules("<?php function &__autoload($c) {}", "8.0"), vec!["removed.__autoload"]);
        assert_eq!(rules("<?php $a = &each($b);", "8.0"), vec!["removed.each"]);
    }

    #[test]
    fn brace_dollar_brace_is_not_deprecated() {
        let f = analyze_source("t.php", "<?php\n$a = \"{${expr}} ${x}\";\n$b = \"x\n  ${y}\";", "8.2".parse().unwrap());
        let at: Vec<_> = f.iter().map(|f| (f.line, f.column)).collect();
        assert_eq!(at, vec![(2, 17), (4, 3)]);
    }

    #[test]
    fn nested_brackets_in_parameter_attributes() {
        assert!(rules("<?php function f(#[A([1])] $x = null, #[B(['k' => [2]])] ?int $y = null) {}", "8.4").is_empty());
        assert_eq!(rules("<?php function f(#[A([1])] Foo $x = null) {}", "8.4"), vec!["deprecated.implicit_nullable"]);
    }

    #[test]
    fn root_qualified_trigger_error() {
        assert_eq!(rules("<?php \\trigger_error('x', E_USER_ERROR);", "8.4"), vec!["deprecated.e_user_error"]);
    }

    #[test]
    fn attribute_names_are_not_calls() {
        assert!(rules("<?php #[Split()] class A {} #[Each, Ereg(E_STRICT)] function f(#[Split] $x) {}", "8.5").is_empty());
        assert_eq!(rules("<?php #[A([1])] function f() { split(); }", "8.0"), vec!["removed.split"]);
    }

    #[test]
    fn hash_bracket_comment_in_php7_code() {
        let src = "<?php\n#[note] don't use this\nmysql_query($q);\n$x = 'a';";
        assert_eq!(rules(src, "7.4"), vec!["removed.mysql"]);
        let f = analyze_source("t.php", src, "8.0".parse().unwrap());
        let ids: Vec<_> = f.iter().map(|f| (f.rule_id.as_str(), f.line, f.column)).collect();
        assert_eq!(ids, vec![("removed.hash_bracket_comment", 2, 1), ("removed.mysql", 3, 1)]);
        assert_eq!(f[0].snippet, "#[note] don't use this");
    }

    #[test]
    fn bom_does_not_shift_columns() {
        let f = &analyze_source("t.php", "\u{FEFF}<?php each($a);", "8.0".parse().unwrap())[0];
        assert_eq!((f.line, f.column), (1, 7));
        assert_eq!(f.snippet, "<?php each($a);");
    }

    #[test]
    fn methods_in_classes_with_long_headers() {
        let src = "<?php\nfinal class Store extends Base implements \\A\\One, \\B\\Two, \\C\\Three, \\D\\Four, \\E\\Five {\n  function __autoload($c) {}\n}";
        assert!(rules(src, "8.0").is_empty(), "{:?}", rules(src, "8.0"));
        assert_eq!(rules("<?php if ($x === Foo::class) { function __autoload($c) {} }", "8.0"), vec!["removed.__autoload"]);
    }

    #[test]
    fn long_lines_stay_fast_and_snippets_are_cut() {
        let line = |n: usize| format!("<?php {}", "each($a); ".repeat(n));
        let src = line(40_000);
        let (small, large) = (fastest_run(&line(10_000), "8.0"), fastest_run(&src, "8.0"));
        assert!(large < small * 10.0, "10k: {small:.3}s, 40k: {large:.3}s");
        let f = analyze_source("t.php", &src, "8.0".parse().unwrap());
        assert_eq!(f.len(), 40_000);
        assert_eq!(f[0].snippet.chars().count(), 201);
        assert_eq!(f[39_999].column, 6 + 39_999 * 10 + 1);
    }

    #[test]
    fn oversized_files_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let big = dir.path().join("big.php");
        std::fs::write(&big, vec![b' '; crate::project::MAX_FILE_BYTES as usize + 1]).unwrap();
        let result = analyze_files(dir.path(), &[big], "8.0".parse().unwrap(), |_, _, _| {});
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].file, "big.php");
        assert!(result.errors[0].message.contains("2 MB"));
    }

    #[test]
    fn review_patterns() {
        let src = "<?php eval($code); $$name = 1; extract($_POST);";
        assert_eq!(rules(src, "8.1"), vec!["review.eval", "review.variable_variable", "review.extract"]);
    }
}
