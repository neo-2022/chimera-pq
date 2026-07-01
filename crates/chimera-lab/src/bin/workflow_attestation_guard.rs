#![forbid(unsafe_code)]

use std::env;

#[path = "../workflow_attestation_guard.rs"]
mod workflow_attestation_guard;

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "docs/WORKFLOW_ATTESTATION.json".to_string());
    if let Err(msg) = workflow_attestation_guard::validate_file(&path) {
        fail(&msg);
    }
    println!("workflow attestation guard: PASS");
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
