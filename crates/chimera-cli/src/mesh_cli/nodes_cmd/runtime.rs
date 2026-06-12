use chimera_mesh::{MeshNodeRuntime, MeshNodesPolicy};

use crate::mesh_cli::nodes_inventory::MeshNodesInventory;

pub(super) fn build_runtime_from_inventory(
    inventory: &MeshNodesInventory,
    policy: &MeshNodesPolicy,
    operation: &str,
) -> Result<MeshNodeRuntime, i32> {
    let mut runtime = match MeshNodeRuntime::new(policy.clone()) {
        Ok(runtime) => runtime,
        Err(errors) => {
            eprintln!("mesh nodes {operation} error: {}", errors.join("; "));
            return Err(2);
        }
    };
    runtime.state.current_node = inventory.current_node.clone();
    runtime.state.pinned_node = inventory.pinned_node.clone();
    if let Some(enabled) = inventory.autoconnect_enabled {
        runtime.set_autoconnect(enabled);
    }
    Ok(runtime)
}
