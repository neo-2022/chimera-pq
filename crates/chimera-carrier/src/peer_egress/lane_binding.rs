use chimera_mesh::{MeshCarrierLaneBinding, MeshPathPlan};

use crate::peer_egress::options::split_host_port;
use crate::peer_egress::transit_binding::{TransitLaneId, TransitPathBinding, TransitRouteId};
use std::io::ErrorKind;
use std::io::Write;
use std::path::{Path, PathBuf};

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

pub fn transit_lane_registrations_from_mesh_plan(
    plan: &MeshPathPlan,
) -> Result<Vec<TransitLaneRegistration>, String> {
    let bindings = &plan.multipath_schedule.carrier_lane_bindings;
    if bindings.is_empty() {
        return Err("mesh path plan has no carrier lane bindings".to_string());
    }
    bindings
        .iter()
        .map(transit_lane_registration_from_mesh_lane)
        .collect()
}

pub fn render_transit_lane_registrations(
    registrations: &[TransitLaneRegistration],
) -> Result<String, String> {
    if registrations.is_empty() {
        return Err("sealed transit lane registrations are empty".to_string());
    }
    let mut output = String::from("# route_id,lane_index,endpoint\n");
    for registration in registrations {
        let route_id = registration.binding().route_id().get();
        let lane_index = registration
            .binding()
            .lane_id()
            .get()
            .checked_sub(1)
            .ok_or_else(|| "sealed transit lane binding id underflow".to_string())?;
        output.push_str(&format!(
            "{route_id},{lane_index},{}\n",
            registration.endpoint()
        ));
    }
    Ok(output)
}

pub fn render_transit_lane_registrations_from_mesh_plan(
    plan: &MeshPathPlan,
) -> Result<String, String> {
    let registrations = transit_lane_registrations_from_mesh_plan(plan)?;
    render_transit_lane_registrations(&registrations)
}

pub fn write_transit_lane_registrations_from_mesh_plan(
    plan: &MeshPathPlan,
    path: &str,
) -> Result<usize, String> {
    let contents = render_transit_lane_registrations_from_mesh_plan(plan)?;
    write_sensitive_text_file(Path::new(path), &contents)?;
    Ok(plan.multipath_schedule.carrier_lane_bindings.len())
}

