//! Hand-written ToastTTA assembler lexer.

use crate::diag::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Ident(String),
    Number(i64),       // signed wide enough to hold any literal we'll emit
    Char(u16),         // already-resolved code point (low byte = ASCII)
    KwEqu,             // .equ
    Hash,              // #
    Arrow,             // ->
    Semi,              // ;
    Colon,             // :
    LBracket,          // [
    RBracket,          // ]
    Bang,              // !
    Eq,                // =
    Newline,           // significant
    Eof,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_kind_equality() {
        assert_eq!(TokenKind::Hash, TokenKind::Hash);
        assert_ne!(TokenKind::Hash, TokenKind::Bang);
    }
}

pub fn lex(source: &str, filename: &str) -> Result<Vec<Token>, crate::diag::Diagnostics> {
    let mut lexer = Lexer::new(source, filename);
    lexer.run();
    if lexer.diags.has_errors() {
        Err(lexer.diags)
    } else {
        Ok(lexer.tokens)
    }
}

struct Lexer<'src> {
    src: &'src [u8],
    pos: usize,
    line: u32,
    line_start: usize,
    file: std::sync::Arc<str>,
    tokens: Vec<Token>,
    diags: crate::diag::Diagnostics,
}

impl<'src> Lexer<'src> {
    fn new(source: &'src str, filename: &str) -> Self {
        Self {
            src: source.as_bytes(),
            pos: 0,
            line: 1,
            line_start: 0,
            file: std::sync::Arc::from(filename),
            tokens: Vec::new(),
            diags: crate::diag::Diagnostics::new(),
        }
    }

    fn col(&self) -> u32 {
        (self.pos - self.line_start + 1) as u32
    }

    fn span(&self, start: usize, len: u32) -> Span {
        Span {
            file: self.file.clone(),
            line: self.line,
            col: (start - self.line_start + 1) as u32,
            len,
        }
    }

    fn peek(&self) -> Option<u8> { self.src.get(self.pos).copied() }
    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        if c == b'\n' {
            self.line += 1;
            self.line_start = self.pos;
        }
        Some(c)
    }

    fn lex_ident(&mut self) -> (String, Span) {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' { self.bump(); } else { break; }
        }
        let s = std::str::from_utf8(&self.src[start..self.pos]).unwrap().to_string();
        let span = self.span(start, (self.pos - start) as u32);
        (s, span)
    }

    fn lex_number(&mut self, signed_negative: bool) -> Token {
        let start = if signed_negative { self.pos - 1 } else { self.pos };

        let mut radix: u32 = 10;
        if self.peek() == Some(b'0') {
            match self.src.get(self.pos + 1) {
                Some(b'x') | Some(b'X') => { self.bump(); self.bump(); radix = 16; }
                Some(b'b') | Some(b'B') => { self.bump(); self.bump(); radix = 2; }
                _ => {}
            }
        }

        let digits_start = self.pos;
        while let Some(c) = self.peek() {
            let ok = match radix {
                10 => c.is_ascii_digit(),
                16 => c.is_ascii_hexdigit(),
                2  => c == b'0' || c == b'1',
                _  => false,
            };
            if !ok { break; }
            self.bump();
        }

        let body = std::str::from_utf8(&self.src[digits_start..self.pos]).unwrap();
        let value = i64::from_str_radix(body, radix).unwrap_or(0);
        let value = if signed_negative { -value } else { value };
        let span = self.span(start, (self.pos - start) as u32);
        Token { kind: TokenKind::Number(value), span }
    }

    fn run(&mut self) {
        while let Some(c) = self.peek() {
            match c {
                b' ' | b'\t' | b'\r' => { self.bump(); }
                b'\n' => {
                    let span = self.span(self.pos, 1);
                    self.bump();
                    self.tokens.push(Token { kind: TokenKind::Newline, span });
                }
                b'/' if self.src.get(self.pos + 1) == Some(&b'/') => {
                    while let Some(c) = self.peek() {
                        if c == b'\n' { break; }
                        self.bump();
                    }
                }
                b'.' => {
                    // Only legal start of `.equ`; anything else is an error.
                    let start = self.pos;
                    self.bump(); // consume '.'
                    let (kw, _) = self.lex_ident();
                    let span = self.span(start, (self.pos - start) as u32);
                    if kw == "equ" {
                        self.tokens.push(Token { kind: TokenKind::KwEqu, span });
                    } else {
                        self.diags.error(span, format!("unknown directive .{kw}"));
                    }
                }
                c if c.is_ascii_alphabetic() || c == b'_' => {
                    let (s, span) = self.lex_ident();
                    self.tokens.push(Token { kind: TokenKind::Ident(s), span });
                }
                b'0'..=b'9' => {
                    let tok = self.lex_number(false);
                    self.tokens.push(tok);
                }
                b'-' => {
                    self.bump();
                    let tok = self.lex_number(true);
                    self.tokens.push(tok);
                }
                _ => {
                    let span = self.span(self.pos, 1);
                    self.diags.error(span, format!("unexpected character {:?}", c as char));
                    self.bump(); // skip and continue
                }
            }
        }
        let span = self.span(self.pos, 0);
        self.tokens.push(Token { kind: TokenKind::Eof, span });
    }
}

#[cfg(test)]
mod whitespace_tests {
    use super::*;

    #[test]
    fn skip_whitespace_emit_newlines() {
        let toks = lex("  \n  \n", "x.tasm").unwrap();
        // Two Newlines + Eof.
        assert_eq!(toks.len(), 3);
        assert!(matches!(toks[0].kind, TokenKind::Newline));
        assert!(matches!(toks[1].kind, TokenKind::Newline));
        assert!(matches!(toks[2].kind, TokenKind::Eof));
    }

    #[test]
    fn unknown_char_diagnoses_but_continues() {
        let result = lex("@\n", "x.tasm");
        assert!(result.is_err());
    }

    #[test]
    fn line_comments_skipped() {
        let toks = lex("// hello world\n  // another\n", "x.tasm").unwrap();
        assert_eq!(toks.len(), 3); // two newlines + EOF
        assert!(matches!(toks[0].kind, TokenKind::Newline));
    }

    #[test]
    fn idents_and_equ() {
        let toks = lex("r0 ALU_R foo_bar .equ FOO\n", "x.tasm").unwrap();
        let kinds: Vec<_> = toks.iter().map(|t| &t.kind).collect();
        assert!(matches!(kinds[0], TokenKind::Ident(s) if s == "r0"));
        assert!(matches!(kinds[1], TokenKind::Ident(s) if s == "ALU_R"));
        assert!(matches!(kinds[2], TokenKind::Ident(s) if s == "foo_bar"));
        assert!(matches!(kinds[3], TokenKind::KwEqu));
        assert!(matches!(kinds[4], TokenKind::Ident(s) if s == "FOO"));
    }

    #[test]
    fn numbers_all_radices() {
        let toks = lex("42 -42 0xFF 0b1010\n", "x.tasm").unwrap();
        let nums: Vec<i64> = toks.iter()
            .filter_map(|t| if let TokenKind::Number(n) = t.kind { Some(n) } else { None })
            .collect();
        assert_eq!(nums, vec![42, -42, 0xFF, 0b1010]);
    }
}
