use chimera_mesh::{
    MeshNode, MeshNodeListFilter, group_mesh_nodes_by_country, select_best_mesh_node,
};

use super::nodes_inventory::MeshNodesInventory;

pub(crate) fn render_nodes_list(
    inventory: &MeshNodesInventory,
    filter: &MeshNodeListFilter,
) -> String {
    let mut out = String::new();
    let groups = group_mesh_nodes_by_country(&inventory.nodes, filter);
    if groups.is_empty() {
        out.push_str("Нет узлов, подходящих под фильтры\n\n");
        out.push_str("Следующая команда:\n");
        out.push_str("  chimera nodes\n");
        return out;
    }
    let mut total = 0usize;
    let mut first_node_id = String::new();
    for group in groups {
        out.push_str(&format!(
            "\nСтрана: {}\nУзлов: {}\n",
            group.country_name,
            group.nodes.len()
        ));        out.push_str("----------------------------------------\n");
        for node in group.nodes {
            if first_node_id.is_empty() {
                first_node_id = node.node_id.0.clone();
            }
            total += 1;
            out.push_str(&format!("id: {}\n", node.node_id));
            out.push_str(&format!("  статус: {}\n", node.status.as_str()));
            out.push_str(&format!("  endpoint: {}\n", node.endpoint));
            out.push_str(&format!("  задержка: {}\n", fmt_ms(node.latency_ms)));
            out.push_str(&format!("  оценка: {:.1}\n", node.score));
            if let Some(reason) = reason_suffix(&node).strip_prefix(" reason=\"") {
                out.push_str(&format!("  примечание: {}\n", reason.trim_end_matches('"')));
            }
            if inventory.current_node.as_ref() == Some(&node.node_id) {
                out.push_str("  роль: текущий\n");
            }
            if inventory.pinned_node.as_ref() == Some(&node.node_id) {
                out.push_str("  роль: закрепленный\n");
            }
            out.push('\n');
        }
    }
    out.push_str("----------------------------------------\n");
    out.push_str(&format!("Всего узлов: {total}\n"));
    out.push('\n');
    out.push_str("Следующая команда:\n");
    out.push_str("  chimera connect <node_id>\n");
    if !first_node_id.is_empty() {
        out.push_str("Пример:\n");
        out.push_str(&format!("  chimera connect {}\n", first_node_id));
    }
    out
}

pub(crate) fn render_best(nodes: &[MeshNode]) -> String {
    match select_best_mesh_node(nodes) {
        Some(node) => format!(
            "node_id={} страна={} ({}) статус={} оценка={:.2} задержка={} джиттер={} потери={} причина=\"{}\"",
            node.node_id,
            node.country.country_name,
            node.country.country_code,
            node.status.as_str(),
            node.score,
            fmt_ms(node.latency_ms),
            fmt_ms(node.jitter_ms),
            fmt_pct(node.loss_pct),
            node.explain_reason
        ),
        None => "нет доступных узлов для выбора".to_string(),
    }
}

pub(crate) fn fmt_ms(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.0}ms"))
        .unwrap_or_else(|| "-".to_string())
}

pub(crate) fn fmt_pct(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.1}%"))
        .unwrap_or_else(|| "-".to_string())
}

fn reason_suffix(node: &MeshNode) -> String {
    if node.country.is_unknown() {
        return " reason=\"страна не определена\"".to_string();
    }
    if node.country.country_conflict {
        return format!(
            " reason=\"{}\"",
            node.country
                .country_conflict_reason
                .as_deref()
                .unwrap_or("geoip_conflict")
        );
    }
    if node.explain_reason.trim().is_empty() {
        String::new()
    } else {
        format!(" reason=\"{}\"", node.explain_reason)
    }
}
