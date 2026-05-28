pub(crate) fn validate_non_empty_label(value: &str, label: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} is empty"));
    }
    if value != trimmed {
        return Err(format!("{label} contains surrounding spaces"));
    }
    if value.contains(',') {
        return Err(format!("{label} contains comma"));
    }
    if value.contains('\n') || value.contains('\r') || value.contains('\t') {
        return Err(format!("{label} contains control whitespace"));
    }
    Ok(())
}

pub(crate) fn validate_endpoint(endpoint: &str, label: &str) -> Result<(), String> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return Err(format!("{label} is empty"));
    }
    if endpoint.starts_with('[') {
        return validate_bracketed_ipv6_endpoint(endpoint, label);
    }
    let (host, port_raw) = endpoint
        .rsplit_once(':')
        .ok_or_else(|| format!("{label} must be in host:port format"))?;
    if host.contains(':') {
        return Err(format!("{label} IPv6 endpoint must use [addr]:port format"));
    }
    validate_host_and_port(host, port_raw, label)
}

fn validate_bracketed_ipv6_endpoint(endpoint: &str, label: &str) -> Result<(), String> {
    let close = endpoint
        .find(']')
        .ok_or_else(|| format!("{label} is missing closing bracket"))?;
    let host = &endpoint[1..close];
    let tail = &endpoint[(close + 1)..];
    let port_raw = tail
        .strip_prefix(':')
        .ok_or_else(|| format!("{label} must be in [addr]:port format"))?;
    validate_host_and_port(host, port_raw, label)
}

fn validate_host_and_port(host: &str, port_raw: &str, label: &str) -> Result<(), String> {
    if host.trim().is_empty() {
        return Err(format!("{label} host is empty"));
    }
    if host != host.trim() {
        return Err(format!("{label} host contains surrounding spaces"));
    }
    if host.chars().any(char::is_whitespace) {
        return Err(format!("{label} host contains whitespace"));
    }
    if port_raw != port_raw.trim() {
        return Err(format!("{label} port contains surrounding spaces"));
    }
    if port_raw.chars().any(char::is_whitespace) {
        return Err(format!("{label} port contains whitespace"));
    }
    let port = port_raw
        .parse::<u16>()
        .map_err(|_| format!("{label} port is invalid"))?;
    if port == 0 {
        return Err(format!("{label} port must be > 0"));
    }
    Ok(())
}
