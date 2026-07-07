use chimera_mesh::{REQUIRED_WEAVE_NODE_CAPABILITIES, WeaveNodeCapability, WeaveNodeContract};
use std::path::Path;

use crate::peer_egress::options::{Mode, Options, split_host_port};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeStartupContract {
    pub mode: Mode,
    pub local_listen: String,
    pub peer_listen: String,
    pub outbound_bootstrap_configured: bool,
    pub pool_transit_allowed: bool,
    pub bound_transit_allowed: bool,
    pub capabilities: Vec<WeaveNodeCapability>,
}

impl NodeStartupContract {
    pub fn capability_names(&self) -> Vec<&'static str> {
        self.capabilities
            .iter()
            .map(|capability| capability.as_str())
            .collect()
    }
}

pub fn validate_node_startup_contract(options: &Options) -> Result<NodeStartupContract, String> {
    validate_node_startup_contract_with_contract(options, WeaveNodeContract::symmetric_mesh_node())
}

fn validate_node_startup_contract_with_contract(
    options: &Options,
    contract: WeaveNodeContract,
) -> Result<NodeStartupContract, String> {
    if options.mode != Mode::Node {
        return Err("WEAVE node startup contract requires node mode".to_string());
    }

    let local_listen = required_node_listener(&options.local_listen, "local_listen")?;
    let peer_listen = required_node_listener(&options.peer_listen, "peer_listen")?;
    if !options.server.trim().is_empty() {
        let _ = split_host_port(options.server.trim())
            .map_err(|error| format!("WEAVE node outbound bootstrap invalid: {error}"))?;
    }
    if options.transit_lane_bindings_file.is_some() && !options.allow_bound_transit {
        return Err(
            "WEAVE node sealed transit lane bindings require allow_bound_transit=true".to_string(),
        );
    }
    if options.allow_bound_transit {
        let lane_file = options
            .transit_lane_bindings_file
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let server_empty = options.server.trim().is_empty();
        let discovery_configured = options.discovery_configured();
        let lane_file_ready = lane_file.map_or(false, |path| {
            std::fs::metadata(Path::new(path))
                .map(|metadata| metadata.is_file() && metadata.len() > 0)
                .unwrap_or(false)
        });
        if !lane_file_ready {
            let bootstrap_pending = server_empty
                && (discovery_configured || lane_file.is_none());
            if bootstrap_pending {
                // Fresh installs or dynamic-discovery nodes may enable bound transit
                // before a first lane document exists. Allow the node to boot as a
                // listener until authority appears.
                contract
                    .validate_symmetric()
                    .map_err(|error| format!("WEAVE node symmetric contract invalid: {error}"))?;

                let capabilities = REQUIRED_WEAVE_NODE_CAPABILITIES.to_vec();

                return Ok(NodeStartupContract {
                    mode: options.mode.clone(),
                    local_listen,
                    peer_listen,
                    outbound_bootstrap_configured: false,
                    pool_transit_allowed: options.allow_pool_transit,
                    bound_transit_allowed: options.allow_bound_transit,
                    capabilities,
                });
            }
            return Err(
                "WEAVE node bound transit requires a non-empty transit lane bindings file when allow_bound_transit=true"
                    .to_string(),
            );
        }
    }
    contract
        .validate_symmetric()
        .map_err(|error| format!("WEAVE node symmetric contract invalid: {error}"))?;

    let capabilities = REQUIRED_WEAVE_NODE_CAPABILITIES.to_vec();

    Ok(NodeStartupContract {
        mode: options.mode.clone(),
        local_listen,
        peer_listen,
        outbound_bootstrap_configured: !options.server.trim().is_empty(),
        pool_transit_allowed: options.allow_pool_transit,
        bound_transit_allowed: options.allow_bound_transit,
        capabilities,
    })
}

