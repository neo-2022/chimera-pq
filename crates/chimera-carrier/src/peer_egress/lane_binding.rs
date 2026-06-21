use crate::peer_egress::options::split_host_port;
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};
use chimera_mesh::{MeshCarrierLaneBinding, MeshMultipathLaneRole};

#[derive(Clone, PartialEq, Eq)]
pub struct TransitLaneRegistration {
    binding: TransitPathBinding,
    endpoint: String,
    role: Option<MeshMultipathLaneRole>,
    weight_pct: Option<u8>,
    capacity_weight_pct: Option<u8>,
}

impl TransitLaneRegistration {
    pub fn new(binding: TransitPathBinding, endpoint: String) -> Result<Self, String> {
        Self::new_with_lane_plan(binding, endpoint, None, None, None)
    }

    pub fn new_with_lane_plan(
        binding: TransitPathBinding,
        endpoint: String,
        role: Option<MeshMultipathLaneRole>,
        weight_pct: Option<u8>,
        capacity_weight_pct: Option<u8>,
    ) -> Result<Self, String> {
        let endpoint = endpoint.trim();
        split_host_port(endpoint)
            .map_err(|error| format!("sealed transit lane endpoint invalid: {error}"))?;
        Ok(Self {
            binding,
            endpoint: endpoint.to_string(),
            role,
            weight_pct,
            capacity_weight_pct,
        })
    }

    pub fn binding(&self) -> TransitPathBinding {
        self.binding
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn role(&self) -> Option<MeshMultipathLaneRole> {
        self.role.clone()
    }

    pub fn weight_pct(&self) -> Option<u8> {
        self.weight_pct
    }

    pub fn capacity_weight_pct(&self) -> Option<u8> {
        self.capacity_weight_pct
    }
}

impl std::fmt::Debug for TransitLaneRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitLaneRegistration")
            .field("binding", &self.binding)
            .field("endpoint", &"<redacted>")
            .field("role", &self.role)
            .field("weight_pct", &self.weight_pct)
            .field("capacity_weight_pct", &self.capacity_weight_pct)
            .finish()
    }
}

pub use crate::peer_egress::lane_document::{
    TransitLaneDocument, TransitLanePlanSnapshot, load_transit_lane_document,
    load_transit_lane_registrations, parse_transit_lane_document, parse_transit_lane_registrations,
    render_transit_lane_document, render_transit_lane_registrations,
    render_transit_lane_registrations_from_mesh_plan, transit_lane_document_from_mesh_plan,
    transit_lane_registrations_from_mesh_plan, write_transit_lane_document_from_mesh_plan,
    write_transit_lane_registrations_from_mesh_plan,
};

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
    TransitLaneRegistration::new_with_lane_plan(
        transit_path_binding_from_mesh_lane(binding)?,
        binding.carrier_endpoint.clone(),
        Some(binding.role.clone()),
        Some(binding.weight_pct),
        Some(binding.capacity_weight_pct),
    )
}

#[cfg(test)]
mod tests {
    use super::{transit_lane_registration_from_mesh_lane, transit_path_binding_from_mesh_lane};
    use crate::peer_egress::parse_transit_lane_registrations;
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
            capacity_weight_pct: 90,
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
