//! ToastTTA assembler library.
//!
//! See `docs/plans/2026-04-27-toasttta-assembler-design.md`.

use toasttta::IWord;

/// Top-level assembly entry point. Lex → parse → encode.
///
/// Returns the assembled instruction words on success, or a non-empty list
/// of diagnostics on failure.
pub fn assemble(_source: &str, _filename: &str) -> Result<Vec<IWord>, Vec<String>> {
    todo!("implemented incrementally — see plan")
}
