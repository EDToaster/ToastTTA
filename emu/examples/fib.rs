//! Builds a program that prints the first 10 Fibonacci numbers, one per line.
//!
//! Sequence printed: 1, 1, 2, 3, 5, 8, 13, 21, 34, 55.
//!
//! Run with:
//!     cargo run --example fib
//!     cargo run -- fib.bin
//!
//! Algorithm:
//!     a, b = 0, 1
//!     for _ in 0..10:
//!         print b
//!         a, b = b, a + b
//!
//! Register map:
//!     r0 = a   (previous Fibonacci)
//!     r1 = b   (current  Fibonacci — the one being printed)
//!     r2 = counter (starts at 10, decremented each iter)

use std::fs;

use toasttta::isa::{dst, guard, mmio, src};
use toasttta::{IWord, Slot};

fn s(g: u8, src_sock: u8, data: u16, dst_sock: u8) -> Slot {
    Slot::new(g, src_sock, data, dst_sock)
}
fn nop() -> Slot {
    Slot::nop()
}

fn emit_putc(c: char) -> IWord {
    IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, mmio::STDOUT_CHAR, dst::LSU_ST_A),
        s(guard::ALWAYS, src::IMMEDIATE, c as u16, dst::LSU_ST_T),
        nop(),
        nop(),
    )
}

fn main() {
    let banner = "Fibonacci:\n";
    let mut imem: Vec<IWord> = banner.chars().map(emit_putc).collect();

    // Init: r0 = 0 (a), r1 = 1 (b), r2 = 10 (counter)
    imem.push(IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, 0, dst::GPR_R0),
        s(guard::ALWAYS, src::IMMEDIATE, 1, dst::GPR_R1),
        s(guard::ALWAYS, src::IMMEDIATE, 10, dst::GPR_R2),
        nop(),
    ));

    // Reserve label addresses ahead of time (loop is exactly 6 words long).
    let loop_label = imem.len() as u16;        // start of LOOP
    let end_label  = loop_label + 6;           // word right after the loop body

    // ─── LOOP body (6 cycles per iteration) ──────────────────────────────
    //
    // LOOP+0: trigger compare counter != 0; preset LSU.ST_A for int output.
    // LOOP+1: capture predicate ALU.P -> p0.
    // LOOP+2: [!p0] jump END;  [p0] print b;  in parallel start a+b.
    // LOOP+3: print '\n'; capture sum into r1 (b ← c); copy old r1 to r0 (a ← b).
    // LOOP+4: trigger counter += -1.
    // LOOP+5: capture decremented counter; jump LOOP.

    // LOOP+0
    imem.push(IWord::new(
        s(guard::ALWAYS, src::GPR_R2,    0,                dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 0,                dst::ALU_NE_T),
        s(guard::ALWAYS, src::IMMEDIATE, mmio::STDOUT_INT, dst::LSU_ST_A),
        nop(),
    ));

    // LOOP+1
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_P, 0, dst::BRF_P0),
        nop(),
        nop(),
        nop(),
    ));

    // LOOP+2
    imem.push(IWord::new(
        s(guard::IF_NP0, src::IMMEDIATE, end_label, dst::GCU_JMP_T),
        s(guard::IF_P0,  src::GPR_R1,    0,         dst::LSU_ST_T),
        s(guard::ALWAYS, src::GPR_R0,    0,         dst::ALU_A),
        s(guard::ALWAYS, src::GPR_R1,    0,         dst::ALU_ADD_T),
    ));

    // LOOP+3
    imem.push(IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, mmio::STDOUT_CHAR, dst::LSU_ST_A),
        s(guard::ALWAYS, src::IMMEDIATE, '\n' as u16,        dst::LSU_ST_T),
        s(guard::ALWAYS, src::ALU_R,    0,                   dst::GPR_R1),
        s(guard::ALWAYS, src::GPR_R1,   0,                   dst::GPR_R0),
    ));

    // LOOP+4
    imem.push(IWord::new(
        s(guard::ALWAYS, src::GPR_R2,    0,      dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 0xFFFF, dst::ALU_ADD_T), // +(-1)
        nop(),
        nop(),
    ));

    // LOOP+5
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R,    0,          dst::GPR_R2),
        s(guard::ALWAYS, src::IMMEDIATE, loop_label, dst::GCU_JMP_T),
        nop(),
        nop(),
    ));

    // Sanity check that we computed end_label correctly.
    assert_eq!(imem.len() as u16, end_label);

    // END: halt with exit code 0
    imem.push(IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, mmio::HALT, dst::LSU_ST_A),
        s(guard::ALWAYS, src::IMMEDIATE, 0,          dst::LSU_ST_T),
        nop(),
        nop(),
    ));

    // ─── Encode and write ────────────────────────────────────────────────

    let mut bytes = Vec::with_capacity(imem.len() * 16);
    for word in &imem {
        bytes.extend_from_slice(&word.encode().to_le_bytes());
    }

    let path = "fib.bin";
    fs::write(path, &bytes).expect("failed to write fib.bin");
    println!(
        "wrote {} instruction words ({} bytes) to {}",
        imem.len(),
        bytes.len(),
        path
    );
    println!("expected output: \"Fibonacci:\\n1\\n1\\n2\\n3\\n5\\n8\\n13\\n21\\n34\\n55\\n\"");
}
