use super::*;

pub(crate) fn build_region_selection_diagnostics(
    selected_peer_count: usize,
    selected_region_count: usize,
    min_distinct_regions_target: usize,
) -> RegionSelectionDiagnostics {
    let distinct_region_ratio_pct = selected_region_count
        .saturating_mul(100)
        .checked_div(selected_peer_count)
        .unwrap_or(0);
    let min_distinct_regions_met = selected_region_count >= min_distinct_regions_target;
    let distinct_region_deficit = min_distinct_regions_target.saturating_sub(selected_region_count);
    RegionSelectionDiagnostics {
        min_distinct_regions_target,
        min_distinct_regions_met,
        distinct_region_deficit,
        distinct_region_ratio_pct,
    }
}
