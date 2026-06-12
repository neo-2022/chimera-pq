use chimera_mesh::{MeshNodeListFilter, MeshNodeStatus};

use crate::mesh_cli::nodes_inventory::extract_flag_value;

pub(super) fn parse_filter(args: &[String]) -> Result<MeshNodeListFilter, String> {
    let mut filter = MeshNodeListFilter::default();
    if let Some(countries) = extract_flag_value(args, "--country") {
        filter.countries = countries
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_uppercase)
            .collect();
    }
    if let Some(statuses) = extract_flag_value(args, "--status") {
        for status in statuses
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            filter.statuses.insert(MeshNodeStatus::parse(status)?);
        }
    }
    filter.available_only = args.iter().any(|arg| arg == "--available-only");
    filter.search = extract_flag_value(args, "--search").map(str::to_string);
    Ok(filter)
}
