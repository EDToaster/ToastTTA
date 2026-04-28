//! Encoder: translates parsed Lines into Vec<IWord>, resolving symbols.

use std::collections::HashMap;

use toasttta::{IWord, Slot};
use toasttta::isa::{dst, guard, src};

use crate::diag::{Diagnostics, Span};
use crate::parser::{
    CycleSpec, Destination, Guard, ImmExpr, Line, SlotSpec, Source,
};

pub fn encode(lines: Vec<Line>) -> Result<Vec<IWord>, Diagnostics> {
    let mut e = Encoder::new();
    e.run(lines);
    if e.diags.has_errors() { Err(e.diags) } else { Ok(e.iwords) }
}

struct Encoder {
    iwords: Vec<IWord>,
    symbols: HashMap<String, u16>,
    pending: Vec<Patch>,
    diags: Diagnostics,
}

struct Patch {
    addr: u16,
    slot_idx: usize,
    name: String,
    span: Span,
}

impl Encoder {
    fn new() -> Self {
        Self {
            iwords: Vec::new(),
            symbols: HashMap::new(),
            pending: Vec::new(),
            diags: Diagnostics::new(),
        }
    }

    fn run(&mut self, lines: Vec<Line>) {
        // Pass 1
        for line in lines {
            self.handle_line(line);
        }
        // Pass 2: backpatch forward references
        for patch in self.pending.drain(..).collect::<Vec<_>>() {
            match self.symbols.get(&patch.name) {
                Some(&addr) => {
                    let raw = self.iwords[patch.addr as usize].slots[patch.slot_idx].encode();
                    let mut decoded = Slot::decode(raw);
                    decoded.src_data = addr;
                    self.iwords[patch.addr as usize].slots[patch.slot_idx] = decoded;
                }
                None => self.diags.error(patch.span,
                    format!("undefined symbol '{}'", patch.name)),
            }
        }
    }

    fn handle_line(&mut self, line: Line) {
        match line {
            Line::Empty => {}
            Line::Equ { name, value, span } => {
                if self.symbols.insert(name.clone(), value).is_some() {
                    self.diags.error(span, format!("duplicate symbol '{name}'"));
                }
            }
            Line::Label { name, span, attached } => {
                let addr = self.iwords.len() as u16;
                if self.symbols.insert(name.clone(), addr).is_some() {
                    self.diags.error(span, format!("duplicate symbol '{name}'"));
                }
                if let Some(cycle) = attached {
                    self.encode_cycle(cycle);
                }
            }
            Line::Cycle(cycle) => self.encode_cycle(cycle),
        }
    }

    fn encode_cycle(&mut self, cycle: CycleSpec) {
        let addr = self.iwords.len() as u16;
        let mut slots = [Slot::nop(); 4];
        if cycle.slots.len() > 4 {
            self.diags.error(cycle.span,
                format!("more than 4 slots in a single cycle ({})", cycle.slots.len()));
        }
        for (i, spec) in cycle.slots.iter().enumerate().take(4) {
            slots[i] = self.encode_slot(addr, i, spec);
        }
        self.iwords.push(IWord::new(slots[0], slots[1], slots[2], slots[3]));
    }

    fn encode_slot(&mut self, addr: u16, idx: usize, spec: &SlotSpec) -> Slot {
        let g = match spec.guard {
            Guard::Always   => guard::ALWAYS,
            Guard::IfP0     => guard::IF_P0,
            Guard::IfNotP0  => guard::IF_NP0,
        };

        let (src_sock, src_data) = self.encode_source(addr, idx, &spec.src);
        let dst_sock = encode_destination(&spec.dst);

        Slot::new(g, src_sock, src_data, dst_sock)
    }

    fn encode_source(&mut self, addr: u16, idx: usize, source: &Source) -> (u8, u16) {
        match source {
            Source::Gpr(n) => (*n, 0),
            Source::BrfP0  => (src::BRF_P0, 0),
            Source::AluR   => (src::ALU_R, 0),
            Source::AluP   => (src::ALU_P, 0),
            Source::LsuR   => (src::LSU_R, 0),
            Source::MulR   => (src::MUL_R, 0),
            Source::Imm(ImmExpr::Literal(v)) => (src::IMMEDIATE, *v),
            Source::Imm(ImmExpr::Symbol(name)) => {
                if let Some(&v) = self.symbols.get(name) {
                    (src::IMMEDIATE, v)
                } else {
                    self.pending.push(Patch {
                        addr, slot_idx: idx,
                        name: name.clone(),
                        span: Span { file: std::sync::Arc::from(""), line: 0, col: 0, len: 0 },
                    });
                    (src::IMMEDIATE, 0)
                }
            }
        }
    }
}

