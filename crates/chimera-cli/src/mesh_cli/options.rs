use chimera_mesh::MeshDiscoveryRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MeshRouteExplainOptions {
    pub(crate) namespace: String,
    pub(crate) node_name: String,
    pub(crate) invite_token: Option<String>,
    pub(crate) policy_payload: String,
    pub(crate) failed_node_id: Option<String>,
    pub(crate) cooldown_node_id: Option<String>,
    pub(crate) table_max_entries: Option<usize>,
    pub(crate) table_max_entries_per_region: Option<usize>,
    pub(crate) table_stale_after_ticks: Option<u64>,
    pub(crate) connect_timeout_ms: Option<u64>,
    pub(crate) peers: Vec<String>,
    pub(crate) json_output: bool,
    pub(crate) out_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MeshTrafficProfilePreset {
    HighSpeedAnonymous,
    PrivacyFirst,
    SpeedFirst,
    LowLatencyPrivate,
}

impl MeshTrafficProfilePreset {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "high_speed_anonymous" => Ok(Self::HighSpeedAnonymous),
            "privacy_first" => Ok(Self::PrivacyFirst),
            "speed_first" => Ok(Self::SpeedFirst),
            "low_latency_private" => Ok(Self::LowLatencyPrivate),
            _ => Err("invalid --traffic-profile value (expected one of: high_speed_anonymous, privacy_first, speed_first, low_latency_private)".to_string()),
        }
    }

    fn to_policy_payload(self) -> &'static str {
        match self {
            Self::HighSpeedAnonymous => {
                "allow=mesh;mesh_traffic_class=bulk_transfer;mesh_multipath_mode=flow_shard;mesh_continuity_policy=allow_flow_drain;mesh_max_peers=3;mesh_min_reliability=70;mesh_max_load=85;mesh_connect_fallback_ports=443,8443"
            }
            Self::PrivacyFirst => {
                "allow=mesh;mesh_traffic_class=web_interactive;mesh_multipath_mode=standby_only;mesh_continuity_policy=same_egress_only;mesh_max_peers=1;mesh_min_reliability=85;mesh_max_load=60;mesh_connect_fallback_ports=443,8443"
            }
            Self::SpeedFirst => {
                "allow=mesh;mesh_traffic_class=artifact_download;mesh_multipath_mode=flow_shard;mesh_continuity_policy=allow_flow_drain;mesh_max_peers=3;mesh_min_reliability=65;mesh_max_load=90;mesh_connect_fallback_ports=443,8443"
            }
            Self::LowLatencyPrivate => {
                "allow=mesh;mesh_traffic_class=realtime_interactive;mesh_multipath_mode=standby_only;mesh_continuity_policy=same_egress_only;mesh_max_peers=1;mesh_min_reliability=90;mesh_max_load=55;mesh_connect_fallback_ports=443,8443"
            }
        }
    }
}

