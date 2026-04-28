//! Golden test: assemble mandelbrot.tasm and compare bytes against the
//! hand-encoded `emu/examples/mandelbrot.rs` output.
//!
//! This is the strongest regression test in the assembler suite: 46 instruction
//! words including nested loops, predicated branches, fixed-point math, and
//! MMIO. If the assembled `.tasm` produces the same 736 bytes as the hand-coded
//! Rust example, the entire assembler stack is correct.

use std::process::Command;
use toasttta_asm::assemble;

#[test]
fn mandelbrot_byte_identical() {
    let status = Command::new("cargo")
        .args(&["run", "--quiet", "--example", "mandelbrot", "-p", "toasttta-emu"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("failed to run example");
    assert!(status.success(), "example program failed");

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let reference = std::fs::read(workspace_root.join("mandelbrot.bin"))
        .expect("reference mandelbrot.bin not found");

    let source = include_str!("fixtures/mandelbrot.tasm");
    let words = assemble(source, "mandelbrot.tasm").unwrap();
    let mut assembled = Vec::with_capacity(words.len() * 16);
    for w in &words {
        assembled.extend_from_slice(&w.encode().to_le_bytes());
    }

    assert_eq!(assembled, reference, "mandelbrot bytes differ");
}
