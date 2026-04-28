//! Recursive-descent parser for ToastTTA assembly.

use crate::diag::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    Empty,
    Label  { name: String, span: Span, attached: Option<CycleSpec> },
    Equ    { name: String, value: u16, span: Span },
    Cycle  (CycleSpec),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CycleSpec {
    pub slots: Vec<SlotSpec>,
    pub span:  Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotSpec {
    pub guard: Guard,
    pub src:   Source,
    pub dst:   Destination,
    pub span:  Span,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Guard {
    Always,
    IfP0,
    IfNotP0,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Gpr(u8),                  // 0..15
    BrfP0,
    AluR, AluP, LsuR, MulR,
    Imm(ImmExpr),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImmExpr {
    Literal(u16),
    Symbol(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    Gpr(u8),
    BrfP0,
    AluA,
    AluAddT, AluSubT, AluAndT, AluOrT, AluXorT,
    AluShlT, AluShrT, AluSshrT,
    AluEqT, AluNeT, AluLtT, AluLeT, AluGtT, AluGeT,
    LsuLdT, LsuStA, LsuStT,
    GcuJmpT,
    Discard,
    MulA, MulT,
}

#[cfg(test)]
mod ast_smoke_tests {
    use super::*;
    #[test]
    fn variants_distinct() {
        assert_ne!(Source::AluR, Source::AluP);
        assert_ne!(Destination::AluA, Destination::AluAddT);
    }
}

/// Look up an identifier as a source. Case-insensitive.
pub fn source_from_name(name: &str) -> Option<Source> {
    let lower = name.to_ascii_lowercase();
    if let Some(stripped) = lower.strip_prefix('r') {
        if let Ok(n) = stripped.parse::<u8>() {
            if n < 16 { return Some(Source::Gpr(n)); }
        }
    }
    Some(match lower.as_str() {
        "p0"     => Source::BrfP0,
        "alu_r"  => Source::AluR,
        "alu_p"  => Source::AluP,
        "lsu_r"  => Source::LsuR,
        "mul_r"  => Source::MulR,
        _        => return None,
    })
}

/// Look up an identifier as a destination. Case-insensitive.
pub fn destination_from_name(name: &str) -> Option<Destination> {
    let lower = name.to_ascii_lowercase();
    if let Some(stripped) = lower.strip_prefix('r') {
        if let Ok(n) = stripped.parse::<u8>() {
            if n < 16 { return Some(Destination::Gpr(n)); }
        }
    }
    Some(match lower.as_str() {
        "p0"        => Destination::BrfP0,
        "alu_a"     => Destination::AluA,
        "alu_add_t" => Destination::AluAddT,
        "alu_sub_t" => Destination::AluSubT,
        "alu_and_t" => Destination::AluAndT,
        "alu_or_t"  => Destination::AluOrT,
        "alu_xor_t" => Destination::AluXorT,
        "alu_shl_t" => Destination::AluShlT,
        "alu_shr_t" => Destination::AluShrT,
        "alu_sshr_t"=> Destination::AluSshrT,
        "alu_eq_t"  => Destination::AluEqT,
        "alu_ne_t"  => Destination::AluNeT,
        "alu_lt_t"  => Destination::AluLtT,
        "alu_le_t"  => Destination::AluLeT,
        "alu_gt_t"  => Destination::AluGtT,
        "alu_ge_t"  => Destination::AluGeT,
        "lsu_ld_t"  => Destination::LsuLdT,
        "lsu_st_a"  => Destination::LsuStA,
        "lsu_st_t"  => Destination::LsuStT,
        "gcu_jmp_t" => Destination::GcuJmpT,
        "discard"   => Destination::Discard,
        "mul_a"     => Destination::MulA,
        "mul_t"     => Destination::MulT,
        _ => return None,
    })
}

#[cfg(test)]
mod lookup_tests {
    use super::*;

    #[test]
    fn gpr_source() {
        assert_eq!(source_from_name("r0"),  Some(Source::Gpr(0)));
        assert_eq!(source_from_name("R15"), Some(Source::Gpr(15)));
        assert_eq!(source_from_name("r16"), None);
    }

    #[test]
    fn fu_sources_case_insensitive() {
        assert_eq!(source_from_name("ALU_R"), Some(Source::AluR));
        assert_eq!(source_from_name("alu_r"), Some(Source::AluR));
        assert_eq!(source_from_name("Mul_R"), Some(Source::MulR));
    }

    #[test]
    fn unknown_returns_none() {
        assert_eq!(source_from_name("garbage"),       None);
        assert_eq!(destination_from_name("MUL_HI_T"), None);
    }

    #[test]
    fn destinations_complete() {
        assert_eq!(destination_from_name("ALU_ADD_T"), Some(Destination::AluAddT));
        assert_eq!(destination_from_name("GCU_JMP_T"), Some(Destination::GcuJmpT));
        assert_eq!(destination_from_name("DISCARD"),   Some(Destination::Discard));
    }
}

use crate::diag::Diagnostics;
use crate::lexer::{Token, TokenKind};

pub fn parse(tokens: Vec<Token>) -> Result<Vec<Line>, Diagnostics> {
    let mut p = Parser::new(tokens);
    p.parse_program();
    if p.diags.has_errors() { Err(p.diags) } else { Ok(p.lines) }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    lines: Vec<Line>,
    diags: Diagnostics,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0, lines: Vec::new(), diags: Diagnostics::new() }
    }

    fn peek(&self) -> &TokenKind { &self.tokens[self.pos].kind }
    fn peek_span(&self) -> Span { self.tokens[self.pos].span.clone() }
    fn bump(&mut self) -> &Token {
        let t = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() { self.pos += 1; }
        t
    }
    fn eat(&mut self, kind: &TokenKind) -> bool {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(kind) {
            self.bump();
            true
        } else { false }
    }

    fn parse_program(&mut self) {
        while !matches!(self.peek(), TokenKind::Eof) {
            if matches!(self.peek(), TokenKind::Newline) {
                self.bump();
                self.lines.push(Line::Empty);
                continue;
            }
            // Other line kinds wired up in subsequent tasks.
            // For now, skip unknown tokens by advancing.
            self.bump();
        }
    }
}

#[cfg(test)]
mod parse_skeleton_tests {
    use super::*;
    use crate::lexer::lex;

    #[test]
    fn empty_input() {
        let toks = lex("", "x.tasm").unwrap();
        let lines = parse(toks).unwrap();
        assert!(lines.is_empty());
    }

    #[test]
    fn empty_lines() {
        let toks = lex("\n\n\n", "x.tasm").unwrap();
        let lines = parse(toks).unwrap();
        assert_eq!(lines, vec![Line::Empty, Line::Empty, Line::Empty]);
    }
}
