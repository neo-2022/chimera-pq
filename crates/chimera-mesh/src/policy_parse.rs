use std::collections::BTreeSet;

pub(crate) fn parse_csv_unique(input: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for raw in input.split(',') {
        let value = raw.trim();
        if value.is_empty() {
            continue;
        }
        if value.contains(' ') {
            return Err("mesh policy csv value contains spaces".to_string());
        }
        if seen.insert(value.to_string()) {
            out.push(value.to_string());
        }
    }
    Ok(out)
}

pub(crate) fn parse_csv_unique_normalized(input: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for raw in input.split(',') {
        let value = raw.trim();
        if value.is_empty() {
            continue;
        }
        if value.contains(' ') {
            return Err("mesh policy csv value contains spaces".to_string());
        }
        let normalized = value.to_ascii_lowercase();
        if seen.insert(normalized.clone()) {
            out.push(normalized);
        }
    }
    Ok(out)
}

pub(crate) fn parse_u8_field(value: &str, key: &str) -> Result<u8, String> {
    value
        .parse::<u8>()
        .map_err(|_| format!("mesh policy '{key}' is not a valid u8"))
}

pub(crate) fn parse_u64_field(value: &str, key: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("mesh policy '{key}' is not a valid u64"))
}

pub(crate) fn parse_i32_field(value: &str, key: &str) -> Result<i32, String> {
    value
        .parse::<i32>()
        .map_err(|_| format!("mesh policy '{key}' is not a valid i32"))
}

pub(crate) fn parse_usize_field(value: &str, key: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("mesh policy '{key}' is not a valid usize"))
}

pub(crate) fn parse_bool_field(value: &str, key: &str) -> Result<bool, String> {
    match value.to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("mesh policy '{key}' is not a valid bool")),
    }
}

pub(crate) fn parse_u16_csv_field(value: &str, key: &str) -> Result<Vec<u16>, String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for raw in value.split(',') {
        let item = raw.trim();
        if item.is_empty() {
            continue;
        }
        let port = item
            .parse::<u16>()
            .map_err(|_| format!("mesh policy '{key}' contains non-u16 port"))?;
        if port == 0 {
            return Err(format!("mesh policy '{key}' contains port 0"));
        }
        if seen.insert(port) {
            out.push(port);
        }
    }
    if out.is_empty() {
        return Err(format!(
            "mesh policy '{key}' must include at least one port"
        ));
    }
    Ok(out)
}
