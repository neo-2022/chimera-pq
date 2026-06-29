const REBUILD_CONTROL_POLICY: &str = "runtime_owned_debounce_v1";
const REBUILD_CONTROL_PRIVACY: &str = "aggregate_only";
const ACTION_ALLOW_REBUILD: &str = "allow_rebuild";
const ACTION_SUPPRESS_REBUILD: &str = "suppress_rebuild";
const ACTION_FAIL_CLOSED: &str = "fail_closed";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshMultipathRebuildUrgency {
    Soft,
    UrgentFailover,
    HardSafety,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshMultipathRebuildDirtyScope {
    Unknown,
    PeerSet,
}

impl MeshMultipathRebuildDirtyScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::PeerSet => "peer_set",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshMultipathRebuildDirtyMetadata {
    scope: MeshMultipathRebuildDirtyScope,
    affected_peer_count: usize,
}

impl MeshMultipathRebuildDirtyMetadata {
    pub fn unknown() -> Self {
        Self {
            scope: MeshMultipathRebuildDirtyScope::Unknown,
            affected_peer_count: 0,
        }
    }

    pub fn peer_set(affected_peer_count: usize) -> Result<Self, String> {
        let metadata = Self {
            scope: MeshMultipathRebuildDirtyScope::PeerSet,
            affected_peer_count,
        };
        metadata.validate()?;
        Ok(metadata)
    }

    pub fn scope(self) -> MeshMultipathRebuildDirtyScope {
        self.scope
    }

    pub fn affected_peer_count(self) -> usize {
        self.affected_peer_count
    }

    fn validate(self) -> Result<(), String> {
        validate_dirty_scope(self.scope, self.affected_peer_count)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshMultipathRebuildSignalInput<'a> {
    pub rebuild_recommended: bool,
    pub reason: &'a str,
    pub urgency: MeshMultipathRebuildUrgency,
    pub schedule_generation: u64,
    pub schedule_fingerprint: u64,
    pub telemetry_epoch: u64,
    pub telemetry_observed_tick: u64,
    pub dirty_metadata: MeshMultipathRebuildDirtyMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshMultipathRebuildAction {
    AllowRebuild,
    SuppressRebuild,
    FailClosed,
}

impl MeshMultipathRebuildAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AllowRebuild => ACTION_ALLOW_REBUILD,
            Self::SuppressRebuild => ACTION_SUPPRESS_REBUILD,
            Self::FailClosed => ACTION_FAIL_CLOSED,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshMultipathRebuildPolicy {
    pub debounce_window_ticks: u64,
    pub freshness_ttl_ticks: u64,
}

impl MeshMultipathRebuildPolicy {
    pub fn new(debounce_window_ticks: u64, freshness_ttl_ticks: u64) -> Result<Self, String> {
        if debounce_window_ticks == 0 {
            return Err("multipath rebuild debounce_window_ticks must be > 0".to_string());
        }
        if freshness_ttl_ticks == 0 {
            return Err("multipath rebuild freshness_ttl_ticks must be > 0".to_string());
        }
        Ok(Self {
            debounce_window_ticks,
            freshness_ttl_ticks,
        })
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        if self.debounce_window_ticks == 0 {
            return Err("multipath rebuild debounce_window_ticks must be > 0".to_string());
        }
        if self.freshness_ttl_ticks == 0 {
            return Err("multipath rebuild freshness_ttl_ticks must be > 0".to_string());
        }
        Ok(())
    }
}

/// Runtime rebuild input. It must be constructed through validating
/// constructors so reason labels cannot carry route, peer, payload or
/// fingerprint-shaped data.
///
/// ```compile_fail
/// use chimera_mesh::{MeshMultipathRebuildSignal, MeshMultipathRebuildUrgency};
///
/// let _ = MeshMultipathRebuildSignal {
///     rebuild_recommended: true,
///     reason: "route_7009".to_string(),
///     urgency: MeshMultipathRebuildUrgency::Soft,
///     schedule_generation: 1,
///     schedule_fingerprint: 1,
///     telemetry_epoch: 1,
///     telemetry_observed_tick: 1,
/// };
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct MeshMultipathRebuildSignal {
    rebuild_recommended: bool,
    reason: String,
    urgency: MeshMultipathRebuildUrgency,
    schedule_generation: u64,
    schedule_fingerprint: u64,
    telemetry_epoch: u64,
    telemetry_observed_tick: u64,
    dirty_scope: MeshMultipathRebuildDirtyScope,
    affected_peer_count: usize,
}

impl MeshMultipathRebuildSignal {
    pub fn new(
        rebuild_recommended: bool,
        reason: &str,
        urgency: MeshMultipathRebuildUrgency,
        schedule_generation: u64,
        schedule_fingerprint: u64,
        telemetry_epoch: u64,
        telemetry_observed_tick: u64,
    ) -> Result<Self, String> {
        Self::from_input(MeshMultipathRebuildSignalInput {
            rebuild_recommended,
            reason,
            urgency,
            schedule_generation,
            schedule_fingerprint,
            telemetry_epoch,
            telemetry_observed_tick,
            dirty_metadata: MeshMultipathRebuildDirtyMetadata::unknown(),
        })
    }

    pub fn from_input(input: MeshMultipathRebuildSignalInput<'_>) -> Result<Self, String> {
        validate_rebuild_reason(input.reason)?;
        input.dirty_metadata.validate()?;
        Ok(Self {
            rebuild_recommended: input.rebuild_recommended,
            reason: input.reason.to_string(),
            urgency: input.urgency,
            schedule_generation: input.schedule_generation,
            schedule_fingerprint: input.schedule_fingerprint,
            telemetry_epoch: input.telemetry_epoch,
            telemetry_observed_tick: input.telemetry_observed_tick,
            dirty_scope: input.dirty_metadata.scope(),
            affected_peer_count: input.dirty_metadata.affected_peer_count(),
        })
    }

    pub fn soft(
        reason: &str,
        schedule_generation: u64,
        schedule_fingerprint: u64,
        telemetry_epoch: u64,
        telemetry_observed_tick: u64,
    ) -> Result<Self, String> {
        Self::new(
            true,
            reason,
            MeshMultipathRebuildUrgency::Soft,
            schedule_generation,
            schedule_fingerprint,
            telemetry_epoch,
            telemetry_observed_tick,
        )
    }

    pub fn soft_with_dirty_scope(
        reason: &str,
        schedule_generation: u64,
        schedule_fingerprint: u64,
        telemetry_epoch: u64,
        telemetry_observed_tick: u64,
        dirty_metadata: MeshMultipathRebuildDirtyMetadata,
    ) -> Result<Self, String> {
        Self::from_input(MeshMultipathRebuildSignalInput {
            rebuild_recommended: true,
            reason,
            urgency: MeshMultipathRebuildUrgency::Soft,
            schedule_generation,
            schedule_fingerprint,
            telemetry_epoch,
            telemetry_observed_tick,
            dirty_metadata,
        })
    }

    pub fn urgent_failover(
        reason: &str,
        schedule_generation: u64,
        schedule_fingerprint: u64,
        telemetry_epoch: u64,
        telemetry_observed_tick: u64,
    ) -> Result<Self, String> {
        Self::new(
            true,
            reason,
            MeshMultipathRebuildUrgency::UrgentFailover,
            schedule_generation,
            schedule_fingerprint,
            telemetry_epoch,
            telemetry_observed_tick,
        )
    }

    pub fn urgent_failover_with_dirty_scope(
        reason: &str,
        schedule_generation: u64,
        schedule_fingerprint: u64,
        telemetry_epoch: u64,
        telemetry_observed_tick: u64,
        dirty_metadata: MeshMultipathRebuildDirtyMetadata,
    ) -> Result<Self, String> {
        Self::from_input(MeshMultipathRebuildSignalInput {
            rebuild_recommended: true,
            reason,
            urgency: MeshMultipathRebuildUrgency::UrgentFailover,
            schedule_generation,
            schedule_fingerprint,
            telemetry_epoch,
            telemetry_observed_tick,
            dirty_metadata,
        })
    }

    pub fn hard_safety(
        reason: &str,
        schedule_generation: u64,
        schedule_fingerprint: u64,
        telemetry_epoch: u64,
        telemetry_observed_tick: u64,
    ) -> Result<Self, String> {
        Self::new(
            true,
            reason,
            MeshMultipathRebuildUrgency::HardSafety,
            schedule_generation,
            schedule_fingerprint,
            telemetry_epoch,
            telemetry_observed_tick,
        )
    }

    pub fn rebuild_recommended(&self) -> bool {
        self.rebuild_recommended
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn urgency(&self) -> MeshMultipathRebuildUrgency {
        self.urgency
    }

    pub fn schedule_generation(&self) -> u64 {
        self.schedule_generation
    }

    pub(super) fn schedule_fingerprint(&self) -> u64 {
        self.schedule_fingerprint
    }

    pub fn telemetry_epoch(&self) -> u64 {
        self.telemetry_epoch
    }

    pub fn telemetry_observed_tick(&self) -> u64 {
        self.telemetry_observed_tick
    }

    pub fn dirty_scope(&self) -> MeshMultipathRebuildDirtyScope {
        self.dirty_scope
    }

    pub fn affected_peer_count(&self) -> usize {
        self.affected_peer_count
    }
}

impl std::fmt::Debug for MeshMultipathRebuildSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshMultipathRebuildSignal")
            .field("rebuild_recommended", &self.rebuild_recommended)
            .field("reason", &self.reason)
            .field("urgency", &self.urgency)
            .field("schedule_generation", &self.schedule_generation)
            .field("schedule_fingerprint", &"<redacted>")
            .field("telemetry_epoch", &self.telemetry_epoch)
            .field("telemetry_observed_tick", &self.telemetry_observed_tick)
            .field("dirty_scope", &self.dirty_scope)
            .field("affected_peer_count", &self.affected_peer_count)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct MeshMultipathRebuildDecision {
    pub action: MeshMultipathRebuildAction,
    pub reason: String,
    pub signal_reason: String,
    pub rebuild_allowed: bool,
    pub debounced: bool,
    pub stale: bool,
    pub generation_changed: bool,
    pub fingerprint_changed: bool,
    pub pending_count: u64,
    pub dirty_scope: MeshMultipathRebuildDirtyScope,
    pub affected_peer_count: usize,
    pub policy: String,
    pub privacy: String,
    pub explain: Vec<String>,
}

impl std::fmt::Debug for MeshMultipathRebuildDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshMultipathRebuildDecision")
            .field("action", &self.action)
            .field("reason", &self.reason)
            .field("signal_reason", &self.signal_reason)
            .field("rebuild_allowed", &self.rebuild_allowed)
            .field("debounced", &self.debounced)
            .field("stale", &self.stale)
            .field("generation_changed", &self.generation_changed)
            .field("fingerprint_changed", &self.fingerprint_changed)
            .field("pending_count", &self.pending_count)
            .field("dirty_scope", &self.dirty_scope)
            .field("affected_peer_count", &self.affected_peer_count)
            .field("policy", &self.policy)
            .field("privacy", &self.privacy)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MeshMultipathRebuildChanges {
    pub generation_changed: bool,
    pub fingerprint_changed: bool,
}

pub(super) fn build_decision(
    action: MeshMultipathRebuildAction,
    reason: &str,
    signal: &MeshMultipathRebuildSignal,
    changes: MeshMultipathRebuildChanges,
    stale: bool,
    debounced: bool,
    pending_count: u64,
) -> MeshMultipathRebuildDecision {
    let rebuild_allowed = action == MeshMultipathRebuildAction::AllowRebuild;
    let explain = vec![
        format!("multipath_rebuild_action={}", action.as_str()),
        format!("multipath_rebuild_reason={reason}"),
        format!("multipath_rebuild_signal_reason={}", signal.reason()),
        format!("multipath_rebuild_allowed={rebuild_allowed}"),
        format!("multipath_rebuild_debounced={debounced}"),
        format!("multipath_rebuild_stale={stale}"),
        format!(
            "multipath_rebuild_generation_changed={}",
            changes.generation_changed
        ),
        format!(
            "multipath_rebuild_fingerprint_changed={}",
            changes.fingerprint_changed
        ),
        format!(
            "multipath_rebuild_dirty_scope={}",
            signal.dirty_scope().as_str()
        ),
        format!(
            "multipath_rebuild_affected_peer_count={}",
            signal.affected_peer_count()
        ),
        format!("multipath_rebuild_pending_count={pending_count}"),
        format!("multipath_rebuild_policy={REBUILD_CONTROL_POLICY}"),
        format!("multipath_rebuild_privacy={REBUILD_CONTROL_PRIVACY}"),
    ];
    MeshMultipathRebuildDecision {
        action,
        reason: reason.to_string(),
        signal_reason: signal.reason().to_string(),
        rebuild_allowed,
        debounced,
        stale,
        generation_changed: changes.generation_changed,
        fingerprint_changed: changes.fingerprint_changed,
        pending_count,
        dirty_scope: signal.dirty_scope(),
        affected_peer_count: signal.affected_peer_count(),
        policy: REBUILD_CONTROL_POLICY.to_string(),
        privacy: REBUILD_CONTROL_PRIVACY.to_string(),
        explain,
    }
}

pub(super) fn validate_rebuild_reason(reason: &str) -> Result<(), String> {
    if reason.is_empty() {
        return Err("multipath rebuild reason is empty".to_string());
    }
    if reason.trim() != reason {
        return Err("multipath rebuild reason must not contain surrounding whitespace".to_string());
    }
    if reason.len() > 64 {
        return Err("multipath rebuild reason is too long".to_string());
    }
    if !reason
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err("multipath rebuild reason must be a lowercase enum label".to_string());
    }
    if !allowed_rebuild_reason(reason) {
        return Err("multipath rebuild reason is not allowlisted".to_string());
    }
    Ok(())
}

fn validate_dirty_scope(
    dirty_scope: MeshMultipathRebuildDirtyScope,
    affected_peer_count: usize,
) -> Result<(), String> {
    match dirty_scope {
        MeshMultipathRebuildDirtyScope::Unknown => {
            if affected_peer_count == 0 {
                Ok(())
            } else {
                Err(
                    "multipath rebuild dirty scope unknown must use affected_peer_count=0"
                        .to_string(),
                )
            }
        }
        MeshMultipathRebuildDirtyScope::PeerSet => {
            if affected_peer_count == 0 {
                Err(
                    "multipath rebuild peer-set dirty scope requires affected_peer_count > 0"
                        .to_string(),
                )
            } else {
                Ok(())
            }
        }
    }
}

fn allowed_rebuild_reason(reason: &str) -> bool {
    matches!(
        reason,
        "active_lanes_below_plan"
            | "active_binding_capacity_missing"
            | "active_binding_capacity_over_budget"
            | "active_binding_missing"
            | "capacity_pressure"
            | "capacity_overflow"
            | "demand_rebuild_recommended"
            | "duplicate_active_lane"
            | "published_endpoint_changed"
            | "peer_health_changed"
            | "peer_performance_changed"
            | "peer_table_changed"
            | "local_reserve_invalid"
            | "route_binding_mismatch"
            | "transit_payload_policy_not_opaque"
            | "urgent_failover"
            | "weighted_selection_no_match"
    )
}
