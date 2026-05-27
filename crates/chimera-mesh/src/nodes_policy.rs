#[derive(Debug, Clone, PartialEq, Default)]
pub struct MeshNodesPolicy {
    pub score: MeshNodeScorePolicy,
    pub anti_flap: MeshNodeAntiFlapPolicy,
    pub geo: MeshNodeGeoPolicy,
    pub autoconnect_enabled_by_default: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshNodeScorePolicy {
    pub thresholds: MeshNodeScoreThresholds,
    pub weights: MeshNodeScoreWeights,
    pub failure_penalty_weight: f64,
    pub max_consecutive_failures_for_penalty: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshNodeScoreThresholds {
    pub latency_good_ms: f64,
    pub latency_bad_ms: f64,
    pub jitter_good_ms: f64,
    pub jitter_bad_ms: f64,
    pub loss_good_pct: f64,
    pub loss_bad_pct: f64,
    pub min_observation_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshNodeScoreWeights {
    pub latency: f64,
    pub jitter: f64,
    pub loss: f64,
    pub success_5m: f64,
    pub success_1h: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshNodeAntiFlapPolicy {
    pub hysteresis_margin: u8,
    pub hold_down_ticks: u64,
    pub max_switches_per_window: usize,
    pub switch_window_ticks: u64,
    pub return_success_required: u32,
    pub jittered_retry_pct: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshNodeGeoPolicy {
    pub enabled: bool,
    pub ttl_ticks: u64,
    pub cache_max_entries: usize,
    pub timeout_ms: u64,
}

impl Default for MeshNodeScorePolicy {
    fn default() -> Self {
        Self {
            thresholds: MeshNodeScoreThresholds::default(),
            weights: MeshNodeScoreWeights::default(),
            failure_penalty_weight: 0.5,
            max_consecutive_failures_for_penalty: 5,
        }
    }
}

impl Default for MeshNodeScoreThresholds {
    fn default() -> Self {
        Self {
            latency_good_ms: 30.0,
            latency_bad_ms: 250.0,
            jitter_good_ms: 5.0,
            jitter_bad_ms: 80.0,
            loss_good_pct: 0.1,
            loss_bad_pct: 5.0,
            min_observation_count: 5,
        }
    }
}

impl Default for MeshNodeScoreWeights {
    fn default() -> Self {
        Self {
            latency: 0.30,
            jitter: 0.20,
            loss: 0.20,
            success_5m: 0.20,
            success_1h: 0.10,
        }
    }
}

impl Default for MeshNodeAntiFlapPolicy {
    fn default() -> Self {
        Self {
            hysteresis_margin: 8,
            hold_down_ticks: 60,
            max_switches_per_window: 3,
            switch_window_ticks: 600,
            return_success_required: 3,
            jittered_retry_pct: 20,
        }
    }
}

impl Default for MeshNodeGeoPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_ticks: 86_400,
            cache_max_entries: 10_000,
            timeout_ms: 1_000,
        }
    }
}

impl MeshNodesPolicy {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        self.score.validate_into(&mut errors);
        self.anti_flap.validate_into(&mut errors);
        self.geo.validate_into(&mut errors);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl MeshNodeScorePolicy {
    fn validate_into(&self, errors: &mut Vec<String>) {
        self.thresholds.validate_into(errors);
        self.weights.validate_into(errors);
        if !(0.0..=1.0).contains(&self.failure_penalty_weight) {
            errors.push("mesh node failure_penalty_weight must be in 0..1".to_string());
        }
        if self.max_consecutive_failures_for_penalty == 0 {
            errors.push("mesh node max_consecutive_failures_for_penalty must be > 0".to_string());
        }
    }
}

impl MeshNodeScoreThresholds {
    fn validate_into(&self, errors: &mut Vec<String>) {
        if !(self.latency_bad_ms > self.latency_good_ms && self.latency_good_ms >= 0.0) {
            errors.push("mesh node latency thresholds must be monotonic".to_string());
        }
        if !(self.jitter_bad_ms > self.jitter_good_ms && self.jitter_good_ms >= 0.0) {
            errors.push("mesh node jitter thresholds must be monotonic".to_string());
        }
        if !(self.loss_bad_pct > self.loss_good_pct
            && self.loss_good_pct >= 0.0
            && self.loss_bad_pct <= 100.0)
        {
            errors.push("mesh node loss thresholds must be monotonic and <= 100".to_string());
        }
        if self.min_observation_count == 0 {
            errors.push("mesh node min_observation_count must be > 0".to_string());
        }
    }
}

impl MeshNodeScoreWeights {
    fn validate_into(&self, errors: &mut Vec<String>) {
        for (name, value) in [
            ("latency", self.latency),
            ("jitter", self.jitter),
            ("loss", self.loss),
            ("success_5m", self.success_5m),
            ("success_1h", self.success_1h),
        ] {
            if !(0.0..=1.0).contains(&value) {
                errors.push(format!("mesh node score weight {name} must be in 0..1"));
            }
        }
        let sum = self.latency + self.jitter + self.loss + self.success_5m + self.success_1h;
        if (sum - 1.0).abs() > 0.0001 {
            errors.push(format!(
                "mesh node score weights must sum to 1.0; got {sum}"
            ));
        }
    }
}

impl MeshNodeAntiFlapPolicy {
    fn validate_into(&self, errors: &mut Vec<String>) {
        if self.max_switches_per_window == 0 {
            errors.push("mesh node max_switches_per_window must be >= 1".to_string());
        }
        if self.switch_window_ticks == 0 {
            errors.push("mesh node switch_window_ticks must be > 0".to_string());
        }
        if self.jittered_retry_pct > 100 {
            errors.push("mesh node jittered_retry_pct must be <= 100".to_string());
        }
    }
}

impl MeshNodeGeoPolicy {
    fn validate_into(&self, errors: &mut Vec<String>) {
        if self.ttl_ticks == 0 {
            errors.push("mesh node geo ttl_ticks must be > 0".to_string());
        }
        if self.cache_max_entries == 0 {
            errors.push("mesh node geo cache_max_entries must be > 0".to_string());
        }
        if self.timeout_ms == 0 {
            errors.push("mesh node geo timeout_ms must be > 0".to_string());
        }
    }
}

pub(crate) fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

pub(crate) fn clamp_score(value: f64) -> f64 {
    value.clamp(0.0, 100.0)
}
