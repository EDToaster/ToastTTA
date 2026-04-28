//! Golden test: assemble sample_prog.tasm and compare bytes against the
//! hand-encoded `emu/examples/sample_prog.rs` output.

use std::path::Path;
use std::process::Command;
use toasttta_asm::assemble;

#[test]
fn sample_prog_byte_identical() {
    // The example writes its `.bin` to the current working directory. Run it
    // from the workspace root so the output lands where this test reads it
    // from, matching how a developer would invoke `cargo run --example` by
    // hand from the repo root.
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();

    // Generate the hand-coded reference binary.
    let status = Command::new("cargo")
        .args(&["run", "--quiet", "--example", "sample_prog", "-p", "toasttta-emu"])
        .current_dir(workspace_root)
        .status()
        .expect("failed to run example");
    assert!(status.success(), "example program failed");

    let reference = std::fs::read(workspace_root.join("prog.bin"))
        .expect("reference prog.bin not found");

    let source = include_str!("fixtures/sample_prog.tasm");
    let words = assemble(source, "sample_prog.tasm").unwrap();
    let mut assembled = Vec::with_capacity(words.len() * 16);
    for w in &words {
        assembled.extend_from_slice(&w.encode().to_le_bytes());
    }

    assert_eq!(
        assembled, reference,
        "assembled bytes differ from hand-coded reference"
    );
}
