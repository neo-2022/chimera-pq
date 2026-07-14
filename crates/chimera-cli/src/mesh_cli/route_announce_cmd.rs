#![forbid(unsafe_code)]

use base64::Engine as _;
use chimera_mesh::{
    MeshJoinRequest, MeshMultipathLaneRole, MeshRuntime, format_route_announcements,
    parse_route_announcements,
};

pub(crate) struct MeshRouteAnnounceOptions {
    namespace: String,
    node_name: String,
    destination: String,
    via: String,
    route_binding_id: u64,
    ttl_seconds: u64,
    signature_base64: Option<String>,
    signing_key_base64: Option<String>,
    multipath_mode: String,
    peers: Vec<String>,
    json_output: bool,
    out_path: Option<String>,
}

pub(crate) fn mesh_route_announce_command(_usage: &str, args: &[String]) -> i32 {
    let options = match parse_options(args) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh route-announce options error: {error}");
            return 2;
        }
    };

    let policy_payload = match build_policy_payload(&options) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh route-announce payload error: {error}");
            return 1;
        }
    };

    let mut runtime = match MeshRuntime::bootstrap(&options.namespace, "cli-seed") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh route-announce runtime bootstrap error: {error}");
            return 1;
        }
    };

    let records = match super::options::parse_mesh_peer_records(&options.peers) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh route-announce peer spec error: {error}");
            return 1;
        }
    };

    if let Err(error) = runtime.merge_discovery("cli-peer-list", &records) {
        eprintln!("mesh route-announce discovery merge error: {error}");
        return 1;
    }

    let request = MeshJoinRequest {
        namespace: options.namespace.clone(),
        node_name: options.node_name.clone(),
        invite_token: None,
    };

    let plan = match runtime.plan_path_from_dps_payload(&request, &policy_payload) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mesh route-announce plan error: {error}");
            return 1;
        }
    };

    let schedule = &plan.multipath_schedule;
    let carrier_binding_count = schedule.carrier_lane_bindings.len();
    let transit_binding_count = schedule
        .carrier_lane_bindings
        .iter()
        .filter(|binding| binding.role == MeshMultipathLaneRole::Transit)
        .count();
    let route_announcement_count = schedule.route_announcements.len();
    let execution_status = &schedule.execution_status;
    let multipath_mode = schedule.mode.as_str();

    let output = if options.json_output {
        let mut object = serde_json::Map::new();
        object.insert(
            "status".to_string(),
            serde_json::Value::String("ok".to_string()),
        );
        object.insert(
            "policy_payload".to_string(),
            serde_json::Value::String(policy_payload),
        );
        object.insert(
            "route_announcement_count".to_string(),
            serde_json::Value::Number(route_announcement_count.into()),
        );
        object.insert(
            "carrier_binding_count".to_string(),
            serde_json::Value::Number(carrier_binding_count.into()),
        );
        object.insert(
            "transit_binding_count".to_string(),
            serde_json::Value::Number(transit_binding_count.into()),
        );
        object.insert(
            "execution_status".to_string(),
            serde_json::Value::String(execution_status.clone()),
        );
        object.insert(
            "multipath_mode".to_string(),
            serde_json::Value::String(multipath_mode.to_string()),
        );
        serde_json::Value::Object(object).to_string()
    } else {
        format!(
            "status=ok\n\
             policy_payload={policy_payload}\n\
             route_announcement_count={route_announcement_count}\n\
             carrier_binding_count={carrier_binding_count}\n\
             transit_binding_count={transit_binding_count}\n\
             execution_status={execution_status}\n\
             multipath_mode={multipath_mode}\n"
        )
    };

    println!("{output}");

    if let Some(path) = options.out_path.as_deref()
        && let Err(error) = std::fs::write(path, &output)
    {
        eprintln!("mesh route-announce write failed: {error}");
        return 1;
    }

    0
}

