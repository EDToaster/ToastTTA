//! ToastTTA emulator library.
//!
//! Implements a cycle-accurate emulator for ToastTTA ISA v1.0.
//! See `spec/isa.md` at the repo root for the architectural reference.

pub mod isa;
pub mod encoding;
pub mod machine;

pub use encoding::{Slot, IWord};
pub use machine::Machine;
