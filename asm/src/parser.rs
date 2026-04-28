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
            match self.peek().clone() {
                TokenKind::Newline => { self.bump(); self.lines.push(Line::Empty); }
                TokenKind::KwEqu   => self.parse_equ(),
                TokenKind::Ident(name) if self.tokens.get(self.pos + 1).map(|t| &t.kind)
                                          == Some(&TokenKind::Colon) => {
                    self.parse_label(name);
                }
                _ => self.parse_cycle_line(),
            }
        }
    }

    fn parse_label(&mut self, name: String) {
        let span = self.peek_span();
        self.bump(); // ident
        self.bump(); // ':'
        // attached cycle on same line?
        let attached = if matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
            self.expect_newline_or_eof();
            None
        } else {
            // parse a cycle inline
            let start = self.peek_span();
            let mut slots = Vec::new();
            if let Some(s) = self.parse_slot() { slots.push(s); }
            while matches!(self.peek(), TokenKind::Semi) {
                self.bump();
                if matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) { break; }
                if let Some(s) = self.parse_slot() { slots.push(s); }
            }
            self.expect_newline_or_eof();
            Some(CycleSpec { slots, span: start })
        };
        self.lines.push(Line::Label { name, span, attached });
    }

    fn parse_cycle_line(&mut self) {
        let start = self.peek_span();
        let mut slots = Vec::new();
        if let Some(s) = self.parse_slot() { slots.push(s); }
        while matches!(self.peek(), TokenKind::Semi) {
            self.bump();
            if matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) { break; }
            if let Some(s) = self.parse_slot() { slots.push(s); }
        }
        self.expect_newline_or_eof();
        if !slots.is_empty() {
            self.lines.push(Line::Cycle(CycleSpec { slots, span: start }));
        }
    }

    /// Parse one slot. Returns None and emits a diagnostic on failure.
    fn parse_slot(&mut self) -> Option<SlotSpec> {
        let start_span = self.peek_span();

        let guard = self.parse_guard()?;

        let src = self.parse_source()?;

        if !self.eat(&TokenKind::Arrow) {
            self.diags.error(self.peek_span(), "expected '->'");
            return None;
        }

        let dst = self.parse_destination()?;

        Some(SlotSpec { guard, src, dst, span: start_span })
    }

    fn parse_guard(&mut self) -> Option<Guard> {
        if !matches!(self.peek(), TokenKind::LBracket) {
            return Some(Guard::Always);
        }
        self.bump(); // [
        let inverted = self.eat(&TokenKind::Bang);
        let pname = if let TokenKind::Ident(s) = self.peek().clone() {
            self.bump(); s
        } else {
            self.diags.error(self.peek_span(), "expected predicate name in guard");
            return None;
        };
        if pname.to_ascii_lowercase() != "p0" {
            self.diags.error(self.peek_span(), format!("unknown predicate {pname}"));
            return None;
        }
        if !self.eat(&TokenKind::RBracket) {
            self.diags.error(self.peek_span(), "expected ']'");
            return None;
        }
        Some(if inverted { Guard::IfNotP0 } else { Guard::IfP0 })
    }

    fn parse_source(&mut self) -> Option<Source> {
        if matches!(self.peek(), TokenKind::Hash) {
            self.bump();
            return self.parse_immediate();
        }
        let name = if let TokenKind::Ident(s) = self.peek().clone() {
            self.bump(); s
        } else {
            self.diags.error(self.peek_span(), "expected source identifier or #immediate");
            return None;
        };
        match source_from_name(&name) {
            Some(s) => Some(s),
            None => {
                self.diags.error(self.peek_span(), format!("unknown source {name}"));
                None
            }
        }
    }

    fn parse_destination(&mut self) -> Option<Destination> {
        let name = if let TokenKind::Ident(s) = self.peek().clone() {
            self.bump(); s
        } else {
            self.diags.error(self.peek_span(), "expected destination identifier");
            return None;
        };
        match destination_from_name(&name) {
            Some(d) => Some(d),
            None => {
                self.diags.error(self.peek_span(), format!("unknown destination {name}"));
                None
            }
        }
    }

    fn parse_immediate(&mut self) -> Option<Source> {
        match self.peek().clone() {
            TokenKind::Number(n) => {
                if !(n >= -32768 && n <= 65535) {
                    self.diags.error(self.peek_span(),
                        format!("immediate {n} out of 16-bit range"));
                }
                self.bump();
                Some(Source::Imm(ImmExpr::Literal((n as i32 & 0xFFFF) as u16)))
            }
            TokenKind::Char(c) => {
                self.bump();
                Some(Source::Imm(ImmExpr::Literal(c)))
            }
            TokenKind::Ident(s) => {
                self.bump();
                Some(Source::Imm(ImmExpr::Symbol(s)))
            }
            _ => {
                self.diags.error(self.peek_span(), "expected literal or identifier after #");
                None
            }
        }
    }

    fn parse_equ(&mut self) {
        let kw_span = self.peek_span();
        self.bump(); // .equ

        let name = if let TokenKind::Ident(s) = self.peek().clone() {
            self.bump(); s
        } else {
            self.diags.error(self.peek_span(), "expected identifier after .equ");
            self.skip_to_newline();
            return;
        };

        let value = match self.peek().clone() {
            TokenKind::Number(n) => { self.bump(); n as i64 }
            TokenKind::Char(c)   => { self.bump(); c as i64 }
            _ => {
                self.diags.error(self.peek_span(), "expected literal value after .equ name");
                self.skip_to_newline();
                return;
            }
        };

        if !(value >= -32768 && value <= 65535) {
            self.diags.error(self.peek_span(), format!(".equ value {value} out of 16-bit range"));
        }

        self.expect_newline_or_eof();
        self.lines.push(Line::Equ {
            name,
            value: (value as i32 & 0xFFFF) as u16,
            span: kw_span,
        });
    }

    fn skip_to_newline(&mut self) {
        while !matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
            self.bump();
        }
        if matches!(self.peek(), TokenKind::Newline) { self.bump(); }
    }

    fn expect_newline_or_eof(&mut self) {
        match self.peek() {
            TokenKind::Newline => { self.bump(); }
            TokenKind::Eof => {}
            _ => {
                self.diags.error(self.peek_span(), "expected newline");
                self.skip_to_newline();
            }
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

    #[test]
    fn parses_equ() {
        let toks = crate::lexer::lex(".equ FOO 42\n.equ BAR 0xFF\n", "x.tasm").unwrap();
        let lines = parse(toks).unwrap();
        assert_eq!(lines.len(), 2);
        assert!(matches!(&lines[0], Line::Equ { name, value: 42, .. } if name == "FOO"));
        assert!(matches!(&lines[1], Line::Equ { name, value: 0xFF, .. } if name == "BAR"));
    }

    #[test]
    fn equ_rejects_out_of_range() {
        let toks = crate::lexer::lex(".equ X 100000\n", "x.tasm").unwrap();
        let result = parse(toks);
        assert!(result.is_err());
    }

    #[test]
    fn parses_simple_slot() {
        let toks = crate::lexer::lex("r0 -> r3\n", "x.tasm").unwrap();
        let lines = parse(toks).unwrap();
        let cycle = match &lines[0] { Line::Cycle(c) => c, _ => panic!() };
        assert_eq!(cycle.slots.len(), 1);
        assert_eq!(cycle.slots[0].guard, Guard::Always);
        assert_eq!(cycle.slots[0].src, Source::Gpr(0));
        assert_eq!(cycle.slots[0].dst, Destination::Gpr(3));
    }

    #[test]
    fn parses_immediate_literal() {
        let toks = crate::lexer::lex("#42 -> r0\n", "x.tasm").unwrap();
        let lines = parse(toks).unwrap();
        let cycle = match &lines[0] { Line::Cycle(c) => c, _ => panic!() };
        assert_eq!(cycle.slots[0].src, Source::Imm(ImmExpr::Literal(42)));
    }

    #[test]
    fn parses_guards() {
        let toks = crate::lexer::lex(
            "[p0] r0 -> r3\n[!p0] r1 -> r4\n",
            "x.tasm",
        ).unwrap();
        let lines = parse(toks).unwrap();
        let c0 = match &lines[0] { Line::Cycle(c) => c, _ => panic!() };
        let c1 = match &lines[1] { Line::Cycle(c) => c, _ => panic!() };
        assert_eq!(c0.slots[0].guard, Guard::IfP0);
        assert_eq!(c1.slots[0].guard, Guard::IfNotP0);
    }

    #[test]
    fn parses_immediate_char() {
        let toks = crate::lexer::lex("#'A' -> r0\n", "x.tasm").unwrap();
        let lines = parse(toks).unwrap();
        let c = match &lines[0] { Line::Cycle(c) => c, _ => panic!() };
        assert_eq!(c.slots[0].src, Source::Imm(ImmExpr::Literal(b'A' as u16)));
    }

    #[test]
    fn parses_immediate_symbol() {
        let toks = crate::lexer::lex("#loop -> r0\n", "x.tasm").unwrap();
        let lines = parse(toks).unwrap();
        let c = match &lines[0] { Line::Cycle(c) => c, _ => panic!() };
        assert_eq!(c.slots[0].src, Source::Imm(ImmExpr::Symbol("loop".into())));
    }

    #[test]
    fn parses_immediate_negative() {
        let toks = crate::lexer::lex("#-1 -> r0\n", "x.tasm").unwrap();
        let lines = parse(toks).unwrap();
        let c = match &lines[0] { Line::Cycle(c) => c, _ => panic!() };
        assert_eq!(c.slots[0].src, Source::Imm(ImmExpr::Literal(0xFFFF))); // two's complement -1
    }

    #[test]
    fn rejects_oversized_immediate() {
        let toks = crate::lexer::lex("#100000 -> r0\n", "x.tasm").unwrap();
        assert!(parse(toks).is_err());
    }

    #[test]
    fn parses_multi_slot_cycle() {
        let toks = crate::lexer::lex("r0 -> r3; r1 -> r4; r2 -> r5\n", "x.tasm").unwrap();
        let lines = parse(toks).unwrap();
        let c = match &lines[0] { Line::Cycle(c) => c, _ => panic!() };
        assert_eq!(c.slots.len(), 3);
    }

    #[test]
    fn allows_trailing_semicolon() {
        let toks = crate::lexer::lex("r0 -> r3;\n", "x.tasm").unwrap();
        let lines = parse(toks).unwrap();
        let c = match &lines[0] { Line::Cycle(c) => c, _ => panic!() };
        assert_eq!(c.slots.len(), 1);
    }

    #[test]
    fn parses_standalone_label() {
        let toks = crate::lexer::lex("loop:\n", "x.tasm").unwrap();
        let lines = parse(toks).unwrap();
        assert!(matches!(&lines[0], Line::Label { name, attached: None, .. } if name == "loop"));
    }

    #[test]
    fn parses_label_with_attached_cycle() {
        let toks = crate::lexer::lex("loop: r0 -> r3\n", "x.tasm").unwrap();
        let lines = parse(toks).unwrap();
        let l = match &lines[0] { Line::Label { name, attached: Some(c), .. } => (name, c), _ => panic!() };
        assert_eq!(l.0, "loop");
        assert_eq!(l.1.slots.len(), 1);
    }

    #[test]
    fn recovers_after_bad_line() {
        let toks = crate::lexer::lex("r0 ->\n#42 -> r0\n", "x.tasm").unwrap();
        let result = parse(toks);
        assert!(result.is_err());
    }
}
