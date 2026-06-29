use super::*;

#[test]
fn worker_reconcile_restarts_binding_when_endpoint_changes() -> Result<(), String> {
    let dispatcher = Arc::new(TransitNextHopDispatcher::default());
    let initial = registration(7, 1, "198.51.100.7:443")?;
    let changed = registration(7, 1, "198.51.100.8:443")?;
    let old_cancel = Arc::new(AtomicBool::new(false));
    let mut workers = BTreeMap::from([(
        initial.binding(),
        LiveTransitLaneWorker {
            registration: initial.clone(),
            cancel: old_cancel.clone(),
        },
    )]);
    dispatcher.register(initial.binding(), test_peer_stream()?)?;
    let spawned = Arc::new(Mutex::new(Vec::new()));
    let spawned_log = spawned.clone();

    reconcile_live_transit_lane_workers(
        &mut workers,
        std::slice::from_ref(&changed),
        &dispatcher,
        move |registration, _cancel| {
            spawned_log
                .lock()
                .unwrap_or_else(|_| unreachable!("spawn log must lock"))
                .push(registration.clone());
        },
    );

    assert!(old_cancel.load(Ordering::Relaxed));
    assert!(!dispatcher.contains_binding(initial.binding())?);
    assert_eq!(workers.len(), 1);
    assert_eq!(
        workers
            .get(&changed.binding())
            .ok_or_else(|| "updated worker missing".to_string())?
            .registration,
        changed
    );
    assert_eq!(
        spawned
            .lock()
            .unwrap_or_else(|_| unreachable!("spawn log must lock"))
            .as_slice(),
        &[changed]
    );
    Ok(())
}

#[test]
fn worker_reconcile_keeps_same_binding_without_spawn_churn() -> Result<(), String> {
    let dispatcher = Arc::new(TransitNextHopDispatcher::default());
    let registration = registration(8, 2, "198.51.100.8:443")?;
    let cancel = Arc::new(AtomicBool::new(false));
    let mut workers = BTreeMap::from([(
        registration.binding(),
        LiveTransitLaneWorker {
            registration: registration.clone(),
            cancel: cancel.clone(),
        },
    )]);
    dispatcher.register(registration.binding(), test_peer_stream()?)?;
    let spawned = Arc::new(Mutex::new(Vec::new()));
    let spawned_log = spawned.clone();

    reconcile_live_transit_lane_workers(
        &mut workers,
        std::slice::from_ref(&registration),
        &dispatcher,
        move |registration, _cancel| {
            spawned_log
                .lock()
                .unwrap_or_else(|_| unreachable!("spawn log must lock"))
                .push(registration.clone());
        },
    );

    assert!(!cancel.load(Ordering::Relaxed));
    assert!(dispatcher.contains_binding(registration.binding())?);
    assert_eq!(workers.len(), 1);
    assert_eq!(
        workers
            .get(&registration.binding())
            .ok_or_else(|| "worker missing".to_string())?
            .registration,
        registration
    );
    assert!(
        spawned
            .lock()
            .unwrap_or_else(|_| unreachable!("spawn log must lock"))
            .is_empty()
    );
    Ok(())
}

#[test]
fn worker_reconcile_sorted_unique_replaces_changed_binding() -> Result<(), String> {
    let dispatcher = Arc::new(TransitNextHopDispatcher::default());
    let unchanged = registration(12, 1, "198.51.100.12:443")?;
    let old = registration(13, 1, "198.51.100.13:443")?;
    let changed = registration(13, 1, "198.51.100.113:443")?;
    let unchanged_cancel = Arc::new(AtomicBool::new(false));
    let old_cancel = Arc::new(AtomicBool::new(false));
    let mut workers = BTreeMap::from([
        (
            unchanged.binding(),
            LiveTransitLaneWorker {
                registration: unchanged.clone(),
                cancel: unchanged_cancel.clone(),
            },
        ),
        (
            old.binding(),
            LiveTransitLaneWorker {
                registration: old.clone(),
                cancel: old_cancel.clone(),
            },
        ),
    ]);
    dispatcher.register(unchanged.binding(), test_peer_stream()?)?;
    dispatcher.register(old.binding(), test_peer_stream()?)?;
    let spawned = Arc::new(Mutex::new(Vec::new()));
    let spawned_log = spawned.clone();

    reconcile_live_transit_lane_workers(
        &mut workers,
        &[unchanged.clone(), changed.clone()],
        &dispatcher,
        move |registration, _cancel| {
            spawned_log
                .lock()
                .unwrap_or_else(|_| unreachable!("spawn log must lock"))
                .push(registration.clone());
        },
    );

    assert!(!unchanged_cancel.load(Ordering::Relaxed));
    assert!(old_cancel.load(Ordering::Relaxed));
    assert!(dispatcher.contains_binding(unchanged.binding())?);
    assert!(!dispatcher.contains_binding(old.binding())?);
    assert_eq!(workers.len(), 2);
    assert_eq!(
        workers
            .get(&unchanged.binding())
            .ok_or_else(|| "unchanged worker missing".to_string())?
            .registration,
        unchanged
    );
    assert_eq!(
        workers
            .get(&changed.binding())
            .ok_or_else(|| "changed worker missing".to_string())?
            .registration,
        changed
    );
    assert_eq!(
        spawned
            .lock()
            .map_err(|_| "spawn log lock poisoned".to_string())?
            .as_slice(),
        &[changed]
    );
    Ok(())
}

