use crate::preemptive::format_tuning_source;

pub(crate) fn tuning_source_label(
    source: crate::preemptive::ShadowPriTuningSource,
) -> &'static str {
    format_tuning_source(source)
}

pub(crate) fn explain_value<'a>(lines: &'a [String], prefix: &str) -> Option<&'a str> {
    lines.iter().find_map(|line| line.strip_prefix(prefix))
}
