#![forbid(unsafe_code)]

use std::env;
use std::fs;

use serde_json::Value;

#[path = "../current_workline_attestation_guard.rs"]
mod current_workline_attestation_guard;

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "docs/CURRENT_WORKLINE_ATTESTATION.json".to_string());
    if let Err(msg) = current_workline_attestation_guard::validate_file(&path) {
        fail(&msg);
    }
    let status = fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| {
            value
                .get("status")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown".to_string());
    println!("current workline attestation guard: VALID status={status}");
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
