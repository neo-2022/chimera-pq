use super::payload_utils::ensure_mesh_payload_nonempty;
use super::standby_shadow_explain::adapt_standby_shadow_from_dps;
use super::*;

#[path = "plan_ops_dps_eval.rs"]
mod dps_eval;
#[path = "plan_ops_failover.rs"]
mod failover;
#[path = "plan_ops_health.rs"]
mod health;

impl MeshRuntime {
    pub fn plan_path_from_dps_payload(
        &self,
        request: &MeshJoinRequest,
        payload: &str,
    ) -> Result<MeshPathPlan, String> {
        dps_eval::plan_path_from_dps_payload(self, request, payload)
    }

    pub fn evaluate_dps_policy_payload(
        &self,
        request: &MeshJoinRequest,
        payload: &str,
    ) -> Result<MeshPathPlan, String> {
        dps_eval::evaluate_dps_policy_payload(self, request, payload)
    }

    pub fn evaluate_dps_policy_payload_with_health(
        &self,
        request: &MeshJoinRequest,
        payload: &str,
        health: &[MeshPeerHealth],
    ) -> Result<MeshPathPlan, String> {
        dps_eval::evaluate_dps_policy_payload_with_health(self, request, payload, health)
    }

    pub fn evaluate_dps_failover_payload(
        &self,
        request: &MeshJoinRequest,
        payload: &str,
        event: &MeshFailoverEvent,
    ) -> Result<MeshPathPlan, String> {
        dps_eval::evaluate_dps_failover_payload(self, request, payload, event)
    }

    pub fn failover_plan(
        &self,
        request: &MeshJoinRequest,
        policy: &MeshPathPolicy,
        event: &MeshFailoverEvent,
    ) -> Result<MeshPathPlan, String> {
        failover::failover_plan(self, request, policy, event)
    }

    pub fn failover_plan_from_dps_payload(
        &self,
        request: &MeshJoinRequest,
        payload: &str,
        event: &MeshFailoverEvent,
    ) -> Result<MeshPathPlan, String> {
        ensure_mesh_payload_nonempty(payload)?;
        let policy = MeshPathPolicy::from_dps_payload(payload)?;
        let mut plan = self.failover_plan(request, &policy, event)?;
        annotate_dps_payload_explain(&mut plan.explain, payload, "failover");
        adapt_standby_shadow_from_dps(&mut plan.explain);
        Ok(plan)
    }

    pub fn reselection_plan_with_health(
        &self,
        request: &MeshJoinRequest,
        policy: &MeshPathPolicy,
        health: &[MeshPeerHealth],
    ) -> Result<MeshPathPlan, String> {
        health::reselection_plan_with_health(self, request, policy, health)
    }

    pub fn reselection_plan_with_health_from_dps_payload(
        &self,
        request: &MeshJoinRequest,
        payload: &str,
        health: &[MeshPeerHealth],
    ) -> Result<MeshPathPlan, String> {
        ensure_mesh_payload_nonempty(payload)?;
        let policy = MeshPathPolicy::from_dps_payload(payload)?;
        let mut plan = self.reselection_plan_with_health(request, &policy, health)?;
        annotate_dps_payload_explain(&mut plan.explain, payload, "reselection");
        adapt_standby_shadow_from_dps(&mut plan.explain);
        Ok(plan)
    }
}
