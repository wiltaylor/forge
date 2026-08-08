//! Write the JSON the block generators read.
//!
//!     cargo run -p forge-blocks --bin dump-contract   (`just generate-blocks`)
//!
//! The registry lives in Rust, and the web kit's generators run on Node
//! alone. This binary is the crossing point: it renders `contract/*.json`
//! from the registry, and `cargo test -p forge-blocks` fails while the
//! committed copies are stale.
//!
//! Paths are relative to the repository root; run it from there.

use std::fs;
use std::process::ExitCode;

use forge_blocks::export::{emoji_json, registry_json, EMOJI_PATH, REGISTRY_PATH};

fn main() -> ExitCode {
    for (path, text) in [(REGISTRY_PATH, registry_json()), (EMOJI_PATH, emoji_json())] {
        if fs::read_to_string(path).ok().as_deref() == Some(text.as_str()) {
            println!("{path} up to date");
            continue;
        }
        if let Err(err) = fs::write(path, text) {
            eprintln!("cannot write {path}: {err}");
            return ExitCode::FAILURE;
        }
        println!("wrote {path}");
    }
    ExitCode::SUCCESS
}
