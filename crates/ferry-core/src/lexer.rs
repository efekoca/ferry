#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Ident,
    Variable,
    Str { interpolates: bool, backtick: bool },
    Number,
    Punct,
    CloseTag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn is_punct(&self, p: &str) -> bool {
        self.kind == TokenKind::Punct && self.text == p
    }

    pub fn is_ident_ci(&self, name: &str) -> bool {
        self.kind == TokenKind::Ident && self.text.eq_ignore_ascii_case(name)
    }
}

const MULTI_PUNCT: [&str; 22] = [
    "<=>", "===", "!==", "??=", "?->", "...", "**=", "<<=", ">>=", "->", "::", "=>", "==", "!=",
    "<>", "<=", ">=", "&&", "||", "??", "++", "--",
];

pub struct Lexed {
    pub tokens: Vec<Token>,
    pub stray_hash_brackets: Vec<(usize, usize)>,
}

pub fn tokenize(src: &str) -> Vec<Token> {
    lex(src).tokens
}

pub fn lex(src: &str) -> Lexed {
    Lexer::new(src).run()
}

struct Lexer<'a> {
    src: &'a str,
    b: &'a [u8],
    pos: usize,
    line_starts: Vec<usize>,
    cursor: (usize, usize, usize),
    tokens: Vec<Token>,
    stray_hash_brackets: Vec<(usize, usize)>,
}

fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_' || c >= 0x80
}

