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

pub fn tokenize(src: &str) -> Vec<Token> {
    Lexer::new(src).run()
}

struct Lexer<'a> {
    src: &'a str,
    b: &'a [u8],
    pos: usize,
    line_starts: Vec<usize>,
    tokens: Vec<Token>,
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
            tokens: Vec::new(),
        }
    }

    fn line_col(&self, offset: usize) -> (usize, usize) {
        let line = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let col = self.src[self.line_starts[line]..offset].chars().count() + 1;
        (line + 1, col)
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

    fn run(mut self) -> Vec<Token> {
        self.skip_inline_html();
        while self.pos < self.b.len() {
            self.lex_php();
        }
        self.tokens
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
            self.pos += 2;
            self.push(TokenKind::Punct, start, self.pos, "#[".into());
            return;
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
            self.lex_quoted(b'\'', false, false);
            return;
        }
        if c == b'"' {
            self.lex_quoted(b'"', true, false);
            return;
        }
        if c == b'`' {
            self.lex_quoted(b'`', true, true);
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

    fn lex_quoted(&mut self, quote: u8, interpolates: bool, backtick: bool) {
        let start = self.pos;
        self.pos += 1;
        while self.pos < self.b.len() {
            let ch = self.b[self.pos];
            if ch == b'\\' {
                self.pos += 2;
                continue;
            }
            self.pos += 1;
            if ch == quote {
                break;
            }
        }
        self.pos = self.pos.min(self.b.len());
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
    fn never_panics_on_garbage() {
        for src in ["<?php '", "<?php \"\\", "<?php <<<", "<?php /*", "<?php $", "<?php ü€ $ç = 1;"] {
            let _ = tokenize(src);
        }
    }
}
