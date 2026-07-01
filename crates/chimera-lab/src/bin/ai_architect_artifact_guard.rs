#![forbid(unsafe_code)]

use std::{env, fs};

#[path = "../workflow_attestation_guard_redaction.rs"]
#[allow(dead_code)]
mod workflow_attestation_guard_redaction;

fn main() {
    let paths: Vec<String> = env::args().skip(1).collect();
    let paths = if paths.is_empty() {
        vec![
            "docs/AI_ARCHITECT_LIFECYCLE_GUARD.md".to_string(),
            "docs/AI_ARCHITECT_ALGORITHM_COVERAGE.json".to_string(),
            "docs/WORKFLOW_ATTESTATION.json".to_string(),
            "docs/RESEARCH_DEBT.md".to_string(),
        ]
    } else {
        paths
    };

    for path in paths {
        let raw = fs::read_to_string(&path).unwrap_or_else(|err| {
            eprintln!("AI architect artifact guard: cannot read {path}: {err}");
            std::process::exit(1);
        });
        if let Err(err) = workflow_attestation_guard_redaction::reject_sensitive_raw_text(
            "AI architect artifact guard",
            &raw,
        ) {
            eprintln!("{path}: {err}");
            std::process::exit(1);
        }
    }
    println!("AI architect artifact guard: PASS");
}
