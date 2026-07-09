//! forge-hash — print an argon2id PHC hash for a password, for use in
//! `FORGE_AUTH_USERS`. Password comes from the first argument, or stdin.
//!
//! Build with: `cargo install forge-server --features cli` or
//! `cargo run -p forge-server --features cli --bin forge-hash`.

use std::io::Read;

use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};

fn main() {
    let password = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .expect("failed to read password from stdin");
            buf.trim_end_matches(['\r', '\n']).to_string()
        }
    };
    if password.is_empty() {
        eprintln!("usage: forge-hash <password>   (or pipe the password on stdin)");
        std::process::exit(2);
    }
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("argon2 hashing failed");
    println!("{hash}");
}
