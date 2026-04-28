//! Golden test: assemble fib.tasm and compare bytes against the
//! hand-encoded `emu/examples/fib.rs` output.

use std::path::Path;
use std::process::Command;
use toasttta_asm::assemble;

#[test]
fn fib_byte_identical() {
    // Run the example from the workspace root so its `.bin` lands where this
    // test reads it from. Matches the cwd a developer would use when running
    // `cargo run --example fib` by hand.
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();

    let status = Command::new("cargo")
        .args(&["run", "--quiet", "--example", "fib", "-p", "toasttta-emu"])
        .current_dir(workspace_root)
        .status()
        .expect("failed to run example");
    assert!(status.success(), "example program failed");

    let reference = std::fs::read(workspace_root.join("fib.bin"))
        .expect("reference fib.bin not found");

    let source = include_str!("fixtures/fib.tasm");
    let words = assemble(source, "fib.tasm").unwrap();
    let mut assembled = Vec::with_capacity(words.len() * 16);
    for w in &words {
        assembled.extend_from_slice(&w.encode().to_le_bytes());
    }

    assert_eq!(
        assembled, reference,
        "assembled bytes differ from hand-coded reference"
    );
}
