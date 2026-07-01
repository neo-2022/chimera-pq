use serde_json::{Map, Value};
use std::net::Ipv6Addr;
use std::str::FromStr;

pub(crate) fn reject_sensitive_text(root: &Map<String, Value>) -> Result<(), String> {
    let raw = serde_json::to_string(root)
        .map_err(|_| "workflow attestation guard: cannot serialize json".to_string())?;
    reject_sensitive_raw_text("workflow attestation guard", &raw)
}

pub(crate) fn reject_sensitive_raw_text(label: &str, raw: &str) -> Result<(), String> {
    let lower = raw.to_ascii_lowercase();
    for banned in [
        "ssh root",
        "wg-client1.conf",
        "privatekey",
        "private_key",
        "presharedkey",
        "preshared_key",
        "password",
        "token=",
        "\"token\"",
        "access_token",
        "refresh_token",
        "client_secret",
        "secret=",
        "\"secret\"",
        "api_key",
        "api-key",
        "apikey",
        "credential=",
        "auth=",
        "session=",
        "key=",
        "bearer ",
        "authorization:",
        "cookie:",
        "set-cookie",
        "begin openssh private key",
        "begin private key",
        "begin rsa private key",
        "begin ec private key",
        "begin dsa private key",
        "raw_payload",
        "raw payload",
        "payload_contents",
        "packet_payload",
        "request body",
        "response body",
        "http body",
        "packet dump",
        "hexdump",
        "base64 payload",
        "ghp_",
        "github_pat_",
        "sk-",
        "xoxb-",
        "xoxp-",
        "akia",
        "://",
    ] {
        if lower.contains(banned) {
            return Err(format!(
                "{label}: sensitive or stand-specific text found: {banned}"
            ));
        }
    }
    if contains_ipv4_literal(raw) {
        return Err(format!("{label}: IP address literal found"));
    }
    if contains_public_ipv6_literal(raw) {
        return Err(format!("{label}: public IPv6 address literal found"));
    }
    if contains_local_address_marker(&lower) {
        return Err(format!("{label}: local address marker found"));
    }
    if contains_local_path_marker(&lower) {
        return Err(format!("{label}: local path marker found"));
    }
    if contains_principal_at_host(raw) {
        return Err(format!("{label}: principal@host marker found"));
    }
    if contains_host_port_endpoint(raw) {
        return Err(format!("{label}: host:port endpoint marker found"));
    }
    if contains_hostname_literal(raw) {
        return Err(format!("{label}: hostname/FQDN marker found"));
    }
    if contains_jwt_like_token(raw) {
        return Err(format!("{label}: JWT-like token found"));
    }
    if contains_unlabeled_high_entropy_token(raw) {
        return Err(format!("{label}: unlabeled high-entropy token found"));
    }
    Ok(())
}

pub(crate) fn contains_ipv4_literal(text: &str) -> bool {
    for token in text.split(|c: char| !(c.is_ascii_digit() || c == '.')) {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 4 {
            continue;
        }
        if parse_ipv4_octets(&parts).is_err() {
            continue;
        }
        return true;
    }
    false
}

pub(crate) fn contains_public_ipv6_literal(text: &str) -> bool {
    for token in text.split(|c: char| !(c.is_ascii_hexdigit() || c == ':')) {
        if !token.contains(':') || token.len() < 3 {
            continue;
        }
        let Ok(addr) = Ipv6Addr::from_str(token) else {
            continue;
        };
        let octets = addr.octets();
        let first = octets[0];
        let second = octets[1];
        let unique_local = (first & 0xfe) == 0xfc;
        let link_local = first == 0xfe && (second & 0xc0) == 0x80;
        if !(addr.is_loopback() || addr.is_unspecified() || unique_local || link_local) {
            return true;
        }
    }
    false
}

pub(crate) fn contains_principal_at_host(text: &str) -> bool {
    for token in text.split(token_boundary) {
        let trimmed = token.trim_matches(|c: char| {
            matches!(
                c,
                '"' | '\'' | ',' | ';' | ')' | '(' | '[' | ']' | '{' | '}'
            )
        });
        let Some((left, right)) = trimmed.split_once('@') else {
            continue;
        };
        if left.is_empty() || right.is_empty() || right.contains('@') {
            continue;
        }
        if is_principal_like(left) && is_hostname_like(right) {
            return true;
        }
    }
    false
}

pub(crate) fn contains_host_port_endpoint(text: &str) -> bool {
    for token in text.split(token_boundary) {
        let trimmed = token.trim_matches(|c: char| {
            matches!(
                c,
                '"' | '\'' | ',' | ';' | ')' | '(' | '[' | ']' | '{' | '}'
            )
        });
        let Some((host, port)) = trimmed.rsplit_once(':') else {
            continue;
        };
        if host.is_empty() || port.is_empty() || host.contains('/') {
            continue;
        }
        if port.parse::<u16>().is_err() {
            continue;
        }
        if is_hostname_like(host) {
            return true;
        }
    }
    false
}

