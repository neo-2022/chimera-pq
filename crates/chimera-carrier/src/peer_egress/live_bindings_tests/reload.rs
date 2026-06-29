use super::*;

#[test]
fn reload_error_replaces_snapshot_with_fail_closed_error() -> Result<(), String> {
    let snapshot = Arc::new(Mutex::new(Ok(Arc::new(TransitLaneDocument::new(
        vec![registration(19, 3, "198.51.100.19:443")?],
        None,
    )))));

    super::super::replace_live_transit_lane_snapshot(
        &snapshot,
        Err("read sealed transit lane bindings failed: missing".to_string()),
    );

    let current = snapshot
        .lock()
        .map_err(|_| "snapshot lock poisoned".to_string())?
        .clone();
    let error = match current {
        Ok(_) => return Err("reload error must replace live snapshot".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("read sealed transit lane bindings failed"));
    Ok(())
}

#[test]
fn reload_noop_keeps_snapshot_and_workers_when_document_is_unchanged() -> Result<(), String> {
    let dispatcher = Arc::new(TransitNextHopDispatcher::default());
    let registration = registration(21, 4, "198.51.100.21:443")?;
    let initial_document = TransitLaneDocument::new(vec![registration.clone()], None);
    let initial_snapshot = Arc::new(initial_document.clone());
    let snapshot = Arc::new(Mutex::new(Ok(initial_snapshot.clone())));
    let cancel = Arc::new(AtomicBool::new(false));
    let mut workers = BTreeMap::from([(
        registration.binding(),
        LiveTransitLaneWorker {
            registration: registration.clone(),
            cancel: cancel.clone(),
        },
    )]);
    dispatcher.register(registration.binding(), test_peer_stream()?)?;
    let spawned = Arc::new(Mutex::new(0usize));
    let spawned_log = spawned.clone();

    apply_live_transit_lane_reload(
        &snapshot,
        &mut workers,
        &dispatcher,
        Ok(initial_document),
        move |_registration, _cancel| {
            let Ok(mut guard) = spawned_log.lock() else {
                unreachable!("spawn counter must lock");
            };
            *guard += 1;
        },
    );

    assert!(Arc::ptr_eq(
        snapshot
            .lock()
            .map_err(|_| "snapshot lock poisoned".to_string())?
            .as_ref()
            .map_err(|_| "snapshot must remain ok".to_string())?,
        &initial_snapshot
    ));
    assert_eq!(
        *spawned
            .lock()
            .map_err(|_| "spawn counter lock poisoned".to_string())?,
        0
    );
    assert!(!cancel.load(Ordering::Relaxed));
    assert_eq!(workers.len(), 1);
    assert_eq!(
        workers
            .get(&registration.binding())
            .ok_or_else(|| "worker missing".to_string())?
            .registration,
        registration
    );
    assert!(dispatcher.contains_binding(registration.binding())?);
    Ok(())
}

#[test]
fn reload_changed_document_replaces_stale_workers_and_spawns_new_bindings() -> Result<(), String> {
    let dispatcher = Arc::new(TransitNextHopDispatcher::default());
    let unchanged = registration(31, 5, "198.51.100.31:443")?;
    let stale = registration(32, 6, "198.51.100.32:443")?;
    let changed = registration(32, 6, "198.51.100.132:443")?;
    let added = registration(33, 7, "198.51.100.33:443")?;
    let initial_document = TransitLaneDocument::new(vec![unchanged.clone(), stale.clone()], None);
    let snapshot = Arc::new(Mutex::new(Ok(Arc::new(initial_document.clone()))));
    let unchanged_cancel = Arc::new(AtomicBool::new(false));
    let stale_cancel = Arc::new(AtomicBool::new(false));
    let mut workers = BTreeMap::from([
        (
            unchanged.binding(),
            LiveTransitLaneWorker {
                registration: unchanged.clone(),
                cancel: unchanged_cancel.clone(),
            },
        ),
        (
            stale.binding(),
            LiveTransitLaneWorker {
                registration: stale.clone(),
                cancel: stale_cancel.clone(),
            },
        ),
    ]);
    dispatcher.register(unchanged.binding(), test_peer_stream()?)?;
    dispatcher.register(stale.binding(), test_peer_stream()?)?;
    let spawned = Arc::new(Mutex::new(Vec::new()));
    let spawned_log = spawned.clone();

    apply_live_transit_lane_reload(
        &snapshot,
        &mut workers,
        &dispatcher,
        Ok(TransitLaneDocument::new(
            vec![unchanged.clone(), changed.clone(), added.clone()],
            None,
        )),
        move |registration, _cancel| {
            spawned_log
                .lock()
                .unwrap_or_else(|_| unreachable!("spawn log must lock"))
                .push(registration.clone());
        },
    );

    assert!(!unchanged_cancel.load(Ordering::Relaxed));
    assert!(stale_cancel.load(Ordering::Relaxed));
    assert_eq!(workers.len(), 3);
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
        workers
            .get(&added.binding())
            .ok_or_else(|| "added worker missing".to_string())?
            .registration,
        added
    );
    assert!(dispatcher.contains_binding(unchanged.binding())?);
    assert!(!dispatcher.contains_binding(stale.binding())?);
    assert_eq!(
        spawned
            .lock()
            .map_err(|_| "spawn log lock poisoned".to_string())?
            .as_slice(),
        &[changed, added]
    );
    Ok(())
}