#[test]
fn worker_reconcile_unsorted_unique_uses_fallback_without_semantic_drift() -> Result<(), String> {
    let dispatcher = Arc::new(TransitNextHopDispatcher::default());
    let first = registration(14, 1, "198.51.100.14:443")?;
    let second = registration(15, 1, "198.51.100.15:443")?;
    let mut workers = BTreeMap::new();
    let spawned = Arc::new(Mutex::new(Vec::new()));
    let spawned_log = spawned.clone();

    reconcile_live_transit_lane_workers(
        &mut workers,
        &[second.clone(), first.clone()],
        &dispatcher,
        move |registration, _cancel| {
            spawned_log
                .lock()
                .unwrap_or_else(|_| unreachable!("spawn log must lock"))
                .push(registration.clone());
        },
    );

    assert_eq!(workers.len(), 2);
    assert_eq!(
        workers
            .get(&first.binding())
            .ok_or_else(|| "first worker missing".to_string())?
            .registration,
        first
    );
    assert_eq!(
        workers
            .get(&second.binding())
            .ok_or_else(|| "second worker missing".to_string())?
            .registration,
        second
    );
    assert_eq!(
        spawned
            .lock()
            .map_err(|_| "spawn log lock poisoned".to_string())?
            .as_slice(),
        &[second, first]
    );
    Ok(())
}

#[test]
fn clear_workers_evicts_registered_bindings_and_sets_cancel() -> Result<(), String> {
    let dispatcher = Arc::new(TransitNextHopDispatcher::default());
    let registration = registration(11, 2, "198.51.100.11:443")?;
    let cancel = Arc::new(AtomicBool::new(false));
    let mut workers = BTreeMap::from([(
        registration.binding(),
        LiveTransitLaneWorker {
            registration: registration.clone(),
            cancel: cancel.clone(),
        },
    )]);
    dispatcher.register(registration.binding(), test_peer_stream()?)?;

    super::super::clear_live_transit_lane_workers(&mut workers, &dispatcher);

    assert!(cancel.load(Ordering::Relaxed));
    assert!(workers.is_empty());
    assert!(!dispatcher.contains_binding(registration.binding())?);
    Ok(())
}

#[test]
fn worker_reconcile_duplicate_desired_binding_keeps_last_registration() -> Result<(), String> {
    let dispatcher = Arc::new(TransitNextHopDispatcher::default());
    let old = registration(36, 3, "198.51.100.36:443")?;
    let stale_desired = old.clone();
    let last_desired = registration(36, 3, "198.51.100.136:443")?;
    let old_cancel = Arc::new(AtomicBool::new(false));
    let mut workers = BTreeMap::from([(
        old.binding(),
        LiveTransitLaneWorker {
            registration: old.clone(),
            cancel: old_cancel.clone(),
        },
    )]);
    dispatcher.register(old.binding(), test_peer_stream()?)?;
    let spawned = Arc::new(Mutex::new(Vec::new()));
    let spawned_log = spawned.clone();

    reconcile_live_transit_lane_workers(
        &mut workers,
        &[stale_desired, last_desired.clone()],
        &dispatcher,
        move |registration, _cancel| {
            spawned_log
                .lock()
                .unwrap_or_else(|_| unreachable!("spawn log must lock"))
                .push(registration.clone());
        },
    );

    assert!(old_cancel.load(Ordering::Relaxed));
    assert!(!dispatcher.contains_binding(old.binding())?);
    assert_eq!(workers.len(), 1);
    assert_eq!(
        workers
            .get(&last_desired.binding())
            .ok_or_else(|| "last desired worker missing".to_string())?
            .registration,
        last_desired
    );
    assert_eq!(
        spawned
            .lock()
            .map_err(|_| "spawn log lock poisoned".to_string())?
            .as_slice(),
        &[last_desired]
    );
    Ok(())
}
