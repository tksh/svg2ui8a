use std::io::{self, Read};

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();
    let svg = String::from_utf8_lossy(&input);
    match svg2usvg::core::svg_to_usvg_bytes(&svg) {
        Ok(bytes) => {
            use std::io::Write;
            io::stdout().write_all(&bytes).unwrap();
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
