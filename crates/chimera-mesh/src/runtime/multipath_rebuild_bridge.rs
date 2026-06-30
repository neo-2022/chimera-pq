use super::multipath_demand::{DEMAND_POLICY_SOURCE_CONTROL, DEMAND_POLICY_SOURCE_DEFAULT};
use super::{
    MeshJoinRequest, MeshMultipathRebuildAction, MeshMultipathRebuildDecision,
    MeshMultipathRebuildPolicy, MeshMultipathRebuildSignal, MeshMultipathSchedule, MeshPathPlan,
    MeshPathPlanCore, MeshPathPolicy, MeshRuntime, replace_multipath_schedule,
    replace_multipath_schedule_core,
};
use crate::policy::MultipathDemand;

impl MeshRuntime {
    pub fn plan_path_with_pending_multipath_rebuild(
        &mut self,
        request: &MeshJoinRequest,
        planning_policy: &MeshPathPolicy,
        rebuild_policy: &MeshMultipathRebuildPolicy,
    ) -> Result<(MeshPathPlan, Option<MeshMultipathRebuildDecision>), String> {
        let mut plan = self.plan_path(request, planning_policy)?;
        let decision =
            self.apply_pending_multipath_rebuild_to_fresh_plan(&mut plan, rebuild_policy)?;
        Ok((plan, decision))
    }

    pub fn plan_path_core_with_pending_multipath_rebuild(
        &mut self,
        request: &MeshJoinRequest,
        planning_policy: &MeshPathPolicy,
        rebuild_policy: &MeshMultipathRebuildPolicy,
    ) -> Result<(MeshPathPlanCore, Option<MeshMultipathRebuildDecision>), String> {
        let mut plan = self.plan_path_core(request, planning_policy)?;
        let decision =
            self.apply_pending_multipath_rebuild_to_fresh_plan_core(&mut plan, rebuild_policy)?;
        Ok((plan, decision))
    }

    pub fn apply_multipath_rebuild_to_plan(
        &mut self,
        plan: &mut MeshPathPlan,
        signal: &MeshMultipathRebuildSignal,
        policy: &MeshMultipathRebuildPolicy,
    ) -> Result<MeshMultipathRebuildDecision, String> {
        let decision = self.evaluate_multipath_rebuild(signal, policy)?;
        remove_multipath_rebuild_explain(&mut plan.explain);

        if decision.rebuild_allowed {
            let mode = plan.multipath_schedule.mode;
            let route_binding_id = plan.multipath_schedule.route_binding_id;
            let demand = current_schedule_demand(&plan.multipath_schedule)?;
            replace_multipath_schedule(plan, mode, route_binding_id, demand)?;
        }

        decision.append_explain_to(&mut plan.explain);
        Ok(decision)
    }

    pub fn apply_multipath_rebuild_with_policy_to_plan(
        &mut self,
        request: &MeshJoinRequest,
        planning_policy: &MeshPathPolicy,
        plan: &mut MeshPathPlan,
        signal: &MeshMultipathRebuildSignal,
        rebuild_policy: &MeshMultipathRebuildPolicy,
    ) -> Result<MeshMultipathRebuildDecision, String> {
        let decision = self.evaluate_multipath_rebuild(signal, rebuild_policy)?;
        remove_multipath_rebuild_explain(&mut plan.explain);

        if decision.rebuild_allowed {
            let route_binding_id = plan.multipath_schedule.route_binding_id;
            let mut rebuilt = self.plan_path(request, planning_policy)?;
            let mode = rebuilt.multipath_schedule.mode;
            let demand = current_schedule_demand(&rebuilt.multipath_schedule)?;
            replace_multipath_schedule(&mut rebuilt, mode, route_binding_id, demand)?;
            *plan = rebuilt;
        }

        decision.append_explain_to(&mut plan.explain);
        Ok(decision)
    }

    pub fn apply_pending_multipath_rebuild_with_policy_to_plan(
        &mut self,
        request: &MeshJoinRequest,
        planning_policy: &MeshPathPolicy,
        plan: &mut MeshPathPlan,
        rebuild_policy: &MeshMultipathRebuildPolicy,
    ) -> Result<Option<MeshMultipathRebuildDecision>, String> {
        let Some(signal) = self.take_pending_multipath_rebuild_signal() else {
            return Ok(None);
        };
        match self.apply_multipath_rebuild_with_policy_to_plan(
            request,
            planning_policy,
            plan,
            &signal,
            rebuild_policy,
        ) {
            Ok(decision) => {
                if decision.action == MeshMultipathRebuildAction::FailClosed {
                    return Err(format!(
                        "mesh multipath pending rebuild failed closed: {}",
                        decision.reason
                    ));
                }
                Ok(Some(decision))
            }
            Err(error) => {
                self.restore_pending_multipath_rebuild_signal(signal);
                Err(error)
            }
        }
    }

