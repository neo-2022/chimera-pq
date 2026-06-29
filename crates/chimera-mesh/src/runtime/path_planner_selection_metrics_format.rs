use std::fmt::Write;

pub(super) fn push_redacted_endpoint_label(out: &mut String, index: usize) {
    out.push_str("endpoint#");
    let _ = write!(out, "{}", index + 1);
    out.push_str(":<redacted>");
}

pub(super) fn push_redacted_peer_label(out: &mut String, index: usize) {
    out.push_str("peer#");
    let _ = write!(out, "{}", index + 1);
}