pub(crate) fn contains_hostname_literal(text: &str) -> bool {
    for token in text.split(token_boundary) {
        let trimmed = token.trim_matches(|c: char| {
            matches!(
                c,
                '"' | '\'' | ',' | ';' | ')' | '(' | '[' | ']' | '{' | '}' | '`'
            )
        });
        if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains('@') {
            continue;
        }
        let labels: Vec<_> = trimmed.split('.').collect();
        if labels.len() < 2 {
            continue;
        }
        if labels.iter().any(|label| {
            label.is_empty()
                || label.starts_with('-')
                || label.ends_with('-')
                || !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        }) {
            continue;
        }
        let Some(tld) = labels.last() else {
            continue;
        };
        if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.ends_with(".md")
            || lower.ends_with(".rs")
            || lower.ends_with(".json")
            || lower.ends_with(".sh")
            || lower.ends_with(".toml")
            || lower.ends_with(".lock")
        {
            continue;
        }
        return true;
    }
    false
}

pub(crate) fn contains_jwt_like_token(text: &str) -> bool {
    for token in text.split(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | ',' | ';')) {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            continue;
        }
        if parts.iter().all(|part| {
            part.len() >= 10
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        }) {
            return true;
        }
    }
    false
}

pub(crate) fn contains_unlabeled_high_entropy_token(text: &str) -> bool {
    const ALLOWED_HEX: [&str; 1] =
        ["c03a66bf5359c83696233b3abc20681eada471d30ba3f9191a2e20abd36f901a"];
    for token in text.split(|c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '"' | '\'' | ',' | ';' | ':' | '(' | ')' | '[' | ']' | '{' | '}'
            )
    }) {
        let trimmed = token.trim();
        if trimmed.len() < 40 || trimmed.contains('/') || trimmed.contains('\\') {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if ALLOWED_HEX.contains(&lower.as_str()) {
            continue;
        }
        let hex_like = trimmed.chars().all(|c| c.is_ascii_hexdigit());
        if hex_like {
            return true;
        }
        let base64_like = trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '_' | '-' | '='));
        let has_lower = trimmed.chars().any(|c| c.is_ascii_lowercase());
        let has_upper = trimmed.chars().any(|c| c.is_ascii_uppercase());
        let has_digit = trimmed.chars().any(|c| c.is_ascii_digit());
        if base64_like && has_lower && has_upper && has_digit {
            return true;
        }
    }
    false
}

fn is_principal_like(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 64
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        && trimmed.chars().any(|c| c.is_ascii_alphabetic())
}

fn is_hostname_like(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > 253
        || trimmed.starts_with('-')
        || trimmed.ends_with('-')
        || trimmed.contains('_')
        || trimmed.contains('/')
        || trimmed.contains('\\')
    {
        return false;
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
    {
        return false;
    }
    if !trimmed.chars().any(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    trimmed.split('.').all(|label| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    })
}

fn token_boundary(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '"' | '\'' | ',' | ';' | '=' | '(' | ')' | '[' | ']' | '{' | '}' | '`'
        )
}

fn contains_local_address_marker(lower: &str) -> bool {
    lower.contains("localhost")
        || lower.contains("::1")
        || lower.contains("fe80:")
        || lower.contains(".local")
        || contains_unique_local_ipv6(lower)
}

fn contains_unique_local_ipv6(lower: &str) -> bool {
    for token in lower.split(|c: char| !(c.is_ascii_hexdigit() || c == ':')) {
        let bytes = token.as_bytes();
        if bytes.len() >= 5
            && (bytes[0] == b'f')
            && (bytes[1] == b'c' || bytes[1] == b'd')
            && bytes[2].is_ascii_hexdigit()
            && bytes[3].is_ascii_hexdigit()
            && bytes[4] == b':'
        {
            return true;
        }
    }
    false
}

fn contains_local_path_marker(lower: &str) -> bool {
    lower.contains("/home/")
        || lower.contains("/users/")
        || lower.contains("c:/users/")
        || lower.contains("c:\\users\\")
        || lower.contains("/tmp/chimera")
        || lower.contains("/var/tmp/chimera")
}

fn parse_ipv4_octets(parts: &[&str]) -> Result<[u8; 4], ()> {
    let mut octets = [0u8; 4];
    for (idx, part) in parts.iter().enumerate() {
        if part.is_empty() || part.len() > 3 {
            return Err(());
        }
        octets[idx] = part.parse::<u8>().map_err(|_| ())?;
    }
    Ok(octets)
}
