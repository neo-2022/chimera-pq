use crate::preemptive::format_tuning_source;

pub(crate) fn tuning_source_label(
    source: crate::preemptive::ShadowPriTuningSource,
) -> &'static str {
    format_tuning_source(source)
}
