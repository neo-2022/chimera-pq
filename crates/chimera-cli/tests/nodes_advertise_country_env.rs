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

#[test]
fn advertise_uses_country_code_env_var() -> Result<(), Box<dyn std::error::Error>> {
    let bin = chimera_cli_bin();
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos() as u64;
    let out = env::temp_dir().join(format!("chimera_advertise_country_env_{suffix}.json"));
    let pubkey = env::temp_dir().join(format!("chimera_advertise_country_env_{suffix}.pub"));
    let keypair = env::temp_dir().join(format!("chimera_advertise_country_env_{suffix}.keypair"));

    let output = Command::new(&bin)
        .env("CHIMERA_MESH_LOCAL_NODE_COUNTRY_CODE", "RU")
        .arg("mesh")
        .arg("nodes")
        .arg("advertise")
        .arg("--node-id")
        .arg("node-ru-env")
        .arg("--endpoint")
        .arg("198.51.100.55:12345")
        .arg("--out")
        .arg(&out)
        .arg("--pubkey-out")
        .arg(&pubkey)
        .arg("--keypair-path")
        .arg(&keypair)
        .output()?;

    assert!(
        output.status.success(),
        "advertise failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = fs::read_to_string(&out)?;
    assert!(body.contains("\"node_id\":\"node-ru-env\""), "{body}");
    assert!(body.contains("\"country_code\":\"RU\""), "{body}");

    let _ = fs::remove_file(&out);
    let _ = fs::remove_file(&pubkey);
    let _ = fs::remove_file(&keypair);
    Ok(())
}