pub(crate) fn parse_mesh_route_explain_options(
    args: &[String],
) -> Result<MeshRouteExplainOptions, String> {
    let mut namespace = None;
    let mut node_name = None;
    let mut invite_token = None;
    let mut policy_payload = None;
    let mut traffic_profile = None;
    let mut failed_node_id = None;
    let mut cooldown_node_id = None;
    let mut table_max_entries = None;
    let mut table_max_entries_per_region = None;
    let mut table_stale_after_ticks = None;
    let mut connect_timeout_ms = None;
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
            "--namespace" => set_once(
                "--namespace",
                &mut namespace,
                parse_non_empty_arg_value(flag, value)?,
            )?,
            "--node" => set_once(
                "--node",
                &mut node_name,
                parse_non_empty_arg_value(flag, value)?,
            )?,
            "--invite-token" => set_once(
                "--invite-token",
                &mut invite_token,
                parse_non_empty_arg_value(flag, value)?,
            )?,
            "--policy-payload" => set_once(
                "--policy-payload",
                &mut policy_payload,
                parse_non_empty_arg_value(flag, value)?,
            )?,
            "--traffic-profile" => set_once(
                "--traffic-profile",
                &mut traffic_profile,
                MeshTrafficProfilePreset::parse(&parse_non_empty_arg_value(flag, value)?)?,
            )?,
            "--peer" => peers.push(parse_non_empty_arg_value(flag, value)?),
            "--failed-node" => set_once(
                "--failed-node",
                &mut failed_node_id,
                parse_non_empty_arg_value(flag, value)?,
            )?,
            "--cooldown-node" => set_once(
                "--cooldown-node",
                &mut cooldown_node_id,
                parse_non_empty_arg_value(flag, value)?,
            )?,
            "--table-max-entries" => set_once("--table-max-entries", &mut table_max_entries, {
                let normalized = parse_non_empty_arg_value(flag, value)?;
                normalized
                    .parse::<usize>()
                    .map_err(|_| format!("invalid --table-max-entries value '{normalized}'"))?
            })?,
            "--table-max-per-region" => set_once(
                "--table-max-per-region",
                &mut table_max_entries_per_region,
                {
                    let normalized = parse_non_empty_arg_value(flag, value)?;
                    normalized.parse::<usize>().map_err(|_| {
                        format!("invalid --table-max-per-region value '{normalized}'")
                    })?
                },
            )?,
            "--table-stale-after" => {
                set_once("--table-stale-after", &mut table_stale_after_ticks, {
                    let normalized = parse_non_empty_arg_value(flag, value)?;
                    normalized
                        .parse::<u64>()
                        .map_err(|_| format!("invalid --table-stale-after value '{normalized}'"))?
                })?
            }
            "--timeout-ms" => set_once("--timeout-ms", &mut connect_timeout_ms, {
                let normalized = parse_non_empty_arg_value(flag, value)?;
                normalized
                    .parse::<u64>()
                    .map_err(|_| format!("invalid --timeout-ms value '{normalized}'"))?
            })?,
            "--out" => set_once(
                "--out",
                &mut out_path,
                parse_non_empty_arg_value(flag, value)?,
            )?,
            _ => return Err(format!("unknown flag '{flag}'")),
        }
        i += 2;
    }

    let namespace = namespace.ok_or_else(|| "missing --namespace".to_string())?;
    let node_name = node_name.ok_or_else(|| "missing --node".to_string())?;
    let policy_payload = match (policy_payload, traffic_profile) {
        (Some(payload), None) => payload,
        (None, Some(profile)) => profile.to_policy_payload().to_string(),
        (Some(_), Some(_)) => {
            return Err("cannot use both --policy-payload and --traffic-profile".to_string());
        }
        (None, None) => return Err("missing --policy-payload".to_string()),
    };
    if peers.is_empty() {
        return Err("at least one --peer is required".to_string());
    }

    Ok(MeshRouteExplainOptions {
        namespace,
        node_name,
        invite_token,
        policy_payload,
        failed_node_id,
        cooldown_node_id,
        table_max_entries,
        table_max_entries_per_region,
        table_stale_after_ticks,
        connect_timeout_ms,
        peers,
        json_output,
        out_path,
    })
}

fn parse_non_empty_arg_value(flag: &str, value: &str) -> Result<String, String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(format!("blank value for flag '{flag}'"));
    }
    Ok(normalized.to_string())
}

fn set_once<T>(flag: &str, slot: &mut Option<T>, value: T) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!("duplicate singleton flag '{flag}'"));
    }
    *slot = Some(value);
    Ok(())
}

pub(crate) fn parse_mesh_peer_spec(input: &str) -> Result<MeshDiscoveryRecord, String> {
    let parts: Vec<&str> = input.split('@').collect();
    if parts.len() != 5 {
        return Err("expected node@endpoint#region@load@reliability".to_string());
    }
    let node_id = normalize_peer_field(parts[0], "blank peer node_id")?;
    let endpoint = normalize_peer_field(parts[1], "blank peer endpoint")?;
    let region = normalize_peer_field(parts[2], "blank peer region")?;
    let load_score = parts[3]
        .trim()
        .parse::<u8>()
        .map_err(|_| "invalid load score".to_string())?;
    let reliability_score = parts[4]
        .trim()
        .parse::<u8>()
        .map_err(|_| "invalid reliability score".to_string())?;
    let record = MeshDiscoveryRecord {
        node_id,
        endpoint,
        region,
        load_score,
        reliability_score,
    };
    record.validate()?;
    Ok(record)
}

pub(crate) fn parse_mesh_peer_records(
    peers: &[String],
) -> Result<Vec<MeshDiscoveryRecord>, String> {
    let mut records = Vec::with_capacity(peers.len());
    for peer in peers {
        records.push(parse_mesh_peer_spec(peer)?);
    }
    validate_unique_peer_node_ids(&records)?;
    Ok(records)
}

pub(crate) fn validate_unique_peer_node_ids(records: &[MeshDiscoveryRecord]) -> Result<(), String> {
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for record in records {
        if !seen.insert(record.node_id.as_str()) {
            return Err(format!(
                "duplicate peer node_id '{}' in --peer set",
                record.node_id
            ));
        }
    }
    Ok(())
}

fn normalize_peer_field(value: &str, error_message: &str) -> Result<String, String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(error_message.to_string());
    }
    Ok(normalized.to_string())
}

pub(crate) fn wants_json_output(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--json")
}

pub(crate) fn extract_non_empty_flag_value(args: &[String], flag: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == flag {
            let value = args.get(i + 1)?;
            if value.starts_with("--") {
                return None;
            }
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return None;
            }
            return Some(trimmed.to_string());
        }
        i += 1;
    }
    None
}