    pub fn apply_multipath_rebuild_to_plan_core(
        &mut self,
        plan: &mut MeshPathPlanCore,
        signal: &MeshMultipathRebuildSignal,
        policy: &MeshMultipathRebuildPolicy,
    ) -> Result<MeshMultipathRebuildDecision, String> {
        let decision = self.evaluate_multipath_rebuild(signal, policy)?;

        if decision.rebuild_allowed {
            let mode = plan.multipath_schedule.mode;
            let route_binding_id = plan.multipath_schedule.route_binding_id;
            let demand = current_schedule_demand(&plan.multipath_schedule)?;
            replace_multipath_schedule_core(plan, mode, route_binding_id, demand)?;
        }

        Ok(decision)
    }

    pub fn apply_multipath_rebuild_with_policy_to_plan_core(
        &mut self,
        request: &MeshJoinRequest,
        planning_policy: &MeshPathPolicy,
        plan: &mut MeshPathPlanCore,
        signal: &MeshMultipathRebuildSignal,
        rebuild_policy: &MeshMultipathRebuildPolicy,
    ) -> Result<MeshMultipathRebuildDecision, String> {
        let decision = self.evaluate_multipath_rebuild(signal, rebuild_policy)?;

        if decision.rebuild_allowed {
            let route_binding_id = plan.multipath_schedule.route_binding_id;
            let mut rebuilt = self.plan_path_core(request, planning_policy)?;
            let mode = rebuilt.multipath_schedule.mode;
            let demand = current_schedule_demand(&rebuilt.multipath_schedule)?;
            replace_multipath_schedule_core(&mut rebuilt, mode, route_binding_id, demand)?;
            *plan = rebuilt;
        }

        Ok(decision)
    }

    pub fn apply_pending_multipath_rebuild_with_policy_to_plan_core(
        &mut self,
        request: &MeshJoinRequest,
        planning_policy: &MeshPathPolicy,
        plan: &mut MeshPathPlanCore,
        rebuild_policy: &MeshMultipathRebuildPolicy,
    ) -> Result<Option<MeshMultipathRebuildDecision>, String> {
        let Some(signal) = self.take_pending_multipath_rebuild_signal() else {
            return Ok(None);
        };
        match self.apply_multipath_rebuild_with_policy_to_plan_core(
            request,
            planning_policy,
            plan,
            &signal,
            rebuild_policy,
        ) {
            Ok(decision) => {
                if decision.action == MeshMultipathRebuildAction::FailClosed {
                    return Err(format!(
                        "mesh multipath pending rebuild failed closed: {}",
                        decision.reason
                    ));
                }
                Ok(Some(decision))
            }
            Err(error) => {
                self.restore_pending_multipath_rebuild_signal(signal);
                Err(error)
            }
        }
    }

    fn apply_pending_multipath_rebuild_to_fresh_plan(
        &mut self,
        plan: &mut MeshPathPlan,
        rebuild_policy: &MeshMultipathRebuildPolicy,
    ) -> Result<Option<MeshMultipathRebuildDecision>, String> {
        let Some(signal) = self.take_pending_multipath_rebuild_signal() else {
            return Ok(None);
        };
        match self.apply_multipath_rebuild_to_plan(plan, &signal, rebuild_policy) {
            Ok(decision) => {
                if decision.action == MeshMultipathRebuildAction::FailClosed {
                    return Err(format!(
                        "mesh multipath pending rebuild failed closed: {}",
                        decision.reason
                    ));
                }
                Ok(Some(decision))
            }
            Err(error) => {
                self.restore_pending_multipath_rebuild_signal(signal);
                Err(error)
            }
        }
    }

    fn apply_pending_multipath_rebuild_to_fresh_plan_core(
        &mut self,
        plan: &mut MeshPathPlanCore,
        rebuild_policy: &MeshMultipathRebuildPolicy,
    ) -> Result<Option<MeshMultipathRebuildDecision>, String> {
        let Some(signal) = self.take_pending_multipath_rebuild_signal() else {
            return Ok(None);
        };
        match self.apply_multipath_rebuild_to_plan_core(plan, &signal, rebuild_policy) {
            Ok(decision) => {
                if decision.action == MeshMultipathRebuildAction::FailClosed {
                    return Err(format!(
                        "mesh multipath pending rebuild failed closed: {}",
                        decision.reason
                    ));
                }
                Ok(Some(decision))
            }
            Err(error) => {
                self.restore_pending_multipath_rebuild_signal(signal);
                Err(error)
            }
        }
    }
}

fn current_schedule_demand(
    schedule: &MeshMultipathSchedule,
) -> Result<Option<MultipathDemand>, String> {
    match schedule.demand_policy_source.as_str() {
        DEMAND_POLICY_SOURCE_DEFAULT => Ok(None),
        DEMAND_POLICY_SOURCE_CONTROL => MultipathDemand::from_dps_value(&schedule.demand_policy)
            .map(Some)
            .map_err(|_| "mesh multipath rebuild bridge demand policy is invalid".to_string()),
        _ => Err("mesh multipath rebuild bridge demand policy source is invalid".to_string()),
    }
}

fn remove_multipath_rebuild_explain(explain: &mut Vec<String>) {
    explain.retain(|line| !line.starts_with("multipath_rebuild_"));
}
