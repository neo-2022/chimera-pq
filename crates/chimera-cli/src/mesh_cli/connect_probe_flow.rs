use crate::mesh_cli::options;
use crate::mesh_cli::options::MeshRouteExplainOptions;
use crate::mesh_cli::route_explain_error_consts::{
    STAGE_DISCOVERY_MERGE, STAGE_PEER_SPEC, STAGE_PEER_TABLE_POLICY, STAGE_PLAN_PATH,
    STAGE_POLICY_PARSE, STAGE_RUNTIME_BOOTSTRAP, STAGE_SIMULATION_INPUT,
    STAGE_TRANSIT_LANE_BINDINGS_EXPORT,
};
use chimera_carrier::peer_egress::write_transit_lane_document_from_mesh_plan;
use chimera_mesh::{
    MeshConnectProbeReport, MeshJoinRequest, MeshPathPolicy, MeshPeerTablePolicy, MeshRuntime,
};

#[derive(Debug)]
pub(super) struct MeshConnectProbeFlowError {
    pub(super) stage: &'static str,
    pub(super) message: String,
}

pub(super) fn run_mesh_connect_probe_flow(
    options: &MeshRouteExplainOptions,
    bootstrap_source: &str,
) -> Result<(MeshConnectProbeReport, u64), MeshConnectProbeFlowError> {
    let mut runtime =
        MeshRuntime::bootstrap(&options.namespace, bootstrap_source).map_err(|error| {
            MeshConnectProbeFlowError {
                stage: STAGE_RUNTIME_BOOTSTRAP,
                message: error,
            }
        })?;

    if options.table_max_entries.is_some()
        || options.table_max_entries_per_region.is_some()
        || options.table_stale_after_ticks.is_some()
    {
        let mut table_policy = MeshPeerTablePolicy::default();
        if let Some(value) = options.table_max_entries {
            table_policy.max_entries = value;
        }
        if let Some(value) = options.table_max_entries_per_region {
            table_policy.max_entries_per_region = value;
        }
        if let Some(value) = options.table_stale_after_ticks {
            table_policy.stale_after_ticks = value;
        }
        runtime
            .set_peer_table_policy(table_policy)
            .map_err(|error| MeshConnectProbeFlowError {
                stage: STAGE_PEER_TABLE_POLICY,
                message: error,
            })?;
    }

    let records = options::parse_mesh_peer_records(&options.peers).map_err(|error| {
        let stage = if error.contains("duplicate peer node_id") {
            STAGE_SIMULATION_INPUT
        } else {
            STAGE_PEER_SPEC
        };
        MeshConnectProbeFlowError {
            stage,
            message: error,
        }
    })?;

    runtime
        .merge_discovery("cli-peer-list", &records)
        .map_err(|error| MeshConnectProbeFlowError {
            stage: STAGE_DISCOVERY_MERGE,
            message: error,
        })?;

    let request = MeshJoinRequest {
        namespace: options.namespace.clone(),
        node_name: options.node_name.clone(),
        invite_token: options.invite_token.clone(),
    };
    let policy = MeshPathPolicy::from_dps_payload(&options.policy_payload).map_err(|error| {
        MeshConnectProbeFlowError {
            stage: STAGE_POLICY_PARSE,
            message: error,
        }
    })?;

    let timeout_ms = options.connect_timeout_ms.unwrap_or(1200).max(1);
    let report = runtime
        .connect_probe(&request, &policy, timeout_ms)
        .map_err(|error| MeshConnectProbeFlowError {
            stage: STAGE_PLAN_PATH,
            message: error,
        })?;

    if report.success
        && let Some(path) = options.transit_lane_bindings_out_path.as_deref()
    {
        let mut refreshed_plan =
            runtime
                .plan_path(&request, &policy)
                .map_err(|error| MeshConnectProbeFlowError {
                    stage: STAGE_PLAN_PATH,
                    message: error,
                })?;
        if refreshed_plan
            .multipath_schedule
            .carrier_lane_bindings
            .is_empty()
        {
            refreshed_plan = runtime
                .plan_path_from_dps_payload(&request, &options.policy_payload)
                .map_err(|error| MeshConnectProbeFlowError {
                    stage: STAGE_PLAN_PATH,
                    message: error,
                })?;
        }
        write_transit_lane_document_from_mesh_plan(&refreshed_plan, path).map_err(|error| {
            MeshConnectProbeFlowError {
                stage: STAGE_TRANSIT_LANE_BINDINGS_EXPORT,
                message: error,
            }
        })?;
    }

    Ok((report, timeout_ms))
}
