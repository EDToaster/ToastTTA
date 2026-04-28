use std::env;
use std::fs;
use std::process;

use toasttta::{IWord, Machine};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <program.bin>", args[0]);
        process::exit(2);
    }

    let path = &args[1];
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("failed to read {path}: {e}");
            process::exit(2);
        }
    };

    if bytes.len() % 16 != 0 {
        eprintln!(
            "program size must be a multiple of 16 bytes (one 128-bit instruction word); got {}",
            bytes.len()
        );
        process::exit(2);
    }

    let imem: Vec<IWord> = bytes
        .chunks_exact(16)
        .map(|chunk| {
            let mut buf = [0u8; 16];
            buf.copy_from_slice(chunk);
            IWord::decode(u128::from_le_bytes(buf))
        })
        .collect();

    let mut m = Machine::new(imem);
    let exit_code = m.run();
    process::exit(exit_code);
}
