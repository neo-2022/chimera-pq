use std::env;
use std::fs;
use std::process::Command;

fn chimera_cli_bin() -> String {
    env::var("CARGO_BIN_EXE_chimera-cli").unwrap_or_else(|_| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/debug/chimera-cli"
        )
        .to_string()
    })
}

fn random_suffix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

#[test]
fn advertise_uses_country_code_env_var() {
    let bin = chimera_cli_bin();
    let suffix = random_suffix();
    let out = env::temp_dir().join(format!("chimera_advertise_country_env_{suffix}.json"));
    let pubkey = env::temp_dir().join(format!("chimera_advertise_country_env_{suffix}.pub"));
    let keypair = env::temp_dir().join(format!("chimera_advertise_country_env_{suffix}.keypair"));

    let output = Command::new(&bin)
        .env("CHIMERA_MESH_LOCAL_NODE_COUNTRY_CODE", "RU")
        .args([
            "mesh",
            "nodes",
            "advertise",
            "--node-id",
            "node-ru-env",
            "--endpoint",
            "198.51.100.55:12345",
            "--out",
            out.to_str().unwrap(),
            "--pubkey-out",
            pubkey.to_str().unwrap(),
            "--keypair-path",
            keypair.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute chimera-cli");

    assert!(
        output.status.success(),
        "advertise failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = fs::read_to_string(&out).expect("discovery output missing");
    assert!(body.contains("\"node_id\":\"node-ru-env\""), "{body}");
    assert!(body.contains("\"country_code\":\"RU\""), "{body}");

    let _ = fs::remove_file(&out);
    let _ = fs::remove_file(&pubkey);
    let _ = fs::remove_file(&keypair);
}
