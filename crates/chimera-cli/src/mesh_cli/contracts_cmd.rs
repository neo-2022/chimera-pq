use std::fs;
use std::io::{self, Read};
use std::time::{SystemTime, UNIX_EPOCH};

use chimera_contracts::{
    CatalogPersistenceDocument, CatalogPersistenceKind, ContractValidate, ManifestCatalogBundle,
};

pub(crate) fn mesh_contracts_command(usage: &str, args: &[String]) -> i32 {
    let Some(subcommand) = args.first().map(String::as_str) else {
        eprintln!("{usage}");
        return 2;
    };
    let rest = &args[1..];
    match subcommand {
        "sample" => sample_bundle(),
        "validate" => validate_bundle(rest),
        "persist" => persist_bundle(rest),
        "inspect" => inspect_bundle(rest),
        _ => {
            eprintln!("{usage}");
            2
        }
    }
}

fn sample_bundle() -> i32 {
    let bundle = ManifestCatalogBundle::sample();
    match serde_json::to_string_pretty(&bundle) {
        Ok(json) => {
            println!("{json}");
            0
        }
        Err(error) => {
            eprintln!("mesh contracts sample error: serialize failed: {error}");
            1
        }
    }
}

fn validate_bundle(args: &[String]) -> i32 {
    let input = match extract_input(args) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh contracts validate error: {error}");
            return 2;
        }
    };
    let bundle: ManifestCatalogBundle = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh contracts validate error: json parse failed: {error}");
            return 2;
        }
    };
    if let Err(error) = bundle.validate() {
        eprintln!("mesh contracts validate error: {error}");
        return 2;
    }
    println!("mesh contracts validate: ok");
    0
}

fn persist_bundle(args: &[String]) -> i32 {
    let input = match extract_input(args) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh contracts persist error: {error}");
            return 2;
        }
    };
    let bundle: ManifestCatalogBundle = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh contracts persist error: json parse failed: {error}");
            return 2;
        }
    };
    if let Err(error) = bundle.validate() {
        eprintln!("mesh contracts persist error: {error}");
        return 2;
    }
    let out_path = match extract_output_path(args) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("mesh contracts persist error: {error}");
            return 2;
        }
    };
    let document = CatalogPersistenceDocument {
        schema_version: chimera_contracts::ContractVersion::default(),
        kind: CatalogPersistenceKind::DiscoverySnapshot,
        source: "chimera mesh contracts persist".to_string(),
        saved_at_unix: now_unix(),
        bundle,
    };
    if let Err(error) = document.validate() {
        eprintln!("mesh contracts persist error: {error}");
        return 2;
    }
    let json = match serde_json::to_string_pretty(&document) {
        Ok(json) => json,
        Err(error) => {
            eprintln!("mesh contracts persist error: serialize failed: {error}");
            return 1;
        }
    };
    if let Err(error) = fs::write(&out_path, json) {
        eprintln!("mesh contracts persist error: write failed: {error}");
        return 1;
    }
    println!("mesh contracts persist: ok");
    println!("out={out_path}");
    0
}

fn inspect_bundle(args: &[String]) -> i32 {
    let input = match extract_input(args) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh contracts inspect error: {error}");
            return 2;
        }
    };
    let document: CatalogPersistenceDocument = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh contracts inspect error: json parse failed: {error}");
            return 2;
        }
    };
    if let Err(error) = document.validate() {
        eprintln!("mesh contracts inspect error: {error}");
        return 2;
    }
    println!("kind={:?}", document.kind);
    println!("source={}", document.source);
    println!("saved_at_unix={}", document.saved_at_unix);
    println!("node_registry={}", document.bundle.node_registry.node_id);
    println!(
        "public_surface={}",
        document.bundle.public_surface.public_id
    );
    println!("miniapp={}", document.bundle.miniapp.app_id);
    println!("catalog_index={}", document.bundle.index_record.record_id);
    0
}

fn extract_input(args: &[String]) -> Result<String, String> {
    if let Some(path) = extract_flag_value(args, "--input") {
        return fs::read_to_string(path).map_err(|error| format!("read input failed: {error}"));
    }
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("read stdin failed: {error}"))?;
    if input.trim().is_empty() {
        return Err("--input <file> or stdin content is required".to_string());
    }
    Ok(input)
}

fn extract_flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].as_str())
}

fn extract_output_path(args: &[String]) -> Result<String, String> {
    extract_flag_value(args, "--out")
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "--out <file> is required".to_string())
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