#[test]
fn reload_error_clears_workers_and_sets_fail_closed_snapshot() -> Result<(), String> {
    let dispatcher = Arc::new(TransitNextHopDispatcher::default());
    let first = registration(41, 8, "198.51.100.41:443")?;
    let second = registration(42, 9, "198.51.100.42:443")?;
    let snapshot = Arc::new(Mutex::new(Ok(Arc::new(TransitLaneDocument::new(
        vec![first.clone(), second.clone()],
        None,
    )))));
    let first_cancel = Arc::new(AtomicBool::new(false));
    let second_cancel = Arc::new(AtomicBool::new(false));
    let mut workers = BTreeMap::from([
        (
            first.binding(),
            LiveTransitLaneWorker {
                registration: first.clone(),
                cancel: first_cancel.clone(),
            },
        ),
        (
            second.binding(),
            LiveTransitLaneWorker {
                registration: second.clone(),
                cancel: second_cancel.clone(),
            },
        ),
    ]);
    dispatcher.register(first.binding(), test_peer_stream()?)?;
    dispatcher.register(second.binding(), test_peer_stream()?)?;
    let spawned = Arc::new(Mutex::new(0usize));
    let spawned_log = spawned.clone();

    apply_live_transit_lane_reload(
        &snapshot,
        &mut workers,
        &dispatcher,
        Err("read sealed transit lane bindings failed: missing".to_string()),
        move |_registration, _cancel| {
            let Ok(mut guard) = spawned_log.lock() else {
                unreachable!("spawn counter must lock");
            };
            *guard += 1;
        },
    );

    let current = snapshot
        .lock()
        .map_err(|_| "snapshot lock poisoned".to_string())?
        .clone();
    let error = match current {
        Ok(_) => return Err("reload error must replace live snapshot".to_string()),
        Err(error) => error,
    };
    assert!(error.contains("read sealed transit lane bindings failed"));
    assert!(workers.is_empty());
    assert!(first_cancel.load(Ordering::Relaxed));
    assert!(second_cancel.load(Ordering::Relaxed));
    assert_eq!(
        *spawned
            .lock()
            .map_err(|_| "spawn counter lock poisoned".to_string())?,
        0
    );
    assert!(!dispatcher.contains_binding(first.binding())?);
    assert!(!dispatcher.contains_binding(second.binding())?);
    Ok(())
}
