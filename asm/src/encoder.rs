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

    fn encode_cycle(&mut self, _cycle: CycleSpec) {
        // Stubbed for next task.
        self.iwords.push(IWord::new(
            Slot::nop(), Slot::nop(), Slot::nop(), Slot::nop()
        ));
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
}
