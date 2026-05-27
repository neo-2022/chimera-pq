use super::*;

pub(super) fn effective_target_distinct_regions(
    configured_target: usize,
    max_entries: usize,
    profile: MeshPathProfile,
) -> (usize, &'static str) {
    let (adjusted, source) = match profile {
        MeshPathProfile::Fast => (configured_target.min(1), "fast:auto_compact"),
        MeshPathProfile::Balanced => (configured_target, "balanced:configured"),
        MeshPathProfile::Resilient => (configured_target.max(2), "resilient:auto_raise"),
    };
    (adjusted.min(max_entries), source)
}

pub(super) fn effective_min_reliability(
    requested: u8,
    profile: MeshPathProfile,
    avg_reliability: u8,
    auto_mode: bool,
) -> u8 {
    if !auto_mode {
        return requested;
    }
    let adaptive = avg_reliability.saturating_sub(profile_reliability_subtract(profile));
    requested.max(adaptive)
}

pub(super) fn effective_max_peers(
    requested: usize,
    profile: MeshPathProfile,
    auto_mode: bool,
) -> usize {
    if !auto_mode {
        return requested;
    }
    if profile_requires_two_peers(profile) {
        requested.max(2)
    } else {
        requested
    }
}

pub(super) fn effective_min_distinct_regions(
    requested: usize,
    effective_max_peers: usize,
    profile: MeshPathProfile,
    auto_mode: bool,
) -> usize {
    if !auto_mode {
        return requested.min(effective_max_peers);
    }
    let adaptive = profile_min_distinct_regions(profile);
    requested.max(adaptive).min(effective_max_peers)
}

pub(super) fn effective_prefer_region_diversity(
    requested: bool,
    profile: MeshPathProfile,
    auto_mode: bool,
) -> bool {
    if !auto_mode {
        return requested;
    }
    profile_prefers_region_diversity(profile)
}

pub(super) fn effective_max_selected_per_region(
    requested: usize,
    effective_max_peers: usize,
    profile: MeshPathProfile,
    auto_mode: bool,
) -> usize {
    if !auto_mode {
        return requested.min(effective_max_peers);
    }
    let adaptive = if profile_requires_two_peers(profile) {
        requested.max(2)
    } else {
        requested
    };
    adaptive.min(effective_max_peers)
}

pub(super) fn effective_max_load(
    requested: u8,
    profile: MeshPathProfile,
    avg_load: u8,
    auto_mode: bool,
) -> u8 {
    if !auto_mode {
        return requested;
    }
    let adaptive = avg_load
        .saturating_add(profile_max_load_add(profile))
        .min(100);
    requested.min(adaptive)
}

fn profile_reliability_subtract(profile: MeshPathProfile) -> u8 {
    match profile {
        MeshPathProfile::Fast => 5,
        MeshPathProfile::Balanced => 10,
        MeshPathProfile::Resilient => 3,
    }
}

fn profile_max_load_add(profile: MeshPathProfile) -> u8 {
    match profile {
        MeshPathProfile::Fast => 5,
        MeshPathProfile::Balanced => 10,
        MeshPathProfile::Resilient => 15,
    }
}

fn profile_requires_two_peers(profile: MeshPathProfile) -> bool {
    matches!(profile, MeshPathProfile::Resilient)
}

fn profile_min_distinct_regions(profile: MeshPathProfile) -> usize {
    let _ = profile;
    1
}

fn profile_prefers_region_diversity(profile: MeshPathProfile) -> bool {
    !matches!(profile, MeshPathProfile::Fast)
}
