#![forbid(unsafe_code)]

use std::env;

#[path = "../current_workline_attestation_guard.rs"]
mod current_workline_attestation_guard;

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "docs/CURRENT_WORKLINE_ATTESTATION.json".to_string());
    if let Err(msg) = current_workline_attestation_guard::validate_file(&path) {
        fail(&msg);
    }
    println!("current workline attestation guard: PASS");
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
