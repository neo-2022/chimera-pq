use std::collections::BTreeMap;

use crate::model::MeshPeerState;

use super::selection_policy::normalize_region_key;
use super::selection_profile::score_for_profile;
use super::{CandidateFilter, CandidateSlot, CandidateStats};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandidateRejectReason {
    BlockedNode,
    Health,
    Region,
    Reliability,
    Load,
}

impl CandidateRejectReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::BlockedNode => "blocked_node",
            Self::Health => "health",
            Self::Region => "region",
            Self::Reliability => "reliability",
            Self::Load => "load",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CandidateEval {
    Accepted {
        score: i32,
        normalized_region: String,
    },
    Rejected(CandidateRejectReason),
}

pub(super) fn collect_candidates<'p>(
    peers: &'p BTreeMap<String, MeshPeerState>,
    filter: &CandidateFilter<'_>,
    explain: &mut Vec<String>,
) -> (Vec<CandidateSlot<'p>>, CandidateStats) {
    let mut candidates = Vec::with_capacity(peers.len());
    let mut stats = CandidateStats {
        rejected_blocked: 0,
        rejected_health: 0,
        rejected_region: 0,
        rejected_reliability: 0,
        rejected_load: 0,
        accepted_count: 0,
    };
    explain.reserve(peers.len());

    for (index, peer) in peers.values().enumerate() {
        match evaluate_candidate_peer(peer, filter) {
            CandidateEval::Accepted {
                score,
                normalized_region,
            } => {
                stats.accepted_count = stats.accepted_count.saturating_add(1);
                explain.push(format!(
                    "candidate#{} accepted score={score}",
                    index.saturating_add(1)
                ));
                candidates.push(CandidateSlot {
                    peer,
                    normalized_region,
                    selection_score: score,
                });
            }
            CandidateEval::Rejected(reason) => {
                register_candidate_rejection(&mut stats, reason);
                explain.push(format!(
                    "candidate#{} rejected={}",
                    index.saturating_add(1),
                    reason.as_str()
                ));
            }
        }
    }
    (candidates, stats)
}

fn evaluate_candidate_peer(peer: &MeshPeerState, filter: &CandidateFilter<'_>) -> CandidateEval {
    if filter.blocked.contains(peer.node_id.as_str()) {
        return CandidateEval::Rejected(CandidateRejectReason::BlockedNode);
    }
    if filter.health_blocked.contains(peer.node_id.as_str()) {
        return CandidateEval::Rejected(CandidateRejectReason::Health);
    }
    let peer_region_key = if filter.allowed_regions.is_empty() {
        None
    } else {
        Some(normalize_region_key(&peer.region))
    };
    if let Some(peer_region_key) = &peer_region_key
        && !filter.allowed_regions.contains(peer_region_key)
    {
        return CandidateEval::Rejected(CandidateRejectReason::Region);
    }
    if peer.reliability_score < filter.min_reliability {
        return CandidateEval::Rejected(CandidateRejectReason::Reliability);
    }
    if peer.load_score > filter.max_load {
        return CandidateEval::Rejected(CandidateRejectReason::Load);
    }
    let normalized_region = peer_region_key.unwrap_or_else(|| normalize_region_key(&peer.region));
    CandidateEval::Accepted {
        score: score_for_profile(peer, filter.profile),
        normalized_region,
    }
}

fn register_candidate_rejection(stats: &mut CandidateStats, reason: CandidateRejectReason) {
    match reason {
        CandidateRejectReason::BlockedNode => {
            stats.rejected_blocked = stats.rejected_blocked.saturating_add(1);
        }
        CandidateRejectReason::Health => {
            stats.rejected_health = stats.rejected_health.saturating_add(1);
        }
        CandidateRejectReason::Region => {
            stats.rejected_region = stats.rejected_region.saturating_add(1);
        }
        CandidateRejectReason::Reliability => {
            stats.rejected_reliability = stats.rejected_reliability.saturating_add(1);
        }
        CandidateRejectReason::Load => {
            stats.rejected_load = stats.rejected_load.saturating_add(1);
        }
    }
}
