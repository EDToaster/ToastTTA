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
