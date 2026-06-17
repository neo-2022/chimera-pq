use super::*;

pub(super) fn unhealthy_node_ids_from_health_state(
    health_state: &BTreeMap<String, MeshHealthMeta>,
) -> BTreeSet<String> {
    health_state
        .values()
        .filter(|meta| !meta.health.healthy || meta.health.cooldown_active)
        .map(|meta| meta.health.node_id.clone())
        .collect()
}
