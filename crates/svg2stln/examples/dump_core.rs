//! Test-support binary for the Wasm layer (engineering-playbook §3.3).
//!
//! Reads an SVG document from stdin, runs the Rust-native `svg` core
//! function, and writes the canonical-CBOR payload to stdout. Used by
//! `scripts/test-wasm.ts` to compare the Wasm artifact's output against
//! the native core output for the same input.

use std::io::{Read, Write};

fn main() {
    let mut svg = String::new();
    std::io::stdin()
        .read_to_string(&mut svg)
        .expect("dump_core: failed to read stdin");

    let bytes = svg2stln::svg(&svg).unwrap_or_else(|e| {
        eprintln!("dump_core: svg failed: {}", e);
        std::process::exit(1);
    });

    std::io::stdout()
        .write_all(&bytes)
        .expect("dump_core: failed to write stdout");
}
