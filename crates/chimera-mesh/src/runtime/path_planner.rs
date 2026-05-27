use super::join_mode::evaluate_join_mode;
use super::path_planner_finalize::{SelectionFinalizeInput, finalize_selection};
use super::path_planner_recovery::{AutoRecoveryInput, run_auto_recovery};
use super::path_planner_setup::build_plan_setup;
use super::standby_shadow_explain::append_standby_shadow_explain;
use super::*;

impl MeshRuntime {
    pub fn plan_path(
        &self,
        request: &MeshJoinRequest,
        policy: &MeshPathPolicy,
    ) -> Result<MeshPathPlan, String> {
        if request.namespace.trim() != self.namespace {
            return Err("mesh request namespace does not match runtime".to_string());
        }
        let join_mode = evaluate_join_mode(request)?;
        policy.validate()?;

        let mut explain = Vec::new();
        let setup = build_plan_setup(self, join_mode.clone(), policy, &mut explain);
        let blocked: BTreeSet<&str> = setup
            .blocked_node_ids
            .iter()
            .map(std::string::String::as_str)
            .collect();
        let health_blocked_all: BTreeSet<&str> = setup
            .health_blocked_all
            .iter()
            .map(std::string::String::as_str)
            .collect();
        let empty_health_blocked = BTreeSet::new();
        let health_blocked = if setup.auto_mode {
            &health_blocked_all
        } else {
            &empty_health_blocked
        };
        let (candidates, stats) = collect_candidates(
            &self.peers,
            &CandidateFilter {
                blocked: &blocked,
                health_blocked,
                allowed_regions: &setup.allowed_regions,
                min_reliability: setup.effective_reliability,
                max_load: setup.effective_load,
                profile: setup.path_profile,
            },
            &mut explain,
        );
        explain.push(format!(
            "effective_min_reliability={}",
            setup.effective_reliability
        ));
        explain.push(format!("effective_max_load={}", setup.effective_load));
        explain.push(format!("effective_max_peers={}", setup.effective_max_peers));
        explain.push(format!(
            "effective_min_distinct_regions={}",
            setup.effective_min_distinct_regions
        ));
        explain.push(format!(
            "effective_prefer_region_diversity={}",
            setup.effective_prefer_region_diversity
        ));
        explain.push(format!(
            "effective_max_selected_per_region={}",
            setup.effective_max_selected_per_region
        ));
        explain.push(format!(
            "effective_filter_source={}",
            if setup.auto_mode {
                "auto_profile"
            } else {
                "manual_override"
            }
        ));
        explain.push(format!(
            "effective_health_filter_source={}",
            if setup.auto_mode {
                "auto"
            } else {
                "manual_disabled"
            }
        ));
        let recovery = run_auto_recovery(
            &self.peers,
            candidates,
            stats,
            AutoRecoveryInput {
                blocked: &blocked,
                health_blocked_all: &health_blocked_all,
                health_blocked,
                allowed_regions: &setup.allowed_regions,
                effective_reliability: setup.effective_reliability,
                effective_load: setup.effective_load,
                effective_max_peers: setup.effective_max_peers,
                auto_mode: setup.auto_mode,
                path_profile: setup.path_profile,
                spread_bonus_weight: self.table_policy.resilient_region_spread_bonus_weight,
            },
            &mut explain,
        );
        let candidates = recovery.candidates;
        let stats = recovery.stats;

        let selected_peers = finalize_selection(
            self,
            SelectionFinalizeInput {
                policy,
                stats,
                candidates,
                effective_prefer_region_diversity: setup.effective_prefer_region_diversity,
                effective_max_peers: setup.effective_max_peers,
                effective_max_selected_per_region: setup.effective_max_selected_per_region,
                effective_min_distinct_regions: setup.effective_min_distinct_regions,
            },
            &mut explain,
        )?;
        append_standby_shadow_explain(&selected_peers, &mut explain);

        Ok(MeshPathPlan {
            namespace: self.namespace.clone(),
            join_mode,
            selected_peers,
            explain,
        })
    }
}
