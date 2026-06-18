use super::MeshMultipathFlowKey;

#[test]
fn opaque_flow_bytes_reject_empty_input() {
    assert!(MeshMultipathFlowKey::from_opaque_flow_bytes(&[]).is_err());
}

#[test]
fn same_flow_selects_same_slot() -> Result<(), String> {
    let first = MeshMultipathFlowKey::from_opaque_flow_id("stable-flow")?;
    let second = MeshMultipathFlowKey::from_opaque_flow_id("stable-flow")?;

    assert_eq!(first.select_slot_index(3)?, second.select_slot_index(3)?);
    Ok(())
}

#[test]
fn different_flows_spread_across_slots() -> Result<(), String> {
    let mut slots = std::collections::BTreeSet::new();
    for index in 0..64 {
        let key = MeshMultipathFlowKey::from_opaque_flow_id(&format!("opaque-flow-{index}"))?;
        slots.insert(key.select_slot_index(3)?);
    }

    assert!(slots.len() >= 2);
    Ok(())
}

#[test]
fn empty_candidate_set_fails_closed() -> Result<(), String> {
    let key = MeshMultipathFlowKey::from_opaque_flow_id("empty-candidates")?;
    assert!(key.select_slot_index(0).is_err());
    Ok(())
}

#[test]
fn debug_redacts_opaque_flow_key() -> Result<(), String> {
    let key = MeshMultipathFlowKey::from_opaque_flow_id("SECRET_FLOW_SENTINEL")?;
    let debug = format!("{key:?}");

    assert!(debug.contains("<opaque>"));
    assert!(!debug.contains("SECRET_FLOW_SENTINEL"));
    Ok(())
}