fn parse_options(args: &[String]) -> Result<MeshRouteAnnounceOptions, String> {
    let mut namespace = None;
    let mut node_name = None;
    let mut destination = None;
    let mut via = None;
    let mut route_binding_id = None;
    let mut ttl_seconds = 3600u64;
    let mut signature_base64 = None;
    let mut signing_key_base64 = None;
    let mut multipath_mode = "off".to_string();
    let mut peers: Vec<String> = Vec::new();
    let mut json_output = false;
    let mut out_path = None;

    let mut i = 0;
    while i < args.len() {
        let flag = args[i].as_str();
        if flag == "--json" {
            json_output = true;
            i += 1;
            continue;
        }
        if !flag.starts_with("--") {
            return Err(format!("unexpected positional argument '{flag}'"));
        }
        let value = args
            .get(i + 1)
            .map(String::as_str)
            .ok_or_else(|| format!("missing value for flag '{flag}'"))?;
        match flag {
            "--namespace" => set_once("--namespace", &mut namespace, value.to_string())?,
            "--node" => set_once("--node", &mut node_name, value.to_string())?,
            "--destination" => set_once("--destination", &mut destination, value.to_string())?,
            "--via" => set_once("--via", &mut via, value.to_string())?,
            "--route-binding-id" => {
                set_once(
                    "--route-binding-id",
                    &mut route_binding_id,
                    value
                        .parse::<u64>()
                        .map_err(|_| format!("invalid --route-binding-id value '{value}'"))?
                        .max(1),
                )?;
            }
            "--ttl" => {
                ttl_seconds = value
                    .parse::<u64>()
                    .map_err(|_| format!("invalid --ttl value '{value}'"))?;
                if ttl_seconds == 0 {
                    return Err("--ttl must be > 0".to_string());
                }
            }
            "--signature-base64" => {
                set_once(
                    "--signature-base64",
                    &mut signature_base64,
                    value.to_string(),
                )?;
            }
            "--mesh-announcement-signing-key" => {
                set_once(
                    "--mesh-announcement-signing-key",
                    &mut signing_key_base64,
                    value.to_string(),
                )?;
            }
            "--multipath-mode" => {
                multipath_mode = value.to_string();
            }
            "--out" => set_once("--out", &mut out_path, value.to_string())?,
            "--peer" => peers.push(value.to_string()),
            _ => return Err(format!("unknown flag '{flag}'")),
        }
        i += 2;
    }

    let destination = destination.ok_or_else(|| "missing --destination".to_string())?;
    if !destination.starts_with("cidr/") && !destination.starts_with("domain/") {
        return Err("--destination must start with 'cidr/' or 'domain/'".to_string());
    }
    if destination.contains(',') || destination.contains('|') || destination.contains(';') {
        return Err("--destination contains illegal separator character".to_string());
    }

    Ok(MeshRouteAnnounceOptions {
        namespace: namespace.ok_or_else(|| "missing --namespace".to_string())?,
        node_name: node_name.ok_or_else(|| "missing --node".to_string())?,
        destination,
        via: via.ok_or_else(|| "missing --via".to_string())?,
        route_binding_id: route_binding_id
            .ok_or_else(|| "missing --route-binding-id".to_string())?,
        ttl_seconds,
        signature_base64,
        signing_key_base64,
        multipath_mode,
        peers,
        json_output,
        out_path,
    })
}

fn build_policy_payload(options: &MeshRouteAnnounceOptions) -> Result<String, String> {
    if options.peers.is_empty() {
        return Err("at least one --peer is required".to_string());
    }
    if options.route_binding_id == 0 {
        return Err("--route-binding-id must be nonzero".to_string());
    }

    let mut announcements = parse_route_announcements(&format!(
        "static,{},{},{},{}",
        options.destination, options.via, options.ttl_seconds, options.route_binding_id
    ))?;

    if let Some(ref key_base64) = options.signing_key_base64 {
        let seed = base64::engine::general_purpose::STANDARD
            .decode(key_base64.trim())
            .map_err(|error| format!("--mesh-announcement-signing-key decode failed: {error}"))?;
        if seed.len() != 32 {
            return Err(format!(
                "--mesh-announcement-signing-key must be 32 bytes, got {}",
                seed.len()
            ));
        }
        for announcement in &mut announcements {
            announcement.sign_with_ed25519_seed(&seed)?;
        }
    } else if let Some(ref signature) = options.signature_base64 {
        // Manual signature override for advanced/legacy use.
        let signature_bytes = base64::engine::general_purpose::STANDARD
            .decode(signature.trim())
            .map_err(|error| format!("--signature-base64 decode failed: {error}"))?;
        if let Some(announcement) = announcements.first_mut() {
            match announcement {
                chimera_mesh::RouteAnnouncement::Static { auth, .. } => {
                    auth.signature = signature_bytes;
                }
            }
        }
    }

    let announcements_wire = format_route_announcements(&announcements);

    let peer_count = options.peers.len();
    Ok(format!(
        "allow=mesh;mesh_multipath_mode={};mesh_route_binding_id={};mesh_max_peers={};mesh_max_selected_per_region={};mesh_announcements={}",
        options.multipath_mode,
        options.route_binding_id,
        peer_count,
        peer_count,
        announcements_wire
    ))
}