fn is_ident_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c >= 0x80
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        let mut line_starts = vec![0];
        for (i, c) in src.bytes().enumerate() {
            if c == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self {
            src,
            b: src.as_bytes(),
            pos: 0,
            line_starts,
            cursor: (0, 0, 1),
            tokens: Vec::new(),
            stray_hash_brackets: Vec::new(),
        }
    }

    fn line_col(&mut self, offset: usize) -> (usize, usize) {
        let (mut line, mut from, mut col) = self.cursor;
        let next_line = self.line_starts.get(line + 1).copied().unwrap_or(usize::MAX);
        if offset < from || offset >= next_line {
            line = match self.line_starts.binary_search(&offset) {
                Ok(i) => i,
                Err(i) => i - 1,
            };
            from = self.line_starts[line];
            col = 1;
        }
        col += self.src[from..offset].chars().count();
        self.cursor = (line, offset, col);
        let bom = line == 0 && offset > 0 && self.src.starts_with('\u{FEFF}');
        (line + 1, col - usize::from(bom))
    }

    fn push(&mut self, kind: TokenKind, start: usize, end: usize, text: String) {
        let (line, column) = self.line_col(start);
        self.tokens.push(Token {
            kind,
            text,
            start,
            end,
            line,
            column,
        });
    }

    fn peek(&self, offset: usize) -> Option<u8> {
        self.b.get(self.pos + offset).copied()
    }

    fn starts_with(&self, s: &str) -> bool {
        self.b[self.pos..].starts_with(s.as_bytes())
    }

    fn starts_with_ci(&self, s: &str) -> bool {
        let end = self.pos + s.len();
        end <= self.b.len() && self.b[self.pos..end].eq_ignore_ascii_case(s.as_bytes())
    }

    fn run(mut self) -> Lexed {
        self.skip_inline_html();
        while self.pos < self.b.len() {
            self.lex_php();
            if self.halted() {
                self.pos = self.b.len();
            }
        }
        Lexed {
            tokens: self.tokens,
            stray_hash_brackets: self.stray_hash_brackets,
        }
    }

    fn halted(&self) -> bool {
        let t = &self.tokens;
        let n = t.len();
        n >= 4
            && t[n - 4].is_ident_ci("__halt_compiler")
            && t[n - 3].is_punct("(")
            && t[n - 2].is_punct(")")
            && (t[n - 1].is_punct(";") || t[n - 1].kind == TokenKind::CloseTag)
    }

    fn skip_inline_html(&mut self) {
        while self.pos < self.b.len() {
            if self.starts_with_ci("<?php") {
                self.pos += 5;
                return;
            }
            if self.starts_with("<?=") {
                self.pos += 3;
                return;
            }
            if self.starts_with("<?")
                && self.peek(2).is_none_or(|c| c.is_ascii_whitespace())
            {
                self.pos += 2;
                return;
            }
            self.pos += 1;
        }
    }

    fn lex_php(&mut self) {
        let c = self.b[self.pos];
        let start = self.pos;

        if c.is_ascii_whitespace() {
            self.pos += 1;
            return;
        }
        if self.starts_with("?>") {
            self.pos += 2;
            self.push(TokenKind::CloseTag, start, self.pos, "?>".into());
            self.skip_inline_html();
            return;
        }
        if self.starts_with("#[") {
            if is_attribute(self.b, self.pos) {
                self.pos += 2;
                self.push(TokenKind::Punct, start, self.pos, "#[".into());
                return;
            }
            let at = self.line_col(start);
            self.stray_hash_brackets.push(at);
        }
        if c == b'#' || self.starts_with("//") {
            self.skip_line_comment();
            return;
        }
        if self.starts_with("/*") {
            match self.src[self.pos + 2..].find("*/") {
                Some(i) => self.pos += 2 + i + 2,
                None => self.pos = self.b.len(),
            }
            return;
        }
        if c == b'\'' {
            self.lex_quoted(false, false);
            return;
        }
        if c == b'"' {
            self.lex_quoted(true, false);
            return;
        }
        if c == b'`' {
            self.lex_quoted(true, true);
            return;
        }
        if self.starts_with("<<<") && self.lex_heredoc() {
            return;
        }
        if c == b'$' && self.peek(1).is_some_and(is_ident_start) {
            self.pos += 1;
            let name_start = self.pos;
            while self.pos < self.b.len() && is_ident_char(self.b[self.pos]) {
                self.pos += 1;
            }
            let name = self.src[name_start..self.pos].to_string();
            self.push(TokenKind::Variable, start, self.pos, name);
            return;
        }
        if is_ident_start(c) || (c == b'\\' && self.peek(1).is_some_and(is_ident_start)) {
            self.pos += 1;
            while self.pos < self.b.len() {
                let ch = self.b[self.pos];
                if is_ident_char(ch)
                    || (ch == b'\\' && self.peek(1).is_some_and(is_ident_start))
                {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            let text = self.src[start..self.pos].to_string();
            self.push(TokenKind::Ident, start, self.pos, text);
            return;
        }
        if c.is_ascii_digit() || (c == b'.' && self.peek(1).is_some_and(|d| d.is_ascii_digit())) {
            self.pos += 1;
            while self.pos < self.b.len() {
                let ch = self.b[self.pos];
                let exponent_sign = (ch == b'+' || ch == b'-')
                    && matches!(self.b[self.pos - 1], b'e' | b'E')
                    && !self.src[start..self.pos].starts_with("0x");
                if ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'.' || exponent_sign {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            let text = self.src[start..self.pos].to_string();
            self.push(TokenKind::Number, start, self.pos, text);
            return;
        }
        for p in MULTI_PUNCT {
            if self.starts_with(p) {
                self.pos += p.len();
                self.push(TokenKind::Punct, start, self.pos, p.to_string());
                return;
            }
        }
        let ch_len = self.src[self.pos..].chars().next().map_or(1, char::len_utf8);
        self.pos += ch_len;
        let text = self.src[start..self.pos].to_string();
        self.push(TokenKind::Punct, start, self.pos, text);
    }

    fn skip_line_comment(&mut self) {
        while self.pos < self.b.len() {
            if self.b[self.pos] == b'\n' {
                return;
            }
            if self.starts_with("?>") {
                return;
            }
            self.pos += 1;
        }
    }

    fn lex_quoted(&mut self, interpolates: bool, backtick: bool) {
        let start = self.pos;
        self.pos = skip_quoted(self.b, self.pos);
        let text = self.src[start..self.pos].to_string();
        self.push(
            TokenKind::Str {
                interpolates,
                backtick,
            },
            start,
            self.pos,
            text,
        );
    }

    fn lex_heredoc(&mut self) -> bool {
        let start = self.pos;
        let mut p = self.pos + 3;
        while p < self.b.len() && (self.b[p] == b' ' || self.b[p] == b'\t') {
            p += 1;
        }
        let quote = match self.b.get(p) {
            Some(b'\'') => Some(b'\''),
            Some(b'"') => Some(b'"'),
            _ => None,
        };
        if quote.is_some() {
            p += 1;
        }
        let id_start = p;
        while p < self.b.len() && is_ident_char(self.b[p]) {
            p += 1;
        }
        if p == id_start || !is_ident_start(self.b[id_start]) {
            return false;
        }
        let id = &self.src[id_start..p];
        if let Some(q) = quote {
            if self.b.get(p) != Some(&q) {
                return false;
            }
            p += 1;
        }
        let body_start = match self.src[p..].find('\n') {
            Some(i) => p + i + 1,
            None => return false,
        };
        let mut line_start = body_start;
        let mut end = self.b.len();
        while line_start < self.b.len() {
            let line_end = self.src[line_start..]
                .find('\n')
                .map_or(self.b.len(), |i| line_start + i);
            let line = &self.src[line_start..line_end];
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix(id) {
                if !rest.bytes().next().is_some_and(is_ident_char) {
                    end = line_start + (line.len() - trimmed.len()) + id.len();
                    break;
                }
            }
            line_start = line_end + 1;
        }
        self.pos = end;
        let text = self.src[start..end].to_string();
        self.push(
            TokenKind::Str {
                interpolates: quote != Some(b'\''),
                backtick: false,
            },
            start,
            end,
            text,
        );
        true
    }
}

fn skip_quoted(b: &[u8], mut p: usize) -> usize {
    let quote = b[p];
    p += 1;
    while p < b.len() {
        let ch = b[p];
        if ch == b'\\' {
            p += 2;
            continue;
        }
        if quote != b'\'' && ch == b'{' && b.get(p + 1) == Some(&b'$') && b.get(p + 2).is_some_and(|&c| c == b'$' || is_ident_start(c)) {
            p = skip_code(b, p + 1);
            continue;
        }
        if quote != b'\'' && ch == b'$' && b.get(p + 1) == Some(&b'{') {
            p = skip_code(b, p + 2);
            continue;
        }
        p += 1;
        if ch == quote {
            break;
        }
    }
    p.min(b.len())
}

fn skip_code(b: &[u8], mut p: usize) -> usize {
    let mut depth = 1;
    while p < b.len() {
        match b[p] {
            b'\'' | b'"' => {
                p = skip_quoted(b, p);
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return p + 1;
                }
            }
            _ => {}
        }
        p += 1;
    }
    b.len()
}

fn skip_space(b: &[u8], mut p: usize) -> usize {
    while p < b.len() {
        if b[p].is_ascii_whitespace() {
            p += 1;
        } else if b[p..].starts_with(b"/*") {
            p = b[p + 2..].windows(2).position(|w| w == b"*/").map_or(b.len(), |i| p + 2 + i + 2);
        } else if b[p..].starts_with(b"//") || (b[p] == b'#' && b.get(p + 1) != Some(&b'[')) {
            p = b[p..].iter().position(|&c| c == b'\n').map_or(b.len(), |i| p + i);
        } else {
            break;
        }
    }
    p
}

fn skip_name(b: &[u8], mut p: usize) -> Option<usize> {
    let start = p;
    while p < b.len() && (is_ident_char(b[p]) || (b[p] == b'\\' && b.get(p + 1).is_some_and(|&c| is_ident_start(c)))) {
        p += 1;
    }
    let first = if b[start] == b'\\' { b.get(start + 1) } else { b.get(start) };
    (p > start && first.is_some_and(|&c| is_ident_start(c))).then_some(p)
}

const ATTRIBUTE_TARGETS: [&str; 17] = [
    "function", "fn", "class", "interface", "trait", "enum", "abstract", "final", "readonly",
    "public", "protected", "private", "static", "var", "const", "case", "new",
];

fn is_attribute(b: &[u8], at: usize) -> bool {
    let mut p = at + 2;
    loop {
        p = skip_space(b, p);
        if p >= b.len() {
            return false;
        }
        if b[p] == b']' {
            break;
        }
        let Some(end) = skip_name(b, p) else { return false };
        p = skip_space(b, end);
        if b.get(p) == Some(&b'(') {
            let mut depth = 0usize;
            let mut braces = 0usize;
            while p < b.len() {
                match b[p] {
                    b'\'' | b'"' => {
                        p = skip_quoted(b, p);
                        continue;
                    }
                    b'(' | b'[' => depth += 1,
                    b')' | b']' => {
                        depth -= 1;
                        if depth == 0 {
                            p += 1;
                            break;
                        }
                    }
                    b'{' => braces += 1,
                    b'}' => braces = braces.saturating_sub(1),
                    b';' if braces == 0 => return false,
                    _ => {}
                }
                p += 1;
            }
            p = skip_space(b, p);
        }
        match b.get(p) {
            Some(b',') => p += 1,
            Some(b']') => break,
            _ => return false,
        }
    }
    p = skip_space(b, p + 1);
    match b.get(p) {
        Some(b'$' | b'?' | b'&' | b'#') => true,
        Some(b'.') => b[p..].starts_with(b"..."),
        Some(_) => {
            let Some(end) = skip_name(b, p) else { return false };
            let word = std::str::from_utf8(&b[p..end]).unwrap_or("");
            if ATTRIBUTE_TARGETS.iter().any(|k| word.eq_ignore_ascii_case(k)) {
                return true;
            }
            let after = skip_space(b, end);
            matches!(b.get(after), Some(b'$' | b'|' | b'&' | b'.'))
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<(TokenKind, String)> {
        tokenize(src)
            .into_iter()
            .map(|t| (t.kind, t.text))
            .collect()
    }

    #[test]
    fn skips_inline_html_and_comments() {
        let toks = kinds("<html><?php // mysql_query()\n/* each() */ # split()\necho 1; ?><b>x</b>");
        let texts: Vec<_> = toks.iter().map(|t| t.1.as_str()).collect();
        assert_eq!(texts, vec!["echo", "1", ";", "?>"]);
    }

    #[test]
    fn strings_hide_code() {
        let toks = tokenize("<?php $a = 'mysql_query(1)'; $b = \"each(\\\"x\\\")\";");
        assert!(toks.iter().all(|t| t.text != "mysql_query"));
        assert_eq!(toks.iter().filter(|t| matches!(t.kind, TokenKind::Str { .. })).count(), 2);
    }

    #[test]
    fn heredoc_and_nowdoc() {
        let src = "<?php\n$a = <<<EOT\n  each() ${x}\n  EOT;\n$b = <<<'N'\nsplit()\nN;\nfoo();";
        let toks = tokenize(src);
        let strs: Vec<_> = toks
            .iter()
            .filter_map(|t| match t.kind {
                TokenKind::Str { interpolates, .. } => Some(interpolates),
                _ => None,
            })
            .collect();
        assert_eq!(strs, vec![true, false]);
        let foo = toks.iter().find(|t| t.text == "foo").unwrap();
        assert_eq!(foo.line, 8);
        assert!(toks.iter().all(|t| t.text != "each" && t.text != "split"));
    }

    #[test]
    fn variables_names_and_positions() {
        let toks = tokenize("<?php\n  $x = \\Foo\\bar($y);");
        assert_eq!(toks[0].kind, TokenKind::Variable);
        assert_eq!(toks[0].text, "x");
        assert_eq!((toks[0].line, toks[0].column), (2, 3));
        let name = toks.iter().find(|t| t.kind == TokenKind::Ident).unwrap();
        assert_eq!(name.text, "\\Foo\\bar");
    }

    #[test]
    fn multi_char_punct_and_numbers() {
        let texts: Vec<_> = kinds("<?php $a?->b ?? 1.5e-3 <=> 0x1F;")
            .into_iter()
            .map(|t| t.1)
            .collect();
        assert!(texts.contains(&"?->".to_string()));
        assert!(texts.contains(&"??".to_string()));
        assert!(texts.contains(&"1.5e-3".to_string()));
        assert!(texts.contains(&"<=>".to_string()));
        assert!(texts.contains(&"0x1F".to_string()));
    }

    #[test]
    fn attributes_are_not_comments() {
        let toks = tokenize("<?php #[Attr] function f() {}");
        assert!(toks.iter().any(|t| t.text == "Attr"));
    }

    #[test]
    fn columns_on_a_long_single_line_stay_linear() {
        let src = format!("<?php {}each($a);", "$x = 1; ".repeat(50_000));
        let started = std::time::Instant::now();
        let toks = tokenize(&src);
        assert!(started.elapsed().as_secs() < 2, "{:?}", started.elapsed());
        let each = toks.iter().find(|t| t.text == "each").unwrap();
        assert_eq!((each.line, each.column), (1, each.start + 1));
        let toks = tokenize("<?php\n$a = 'ü'; $b;\n  $c;");
        let cols: Vec<_> = toks.iter().filter(|t| t.kind == TokenKind::Variable).map(|t| (t.line, t.column)).collect();
        assert_eq!(cols, vec![(2, 1), (2, 11), (3, 3)]);
    }

    #[test]
    fn leading_bom_is_not_a_column() {
        let toks = tokenize("\u{FEFF}<?php each($a);\n$b;");
        assert_eq!((toks[0].line, toks[0].column), (1, 7));
        let b = toks.iter().find(|t| t.text == "b").unwrap();
        assert_eq!((b.line, b.column), (2, 1));
    }

    #[test]
    fn quotes_inside_interpolation_do_not_end_the_string() {
        let src = "<?php echo \"{$a[\"it's\"]}\";\nmysql_query($q); echo \"${b[\"x\"]} `{$c['\"']}`\"; split();";
        let toks = tokenize(src);
        let names: Vec<_> = toks.iter().filter(|t| t.kind == TokenKind::Ident).map(|t| t.text.as_str()).collect();
        assert_eq!(names, vec!["echo", "mysql_query", "echo", "split"]);
        let s = tokenize("<?php $s = \"a {$\"; foo();");
        assert!(s.iter().any(|t| t.text == "foo"));
    }

    #[test]
    fn hash_bracket_comments_in_php7_code() {
        let lexed = lex("<?php\n#[note] don't use this\nmysql_query($q);\n$x = 'a';\n  #[ TODO\n");
        assert!(lexed.tokens.iter().any(|t| t.text == "mysql_query"));
        assert!(lexed.tokens.iter().all(|t| !t.is_punct("#[")));
        assert_eq!(lexed.stray_hash_brackets, vec![(2, 1), (5, 3)]);

        let attrs = "<?php\n#[Attribute]\nclass A {\n  #[ORM\\Column(type: 'string', options: ['x' => \"]\"])]\n  private string $s;\n  #[\\ReturnTypeWillChange]\n  public function f(#[A([1])] ?Foo $x, #[B] int|null $y, #[C] Bar ...$z) {}\n  #[A, B(1)] #[C]\n  final const X = 1;\n}\n$f = #[Pure] static fn() => 1;\nfunction g(#[S] $x) {}\n#[\n  Route('/x', methods: ['GET'])\n]\nfunction h() {}\n#[A(static function () { return 1; })] function i() {}";
        let lexed = lex(attrs);
        assert!(lexed.stray_hash_brackets.is_empty(), "{:?}", lexed.stray_hash_brackets);
        assert_eq!(lexed.tokens.iter().filter(|t| t.is_punct("#[")).count(), 12);
    }

    #[test]
    fn never_panics_on_garbage() {
        for src in ["<?php '", "<?php \"\\", "<?php <<<", "<?php /*", "<?php $", "<?php ü€ $ç = 1;", "<?php #[", "<?php #[A(", "<?php #[A] ", "<?php \"{$a", "<?php \"${", "\u{FEFF}"] {
            let _ = tokenize(src);
        }
    }
}
