use chimera_mesh::MeshCarrierLaneBinding;

use crate::peer_egress::options::split_host_port;
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};

#[derive(Clone, PartialEq, Eq)]
pub struct TransitLaneRegistration {
    binding: TransitPathBinding,
    endpoint: String,
}

impl TransitLaneRegistration {
    pub fn new(binding: TransitPathBinding, endpoint: String) -> Result<Self, String> {
        let endpoint = endpoint.trim();
        split_host_port(endpoint)
            .map_err(|error| format!("sealed transit lane endpoint invalid: {error}"))?;
        Ok(Self {
            binding,
            endpoint: endpoint.to_string(),
        })
    }

    pub fn binding(&self) -> TransitPathBinding {
        self.binding
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

impl std::fmt::Debug for TransitLaneRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitLaneRegistration")
            .field("binding", &self.binding)
            .field("endpoint", &"<redacted>")
            .finish()
    }
}

pub fn transit_path_binding_from_mesh_lane(
    binding: &MeshCarrierLaneBinding,
) -> Result<TransitPathBinding, String> {
    Ok(TransitPathBinding::new(
        TransitRouteId::new(binding.route_binding_id.get())?,
        TransitLaneId::from_zero_based_lane_index(binding.lane_id)?,
    ))
}

pub fn transit_lane_registration_from_mesh_lane(
    binding: &MeshCarrierLaneBinding,
) -> Result<TransitLaneRegistration, String> {
    TransitLaneRegistration::new(
        transit_path_binding_from_mesh_lane(binding)?,
        binding.carrier_endpoint.clone(),
    )
}

pub fn parse_transit_lane_registrations(
    input: &str,
) -> Result<Vec<TransitLaneRegistration>, String> {
    let mut registrations = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (index, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split(',').map(str::trim).collect();
        if parts.len() != 3 {
            return Err(format!(
                "sealed transit lane binding line {} must be route_id,lane_index,endpoint",
                index + 1
            ));
        }
        let route_id = parts[0]
            .parse::<u64>()
            .map_err(|_| format!("sealed transit route id invalid on line {}", index + 1))?;
        let lane_index = parts[1]
            .parse::<usize>()
            .map_err(|_| format!("sealed transit lane index invalid on line {}", index + 1))?;
        let binding = TransitPathBinding::new(
            TransitRouteId::new(route_id)?,
            TransitLaneId::from_zero_based_lane_index(lane_index)?,
        );
        if !seen.insert(binding) {
            return Err("sealed transit path binding ambiguous".to_string());
        }
        registrations.push(TransitLaneRegistration::new(binding, parts[2].to_string())?);
    }
    Ok(registrations)
}

pub fn load_transit_lane_registrations(path: &str) -> Result<Vec<TransitLaneRegistration>, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|error| format!("read sealed transit lane bindings failed: {error}"))?;
    let registrations = parse_transit_lane_registrations(&contents)?;
    if registrations.is_empty() {
        return Err("sealed transit lane bindings file has no registrations".to_string());
    }
    Ok(registrations)
}

#[cfg(test)]
mod tests {
    use super::{
        parse_transit_lane_registrations, transit_lane_registration_from_mesh_lane,
        transit_path_binding_from_mesh_lane,
    };
    use chimera_mesh::{MeshCarrierLaneBinding, MeshMultipathLaneRole, MeshRouteBindingId};

    fn mesh_binding(route: u64, lane: usize) -> MeshCarrierLaneBinding {
        MeshCarrierLaneBinding {
            route_binding_id: MeshRouteBindingId::new(route)
                .unwrap_or_else(|error| unreachable!("{error}")),
            lane_id: lane,
            peer_node_id: "node-sensitive".to_string(),
            carrier_endpoint: "198.51.100.10:443".to_string(),
            role: MeshMultipathLaneRole::Active,
            weight_pct: 100,
        }
    }

    #[test]
    fn mesh_lane_binding_maps_zero_based_lane_to_carrier_nonzero_lane() -> Result<(), String> {
        let binding = transit_path_binding_from_mesh_lane(&mesh_binding(77, 0))?;

        assert_eq!(binding.route_id().get(), 77);
        assert_eq!(binding.lane_id().get(), 1);
        Ok(())
    }

    #[test]
    fn mesh_lane_registration_uses_redacted_endpoint_and_matching_binding() -> Result<(), String> {
        let mesh = mesh_binding(77, 0);
        let registration = transit_lane_registration_from_mesh_lane(&mesh)?;
        let debug = format!("{registration:?}");

        assert_eq!(registration.binding().route_id().get(), 77);
        assert_eq!(registration.binding().lane_id().get(), 1);
        assert_eq!(registration.endpoint(), "198.51.100.10:443");
        assert!(!debug.contains("198.51.100.10:443"));
        assert!(!debug.contains("77"));
        assert!(debug.contains("<opaque>"));
        assert!(debug.contains("<redacted>"));
        Ok(())
    }

    #[test]
    fn lane_registration_config_parses_and_rejects_duplicate_bindings() -> Result<(), String> {
        let parsed = parse_transit_lane_registrations(
            "# route,lane,endpoint\n77,0,198.51.100.10:443\n77,1,198.51.100.11:443\n",
        )?;

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].binding().route_id().get(), 77);
        assert_eq!(parsed[0].binding().lane_id().get(), 1);
        assert_eq!(parsed[1].binding().lane_id().get(), 2);

        let error = match parse_transit_lane_registrations(
            "77,0,198.51.100.10:443\n77,0,198.51.100.11:443\n",
        ) {
            Ok(_) => return Err("duplicate transit binding must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("ambiguous"));
        Ok(())
    }

    #[test]
    fn lane_registration_config_rejects_zero_route_and_bad_endpoint() {
        assert!(parse_transit_lane_registrations("0,0,198.51.100.10:443\n").is_err());
        assert!(parse_transit_lane_registrations("77,0,not-an-endpoint\n").is_err());
    }

    #[test]
    fn mesh_lane_binding_rejects_lane_id_overflow() {
        let error = match transit_path_binding_from_mesh_lane(&mesh_binding(77, usize::MAX)) {
            Ok(_) => "unexpected success".to_string(),
            Err(error) => error,
        };

        assert!(error.contains("lane binding index overflow"));
    }

    #[test]
    fn mesh_lane_binding_debug_redacts_route_peer_and_endpoint() -> Result<(), String> {
        let binding = mesh_binding(77, 0);
        let debug = format!("{binding:?}");

        assert!(debug.contains("<opaque>"));
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("77"));
        assert!(!debug.contains("node-sensitive"));
        assert!(!debug.contains("198.51.100.10:443"));
        Ok(())
    }
}
