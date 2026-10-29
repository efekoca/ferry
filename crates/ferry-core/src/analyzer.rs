use crate::i18n::{Localized, Text};
use crate::lexer::{tokenize, Token, TokenKind};
use crate::rules::{patterns, FixKind, PatternRule, Severity, SymbolRule, CONSTANTS, FUNCTIONS};
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
    let lexical = path
        .strip_prefix(root)
        .ok()
        .filter(|rel| rel.components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir)));
    let rel = match lexical {
        Some(rel) => rel.to_path_buf(),
        None => match (root.canonicalize(), path.canonicalize()) {
            (Ok(root), Ok(full)) => full.strip_prefix(&root).map(Path::to_path_buf).unwrap_or_else(|_| path.to_path_buf()),
            _ => path.to_path_buf(),
        },
    };
    rel.to_string_lossy().replace('\\', "/")
}

pub fn analyze_files(
    root: &Path,
    files: &[PathBuf],
    target: PhpVersion,
    mut progress: impl FnMut(usize, usize, &str),
) -> ScanResult {
    let started = Instant::now();
    let mut findings = Vec::new();
    let mut errors = Vec::new();
    let mut lines = 0;
    for (i, path) in files.iter().enumerate() {
        let rel = relative_path(root, path);
        progress(i, files.len(), &rel);
        match std::fs::read(path) {
            Ok(bytes) => {
                let src = String::from_utf8_lossy(&bytes);
                lines += src.lines().count();
                findings.extend(analyze_source(&rel, &src, target));
            }
            Err(e) => errors.push(FileError {
                file: rel,
                message: e.to_string(),
            }),
        }
    }
    progress(files.len(), files.len(), "");
    findings.sort_by(|a, b| {
        (a.severity, &a.file, a.line, a.column).cmp(&(b.severity, &b.file, b.line, b.column))
    });
    ScanResult {
        target,
        files_scanned: files.len(),
        lines_scanned: lines,
        findings,
        errors,
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

pub fn analyze_source(file: &str, src: &str, target: PhpVersion) -> Vec<Finding> {
    let tokens = tokenize(src);
    let mut ctx = Ctx {
        file,
        src,
        target,
        out: Vec::new(),
    };
    for i in 0..tokens.len() {
        let tok = &tokens[i];
        match &tok.kind {
            TokenKind::Ident => ctx.ident(&tokens, i),
            TokenKind::Variable => {
                if let Some(next) = tokens.get(i + 1) {
                    if next.is_punct("{") && next.start == tok.end {
                        ctx.pattern(&patterns::CURLY_OFFSET, tok, None);
                    }
                }
            }
            TokenKind::Punct if tok.text == "$" => {
                if let Some(next) = tokens.get(i + 1) {
                    if next.start == tok.end
                        && (next.kind == TokenKind::Variable || next.is_punct("{"))
                    {
                        ctx.pattern(&patterns::VARIABLE_VARIABLE, tok, None);
                    }
                }
            }
            TokenKind::Str { backtick: true, .. } => ctx.pattern(&patterns::BACKTICK, tok, None),
            TokenKind::Str {
                interpolates: true, ..
            } => ctx.dollar_brace(tok),
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
    replacement: Option<Text>,
}

struct Ctx<'a> {
    file: &'a str,
    src: &'a str,
    target: PhpVersion,
    out: Vec<Finding>,
}

const DECL_KEYWORDS: [&str; 13] = [
    "function", "fn", "new", "const", "class", "interface", "trait", "enum", "use", "goto",
    "instanceof", "extends", "implements",
];

impl<'a> Ctx<'a> {
    fn applies(&self, since: PhpVersion) -> bool {
        since <= self.target
    }

    fn snippet(&self, line: usize) -> String {
        let text = self.src.lines().nth(line - 1).unwrap_or("").trim();
        if text.chars().count() > 200 {
            let cut: String = text.chars().take(200).collect();
            format!("{cut}…")
        } else {
            text.to_string()
        }
    }

    fn push(&mut self, hit: Hit<'_>, line: usize, column: usize) {
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
            replacement: hit.replacement.map(Into::into),
            snippet: self.snippet(line),
        });
    }

    fn symbol(&mut self, rule: &SymbolRule, tok: &Token, is_call: bool) {
        if !self.applies(rule.since()) {
            return;
        }
        let name = tok.text.trim_start_matches('\\');
        let title = if is_call {
            format!("{name}()")
        } else {
            name.to_string()
        };
        let hit = Hit {
            rule_id: rule.id,
            title: Localized::same(title),
            message: rule.note,
            since: rule.since(),
            severity: rule.severity(),
            fix: rule.fix,
            replacement: rule.replacement,
        };
        self.push(hit, tok.line, tok.column);
    }

    fn pattern(&mut self, rule: &PatternRule, tok: &Token, at: Option<(usize, usize)>) {
        if !self.applies(rule.since()) {
            return;
        }
        let (line, column) = at.unwrap_or((tok.line, tok.column));
        let hit = Hit {
            rule_id: rule.id,
            title: rule.title.into(),
            message: rule.note,
            since: rule.since(),
            severity: rule.severity,
            fix: rule.fix,
            replacement: rule.replacement,
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

        let is_member = prev.is_some_and(|p| p.is_punct("->") || p.is_punct("?->") || p.is_punct("::"));
        let after_decl = prev.is_some_and(|p| {
            p.kind == TokenKind::Ident && DECL_KEYWORDS.iter().any(|k| p.text.eq_ignore_ascii_case(k))
        });

        if lower == "function" || lower == "fn" {
            self.function_params(tokens, i);
        }

        if prev.is_some_and(|p| p.is_punct("(")) && next.is_some_and(|n| n.is_punct(")")) {
            let before_paren = i.checked_sub(2).map(|p| &tokens[p]);
            let paren_is_call = before_paren.is_some_and(|b| {
                matches!(b.kind, TokenKind::Ident | TokenKind::Variable)
                    || b.is_punct(")")
                    || b.is_punct("]")
            });
            if !paren_is_call {
                let open = &tokens[i - 1];
                match lower.as_str() {
                    "real" => return self.pattern(&patterns::REAL_CAST, open, None),
                    "unset" => return self.pattern(&patterns::UNSET_CAST, open, None),
                    "boolean" | "integer" | "double" | "binary" => {
                        return self.pattern(&patterns::NON_CANONICAL_CAST, open, None)
                    }
                    _ => {}
                }
            }
        }

        if is_member {
            return;
        }

        if after_decl {
            let is_function_decl = prev.is_some_and(|p| p.is_ident_ci("function"));
            if is_function_decl && lower == "__autoload" && !is_method(tokens, i - 1) {
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
            if let Some(rule) = FUNCTIONS.iter().find(|r| r.matches(&lower)) {
                self.symbol(rule, tok, true);
            }
            return;
        }

        if bare.bytes().any(|b| b.is_ascii_lowercase()) {
            return;
        }
        if let Some(rule) = CONSTANTS.iter().find(|r| r.name == lower) {
            if rule.id == "deprecated.e_user_error" && !inside_call(tokens, i, "trigger_error") {
                return;
            }
            self.symbol(rule, tok, false);
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
                    if t.is_punct("]") {
                        attr_depth -= 1;
                    }
                    continue;
                }
                let modifier = t.kind == TokenKind::Ident
                    && ["public", "protected", "private", "readonly"]
                        .iter()
                        .any(|m| t.text.eq_ignore_ascii_case(m));
                if modifier || t.is_punct("&") || t.is_punct("...") {
                    continue;
                }
                type_tokens.push(*t);
            }
            if type_tokens.is_empty() {
                continue;
            }
            let already_nullable = type_tokens
                .iter()
                .any(|t| t.is_punct("?") || t.is_ident_ci("null") || t.is_ident_ci("mixed"));
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
        let mut k = 0;
        while k + 1 < bytes.len() {
            if bytes[k] == b'\\' {
                k += 2;
                continue;
            }
            if bytes[k] == b'$' && bytes[k + 1] == b'{' {
                let offset = tok.start + k;
                let line = tok.line + tok.text[..k].matches('\n').count();
                let line_start = self.src[..offset].rfind('\n').map_or(0, |p| p + 1);
                let column = self.src[line_start..offset].chars().count() + 1;
                self.pattern(&patterns::DOLLAR_BRACE_INTERPOLATION, tok, Some((line, column)));
            }
            k += 1;
        }
    }
}

fn is_method(tokens: &[Token], fn_idx: usize) -> bool {
    let modifiers = ["public", "protected", "private", "static", "abstract", "final"];
    if fn_idx > 0 {
        let prev = &tokens[fn_idx - 1];
        if modifiers.iter().any(|m| prev.is_ident_ci(m)) {
            return true;
        }
    }
    let mut depth: isize = 0;
    for k in (0..fn_idx).rev() {
        let t = &tokens[k];
        if t.is_punct("}") {
            depth += 1;
        } else if t.is_punct("{") {
            if depth == 0 {
                return tokens[..k].iter().rev().take(12).any(|b| {
                    ["class", "trait", "interface", "enum"].iter().any(|c| b.is_ident_ci(c))
                });
            }
            depth -= 1;
        }
    }
    false
}

fn inside_call(tokens: &[Token], i: usize, func: &str) -> bool {
    let mut depth: isize = 0;
    for k in (0..i).rev() {
        let t = &tokens[k];
        if t.is_punct(";") || t.is_punct("{") || t.is_punct("}") {
            return false;
        }
        if t.is_punct(")") {
            depth += 1;
        } else if t.is_punct("(") {
            if depth == 0 {
                return k > 0 && tokens[k - 1].is_ident_ci(func);
            }
            depth -= 1;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

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
        analyze_source("t.php", src, target.parse().unwrap())
            .into_iter()
            .map(|f| f.rule_id)
            .collect()
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
            vec![
                "removed.curly_offset",
                "removed.real_cast",
                "removed.unset_cast",
                "deprecated.non_canonical_cast",
                "deprecated.backtick"
            ]
        );
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
            vec![
                ("deprecated.implicit_nullable", 2),
                ("deprecated.implicit_nullable", 3),
                ("deprecated.implicit_nullable", 4)
            ]
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
        assert_eq!(
            rules(src, "8.4"),
            vec!["deprecated.filter_sanitize_string", "deprecated.e_strict", "deprecated.e_user_error"]
        );
    }

    #[test]
    fn review_patterns() {
        let src = "<?php eval($code); $$name = 1; extract($_POST);";
        assert_eq!(rules(src, "8.1"), vec!["review.eval", "review.variable_variable", "review.extract"]);
    }
}
