use std::fs;
use std::path::PathBuf;
use std::process;

use toasttta_asm::{assemble, diag::Diagnostics};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} INPUT.tasm [-o OUTPUT.bin]", args[0]);
        process::exit(2);
    }
    let input = PathBuf::from(&args[1]);
    let output = if let Some(i) = args.iter().position(|a| a == "-o") {
        PathBuf::from(args.get(i + 1).unwrap_or(&String::new()).clone())
    } else {
        input.with_extension("bin")
    };

    let source = match fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to read {}: {e}", input.display());
            process::exit(2);
        }
    };

    match assemble(&source, &input.to_string_lossy()) {
        Ok(words) => {
            let mut bytes = Vec::with_capacity(words.len() * 16);
            for w in &words {
                bytes.extend_from_slice(&w.encode().to_le_bytes());
            }
            if let Err(e) = fs::write(&output, &bytes) {
                eprintln!("failed to write {}: {e}", output.display());
                process::exit(2);
            }
            eprintln!("wrote {} instruction words to {}", words.len(), output.display());
        }
        Err(diags) => {
            print_diags(&diags, &source);
            process::exit(1);
        }
    }
}

fn print_diags(d: &Diagnostics, source: &str) {
    eprint!("{}", d.render(source));
}
