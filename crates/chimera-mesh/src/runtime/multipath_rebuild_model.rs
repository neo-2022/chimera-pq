const REBUILD_CONTROL_POLICY: &str = "runtime_owned_debounce_v1";
const REBUILD_CONTROL_PRIVACY: &str = "aggregate_only";
const ACTION_ALLOW_REBUILD: &str = "allow_rebuild";
const ACTION_SUPPRESS_REBUILD: &str = "suppress_rebuild";
const ACTION_FAIL_CLOSED: &str = "fail_closed";

#[path = "multipath_rebuild_model/decision.rs"]
mod decision;
#[path = "multipath_rebuild_model/validation.rs"]
mod validation;
pub use decision::MeshMultipathRebuildDecision;
pub(super) use decision::{MeshMultipathRebuildChanges, build_decision};
use validation::validate_dirty_scope;
pub(super) use validation::validate_rebuild_reason;

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
    RuntimeAnnouncements,
}

impl MeshMultipathRebuildDirtyScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::PeerSet => "peer_set",
            Self::RuntimeAnnouncements => "runtime_announcements",
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

    pub fn runtime_announcements(affected_announcement_count: usize) -> Result<Self, String> {
        let metadata = Self {
            scope: MeshMultipathRebuildDirtyScope::RuntimeAnnouncements,
            affected_peer_count: affected_announcement_count,
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