fn encode_destination(d: &Destination) -> u8 {
    match d {
        Destination::Gpr(n)     => *n,
        Destination::BrfP0      => dst::BRF_P0,
        Destination::AluA       => dst::ALU_A,
        Destination::AluAddT    => dst::ALU_ADD_T,
        Destination::AluSubT    => dst::ALU_SUB_T,
        Destination::AluAndT    => dst::ALU_AND_T,
        Destination::AluOrT     => dst::ALU_OR_T,
        Destination::AluXorT    => dst::ALU_XOR_T,
        Destination::AluShlT    => dst::ALU_SHL_T,
        Destination::AluShrT    => dst::ALU_SHR_T,
        Destination::AluSshrT   => dst::ALU_SSHR_T,
        Destination::AluEqT     => dst::ALU_EQ_T,
        Destination::AluNeT     => dst::ALU_NE_T,
        Destination::AluLtT     => dst::ALU_LT_T,
        Destination::AluLeT     => dst::ALU_LE_T,
        Destination::AluGtT     => dst::ALU_GT_T,
        Destination::AluGeT     => dst::ALU_GE_T,
        Destination::LsuLdT     => dst::LSU_LD_T,
        Destination::LsuStA     => dst::LSU_ST_A,
        Destination::LsuStT     => dst::LSU_ST_T,
        Destination::GcuJmpT    => dst::GCU_JMP_T,
        Destination::Discard    => dst::DISCARD,
        Destination::MulA       => dst::MUL_A,
        Destination::MulT       => dst::MUL_T,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::parse;

    fn pipeline(src: &str) -> Result<Vec<IWord>, Diagnostics> {
        let toks = lex(src, "x.tasm").unwrap();
        let lines = parse(toks).unwrap();
        encode(lines)
    }

    #[test]
    fn empty_program() {
        let words = pipeline("").unwrap();
        assert!(words.is_empty());
    }

    #[test]
    fn equ_recorded() {
        let words = pipeline(".equ FOO 42\n").unwrap();
        assert!(words.is_empty()); // .equ produces no I-mem
    }

    #[test]
    fn duplicate_symbol_errors() {
        let result = pipeline(".equ X 1\n.equ X 2\n");
        assert!(result.is_err());
    }

    #[test]
    fn encodes_single_slot_with_nop_padding() {
        let words = pipeline("r0 -> r3\n").unwrap();
        assert_eq!(words.len(), 1);
        let s = words[0].slots[0];
        assert_eq!(s.guard, guard::ALWAYS);
        assert_eq!(s.src_sock, src::GPR_R0);
        assert_eq!(s.dst_sock, dst::GPR_R3);
        // Other 3 slots should be NOPs (guard = NEVER)
        for i in 1..4 {
            assert_eq!(words[0].slots[i].guard, guard::NEVER);
        }
    }

    #[test]
    fn encodes_immediate_literal() {
        let words = pipeline("#42 -> r0\n").unwrap();
        let s = words[0].slots[0];
        assert_eq!(s.src_sock, src::IMMEDIATE);
        assert_eq!(s.src_data, 42);
    }

    #[test]
    fn backward_label_resolves() {
        let src = "loop:\n#loop -> r0\n";
        let words = pipeline(src).unwrap();
        assert_eq!(words.len(), 1); // only the cycle line; label produces no IWord
        assert_eq!(words[0].slots[0].src_data, 0); // loop is at addr 0
    }

    #[test]
    fn forward_label_resolves() {
        let src = "#end -> r0\nend:\n";
        let words = pipeline(src).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].slots[0].src_data, 1); // end is at addr 1 (after the cycle)
    }

    #[test]
    fn undefined_symbol_errors() {
        let result = pipeline("#nope -> r0\n");
        assert!(result.is_err());
    }

    #[test]
    fn equ_symbol_resolves() {
        let src = ".equ ANS 42\n#ANS -> r0\n";
        let words = pipeline(src).unwrap();
        assert_eq!(words[0].slots[0].src_data, 42);
    }
}