fn required_node_listener(value: &str, name: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("WEAVE node startup requires {name}"));
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer_egress::options::Options;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn write_lane_file_fixture() -> Result<String, String> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "chimera-startup-contract-{}-{stamp}.csv",
            std::process::id()
        ));
        std::fs::write(&path, "7001,0,198.51.100.10:443\n").map_err(|error| error.to_string())?;
        Ok(path.display().to_string())
    }

    fn node_options(server: &str) -> Options {
        Options {
            mode: Mode::Node,
            local_listen: "127.0.0.1:18135".to_string(),
            peer_listen: "0.0.0.0:8443".to_string(),
            state_file: None,
            server: server.to_string(),
            token: "abc".to_string(),
            pool: 8,
            bench_bytes: 1024,
            target: String::new(),
            connect_timeout_ms: 1_000,
            min_throughput_mib_s: 0,
            connections: 1,
            aead: crate::peer_egress::options::AeadSuite::Chacha20Poly1305,
            reverse_connect: false,
            allow_pool_transit: false,
            allow_bound_transit: false,
            transit_lane_bindings_file: None,
            transit_max_frames_per_direction:
                crate::peer_egress::transit_guard::DEFAULT_TRANSIT_MAX_FRAMES_PER_DIRECTION,
            transit_max_bytes_per_direction:
                crate::peer_egress::transit_guard::DEFAULT_TRANSIT_MAX_BYTES_PER_DIRECTION,
            transit_idle_timeout_ms:
                crate::peer_egress::transit_guard::DEFAULT_TRANSIT_IDLE_TIMEOUT_MS,
            transit_payload_bytes: 64,
            transit_packet_number: 1,
            transit_route_id: None,
            transit_lane_index: None,
            discovery_url: None,
            discovery_pubkey: None,
            discovery_keyring: None,
            mesh_namespace: "chimera-mesh".to_string(),
            mesh_self_node_id: "test".to_string(),
            mesh_policy_payload: String::new(),
            lane_document_path: None,
            discovery_poll_interval_ms: 30_000,
            discovery_timeout_ms: 5_000,
        }
    }

    #[test]
    fn node_startup_contract_accepts_symmetric_node_without_server() -> Result<(), String> {
        let contract = validate_node_startup_contract(&node_options(""))?;

        assert_eq!(contract.mode, Mode::Node);
        assert_eq!(contract.local_listen, "127.0.0.1:18135");
        assert_eq!(contract.peer_listen, "0.0.0.0:8443");
        assert!(!contract.outbound_bootstrap_configured);
        assert!(!contract.pool_transit_allowed);
        assert!(!contract.bound_transit_allowed);
        assert_eq!(
            contract.capability_names(),
            vec![
                "local_ingress",
                "peer_ingress",
                "local_egress",
                "peer_transit",
            ]
        );
        Ok(())
    }

    #[test]
    fn node_startup_contract_marks_optional_bootstrap_server() -> Result<(), String> {
        let contract = validate_node_startup_contract(&node_options("peer.example.invalid:8443"))?;

        assert!(contract.outbound_bootstrap_configured);
        assert_eq!(contract.mode, Mode::Node);
        Ok(())
    }

    #[test]
    fn node_startup_contract_marks_explicit_pool_transit_policy() -> Result<(), String> {
        let mut options = node_options("peer.example.invalid:8443");
        options.allow_pool_transit = true;
        let contract = validate_node_startup_contract(&options)?;

        assert!(contract.pool_transit_allowed);
        assert!(!contract.bound_transit_allowed);
        Ok(())
    }

    #[test]
    fn node_startup_contract_marks_explicit_bound_transit_policy() -> Result<(), String> {
        let mut options = node_options("peer.example.invalid:8443");
        options.allow_bound_transit = true;
        options.transit_lane_bindings_file = Some(write_lane_file_fixture()?);
        let contract = validate_node_startup_contract(&options)?;

        assert!(!contract.pool_transit_allowed);
        assert!(contract.bound_transit_allowed);
        Ok(())
    }

    #[test]
    fn node_startup_contract_rejects_bound_transit_without_lane_bindings() {
        let mut options = node_options("peer.example.invalid:8443");
        options.allow_bound_transit = true;

        let result = validate_node_startup_contract(&options);

        assert!(result.is_err_and(|error| error.contains("allow_bound_transit=true")));
    }

    #[test]
    fn node_startup_contract_allows_clean_install_bound_transit_without_lane_bindings()
    -> Result<(), String> {
        let mut options = node_options("");
        options.allow_bound_transit = true;

        let contract = validate_node_startup_contract(&options)?;

        assert!(contract.bound_transit_allowed);
        assert!(!contract.outbound_bootstrap_configured);
        Ok(())
    }

    #[test]
    fn node_startup_contract_rejects_lane_bindings_without_bound_transit_policy() {
        let mut options = node_options("peer.example.invalid:8443");
        options.transit_lane_bindings_file = Some("/tmp/chimera-test-lanes.csv".to_string());

        let result = validate_node_startup_contract(&options);

        assert!(result.is_err_and(|error| error.contains("allow_bound_transit=true")));
    }

    #[test]
    fn node_startup_contract_rejects_lane_bindings_with_pool_transit_only() {
        let mut options = node_options("peer.example.invalid:8443");
        options.allow_pool_transit = true;
        options.transit_lane_bindings_file = Some("/tmp/chimera-test-lanes.csv".to_string());

        let result = validate_node_startup_contract(&options);

        assert!(result.is_err_and(|error| error.contains("allow_bound_transit=true")));
    }

    #[test]
    fn node_startup_contract_accepts_lane_bindings_with_bound_transit_policy() -> Result<(), String>
    {
        let mut options = node_options("peer.example.invalid:8443");
        options.allow_bound_transit = true;
        options.transit_lane_bindings_file = Some(write_lane_file_fixture()?);

        let contract = validate_node_startup_contract(&options)?;

        assert!(contract.bound_transit_allowed);
        Ok(())
    }

    #[test]
    fn node_startup_contract_rejects_invalid_bootstrap_server_shape() {
        let result = validate_node_startup_contract(&node_options("not-an-endpoint"));
        assert!(result.is_err_and(|error| error.contains("bootstrap invalid")));
    }

    #[test]
    fn node_startup_contract_rejects_non_node_mode() {
        let mut options = node_options("");
        options.mode = Mode::SideA;
        let result = validate_node_startup_contract(&options);

        assert!(result.is_err_and(|error| error.contains("node mode")));
    }

    #[test]
    fn node_startup_contract_rejects_missing_local_ingress() {
        let mut options = node_options("");
        options.local_listen = " ".to_string();

        let result = validate_node_startup_contract(&options);

        assert!(result.is_err_and(|error| error.contains("local_listen")));
    }

    #[test]
    fn node_startup_contract_rejects_missing_peer_ingress() {
        let mut options = node_options("");
        options.peer_listen = String::new();

        let result = validate_node_startup_contract(&options);

        assert!(result.is_err_and(|error| error.contains("peer_listen")));
    }

    #[test]
    fn node_startup_contract_rejects_invalid_symmetric_contract() {
        let partial = WeaveNodeContract::from_capabilities([
            WeaveNodeCapability::LocalIngress,
            WeaveNodeCapability::PeerIngress,
            WeaveNodeCapability::LocalEgress,
        ]);

        let result = validate_node_startup_contract_with_contract(&node_options(""), partial);

        assert!(result.is_err_and(
            |error| error.contains("symmetric contract") && error.contains("peer_transit")
        ));
    }
}
