//! Cycle-accurate ToastTTA machine.
//!
//! Implements the snapshot-then-apply execution model from spec §7.1:
//! every cycle, all four slots' sources are read against the current state,
//! then writes are applied with operand-port writes preceding all other writes
//! (including triggers).

use crate::encoding::{IWord, Slot};
use crate::isa::{self, dst, mmio, src};

/// Architectural state of a ToastTTA machine plus emulator runtime fields.
pub struct Machine {
    // ─── Architectural state (visible to programs) ───────────────────────
    pub pc: u16,
    pub gprs: [u16; 16],
    pub p0: bool,

    // FU operand-port latches (non-triggering destinations)
    pub alu_a:    u16,
    pub lsu_st_a: u16,
    pub mul_a:    u16,

    // FU output ports
    pub alu_r: u16,
    pub alu_p: bool,
    pub lsu_r: u16,
    pub mul_r: u16,

    // ─── Memories ────────────────────────────────────────────────────────
    pub imem: Vec<IWord>,
    pub dmem: Vec<u16>,

    // ─── Runtime state (not architecturally visible) ─────────────────────
    pub cycles:    u64,
    pub halted:    bool,
    pub exit_code: i32,
}

impl Machine {
    pub fn new(imem: Vec<IWord>) -> Self {
        Machine {
            pc:        0,
            gprs:      [0; 16],
            p0:        false,
            alu_a:     0,
            lsu_st_a:  0,
            mul_a:     0,
            alu_r:     0,
            alu_p:     false,
            lsu_r:     0,
            mul_r:     0,
            imem,
            dmem:      vec![0; isa::RAM_SIZE],
            cycles:    0,
            halted:    false,
            exit_code: 0,
        }
    }

    /// Execute one instruction word at PC.
    pub fn step(&mut self) {
        if self.halted {
            return;
        }

        let word = self.imem[self.pc as usize];

        // ─── Phase 1: snapshot ───────────────────────────────────────────
        // Evaluate guards and read sources from current state.
        let mut active = [false; 4];
        let mut srcs   = [0u16; 4];
        for i in 0..4 {
            active[i] = self.eval_guard(word.slots[i].guard);
            if active[i] {
                srcs[i] = self.read_source(&word.slots[i]);
            }
        }

        // ─── Phase 2a: operand-port writes ───────────────────────────────
        // Operand ports latch first so a same-cycle trigger can use the
        // freshly-latched operand (spec §7.1).
        for i in 0..4 {
            if !active[i] { continue; }
            let d = word.slots[i].dst_sock;
            if dst::is_operand_port(d) {
                self.write_dest(d, srcs[i]);
            }
        }

        // ─── Phase 2b: all other writes (including FU triggers) ──────────
        let mut next_pc = self.pc.wrapping_add(1);
        for i in 0..4 {
            if !active[i] { continue; }
            let d = word.slots[i].dst_sock;
            if !dst::is_operand_port(d) {
                if let Some(target) = self.write_dest(d, srcs[i]) {
                    next_pc = target;
                }
            }
        }

        self.pc = next_pc;
        self.cycles = self.cycles.wrapping_add(1);
    }

    /// Run until the machine halts (writes to MMIO HALT) and return the
    /// exit code.
    pub fn run(&mut self) -> i32 {
        while !self.halted {
            self.step();
        }
        self.exit_code
    }

    // ─── Guard evaluation (§6) ───────────────────────────────────────────

    fn eval_guard(&self, g: u8) -> bool {
        match g {
            isa::guard::ALWAYS => true,
            isa::guard::IF_P0  => self.p0,
            isa::guard::IF_NP0 => !self.p0,
            isa::guard::NEVER  => false,
            _ => unreachable!("invalid guard code: {g}"),
        }
    }

    // ─── Source read (§5.1) ──────────────────────────────────────────────

    fn read_source(&self, slot: &Slot) -> u16 {
        let s = slot.src_sock;
        if src::is_gpr(s) {
            return self.gprs[src::gpr_idx(s)];
        }
        match s {
            src::BRF_P0    => self.p0 as u16,
            src::ALU_R     => self.alu_r,
            src::ALU_P     => self.alu_p as u16,
            src::LSU_R     => self.lsu_r,
            src::IMMEDIATE => slot.src_data,
            src::MUL_R     => self.mul_r,
            _ => panic!("undefined source socket: {s}"),
        }
    }

