use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::nodes_model::{
    MeshNode, MeshNodeCountry, MeshNodeStatus, cmp_f64_desc, cmp_optional_f64_asc,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MeshNodeListFilter {
    pub countries: BTreeSet<String>,
    pub statuses: BTreeSet<MeshNodeStatus>,
    pub available_only: bool,
    pub search: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshNodeCountryGroup {
    pub country_code: String,
    pub country_name: String,
    pub nodes: Vec<MeshNode>,
}

pub fn group_mesh_nodes_by_country(
    nodes: &[MeshNode],
    filter: &MeshNodeListFilter,
) -> Vec<MeshNodeCountryGroup> {
    let mut groups: Vec<MeshNodeCountryGroup> = Vec::new();
    for node in nodes {
        if !node_matches_filter(node, filter) {
            continue;
        }
        if let Some(group) = groups
            .iter_mut()
            .find(|group| group.country_code == node.country.country_code)
        {
            group.nodes.push(node.clone());
        } else {
            groups.push(MeshNodeCountryGroup {
                country_code: node.country.country_code.clone(),
                country_name: node.country.country_name.clone(),
                nodes: vec![node.clone()],
            });
        }
    }
    groups.sort_by(compare_country_groups);
    for group in &mut groups {
        sort_mesh_nodes_for_list(&mut group.nodes);
    }
    groups
}

pub fn sort_mesh_nodes_for_list(nodes: &mut [MeshNode]) {
    nodes.sort_by(compare_mesh_nodes_for_list);
}

pub(crate) fn compare_mesh_nodes_for_best(a: &MeshNode, b: &MeshNode) -> Ordering {
    cmp_f64_desc(a.score, b.score)
        .then_with(|| a.status.sort_rank().cmp(&b.status.sort_rank()))
        .then_with(|| cmp_optional_f64_asc(a.latency_ms, b.latency_ms))
        .then_with(|| cmp_optional_f64_asc(a.loss_pct, b.loss_pct))
        .then_with(|| b.observation_count.cmp(&a.observation_count))
        .then_with(|| a.node_id.0.cmp(&b.node_id.0))
}

fn compare_mesh_nodes_for_list(a: &MeshNode, b: &MeshNode) -> Ordering {
    a.status
        .sort_rank()
        .cmp(&b.status.sort_rank())
        .then_with(|| cmp_f64_desc(a.score, b.score))
        .then_with(|| cmp_optional_f64_asc(a.latency_ms, b.latency_ms))
        .then_with(|| a.node_id.0.cmp(&b.node_id.0))
}

fn compare_country_groups(a: &MeshNodeCountryGroup, b: &MeshNodeCountryGroup) -> Ordering {
    match (
        a.country_code == MeshNodeCountry::UNKNOWN_CODE,
        b.country_code == MeshNodeCountry::UNKNOWN_CODE,
    ) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => a
            .country_name
            .to_ascii_lowercase()
            .cmp(&b.country_name.to_ascii_lowercase())
            .then_with(|| a.country_code.cmp(&b.country_code)),
    }
}

fn node_matches_filter(node: &MeshNode, filter: &MeshNodeListFilter) -> bool {
    if filter.available_only && !node.status.is_available_now() {
        return false;
    }
    if !filter.statuses.is_empty() && !filter.statuses.contains(&node.status) {
        return false;
    }
    if !filter.countries.is_empty() {
        let country_code = node.country.country_code.to_ascii_uppercase();
        let country_name = node.country.country_name.to_ascii_uppercase();
        if !filter.countries.contains(&country_code) && !filter.countries.contains(&country_name) {
            return false;
        }
    }
    if let Some(search) = filter.search.as_ref() {
        let needle = search.to_ascii_lowercase();
        let found = node.node_id.0.to_ascii_lowercase().contains(&needle)
            || node
                .country
                .country_code
                .to_ascii_lowercase()
                .contains(&needle)
            || node
                .country
                .country_name
                .to_ascii_lowercase()
                .contains(&needle);
        if !found {
            return false;
        }
    }
    true
}
