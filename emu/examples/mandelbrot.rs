//! Mandelbrot set generator for ToastTTA.
//!
//! Image: 32 × 16 ASCII. '#' for points still bounded after 16 iterations,
//! space otherwise.
//!
//! Fixed-point: Q12.4 (raw 16-bit; multiply by 16 to convert real → raw).
//!
//! x range: [-2.25, 1.75]   step 0.125   (raw -36, +2 per col, 32 cols)
//! y range: [-1.0,  0.875]  step 0.125   (raw -16, +2 per row, 16 rows)
//!
//! Algorithm per pixel:
//!     x = 0, y = 0, iter = 0
//!     loop:
//!         if iter >= 16: in_set
//!         x2 = (x*x) >> 4
//!         y2 = (y*y) >> 4
//!         sum = x2 + y2
//!         if (sum >> 6) != 0: escaped     # equivalent to sum >= 64 unsigned
//!         xy = (x*y) >> 4
//!         x  = x2 - y2 + cx
//!         y  = (xy << 1) + cy
//!         iter += 1
//!
//! Escape check uses `(sum >> 6) != 0` instead of `sum > 64` because the
//! intermediate sum can wrap to negative when (correctly) escaping; SHR
//! is logical, so it treats the wrapped value as the large unsigned number
//! it really represents.
//!
//! Register map:
//!   r0  = cx (Q12.4)        r5  = x²       r10 = scratch (xy / 2xy)
//!   r1  = cy (Q12.4)        r6  = y²
//!   r2  = x  (Q12.4)        r7  = col counter
//!   r3  = y  (Q12.4)        r8  = row counter
//!   r4  = iter

use std::fs;

use toasttta::isa::{dst, guard, mmio, src};
use toasttta::{IWord, Slot};

fn s(g: u8, src_sock: u8, data: u16, dst_sock: u8) -> Slot {
    Slot::new(g, src_sock, data, dst_sock)
}
fn nop() -> Slot {
    Slot::nop()
}

// Q12.4 raw helpers
fn q12_4(real: f32) -> u16 {
    (real * 16.0).round() as i16 as u16
}

// ─── Program label addresses ─────────────────────────────────────────────
//
// Layout:
//   0           INIT (1 word)
//   1           ROW: per-row init (1 word)
//   2           COL: per-pixel init (1 word)
//   3..24       INNER: 22-word iteration loop
//   25..26      IN_SET: print '#'; jump INC_COL (2 words)
//   27..28      ESCAPED: print ' '; jump INC_COL (2 words)
//   29..35      INC_COL: step cx; col++; conditional jump COL (7 words)
//   36..43      END_ROW: '\n'; step cy; row++; conditional jump ROW (8 words)
//   44..45      END: final '\n'; halt (2 words)

const INIT:    u16 = 0;
const ROW:     u16 = 1;
const COL:     u16 = 2;
const INNER:   u16 = 3;
const IN_SET:  u16 = 25;
const ESCAPED: u16 = 27;
const INC_COL: u16 = 29;
const END_ROW: u16 = 36;
const END:     u16 = 44;

const TOTAL_WORDS: usize = 46;

