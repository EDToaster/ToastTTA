//! Slot and instruction-word encoding/decoding.
//! See `spec/isa.md` §4.

/// A decoded 32-bit move slot.
///
/// Bit layout (from spec §4.2):
/// ```text
///   [31:30] guard      (2 bits)
///   [29:25] src.sock   (5 bits)
///   [24:9]  src.data   (16 bits)
///   [8:3]   dst.sock   (6 bits)
///   [2:0]   reserved   (3 bits, must be zero)
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    pub guard:    u8,   // 2 bits
    pub src_sock: u8,   // 5 bits
    pub src_data: u16,  // 16 bits
    pub dst_sock: u8,   // 6 bits
}

/// A decoded 128-bit instruction word: four 32-bit slots in slot order
/// (slot 0 is the LSB-most 32 bits, slot 3 the MSB-most 32 bits).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IWord {
    pub slots: [Slot; 4],
}

impl Slot {
    /// Decode a raw 32-bit slot.
    pub fn decode(raw: u32) -> Self {
        Slot {
            guard:    ((raw >> 30) & 0x3)    as u8,
            src_sock: ((raw >> 25) & 0x1F)   as u8,
            src_data: ((raw >> 9)  & 0xFFFF) as u16,
            dst_sock: ((raw >> 3)  & 0x3F)   as u8,
        }
    }

    /// Encode this slot as a raw 32-bit value. The reserved field is always zero.
    pub fn encode(&self) -> u32 {
        ((self.guard    as u32 & 0x3)    << 30) |
        ((self.src_sock as u32 & 0x1F)   << 25) |
        ((self.src_data as u32 & 0xFFFF) << 9)  |
        ((self.dst_sock as u32 & 0x3F)   << 3)
    }

    /// A slot whose guard is `never` — equivalent to a NOP.
    pub fn nop() -> Self {
        Slot {
            guard:    0b11, // never
            src_sock: 0,
            src_data: 0,
            dst_sock: 0,
        }
    }

    /// Convenience constructor.
    pub fn new(guard: u8, src_sock: u8, src_data: u16, dst_sock: u8) -> Self {
        Slot { guard, src_sock, src_data, dst_sock }
    }
}

impl IWord {
    /// Decode a raw 128-bit instruction word.
    pub fn decode(raw: u128) -> Self {
        IWord {
            slots: [
                Slot::decode((raw)         as u32),
                Slot::decode((raw >> 32)   as u32),
                Slot::decode((raw >> 64)   as u32),
                Slot::decode((raw >> 96)   as u32),
            ],
        }
    }

    /// Encode this word as a raw 128-bit value.
    pub fn encode(&self) -> u128 {
        (self.slots[0].encode() as u128)        |
        ((self.slots[1].encode() as u128) << 32) |
        ((self.slots[2].encode() as u128) << 64) |
        ((self.slots[3].encode() as u128) << 96)
    }

    /// Build a word from four explicit slots.
    pub fn new(s0: Slot, s1: Slot, s2: Slot, s3: Slot) -> Self {
        IWord { slots: [s0, s1, s2, s3] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_round_trip() {
        let s = Slot::new(0b01, 20, 0xDEAD, 17);
        assert_eq!(Slot::decode(s.encode()), s);
    }

    #[test]
    fn iword_round_trip() {
        let w = IWord::new(
            Slot::new(0b00, 0, 0, 17),
            Slot::new(0b01, 20, 0x1234, 18),
            Slot::nop(),
            Slot::new(0b10, 17, 0, 5),
        );
        assert_eq!(IWord::decode(w.encode()), w);
    }

    #[test]
    fn nop_has_never_guard() {
        assert_eq!(Slot::nop().guard, 0b11);
    }
}
