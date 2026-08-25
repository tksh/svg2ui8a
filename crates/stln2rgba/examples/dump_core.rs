//! Test-support binary for the Wasm layer (engineering-playbook §3.3).
//!
//! Reads a usvg payload from stdin, runs the Rust-native `rasterize` core
//! function with `<width> <height> <alpha_mode>` arguments (0 = omitted),
//! writes the raw pixel bytes to stdout, and `<width> <height> <alpha_mode>`
//! of the actual result to stderr. Used by `scripts/test-wasm.ts` to compare
//! the Wasm artifact's output against the native core output for the same
//! input.

use std::io::{Read, Write};

use stln2rgba::core::{rasterize, RgbaOptions};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        eprintln!("usage: dump_core <width> <height> <straight|premultiplied>");
        eprintln!("       width/height of 0 mean \"omitted\"");
        std::process::exit(2);
    }
    let width: u32 = args[0].parse().expect("dump_core: width must be a u32");
    let height: u32 = args[1].parse().expect("dump_core: height must be a u32");
    let alpha_mode = args[2].clone();

    let mut usvg_bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut usvg_bytes)
        .expect("dump_core: failed to read stdin");

    let options = RgbaOptions {
        width,
        height,
        alpha_mode,
    };
    let result = rasterize(&usvg_bytes, &options).unwrap_or_else(|e| {
        eprintln!("dump_core: rasterize failed: {}", e);
        std::process::exit(1);
    });

    eprintln!("{} {} {}", result.width, result.height, result.alpha_mode);
    std::io::stdout()
        .write_all(&result.pixels)
        .expect("dump_core: failed to write stdout");
}
