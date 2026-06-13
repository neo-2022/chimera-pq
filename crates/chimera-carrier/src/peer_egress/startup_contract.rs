use chimera_mesh::{REQUIRED_WEAVE_NODE_CAPABILITIES, WeaveNodeCapability, WeaveNodeContract};

use crate::peer_egress::options::{Mode, Options, split_host_port};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeStartupContract {
    pub mode: Mode,
    pub local_listen: String,
    pub peer_listen: String,
    pub outbound_bootstrap_configured: bool,
    pub pool_transit_allowed: bool,
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
        options.mode = Mode::Vps;
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
