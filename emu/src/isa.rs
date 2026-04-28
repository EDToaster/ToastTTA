//! ISA constants — socket IDs, guard codes, MMIO addresses.
//! Cross-references `spec/isa.md` §2.3, §5, §6.1.

/// 32 KW of general data RAM (D-mem region 0x0000..0x7FFF).
pub const RAM_SIZE: usize = 0x8000;

// ─── Guard codes (2 bits, slot field bits [31:30]) ───────────────────────

pub mod guard {
    pub const ALWAYS: u8 = 0b00;
    pub const IF_P0:  u8 = 0b01;
    pub const IF_NP0: u8 = 0b10;
    pub const NEVER:  u8 = 0b11;
}

// ─── Source sockets (5 bits, slot field bits [29:25]) ────────────────────

pub mod src {
    // GPR_READ_r0..r15 occupy IDs 0..15.
    pub const GPR_R0:  u8 = 0;
    pub const GPR_R1:  u8 = 1;
    pub const GPR_R2:  u8 = 2;
    pub const GPR_R3:  u8 = 3;
    pub const GPR_R4:  u8 = 4;
    pub const GPR_R5:  u8 = 5;
    pub const GPR_R6:  u8 = 6;
    pub const GPR_R7:  u8 = 7;
    pub const GPR_R8:  u8 = 8;
    pub const GPR_R9:  u8 = 9;
    pub const GPR_R10: u8 = 10;
    pub const GPR_R11: u8 = 11;
    pub const GPR_R12: u8 = 12;
    pub const GPR_R13: u8 = 13;
    pub const GPR_R14: u8 = 14;
    pub const GPR_R15: u8 = 15;

    pub const BRF_P0:    u8 = 16;
    pub const ALU_R:     u8 = 17;
    pub const ALU_P:     u8 = 18;
    pub const LSU_R:     u8 = 19;
    pub const IMMEDIATE: u8 = 20;
    pub const MUL_R:     u8 = 21;

    pub fn is_gpr(s: u8) -> bool { s < 16 }
    pub fn gpr_idx(s: u8) -> usize { s as usize }
}

// ─── Destination sockets (6 bits, slot field bits [8:3]) ─────────────────

pub mod dst {
    // GPR_WRITE_r0..r15 occupy IDs 0..15.
    pub const GPR_R0:  u8 = 0;
    pub const GPR_R1:  u8 = 1;
    pub const GPR_R2:  u8 = 2;
    pub const GPR_R3:  u8 = 3;
    pub const GPR_R4:  u8 = 4;
    pub const GPR_R5:  u8 = 5;
    pub const GPR_R6:  u8 = 6;
    pub const GPR_R7:  u8 = 7;
    pub const GPR_R8:  u8 = 8;
    pub const GPR_R9:  u8 = 9;
    pub const GPR_R10: u8 = 10;
    pub const GPR_R11: u8 = 11;
    pub const GPR_R12: u8 = 12;
    pub const GPR_R13: u8 = 13;
    pub const GPR_R14: u8 = 14;
    pub const GPR_R15: u8 = 15;

    pub const BRF_P0:     u8 = 16;

    pub const ALU_A:      u8 = 17;
    pub const ALU_ADD_T:  u8 = 18;
    pub const ALU_SUB_T:  u8 = 19;
    pub const ALU_AND_T:  u8 = 20;
    pub const ALU_OR_T:   u8 = 21;
    pub const ALU_XOR_T:  u8 = 22;
    pub const ALU_SHL_T:  u8 = 23;
    pub const ALU_SHR_T:  u8 = 24;
    pub const ALU_SSHR_T: u8 = 25;
    pub const ALU_EQ_T:   u8 = 26;
    pub const ALU_NE_T:   u8 = 27;
    pub const ALU_LT_T:   u8 = 28;
    pub const ALU_LE_T:   u8 = 29;
    pub const ALU_GT_T:   u8 = 30;
    pub const ALU_GE_T:   u8 = 31;

    pub const LSU_LD_T:   u8 = 32;
    pub const LSU_ST_A:   u8 = 33;
    pub const LSU_ST_T:   u8 = 34;

    pub const GCU_JMP_T:  u8 = 35;

    pub const DISCARD:    u8 = 36;

    pub const MUL_A:      u8 = 37;
    pub const MUL_T:      u8 = 38;

    pub fn is_gpr(d: u8) -> bool { d < 16 }
    pub fn gpr_idx(d: u8) -> usize { d as usize }

    /// Operand-port writes apply BEFORE triggers within a cycle (spec §7.1).
    pub fn is_operand_port(d: u8) -> bool {
        matches!(d, ALU_A | LSU_ST_A | MUL_A)
    }
}

// ─── Memory-mapped I/O (spec §2.3) ───────────────────────────────────────

pub mod mmio {
    pub const STDOUT_CHAR: u16 = 0xFF00;
    pub const STDOUT_INT:  u16 = 0xFF01;
    pub const STDOUT_HEX:  u16 = 0xFF02;
    pub const STDIN_CHAR:  u16 = 0xFF10;
    pub const CYCLE_LO:    u16 = 0xFF20;
    pub const CYCLE_HI:    u16 = 0xFF21;
    pub const HALT:        u16 = 0xFFFE;

    pub const REGION_START: u16 = 0xFF00;

    pub fn is_mmio(addr: u16) -> bool { addr >= REGION_START }
}
