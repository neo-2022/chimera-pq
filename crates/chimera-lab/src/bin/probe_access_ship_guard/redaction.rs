pub(crate) fn validate_public_probe_text_fields(fields: &[(&str, &str)]) -> Result<(), String> {
    for (field, value) in fields {
        reject_sensitive_public_text(field, value)?;
    }
    Ok(())
}

fn reject_sensitive_public_text(field: &str, value: &str) -> Result<(), String> {
    let lower = value.to_ascii_lowercase();
    if value.contains("://")
        || value.contains('@')
        || value.contains("/home/")
        || value.contains("/tmp/chimera")
        || lower.contains("bearer ")
        || lower.contains("token=")
        || lower.contains("password")
        || lower.contains("passwd")
        || lower.contains("auth=")
        || lower.contains("payload")
        || lower.contains("body=")
        || lower.contains("hexdump")
        || contains_ip_literal(value)
        || contains_hostname_literal(value)
    {
        return Err(format!(
            "probe access ship guard: public target field leaks raw data: {field}"
        ));
    }
    Ok(())
}

fn contains_ip_literal(value: &str) -> bool {
    value
        .split(|c: char| {
            c.is_whitespace()
                || matches!(
                    c,
                    ',' | '"' | '\'' | '[' | ']' | '(' | ')' | '{' | '}' | '='
                )
        })
        .any(|token| token.parse::<std::net::IpAddr>().is_ok())
}

fn contains_hostname_literal(value: &str) -> bool {
    value
        .split(|c: char| {
            c.is_whitespace() || matches!(c, ',' | '"' | '\'' | '(' | ')' | '{' | '}' | '=')
        })
        .any(|token| {
            let clean = token
                .trim_matches(|c: char| matches!(c, ':' | ';' | '/' | '?' | '#'))
                .trim_end_matches('.');
            clean.contains('.')
                && clean.split('.').filter(|part| !part.is_empty()).count() >= 2
                && clean.chars().any(|c| c.is_ascii_alphabetic())
                && clean
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
        })
}
