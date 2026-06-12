use std::{env, fs};

use chimera_mesh::MeshNodeId;

use super::{config_string_value, extract_flag_value};

#[derive(Debug, Clone, Default)]
pub(super) struct MeshIdentityState {
    pub(super) status: String,
    pub(super) self_node_id: Option<MeshNodeId>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct MeshActivationState {
    pub(super) self_node_id: Option<MeshNodeId>,
    pub(super) activated_at_unix: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct MeshRuntimeState {
    pub(super) current_node: Option<MeshNodeId>,
    pub(super) pinned_node: Option<MeshNodeId>,
    pub(super) autoconnect_enabled: Option<bool>,
}

pub(super) fn resolve_self_node_id(args: &[String]) -> Result<Option<MeshNodeId>, String> {
    let value = extract_flag_value(args, "--self-node-id")
        .map(str::to_string)
        .or_else(|| config_string_value(args, "mesh.nodes.self_node_id"))
        .or_else(|| env::var("CHIMERA_MESH_SELF_NODE_ID").ok());
    let Some(value) = value else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    MeshNodeId::new(&value).validate()?;
    Ok(Some(MeshNodeId::new(value)))
}

pub(super) fn load_identity_state(args: &[String]) -> Result<MeshIdentityState, String> {
    let state_path = extract_flag_value(args, "--identity-state")
        .map(str::to_string)
        .or_else(|| config_string_value(args, "mesh.nodes.identity_state_path"))
        .or_else(|| env::var("CHIMERA_MESH_IDENTITY_STATE_PATH").ok());
    let Some(path) = state_path else {
        return Ok(MeshIdentityState::default());
    };
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(MeshIdentityState::default());
        }
        Err(error) => {
            return Err(format!("read identity state failed: {error}"));
        }
    };
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("identity state json parse failed: {error}"))?;
    let status = value
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let self_node_id = value
        .get("self_node_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .filter(|id| !id.trim().is_empty())
        .map(MeshNodeId::new);
    if let Some(id) = self_node_id.as_ref() {
        id.validate()?;
    }
    Ok(MeshIdentityState {
        status,
        self_node_id,
    })
}

pub(super) fn load_activation_state(args: &[String]) -> Result<MeshActivationState, String> {
    let activation_path = extract_flag_value(args, "--activation-log")
        .map(str::to_string)
        .or_else(|| config_string_value(args, "mesh.nodes.activation_log_path"))
        .or_else(|| env::var("CHIMERA_MESH_ACTIVATION_LOG_PATH").ok());
    let Some(path) = activation_path else {
        return Ok(MeshActivationState::default());
    };
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(MeshActivationState::default());
        }
        Err(error) => {
            return Err(format!("read activation log failed: {error}"));
        }
    };
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("activation log json parse failed: {error}"))?;
    let self_node_id = value
        .get("self_node_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .filter(|id| !id.trim().is_empty())
        .map(MeshNodeId::new);
    if let Some(id) = self_node_id.as_ref() {
        id.validate()?;
    }
    let activated_at_unix = value
        .get("activated_at_unix")
        .and_then(serde_json::Value::as_u64);
    Ok(MeshActivationState {
        self_node_id,
        activated_at_unix,
    })
}

pub(crate) fn default_runtime_state_path() -> String {
    if let Ok(xdg_state_home) = env::var("XDG_STATE_HOME")
        && !xdg_state_home.trim().is_empty()
    {
        return format!(
            "{}/chimera/mesh_nodes_runtime_state.json",
            xdg_state_home.trim_end_matches('/')
        );
    }
    format!(
        "{}/.local/state/chimera/mesh_nodes_runtime_state.json",
        env::var("HOME").unwrap_or_default()
    )
}

pub(super) fn load_runtime_state(args: &[String]) -> Result<MeshRuntimeState, String> {
    let state_path = extract_flag_value(args, "--runtime-state")
        .map(str::to_string)
        .or_else(|| config_string_value(args, "mesh.nodes.runtime_state_path"))
        .or_else(|| env::var("CHIMERA_MESH_NODES_RUNTIME_STATE_PATH").ok())
        .unwrap_or_else(default_runtime_state_path);
    let path = state_path;
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(MeshRuntimeState::default());
        }
        Err(error) => {
            return Err(format!("read mesh runtime state failed: {error}"));
        }
    };
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("mesh runtime state json parse failed: {error}"))?;
    let current_node = value
        .get("current_node_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .filter(|id| !id.trim().is_empty())
        .map(MeshNodeId::new);
    if let Some(id) = current_node.as_ref() {
        id.validate()?;
    }
    let pinned_node = value
        .get("pinned_node_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .filter(|id| !id.trim().is_empty())
        .map(MeshNodeId::new);
    if let Some(id) = pinned_node.as_ref() {
        id.validate()?;
    }
    let autoconnect_enabled = value
        .get("autoconnect")
        .and_then(serde_json::Value::as_bool);
    Ok(MeshRuntimeState {
        current_node,
        pinned_node,
        autoconnect_enabled,
    })
}
