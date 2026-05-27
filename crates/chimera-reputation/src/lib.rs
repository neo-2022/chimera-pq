#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplaintEvidence {
    pub node_id: String,
    pub reason_code: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReputationState {
    pub node_id: String,
    pub score: i32,
}

impl ComplaintEvidence {
    pub fn validate(&self) -> Result<(), String> {
        if self.node_id.trim().is_empty() {
            return Err("complaint node_id is empty".to_string());
        }
        if self.reason_code.trim().is_empty() {
            return Err("complaint reason_code is empty".to_string());
        }
        if self.signature.len() < 8 {
            return Err("complaint signature is too short".to_string());
        }
        Ok(())
    }
}

pub fn apply_penalty(state: &mut ReputationState, penalty: i32) -> Result<(), String> {
    if penalty <= 0 {
        return Err("penalty must be positive".to_string());
    }
    state.score = state.score.saturating_sub(penalty);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ComplaintEvidence, ReputationState, apply_penalty};

    #[test]
    fn complaint_validation_passes_for_valid_input() {
        let evidence = ComplaintEvidence {
            node_id: "node-1".to_string(),
            reason_code: "relay_misuse".to_string(),
            signature: "abcdef123456".to_string(),
        };
        assert!(evidence.validate().is_ok());
    }

    #[test]
    fn short_signature_is_rejected() {
        let evidence = ComplaintEvidence {
            node_id: "node-1".to_string(),
            reason_code: "relay_misuse".to_string(),
            signature: "short".to_string(),
        };
        assert!(evidence.validate().is_err());
    }

    #[test]
    fn penalty_reduces_score() {
        let mut state = ReputationState {
            node_id: "node-1".to_string(),
            score: 100,
        };
        let applied = apply_penalty(&mut state, 7);
        assert!(applied.is_ok());
        assert_eq!(state.score, 93);
    }
}