    // ─── Destination write (§5.2) ────────────────────────────────────────
    //
    // Returns Some(target) if this write was a GCU jump trigger; the caller
    // uses that to override the default PC increment.

    fn write_dest(&mut self, d: u8, val: u16) -> Option<u16> {
        if dst::is_gpr(d) {
            self.gprs[dst::gpr_idx(d)] = val;
            return None;
        }

        match d {
            dst::BRF_P0     => self.p0 = val != 0,

            // ALU operand
            dst::ALU_A      => self.alu_a = val,

            // ALU arithmetic triggers (§8.1)
            dst::ALU_ADD_T  => self.alu_r = self.alu_a.wrapping_add(val),
            dst::ALU_SUB_T  => self.alu_r = self.alu_a.wrapping_sub(val),
            dst::ALU_AND_T  => self.alu_r = self.alu_a & val,
            dst::ALU_OR_T   => self.alu_r = self.alu_a | val,
            dst::ALU_XOR_T  => self.alu_r = self.alu_a ^ val,
            dst::ALU_SHL_T  => self.alu_r = self.alu_a.wrapping_shl((val & 0xF) as u32),
            dst::ALU_SHR_T  => self.alu_r = self.alu_a.wrapping_shr((val & 0xF) as u32),
            dst::ALU_SSHR_T => self.alu_r =
                (self.alu_a as i16).wrapping_shr((val & 0xF) as u32) as u16,

            // ALU compare triggers (signed)
            dst::ALU_EQ_T   => self.alu_p = self.alu_a == val,
            dst::ALU_NE_T   => self.alu_p = self.alu_a != val,
            dst::ALU_LT_T   => self.alu_p = (self.alu_a as i16) <  (val as i16),
            dst::ALU_LE_T   => self.alu_p = (self.alu_a as i16) <= (val as i16),
            dst::ALU_GT_T   => self.alu_p = (self.alu_a as i16) >  (val as i16),
            dst::ALU_GE_T   => self.alu_p = (self.alu_a as i16) >= (val as i16),

            // LSU
            dst::LSU_LD_T   => self.lsu_r = self.dmem_load(val),
            dst::LSU_ST_A   => self.lsu_st_a = val,
            dst::LSU_ST_T   => {
                let addr = self.lsu_st_a;
                self.dmem_store(addr, val);
            }

            // GCU
            dst::GCU_JMP_T  => return Some(val),

            dst::DISCARD    => {}

            // MUL
            dst::MUL_A      => self.mul_a = val,
            dst::MUL_T      => self.mul_r = self.mul_a.wrapping_mul(val),

            _ => panic!("undefined destination socket: {d}"),
        }
        None
    }

    // ─── D-memory load with MMIO dispatch ────────────────────────────────

    fn dmem_load(&self, addr: u16) -> u16 {
        match addr {
            mmio::STDIN_CHAR => 0, // TODO: non-blocking stdin read
            mmio::CYCLE_LO   => self.cycles as u16,
            mmio::CYCLE_HI   => (self.cycles >> 16) as u16,
            a if (a as usize) < isa::RAM_SIZE => self.dmem[a as usize],
            _ => 0, // unmapped — undefined per spec §11; emulator returns 0
        }
    }

    // ─── D-memory store with MMIO dispatch ───────────────────────────────

    fn dmem_store(&mut self, addr: u16, val: u16) {
        use std::io::Write;
        match addr {
            mmio::STDOUT_CHAR => {
                let c = (val as u8) as char;
                print!("{c}");
                let _ = std::io::stdout().flush();
            }
            mmio::STDOUT_INT  => {
                print!("{}", val as i16);
                let _ = std::io::stdout().flush();
            }
            mmio::STDOUT_HEX  => {
                print!("0x{val:04X}");
                let _ = std::io::stdout().flush();
            }
            mmio::HALT        => {
                self.halted = true;
                self.exit_code = (val as i16) as i32;
            }
            a if (a as usize) < isa::RAM_SIZE => self.dmem[a as usize] = val,
            _ => {} // unmapped — undefined per spec §11
        }
    }
}
