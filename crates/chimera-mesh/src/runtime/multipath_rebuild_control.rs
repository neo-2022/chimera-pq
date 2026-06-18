use super::MeshRuntime;
use super::multipath_rebuild_model::{
    MeshMultipathRebuildAction, MeshMultipathRebuildChanges, MeshMultipathRebuildDecision,
    MeshMultipathRebuildPolicy, MeshMultipathRebuildSignal, MeshMultipathRebuildUrgency,
    build_decision, validate_rebuild_reason,
};

#[derive(Clone, PartialEq, Eq, Default)]
pub(super) struct MeshMultipathRebuildState {
    last_allowed: Option<AllowedRebuild>,
    pending_count: u64,
}

#[derive(Clone, PartialEq, Eq)]
struct AllowedRebuild {
    tick: u64,
    reason: String,
    schedule_generation: u64,
    schedule_fingerprint: u64,
    telemetry_epoch: u64,
}

impl std::fmt::Debug for MeshMultipathRebuildState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshMultipathRebuildState")
            .field("last_allowed", &self.last_allowed)
            .field("pending_count", &self.pending_count)
            .finish()
    }
}

impl std::fmt::Debug for AllowedRebuild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AllowedRebuild")
            .field("tick", &self.tick)
            .field("reason", &self.reason)
            .field("schedule_generation", &self.schedule_generation)
            .field("schedule_fingerprint", &"<redacted>")
            .field("telemetry_epoch", &self.telemetry_epoch)
            .finish()
    }
}

impl MeshRuntime {
    pub fn evaluate_multipath_rebuild(
        &mut self,
        signal: &MeshMultipathRebuildSignal,
        policy: &MeshMultipathRebuildPolicy,
    ) -> Result<MeshMultipathRebuildDecision, String> {
        policy.validate()?;
        validate_rebuild_reason(signal.reason())?;
        Ok(self
            .multipath_rebuild_state
            .evaluate(signal, policy, self.tick))
    }
}

impl MeshMultipathRebuildState {
    fn evaluate(
        &mut self,
        signal: &MeshMultipathRebuildSignal,
        policy: &MeshMultipathRebuildPolicy,
        now_tick: u64,
    ) -> MeshMultipathRebuildDecision {
        let comparison = RebuildComparison::new(self.last_allowed.as_ref(), signal);
        if signal.telemetry_observed_tick() > now_tick {
            return self.fail_closed("telemetry_from_future", signal, comparison, true, false);
        }
        let stale =
            now_tick.saturating_sub(signal.telemetry_observed_tick()) > policy.freshness_ttl_ticks;
        if stale {
            return self.fail_closed("stale_telemetry", signal, comparison, true, false);
        }
        if signal.urgency() == MeshMultipathRebuildUrgency::HardSafety {
            return self.fail_closed("hard_safety_fail_closed", signal, comparison, false, false);
        }
        if !signal.rebuild_recommended() {
            return self.suppress("rebuild_not_recommended", signal, comparison, false, false);
        }
        if signal.urgency() == MeshMultipathRebuildUrgency::UrgentFailover {
            return self.allow("urgent_failover", signal, comparison, false, now_tick);
        }
        let Some(last_allowed) = self.last_allowed.as_ref() else {
            return self.allow("initial_observation", signal, comparison, false, now_tick);
        };
        let same_signal = comparison.same_signal();
        let elapsed = now_tick.saturating_sub(last_allowed.tick);
        if same_signal && elapsed < policy.debounce_window_ticks {
            return self.suppress(
                "debounced_same_fingerprint",
                signal,
                comparison,
                false,
                true,
            );
        }
        let reason = comparison.allow_reason();
        self.allow(reason, signal, comparison, false, now_tick)
    }

    fn allow(
        &mut self,
        reason: &str,
        signal: &MeshMultipathRebuildSignal,
        comparison: RebuildComparison,
        debounced: bool,
        now_tick: u64,
    ) -> MeshMultipathRebuildDecision {
        self.last_allowed = Some(AllowedRebuild {
            tick: now_tick,
            reason: signal.reason().to_string(),
            schedule_generation: signal.schedule_generation(),
            schedule_fingerprint: signal.schedule_fingerprint(),
            telemetry_epoch: signal.telemetry_epoch(),
        });
        self.pending_count = 0;
        self.decision(
            MeshMultipathRebuildAction::AllowRebuild,
            reason,
            signal,
            comparison,
            false,
            debounced,
        )
    }

    fn suppress(
        &mut self,
        reason: &str,
        signal: &MeshMultipathRebuildSignal,
        comparison: RebuildComparison,
        stale: bool,
        debounced: bool,
    ) -> MeshMultipathRebuildDecision {
        self.pending_count = self.pending_count.saturating_add(1);
        self.decision(
            MeshMultipathRebuildAction::SuppressRebuild,
            reason,
            signal,
            comparison,
            stale,
            debounced,
        )
    }

    fn fail_closed(
        &mut self,
        reason: &str,
        signal: &MeshMultipathRebuildSignal,
        comparison: RebuildComparison,
        stale: bool,
        debounced: bool,
    ) -> MeshMultipathRebuildDecision {
        self.pending_count = self.pending_count.saturating_add(1);
        self.decision(
            MeshMultipathRebuildAction::FailClosed,
            reason,
            signal,
            comparison,
            stale,
            debounced,
        )
    }

    fn decision(
        &self,
        action: MeshMultipathRebuildAction,
        reason: &str,
        signal: &MeshMultipathRebuildSignal,
        comparison: RebuildComparison,
        stale: bool,
        debounced: bool,
    ) -> MeshMultipathRebuildDecision {
        build_decision(
            action,
            reason,
            signal,
            comparison.changes(),
            stale,
            debounced,
            self.pending_count,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RebuildComparison {
    reason_changed: bool,
    generation_changed: bool,
    fingerprint_changed: bool,
    telemetry_epoch_changed: bool,
}

impl RebuildComparison {
    fn new(last_allowed: Option<&AllowedRebuild>, signal: &MeshMultipathRebuildSignal) -> Self {
        let Some(last_allowed) = last_allowed else {
            return Self {
                reason_changed: true,
                generation_changed: true,
                fingerprint_changed: true,
                telemetry_epoch_changed: true,
            };
        };
        Self {
            reason_changed: last_allowed.reason != signal.reason(),
            generation_changed: last_allowed.schedule_generation != signal.schedule_generation(),
            fingerprint_changed: last_allowed.schedule_fingerprint != signal.schedule_fingerprint(),
            telemetry_epoch_changed: last_allowed.telemetry_epoch != signal.telemetry_epoch(),
        }
    }

    fn same_signal(self) -> bool {
        !self.reason_changed
            && !self.generation_changed
            && !self.fingerprint_changed
            && !self.telemetry_epoch_changed
    }

    fn allow_reason(self) -> &'static str {
        if self.reason_changed {
            "reason_changed"
        } else if self.generation_changed {
            "generation_changed"
        } else if self.fingerprint_changed {
            "fingerprint_changed"
        } else if self.telemetry_epoch_changed {
            "telemetry_epoch_changed"
        } else {
            "debounce_window_elapsed"
        }
    }

    fn changes(self) -> MeshMultipathRebuildChanges {
        MeshMultipathRebuildChanges {
            generation_changed: self.generation_changed,
            fingerprint_changed: self.fingerprint_changed,
        }
    }
}
