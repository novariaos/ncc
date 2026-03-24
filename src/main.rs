mod cc;
mod ffi;
mod nvm;

use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("ncc - C to NVM bytecode compiler");
        eprintln!();
        eprintln!("Usage: ncc [options] <input.c>");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -o <file>       Output file (default: input.bin)");
        eprintln!("  --emit-asm      Stop after codegen, dump NVM assembly");
        eprintln!("  -I <dir>        Include directory for headers");
        std::process::exit(1);
    }

    let mut input_path: Option<PathBuf> = None;
    let mut output_path: Option<PathBuf> = None;
    let mut include_dirs: Vec<PathBuf> = Vec::new();
    let mut emit_asm = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                if i < args.len() {
                    output_path = Some(PathBuf::from(&args[i]));
                }
            }
            "-I" => {
                i += 1;
                if i < args.len() {
                    include_dirs.push(PathBuf::from(&args[i]));
                }
            }
            "--emit-asm" => emit_asm = true,
            other => {
                if other.starts_with('-') {
                    eprintln!("Unknown option: {other}");
                    std::process::exit(1);
                }
                input_path = Some(PathBuf::from(other));
            }
        }
        i += 1;
    }

    let input = input_path.unwrap_or_else(|| {
        eprintln!("Error: no input file specified");
        std::process::exit(1);
    });

    if let Err(e) = run_pipeline(&input, output_path, &include_dirs, emit_asm) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run_pipeline(
    input: &Path,
    output_path: Option<PathBuf>,
    include_dirs: &[PathBuf],
    emit_asm: bool,
) -> Result<(), String> {
    let source =
        fs::read_to_string(input).map_err(|e| format!("Cannot read {}: {e}", input.display()))?;

    let out = output_path.unwrap_or_else(|| input.with_extension("bin"));

    let is_asm = input.extension().map(|e| e == "asm").unwrap_or(false);

    let asm_text = if is_asm {
        source
    } else {
        let mut effective_dirs = include_dirs.to_vec();
        let cwd_include = Path::new("include");
        if cwd_include.is_dir() && !effective_dirs.iter().any(|d| d == cwd_include) {
            effective_dirs.push(cwd_include.to_path_buf());
        }

        let asm = cc::compile(&source, input, &effective_dirs)?;

        let asm_path = out.with_extension("asm");
        fs::write(&asm_path, &asm)
            .map_err(|e| format!("Cannot write {}: {e}", asm_path.display()))?;
        eprintln!("Wrote {}", asm_path.display());

        asm
    };

    if emit_asm {
        println!("{asm_text}");
        return Ok(());
    }

    let binary = ffi::assemble(&asm_text)?;

    fs::write(&out, &binary).map_err(|e| format!("Cannot write {}: {e}", out.display()))?;

    eprintln!(
        "Compiled {} -> {} ({} bytes)",
        input.display(),
        out.display(),
        binary.len()
    );

    Ok(())
}
