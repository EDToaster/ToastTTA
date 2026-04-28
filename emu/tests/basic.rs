//! Integration tests for the ToastTTA emulator.
//!
//! These hand-encode small programs at the IWord level (no assembler yet)
//! to validate the execution model.

use toasttta::isa::{dst, guard, mmio, src};
use toasttta::{IWord, Machine, Slot};

fn s(g: u8, src_sock: u8, data: u16, dst_sock: u8) -> Slot {
    Slot::new(g, src_sock, data, dst_sock)
}

fn nop() -> Slot {
    Slot::nop()
}

/// 3 + 4 = 7. Capture into r0, then halt.
/// (We don't print here — that would require capturing stdout. Halt code is
/// enough to verify correctness.)
#[test]
fn add_then_halt() {
    let imem = vec![
        // { #3 -> ALU.A | #4 -> ALU.ADD_T | nop | nop }
        IWord::new(
            s(guard::ALWAYS, src::IMMEDIATE, 3, dst::ALU_A),
            s(guard::ALWAYS, src::IMMEDIATE, 4, dst::ALU_ADD_T),
            nop(),
            nop(),
        ),
        // { ALU.R -> r0 | nop | nop | nop }
        IWord::new(
            s(guard::ALWAYS, src::ALU_R, 0, dst::GPR_R0),
            nop(),
            nop(),
            nop(),
        ),
        // { #HALT -> LSU.ST_A | r0 -> LSU.ST_T | nop | nop }
        // Halt with the value of r0 as exit code (= 7).
        IWord::new(
            s(guard::ALWAYS, src::IMMEDIATE, mmio::HALT, dst::LSU_ST_A),
            s(guard::ALWAYS, src::GPR_R0, 0, dst::LSU_ST_T),
            nop(),
            nop(),
        ),
    ];

    let mut m = Machine::new(imem);
    let exit = m.run();

    assert_eq!(m.gprs[0], 7, "r0 should be 7");
    assert_eq!(exit, 7, "exit code should be 7 (the halt value)");
    assert!(m.halted);
}

/// (r0 + r1) - (r2 - r0) using the schedule from the quiz answer.
#[test]
fn quiz_q7_schedule() {
    // Initialize: r0=10, r1=3, r2=20 → expect r3 = (10+3) - (20-10) = 13 - 10 = 3
    let imem = vec![
        // Init r0, r1, r2
        IWord::new(
            s(guard::ALWAYS, src::IMMEDIATE, 10, dst::GPR_R0),
            s(guard::ALWAYS, src::IMMEDIATE, 3,  dst::GPR_R1),
            s(guard::ALWAYS, src::IMMEDIATE, 20, dst::GPR_R2),
            nop(),
        ),
        // cycle 0:  r0 -> ALU.A;  r1 -> ALU_ADD_T
        IWord::new(
            s(guard::ALWAYS, src::GPR_R0, 0, dst::ALU_A),
            s(guard::ALWAYS, src::GPR_R1, 0, dst::ALU_ADD_T),
            nop(),
            nop(),
        ),
        // cycle 1:  ALU.R -> r3;  r2 -> ALU.A;  r0 -> ALU_SUB_T
        IWord::new(
            s(guard::ALWAYS, src::ALU_R,  0, dst::GPR_R3),
            s(guard::ALWAYS, src::GPR_R2, 0, dst::ALU_A),
            s(guard::ALWAYS, src::GPR_R0, 0, dst::ALU_SUB_T),
            nop(),
        ),
        // cycle 2:  r3 -> ALU.A;  ALU.R -> ALU_SUB_T
        IWord::new(
            s(guard::ALWAYS, src::GPR_R3, 0, dst::ALU_A),
            s(guard::ALWAYS, src::ALU_R,  0, dst::ALU_SUB_T),
            nop(),
            nop(),
        ),
        // cycle 3:  ALU.R -> r3
        IWord::new(
            s(guard::ALWAYS, src::ALU_R, 0, dst::GPR_R3),
            nop(),
            nop(),
            nop(),
        ),
        // halt
        IWord::new(
            s(guard::ALWAYS, src::IMMEDIATE, mmio::HALT, dst::LSU_ST_A),
            s(guard::ALWAYS, src::IMMEDIATE, 0, dst::LSU_ST_T),
            nop(),
            nop(),
        ),
    ];

    let mut m = Machine::new(imem);
    m.run();

    assert_eq!(m.gprs[3], 3, "r3 should be (10+3) - (20-10) = 3");
    assert!(m.halted);
}

/// Predicated branch: skip a write if condition is true.
/// r0 = 5; if r0 < 10, set r1 = 99 else set r1 = 7. Expect r1 = 99.
#[test]
fn predicated_assignment() {
    let imem = vec![
        // r0 = 5
        IWord::new(
            s(guard::ALWAYS, src::IMMEDIATE, 5, dst::GPR_R0),
            nop(), nop(), nop(),
        ),
        // r0 -> ALU.A;  #10 -> ALU_LT_T  (compute r0 < 10)
        IWord::new(
            s(guard::ALWAYS, src::GPR_R0, 0, dst::ALU_A),
            s(guard::ALWAYS, src::IMMEDIATE, 10, dst::ALU_LT_T),
            nop(), nop(),
        ),
        // ALU.P -> p0
        IWord::new(
            s(guard::ALWAYS, src::ALU_P, 0, dst::BRF_P0),
            nop(), nop(), nop(),
        ),
        // [p0] #99 -> r1;  [!p0] #7 -> r1
        IWord::new(
            s(guard::IF_P0,  src::IMMEDIATE, 99, dst::GPR_R1),
            s(guard::IF_NP0, src::IMMEDIATE,  7, dst::GPR_R1),
            nop(), nop(),
        ),
        // halt
        IWord::new(
            s(guard::ALWAYS, src::IMMEDIATE, mmio::HALT, dst::LSU_ST_A),
            s(guard::ALWAYS, src::IMMEDIATE, 0, dst::LSU_ST_T),
            nop(), nop(),
        ),
    ];

    let mut m = Machine::new(imem);
    m.run();

    assert_eq!(m.gprs[1], 99, "predicate true → r1 = 99");
}