fn main() {
    let mut imem: Vec<IWord> = Vec::with_capacity(TOTAL_WORDS);

    // ─── INIT (word 0) ────────────────────────────────────────────────────
    // r8 = 0 (row); r1 = -16 (cy_init = -1.0 in Q12.4)
    imem.push(IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, 0,           dst::GPR_R8),
        s(guard::ALWAYS, src::IMMEDIATE, q12_4(-1.0), dst::GPR_R1),
        nop(),
        nop(),
    ));

    // ─── ROW (word 1) ─────────────────────────────────────────────────────
    // r7 = 0 (col); r0 = -36 (cx_init = -2.25)
    imem.push(IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, 0,            dst::GPR_R7),
        s(guard::ALWAYS, src::IMMEDIATE, q12_4(-2.25), dst::GPR_R0),
        nop(),
        nop(),
    ));

    // ─── COL (word 2) ─────────────────────────────────────────────────────
    // r2 = 0 (x); r3 = 0 (y); r4 = 0 (iter)
    imem.push(IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, 0, dst::GPR_R2),
        s(guard::ALWAYS, src::IMMEDIATE, 0, dst::GPR_R3),
        s(guard::ALWAYS, src::IMMEDIATE, 0, dst::GPR_R4),
        nop(),
    ));

    // ─── INNER (word 3..24) ───────────────────────────────────────────────

    // 3: iter >= 16?
    imem.push(IWord::new(
        s(guard::ALWAYS, src::GPR_R4,    0,  dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 16, dst::ALU_GE_T),
        nop(), nop(),
    ));
    // 4: capture predicate
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_P, 0, dst::BRF_P0),
        nop(), nop(), nop(),
    ));
    // 5: branch in_set
    imem.push(IWord::new(
        s(guard::IF_P0, src::IMMEDIATE, IN_SET, dst::GCU_JMP_T),
        nop(), nop(), nop(),
    ));
    // 6: trigger x*x
    imem.push(IWord::new(
        s(guard::ALWAYS, src::GPR_R2, 0, dst::MUL_A),
        s(guard::ALWAYS, src::GPR_R2, 0, dst::MUL_T),
        nop(), nop(),
    ));
    // 7: ALU shifts x*x>>4 (=x²); MUL fires y*y
    imem.push(IWord::new(
        s(guard::ALWAYS, src::MUL_R,    0, dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 4, dst::ALU_SSHR_T),
        s(guard::ALWAYS, src::GPR_R3,    0, dst::MUL_A),
        s(guard::ALWAYS, src::GPR_R3,    0, dst::MUL_T),
    ));
    // 8: capture x²; ALU shifts y*y>>4 (=y²)
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R,     0, dst::GPR_R5),
        s(guard::ALWAYS, src::MUL_R,     0, dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 4, dst::ALU_SSHR_T),
        nop(),
    ));
    // 9: capture y²
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R, 0, dst::GPR_R6),
        nop(), nop(), nop(),
    ));
    // 10: sum = x² + y²
    imem.push(IWord::new(
        s(guard::ALWAYS, src::GPR_R5, 0, dst::ALU_A),
        s(guard::ALWAYS, src::GPR_R6, 0, dst::ALU_ADD_T),
        nop(), nop(),
    ));
    // 11: sum >> 6 (logical, catches wrap-to-negative)
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R,    0, dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 6, dst::ALU_SHR_T),
        nop(), nop(),
    ));
    // 12: (sum >> 6) != 0 ?
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R,    0, dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 0, dst::ALU_NE_T),
        nop(), nop(),
    ));
    // 13: capture predicate
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_P, 0, dst::BRF_P0),
        nop(), nop(), nop(),
    ));
    // 14: branch escaped
    imem.push(IWord::new(
        s(guard::IF_P0, src::IMMEDIATE, ESCAPED, dst::GCU_JMP_T),
        nop(), nop(), nop(),
    ));
    // 15: trigger x*y
    imem.push(IWord::new(
        s(guard::ALWAYS, src::GPR_R2, 0, dst::MUL_A),
        s(guard::ALWAYS, src::GPR_R3, 0, dst::MUL_T),
        nop(), nop(),
    ));
    // 16: xy = (x*y) >> 4
    imem.push(IWord::new(
        s(guard::ALWAYS, src::MUL_R,    0, dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 4, dst::ALU_SSHR_T),
        nop(), nop(),
    ));
    // 17: capture xy
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R, 0, dst::GPR_R10),
        nop(), nop(), nop(),
    ));
    // 18: 2*xy = xy << 1
    imem.push(IWord::new(
        s(guard::ALWAYS, src::GPR_R10,   0, dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 1, dst::ALU_SHL_T),
        nop(), nop(),
    ));
    // 19: capture 2xy into r10; trigger x²-y²
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R,  0, dst::GPR_R10),
        s(guard::ALWAYS, src::GPR_R5, 0, dst::ALU_A),
        s(guard::ALWAYS, src::GPR_R6, 0, dst::ALU_SUB_T),
        nop(),
    ));
    // 20: (x²-y²) + cx
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R,  0, dst::ALU_A),
        s(guard::ALWAYS, src::GPR_R0, 0, dst::ALU_ADD_T),
        nop(), nop(),
    ));
    // 21: capture new_x; trigger 2xy + cy
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R,    0, dst::GPR_R2),
        s(guard::ALWAYS, src::GPR_R10,  0, dst::ALU_A),
        s(guard::ALWAYS, src::GPR_R1,   0, dst::ALU_ADD_T),
        nop(),
    ));
    // 22: capture new_y
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R, 0, dst::GPR_R3),
        nop(), nop(), nop(),
    ));
    // 23: iter++
    imem.push(IWord::new(
        s(guard::ALWAYS, src::GPR_R4,    0, dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 1, dst::ALU_ADD_T),
        nop(), nop(),
    ));
    // 24: capture iter; jump INNER
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R,    0,     dst::GPR_R4),
        s(guard::ALWAYS, src::IMMEDIATE, INNER, dst::GCU_JMP_T),
        nop(), nop(),
    ));

    assert_eq!(imem.len() as u16, IN_SET);

    // ─── IN_SET (words 25, 26) ────────────────────────────────────────────
    imem.push(IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, mmio::STDOUT_CHAR, dst::LSU_ST_A),
        s(guard::ALWAYS, src::IMMEDIATE, '#' as u16,         dst::LSU_ST_T),
        nop(), nop(),
    ));
    imem.push(IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, INC_COL, dst::GCU_JMP_T),
        nop(), nop(), nop(),
    ));

    assert_eq!(imem.len() as u16, ESCAPED);

    // ─── ESCAPED (words 27, 28) ───────────────────────────────────────────
    imem.push(IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, mmio::STDOUT_CHAR, dst::LSU_ST_A),
        s(guard::ALWAYS, src::IMMEDIATE, ' ' as u16,         dst::LSU_ST_T),
        nop(), nop(),
    ));
    imem.push(IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, INC_COL, dst::GCU_JMP_T),
        nop(), nop(), nop(),
    ));

    assert_eq!(imem.len() as u16, INC_COL);

    // ─── INC_COL (words 29..35) ───────────────────────────────────────────
    // cx += 0.125 (raw +2)
    imem.push(IWord::new(
        s(guard::ALWAYS, src::GPR_R0,    0,  dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 2,  dst::ALU_ADD_T),
        nop(), nop(),
    ));
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R, 0, dst::GPR_R0),
        nop(), nop(), nop(),
    ));
    // col++
    imem.push(IWord::new(
        s(guard::ALWAYS, src::GPR_R7,    0, dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 1, dst::ALU_ADD_T),
        nop(), nop(),
    ));
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R, 0, dst::GPR_R7),
        nop(), nop(), nop(),
    ));
    // col < 32?
    imem.push(IWord::new(
        s(guard::ALWAYS, src::GPR_R7,    0,  dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 32, dst::ALU_LT_T),
        nop(), nop(),
    ));
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_P, 0, dst::BRF_P0),
        nop(), nop(), nop(),
    ));
    // [p0] jump COL
    imem.push(IWord::new(
        s(guard::IF_P0, src::IMMEDIATE, COL, dst::GCU_JMP_T),
        nop(), nop(), nop(),
    ));

    assert_eq!(imem.len() as u16, END_ROW);

    // ─── END_ROW (words 36..43) ───────────────────────────────────────────
    // print '\n'
    imem.push(IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, mmio::STDOUT_CHAR, dst::LSU_ST_A),
        s(guard::ALWAYS, src::IMMEDIATE, '\n' as u16,        dst::LSU_ST_T),
        nop(), nop(),
    ));
    // cy += 2
    imem.push(IWord::new(
        s(guard::ALWAYS, src::GPR_R1,    0, dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 2, dst::ALU_ADD_T),
        nop(), nop(),
    ));
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R, 0, dst::GPR_R1),
        nop(), nop(), nop(),
    ));
    // row++
    imem.push(IWord::new(
        s(guard::ALWAYS, src::GPR_R8,    0, dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 1, dst::ALU_ADD_T),
        nop(), nop(),
    ));
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_R, 0, dst::GPR_R8),
        nop(), nop(), nop(),
    ));
    // row < 16?
    imem.push(IWord::new(
        s(guard::ALWAYS, src::GPR_R8,    0,  dst::ALU_A),
        s(guard::ALWAYS, src::IMMEDIATE, 16, dst::ALU_LT_T),
        nop(), nop(),
    ));
    imem.push(IWord::new(
        s(guard::ALWAYS, src::ALU_P, 0, dst::BRF_P0),
        nop(), nop(), nop(),
    ));
    // [p0] jump ROW
    imem.push(IWord::new(
        s(guard::IF_P0, src::IMMEDIATE, ROW, dst::GCU_JMP_T),
        nop(), nop(), nop(),
    ));

    assert_eq!(imem.len() as u16, END);

    // ─── END (words 44, 45) ───────────────────────────────────────────────
    imem.push(IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, mmio::STDOUT_CHAR, dst::LSU_ST_A),
        s(guard::ALWAYS, src::IMMEDIATE, '\n' as u16,        dst::LSU_ST_T),
        nop(), nop(),
    ));
    imem.push(IWord::new(
        s(guard::ALWAYS, src::IMMEDIATE, mmio::HALT, dst::LSU_ST_A),
        s(guard::ALWAYS, src::IMMEDIATE, 0,          dst::LSU_ST_T),
        nop(), nop(),
    ));

    assert_eq!(imem.len(), TOTAL_WORDS);

    // suppress unused-var warning for INIT
    let _ = INIT;

    // ─── Encode and write ────────────────────────────────────────────────

    let mut bytes = Vec::with_capacity(imem.len() * 16);
    for word in &imem {
        bytes.extend_from_slice(&word.encode().to_le_bytes());
    }

    let path = "mandelbrot.bin";
    fs::write(path, &bytes).expect("failed to write mandelbrot.bin");
    println!(
        "wrote {} instruction words ({} bytes) to {}",
        imem.len(),
        bytes.len(),
        path
    );
}