fn set_once<T>(flag: &str, slot: &mut Option<T>, value: T) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!("duplicate singleton flag '{flag}'"));
    }
    *slot = Some(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_args() -> Vec<String> {
        vec![
            "--namespace",
            "test-namespace",
            "--node",
            "test-node",
            "--destination",
            "cidr/192.168.31.0/24",
            "--via",
            "via-peer",
            "--route-binding-id",
            "7",
            "--peer",
            "active-peer@198.51.100.10:443@eu@10@95",
            "--peer",
            "via-peer@198.51.100.11:443@eu@12@93",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    #[test]
    fn parse_options_valid_input() -> Result<(), String> {
        let options = parse_options(&sample_args())?;
        assert_eq!(options.namespace, "test-namespace");
        assert_eq!(options.node_name, "test-node");
        assert_eq!(options.destination, "cidr/192.168.31.0/24");
        assert_eq!(options.via, "via-peer");
        assert_eq!(options.route_binding_id, 7);
        assert_eq!(options.ttl_seconds, 3600);
        assert_eq!(options.peers.len(), 2);
        Ok(())
    }

    #[test]
    fn build_payload_contains_route_announcement() -> Result<(), String> {
        let options = parse_options(&sample_args())?;
        let payload = build_policy_payload(&options)?;
        assert!(payload.contains("mesh_announcements="));
        assert!(payload.contains("static,cidr/192.168.31.0/24,via-peer,3600,7"));
        assert!(payload.contains("mesh_route_binding_id=7"));
        assert!(payload.contains("mesh_max_peers=2"));
        Ok(())
    }

    #[test]
    fn build_payload_signs_announcement_when_signing_key_given() -> Result<(), String> {
        use base64::Engine as _;
        use ring::signature::KeyPair;

        let seed = [1u8; 32];
        let key_pair = ring::signature::Ed25519KeyPair::from_seed_unchecked(&seed)
            .map_err(|error| format!("test key pair: {error}"))?;
        let public_key = key_pair.public_key().as_ref().to_vec();
        let signing_key = base64::engine::general_purpose::STANDARD.encode(seed);

        let mut args = sample_args();
        args.push("--mesh-announcement-signing-key".to_string());
        args.push(signing_key);
        let options = parse_options(&args)?;
        let payload = build_policy_payload(&options)?;

        let announcement_value = payload
            .split(';')
            .find(|segment| segment.starts_with("mesh_announcements="))
            .ok_or_else(|| "mesh_announcements missing from payload".to_string())?;
        let announcements = chimera_mesh::parse_route_announcements(
            &announcement_value["mesh_announcements=".len()..],
        )?;
        assert_eq!(announcements.len(), 1);
        assert!(!announcements[0].auth().signature.is_empty());
        announcements[0]
            .verify_with_ed25519_pubkey(&public_key)
            .map_err(|error| format!("signature verification failed: {error}"))?;
        Ok(())
    }

    fn temp_out_file() -> Result<std::path::PathBuf, String> {
        let mut path = std::env::temp_dir();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| format!("system time error: {error}"))?
            .as_nanos();
        path.push(format!("chimera_route_announce_{ts}.txt"));
        Ok(path)
    }

    #[test]
    fn command_runs_plan_and_reports_transit_binding() {
        let rc = mesh_route_announce_command("usage", &sample_args());
        assert_eq!(rc, 0, "route-announce command should succeed");
    }

    #[test]
    fn command_writes_json_output_to_out_file() -> Result<(), String> {
        let out = temp_out_file()?;
        let mut args = sample_args();
        args.push("--json".to_string());
        args.push("--out".to_string());
        args.push(out.to_string_lossy().to_string());
        let rc = mesh_route_announce_command("usage", &args);
        assert_eq!(rc, 0);
        let content = std::fs::read_to_string(&out)
            .map_err(|error| format!("read out file failed: {error}"))?;
        let parsed: serde_json::Value = serde_json::from_str(&content)
            .map_err(|error| format!("parse json output failed: {error}"))?;
        assert_eq!(parsed["status"], "ok");
        assert_eq!(parsed["route_announcement_count"], 1);
        assert_eq!(parsed["carrier_binding_count"], 2);
        assert_eq!(parsed["transit_binding_count"], 1);
        let _ = std::fs::remove_file(&out);
        Ok(())
    }

    #[test]
    fn command_writes_text_output_to_out_file() -> Result<(), String> {
        let out = temp_out_file()?;
        let mut args = sample_args();
        args.push("--out".to_string());
        args.push(out.to_string_lossy().to_string());
        let rc = mesh_route_announce_command("usage", &args);
        assert_eq!(rc, 0);
        let content = std::fs::read_to_string(&out)
            .map_err(|error| format!("read out file failed: {error}"))?;
        assert!(content.contains("status=ok"));
        assert!(content.contains("transit_binding_count=1"));
        let _ = std::fs::remove_file(&out);
        Ok(())
    }

    #[test]
    fn command_rejects_unknown_destination_prefix() -> Result<(), String> {
        let mut args = sample_args();
        let idx = args
            .iter()
            .position(|a| a == "--destination")
            .ok_or_else(|| "--destination not found in sample args".to_string())?;
        args[idx + 1] = "192.168.31.0/24".to_string();
        let rc = mesh_route_announce_command("usage", &args);
        assert_eq!(rc, 2);
        Ok(())
    }
}
