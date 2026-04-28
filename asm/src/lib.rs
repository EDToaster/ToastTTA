//! ToastTTA assembler library.

pub mod diag;
pub mod lexer;
pub mod parser;
pub mod encoder;

use toasttta::IWord;
use crate::diag::Diagnostics;

pub fn assemble(source: &str, filename: &str) -> Result<Vec<IWord>, Diagnostics> {
    let tokens = lexer::lex(source, filename)?;
    let lines  = parser::parse(tokens)?;
    encoder::encode(lines)
}

#[cfg(test)]
mod end_to_end {
    use super::*;

    #[test]
    fn hello_program_assembles() {
        let src = "
.equ HALT 0xFFFE
main: #3 -> ALU_A; #4 -> ALU_ADD_T
      ALU_R -> r0
      #HALT -> LSU_ST_A; r0 -> LSU_ST_T
";
        let words = assemble(src, "test.tasm").unwrap();
        assert_eq!(words.len(), 3);
    }
}
