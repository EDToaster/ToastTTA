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
