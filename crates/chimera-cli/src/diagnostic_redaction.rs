#![forbid(unsafe_code)]

const REDACTED: &str = "<redacted>";

pub(crate) fn redacted_endpoint() -> &'static str {
    REDACTED
}

pub(crate) fn redacted_server_name() -> &'static str {
    REDACTED
}

pub(crate) fn endpoint_state(endpoint: &str) -> &'static str {
    let value = endpoint.trim();
    if value.is_empty() {
        return "unconfigured";
    }
    if is_documentation_endpoint(value) || is_local_or_wildcard_endpoint(value) {
        return "placeholder";
    }
    "configured"
}

pub(crate) fn server_name_state(server_name: &str) -> &'static str {
    let value = server_name.trim();
    if value.is_empty() {
        return "unconfigured";
    }
    if value == "localhost"
        || value.ends_with(".local")
        || value.ends_with(".example")
        || value.ends_with(".example.org")
        || value.ends_with(".example.net")
        || value.ends_with(".example.com")
    {
        return "placeholder";
    }
    "configured"
}

fn is_documentation_endpoint(endpoint: &str) -> bool {
    endpoint.starts_with("192.0.2.")
        || endpoint.starts_with("198.51.100.")
        || endpoint.starts_with("203.0.113.")
}

fn is_local_or_wildcard_endpoint(endpoint: &str) -> bool {
    endpoint.starts_with("127.")
        || endpoint.starts_with("0.0.0.0:")
        || endpoint.starts_with("localhost:")
        || endpoint.starts_with("[::]:")
        || endpoint.starts_with("[::1]:")
}

#[cfg(test)]
mod tests {
    use super::{endpoint_state, redacted_endpoint, redacted_server_name, server_name_state};

    #[test]
    fn endpoint_state_marks_documentation_ranges_as_placeholder() {
        assert_eq!(endpoint_state("203.0.113.10:443"), "placeholder");
        assert_eq!(endpoint_state("198.51.100.10:443"), "placeholder");
        assert_eq!(endpoint_state("192.0.2.10:443"), "placeholder");
        assert_eq!(endpoint_state("127.0.0.1:443"), "placeholder");
        assert_eq!(endpoint_state("127.0.0.1:8443"), "placeholder");
        assert_eq!(endpoint_state("localhost:9443"), "placeholder");
        assert_eq!(endpoint_state("0.0.0.0:9443"), "placeholder");
        assert_eq!(endpoint_state("[::1]:9443"), "placeholder");
    }

    #[test]
    fn endpoint_state_keeps_only_aggregate_config_status() {
        assert_eq!(endpoint_state(""), "unconfigured");
        assert_eq!(endpoint_state("node.mesh.invalid:443"), "configured");
        assert_eq!(redacted_endpoint(), "<redacted>");
    }

    #[test]
    fn server_name_state_keeps_only_aggregate_config_status() {
        assert_eq!(server_name_state(""), "unconfigured");
        assert_eq!(server_name_state("gateway.local"), "placeholder");
        assert_eq!(server_name_state("node.example.org"), "placeholder");
        assert_eq!(server_name_state("node.mesh.invalid"), "configured");
        assert_eq!(redacted_server_name(), "<redacted>");
    }
}
