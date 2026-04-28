//! Builds a sample program and writes it to `prog.bin`.
//!
//! The program:
//!   1. prints the banner "ToastTTA v1.0\n" via STDOUT_CHAR
//!   2. computes sum = 1 + 2 + ... + 10 in a predicated loop
//!   3. prints the result (55) as a decimal int, followed by newline
//!   4. halts with the sum as exit code (so the shell sees `echo $? -> 55`)
//!
//! Run with:
//!     cargo run --example sample_prog
//!     cargo run -- prog.bin
//!     echo $?      # expect 55

use std::fs;

use toasttta::isa::{dst, guard, mmio, src};
use toasttta::{IWord, Slot};

/// Build a slot.
fn s(g: u8, src_sock: u8, data: u16, dst_sock: u8) -> Slot {
    Slot::new(g, src_sock, data, dst_sock)
}

fn nop() -> Slot {
    Slot::nop()
}

/// Emit one cycle that writes a single character to STDOUT_CHAR.
/// (The address LSU.ST_A is re-set every cycle for clarity, even though it
/// would persist if left alone.)
fn emit_putc(c: char) -> IWord {
    IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, mmio::STDOUT_CHAR, dst::LSU_ST_A),
        s(guard::ALWAYS, src::IMMEDIATE, c as u16, dst::LSU_ST_T),
        nop(),
        nop(),
    )
}

fn main() {
    // ─── Build the program ────────────────────────────────────────────────
    //
    // Layout in I-mem:
    //   0..13   banner: "ToastTTA v1.0\n" (14 chars × 1 word each)
    //   14      init r0=0 (sum), r1=1 (i), r2=10 (N)
    //   15      LOOP: trigger compare (i <= N)
    //   16      capture predicate p0
    //   17      [!p0] jump PRINT;  start sum + i
    //   18      capture sum;  start i + 1
    //   19      capture i;  jump LOOP
    //   20      PRINT: print sum as int
    //   21      print '\n'
    //   22      halt with exit = sum

    let banner = "ToastTTA v1.0\n";
    let mut imem: Vec<IWord> = banner.chars().map(emit_putc).collect();

    let loop_label  = (imem.len() + 1) as u16;  // word 15
    let print_label = (imem.len() + 6) as u16;  // word 20

    // word 14: init
    imem.push(IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, 0,  dst::GPR_R0),
        s(guard::ALWAYS, src::IMMEDIATE, 1,  dst::GPR_R1),
        s(guard::ALWAYS, src::IMMEDIATE, 10, dst::GPR_R2),
        nop(),
    ));

    // word 15: LOOP — trigger (i <= N)
    imem.push(IWord::new(
        s(guard::ALWAYS, src::GPR_R1, 0, dst::ALU_A),
        s(guard::ALWAYS, src::GPR_R2, 0, dst::ALU_LE_T),
        nop(),
        nop(),
    ));

    // word 16: capture predicate
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_P, 0, dst::BRF_P0),
        nop(),
        nop(),
        nop(),
    ));

    // word 17: [!p0] jump PRINT;  in parallel: trigger sum + i
    imem.push(IWord::new(
        s(guard::IF_NP0, src::IMMEDIATE, print_label, dst::GCU_JMP_T),
        s(guard::ALWAYS, src::GPR_R0,    0,           dst::ALU_A),
        s(guard::ALWAYS, src::GPR_R1,    0,           dst::ALU_ADD_T),
        nop(),
    ));

    // word 18: capture sum into r0;  trigger i + 1
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R,    0, dst::GPR_R0),
        s(guard::ALWAYS, src::GPR_R1,   0, dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 1, dst::ALU_ADD_T),
        nop(),
    ));

    // word 19: capture i into r1;  jump back to LOOP
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R,    0,          dst::GPR_R1),
        s(guard::ALWAYS, src::IMMEDIATE, loop_label, dst::GCU_JMP_T),
        nop(),
        nop(),
    ));

    // word 20: PRINT — print r0 as decimal int
    imem.push(IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, mmio::STDOUT_INT, dst::LSU_ST_A),
        s(guard::ALWAYS, src::GPR_R0,    0,                dst::LSU_ST_T),
        nop(),
        nop(),
    ));

    // word 21: print newline
    imem.push(emit_putc('\n'));

    // word 22: halt with exit code = r0 (= 55)
    imem.push(IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, mmio::HALT, dst::LSU_ST_A),
        s(guard::ALWAYS, src::GPR_R0,    0,          dst::LSU_ST_T),
        nop(),
        nop(),
    ));

    // ─── Encode and write ────────────────────────────────────────────────

    let mut bytes = Vec::with_capacity(imem.len() * 16);
    for word in &imem {
        bytes.extend_from_slice(&word.encode().to_le_bytes());
    }

    let path = "prog.bin";
    fs::write(path, &bytes).expect("failed to write prog.bin");
    println!(
        "wrote {} instruction words ({} bytes) to {}",
        imem.len(),
        bytes.len(),
        path
    );
    println!("expected output: \"ToastTTA v1.0\\n55\\n\", exit code 55");
}