fn write_sensitive_text_file(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "sealed transit lane bindings path has no parent directory".to_string())?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "sealed transit lane bindings path is not valid utf-8".to_string())?;
    let mut tmp_path = PathBuf::from(parent);
    tmp_path.push(format!(
        ".{file_name}.chimera-tmp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| format!("sealed transit lane bindings clock failed: {error}"))?
            .as_nanos()
    ));

    #[cfg(unix)]
    let mut file = {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&tmp_path)
            .map_err(|error| format!("write sealed transit lane bindings failed: {error}"))?
    };

    #[cfg(not(unix))]
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&tmp_path)
        .map_err(|error| format!("write sealed transit lane bindings failed: {error}"))?;

    file.write_all(contents.as_bytes())
        .map_err(|error| format!("write sealed transit lane bindings failed: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("write sealed transit lane bindings failed: {error}"))?;
    drop(file);

    std::fs::hard_link(&tmp_path, path).map_err(|error| {
        let _ = std::fs::remove_file(&tmp_path);
        if error.kind() == ErrorKind::AlreadyExists {
            return "sealed transit lane bindings target already exists".to_string();
        }
        format!("write sealed transit lane bindings failed: {error}")
    })?;
    let _ = std::fs::remove_file(&tmp_path);
    Ok(())
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
        parse_transit_lane_registrations, render_transit_lane_registrations_from_mesh_plan,
        transit_lane_registration_from_mesh_lane, transit_lane_registrations_from_mesh_plan,
        transit_path_binding_from_mesh_lane, write_transit_lane_registrations_from_mesh_plan,
    };
    use chimera_mesh::{
        MeshCarrierLaneBinding, MeshDiscoveryRecord, MeshJoinRequest, MeshMultipathLaneRole,
        MeshRouteBindingId, MeshRuntime,
    };

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

    fn multipath_plan() -> Result<chimera_mesh::MeshPathPlan, String> {
        let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")?;
        runtime.merge_discovery(
            "seed-b",
            &[
                MeshDiscoveryRecord {
                    node_id: "node-a".to_string(),
                    endpoint: "198.51.100.31:443".to_string(),
                    region: "eu".to_string(),
                    load_score: 20,
                    reliability_score: 90,
                },
                MeshDiscoveryRecord {
                    node_id: "node-b".to_string(),
                    endpoint: "198.51.100.32:443".to_string(),
                    region: "eu".to_string(),
                    load_score: 22,
                    reliability_score: 91,
                },
            ],
        )?;
        runtime.plan_path_from_dps_payload(
            &MeshJoinRequest {
                namespace: "cef-public".to_string(),
                node_name: "node-client".to_string(),
                invite_token: None,
            },
            concat!(
                "mesh_allowed_regions=eu;",
                "mesh_multipath_mode=flow_shard;",
                "mesh_route_binding_id=7003"
            ),
        )
    }

    #[test]
    fn mesh_plan_materializes_transit_lane_registrations() -> Result<(), String> {
        let plan = multipath_plan()?;
        let registrations = transit_lane_registrations_from_mesh_plan(&plan)?;

        assert_eq!(registrations.len(), 2);
        assert_eq!(registrations[0].binding().route_id().get(), 7003);
        assert_eq!(registrations[0].binding().lane_id().get(), 1);
        assert_eq!(registrations[1].binding().lane_id().get(), 2);
        let endpoints: std::collections::BTreeSet<&str> =
            registrations.iter().map(|item| item.endpoint()).collect();
        assert_eq!(
            endpoints,
            std::collections::BTreeSet::from(["198.51.100.31:443", "198.51.100.32:443"])
        );
        Ok(())
    }

    #[test]
    fn mesh_plan_registration_file_round_trips_to_node_parser() -> Result<(), String> {
        let plan = multipath_plan()?;
        let rendered = render_transit_lane_registrations_from_mesh_plan(&plan)?;
        let reparsed = parse_transit_lane_registrations(&rendered)?;

        assert!(rendered.starts_with("# route_id,lane_index,endpoint\n"));
        assert!(rendered.contains("7003,0,"));
        assert!(rendered.contains("7003,1,"));
        assert!(rendered.contains("198.51.100.31:443"));
        assert!(rendered.contains("198.51.100.32:443"));
        assert_eq!(reparsed.len(), 2);
        assert_eq!(reparsed[0].binding().lane_id().get(), 1);
        assert_eq!(reparsed[1].binding().lane_id().get(), 2);
        Ok(())
    }

    #[test]
    fn mesh_plan_without_carrier_bindings_fails_closed() -> Result<(), String> {
        let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")?;
        runtime.merge_discovery(
            "seed-b",
            &[MeshDiscoveryRecord {
                node_id: "node-a".to_string(),
                endpoint: "198.51.100.31:443".to_string(),
                region: "eu".to_string(),
                load_score: 20,
                reliability_score: 90,
            }],
        )?;
        let plan = runtime.plan_path_from_dps_payload(
            &MeshJoinRequest {
                namespace: "cef-public".to_string(),
                node_name: "node-client".to_string(),
                invite_token: None,
            },
            "mesh_allowed_regions=eu;mesh_multipath_mode=flow_shard",
        )?;
        let error = match render_transit_lane_registrations_from_mesh_plan(&plan) {
            Ok(_) => {
                return Err("planner-only schedule must not render runtime bindings".to_string());
            }
            Err(error) => error,
        };

        assert!(error.contains("no carrier lane bindings"));
        Ok(())
    }

    #[test]
    fn mesh_plan_registration_file_write_does_not_log_binding_material() -> Result<(), String> {
        let plan = multipath_plan()?;
        let mut path = std::env::temp_dir();
        path.push(format!(
            "chimera_transit_lane_bindings_{}.csv",
            std::process::id()
        ));
        let written = write_transit_lane_registrations_from_mesh_plan(
            &plan,
            path.to_str()
                .ok_or_else(|| "temp path is not utf-8".to_string())?,
        )?;
        let contents = std::fs::read_to_string(&path)
            .map_err(|error| format!("read rendered bindings failed: {error}"))?;
        let _ = std::fs::remove_file(&path);

        assert_eq!(written, 2);
        assert!(contents.contains("7003,0,"));
        assert!(contents.contains("7003,1,"));
        assert!(contents.contains("198.51.100.31:443"));
        assert!(contents.contains("198.51.100.32:443"));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn mesh_plan_registration_file_is_written_restricted_on_unix() -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt;

        let plan = multipath_plan()?;
        let mut path = std::env::temp_dir();
        path.push(format!(
            "chimera_transit_lane_bindings_perm_{}",
            std::process::id()
        ));
        let written = write_transit_lane_registrations_from_mesh_plan(
            &plan,
            path.to_str()
                .ok_or_else(|| "temp path is not utf-8".to_string())?,
        )?;
        let mode = std::fs::metadata(&path)
            .map_err(|error| format!("stat rendered bindings failed: {error}"))?
            .permissions()
            .mode()
            & 0o777;
        let _ = std::fs::remove_file(&path);

        assert_eq!(written, 2);
        assert_eq!(mode, 0o600);
        Ok(())
    }

    #[test]
    fn mesh_plan_registration_file_refuses_to_overwrite_existing_target() -> Result<(), String> {
        let plan = multipath_plan()?;
        let mut path = std::env::temp_dir();
        path.push(format!(
            "chimera_transit_lane_bindings_existing_{}.csv",
            std::process::id()
        ));
        std::fs::write(&path, "existing\n")
            .map_err(|error| format!("seed existing bindings file failed: {error}"))?;

        let error = match write_transit_lane_registrations_from_mesh_plan(
            &plan,
            path.to_str()
                .ok_or_else(|| "temp path is not utf-8".to_string())?,
        ) {
            Ok(_) => {
                let _ = std::fs::remove_file(&path);
                return Err("existing lane bindings file must not be overwritten".to_string());
            }
            Err(error) => error,
        };
        let contents = std::fs::read_to_string(&path)
            .map_err(|error| format!("read existing bindings file failed: {error}"))?;
        let _ = std::fs::remove_file(&path);

        assert!(error.contains("already exists"));
        assert_eq!(contents, "existing\n");
        Ok(())
    }
}
