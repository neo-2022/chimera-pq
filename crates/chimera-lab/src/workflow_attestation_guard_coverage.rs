use super::source_lists::{
    INTERDISCIPLINARY_SOURCE_LISTS, SOURCE_BULLET_LINE_COUNT, SOURCE_LINE_COUNT,
    SOURCE_NEWLINE_COUNT, SOURCE_NON_EMPTY_LINE_COUNT, SOURCE_NUMBERED_LINE_COUNT,
    SOURCE_REQUIRED_MARKER_LINE_COUNT, SOURCE_SHA256, SOURCE_STRUCTURAL_HEADING_LINE_COUNT,
    SOURCE_UPPER_HEADING_LINE_COUNT, STAGE_SOURCE_RANGES, StageSourceRange,
};
use super::support::*;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct CoverageCatalog {
    pub(crate) ids: BTreeSet<String>,
    pub(crate) source_lines_by_id: BTreeMap<String, i64>,
}

pub(crate) fn validate_algorithm_coverage(
    coverage: &Map<String, Value>,
) -> Result<CoverageCatalog, String> {
    require_str(
        coverage,
        "coverage_file",
        "docs/AI_ARCHITECT_ALGORITHM_COVERAGE.json",
    )?;
    require_str(coverage, "source_sha256", SOURCE_SHA256)?;
    require_i64(coverage, "source_line_count", SOURCE_LINE_COUNT)?;
    require_i64(coverage, "source_newline_count", SOURCE_NEWLINE_COUNT)?;
    require_i64(
        coverage,
        "required_item_count",
        SOURCE_UPPER_HEADING_LINE_COUNT,
    )?;
    require_str(coverage, "coverage_digest_fnv1a", "abdbad3e02da3a7a")?;
    validate_evidence_array(coverage, "evidence")?;

    let coverage_doc = read_project_obj("docs/AI_ARCHITECT_ALGORITHM_COVERAGE.json")?;
    require_str(&coverage_doc, "kind", "ai_architect_algorithm_coverage")?;
    require_i64(&coverage_doc, "schema_version", 1)?;
    require_str(&coverage_doc, "coverage_digest_fnv1a", "abdbad3e02da3a7a")?;
    let source = require_obj(&coverage_doc, "source")?;
    require_str(source, "sha256", SOURCE_SHA256)?;
    require_i64(source, "line_count", SOURCE_LINE_COUNT)?;
    require_i64(source, "newline_count", SOURCE_NEWLINE_COUNT)?;
    require_i64(
        &coverage_doc,
        "required_item_count",
        SOURCE_UPPER_HEADING_LINE_COUNT,
    )?;

    let policy = require_obj(&coverage_doc, "coverage_policy")?;
    require_bool(policy, "no_interpretive_skips", true)?;
    require_bool(policy, "all_structural_headings_required", true)?;
    require_bool(policy, "all_prompt_stage_step_gate_headings_required", true)?;
    require_bool(policy, "machine_guard_required", true)?;
    validate_interdisciplinary_source_lists_catalog(&coverage_doc)?;

    let items = require_array(&coverage_doc, "required_items")?;
    if items.len() != SOURCE_UPPER_HEADING_LINE_COUNT as usize {
        return Err("workflow attestation guard: coverage item count mismatch".to_string());
    }
    let digest = coverage_items_digest(items)?;
    if digest != 0xabdb_ad3e_02da_3a7a {
        return Err("workflow attestation guard: coverage digest mismatch".to_string());
    }
    validate_required_coverage_endpoints(items)?;
    coverage_catalog(items)
}

pub(crate) fn validate_source_text_coverage(root: &Map<String, Value>) -> Result<(), String> {
    let coverage = require_obj(root, "source_text_coverage")?;
    require_str(coverage, "source_sha256", SOURCE_SHA256)?;
    require_i64(coverage, "line_count", SOURCE_LINE_COUNT)?;
    require_i64(coverage, "newline_count", SOURCE_NEWLINE_COUNT)?;
    require_i64(
        coverage,
        "non_empty_line_count",
        SOURCE_NON_EMPTY_LINE_COUNT,
    )?;
    require_i64(coverage, "bullet_line_count", SOURCE_BULLET_LINE_COUNT)?;
    require_i64(coverage, "numbered_line_count", SOURCE_NUMBERED_LINE_COUNT)?;
    require_i64(
        coverage,
        "uppercase_heading_line_count",
        SOURCE_UPPER_HEADING_LINE_COUNT,
    )?;
    require_i64(
        coverage,
        "structural_heading_line_count",
        SOURCE_STRUCTURAL_HEADING_LINE_COUNT,
    )?;
    require_i64(
        coverage,
        "required_marker_line_count",
        SOURCE_REQUIRED_MARKER_LINE_COUNT,
    )?;
    require_bool(coverage, "all_source_lines_accounted_for", true)?;
    require_bool(coverage, "blank_lines_preserved_in_line_ranges", true)?;
    validate_evidence_array(coverage, "evidence")?;

    let full_range = require_obj(coverage, "full_line_range")?;
    require_i64(full_range, "source_line_start", 1)?;
    require_i64(full_range, "source_line_end", SOURCE_LINE_COUNT)?;

    let ranges = require_array(coverage, "stage_line_ranges")?;
    if ranges.len() != STAGE_SOURCE_RANGES.len() {
        return Err(
            "workflow attestation guard: source text stage range count mismatch".to_string(),
        );
    }
    let mut expected_start = 1;
    let mut non_empty_total = 0;
    let mut marker_total = 0;
    for (idx, expected) in STAGE_SOURCE_RANGES.iter().enumerate() {
        let range = value_obj(&ranges[idx], "source text stage range")?;
        validate_stage_source_range_object(range, expected)?;
        if expected.start != expected_start {
            return Err(
                "workflow attestation guard: source text ranges are not contiguous".to_string(),
            );
        }
        expected_start = expected.end + 1;
        non_empty_total += expected.non_empty_lines;
        marker_total += expected.marker_lines;
    }
    if expected_start != SOURCE_LINE_COUNT + 1
        || non_empty_total != SOURCE_NON_EMPTY_LINE_COUNT
        || marker_total != SOURCE_REQUIRED_MARKER_LINE_COUNT
    {
        return Err("workflow attestation guard: source text coverage totals mismatch".to_string());
    }
    Ok(())
}

pub(crate) fn validate_interdisciplinary_source_lists_catalog(
    root: &Map<String, Value>,
) -> Result<(), String> {
    let coverage = require_obj(root, "interdisciplinary_source_lists")?;
    require_str(coverage, "source_sha256", SOURCE_SHA256)?;
    require_bool(coverage, "no_interpretive_list_skips", true)?;
    validate_evidence_array(coverage, "evidence")?;

    let lists = require_array(coverage, "lists")?;
    if lists.len() != INTERDISCIPLINARY_SOURCE_LISTS.len() {
        return Err(
            "workflow attestation guard: interdisciplinary source list count mismatch".to_string(),
        );
    }
    for (idx, expected) in INTERDISCIPLINARY_SOURCE_LISTS.iter().enumerate() {
        let list = value_obj(&lists[idx], "interdisciplinary source list")?;
        require_str(list, "id", expected.id)?;
        require_i64(list, "source_line_start", expected.start)?;
        require_i64(list, "source_line_end", expected.end)?;
        require_i64(list, "item_count", expected.items.len() as i64)?;
        validate_evidence_array(list, "evidence")?;

        let items = require_array(list, "items")?;
        if items.len() != expected.items.len() {
            return Err(format!(
                "workflow attestation guard: interdisciplinary source list item count mismatch: {}",
                expected.id
            ));
        }
        for (item_idx, expected_item) in expected.items.iter().enumerate() {
            let actual = items[item_idx].as_str().ok_or_else(|| {
                "workflow attestation guard: interdisciplinary source list item is not string"
                    .to_string()
            })?;
            if actual != *expected_item {
                return Err(format!(
                    "workflow attestation guard: interdisciplinary source list item mismatch: {}",
                    expected.id
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_interdisciplinary_source_lists_checked(
    research: &Map<String, Value>,
) -> Result<(), String> {
    require_i64(
        research,
        "source_list_count",
        INTERDISCIPLINARY_SOURCE_LISTS.len() as i64,
    )?;
    let checked = require_array(research, "source_lists_checked")?;
    if checked.len() != INTERDISCIPLINARY_SOURCE_LISTS.len() {
        return Err("workflow attestation guard: source_lists_checked count mismatch".to_string());
    }
    for (idx, expected) in INTERDISCIPLINARY_SOURCE_LISTS.iter().enumerate() {
        let item = value_obj(&checked[idx], "source_lists_checked item")?;
        require_str(item, "id", expected.id)?;
        require_i64(item, "source_line_start", expected.start)?;
        require_i64(item, "source_line_end", expected.end)?;
        require_i64(item, "item_count", expected.items.len() as i64)?;
        validate_evidence_array(item, "evidence")?;
    }
    Ok(())
}

pub(crate) fn validate_stage_source_line_coverage(
    report: &Map<String, Value>,
    stage: &str,
) -> Result<(), String> {
    let expected = stage_range(stage)?;
    let coverage = require_obj(report, "source_line_coverage")?;
    validate_stage_source_range_object(coverage, expected)
}

pub(crate) fn collect_and_validate_stage_coverage(
    stage: &str,
    report: &Map<String, Value>,
    catalog: &CoverageCatalog,
    covered: &mut BTreeSet<String>,
) -> Result<(), String> {
    let expected_range = stage_range(stage)?;
    let mut stage_seen = BTreeSet::new();
    for item in require_non_empty_array(report, "covered_required_item_ids")? {
        let id = item.as_str().ok_or_else(|| {
            "workflow attestation guard: covered_required_item_ids item is not string".to_string()
        })?;
        if id.trim().is_empty() {
            return Err(
                "workflow attestation guard: empty covered_required_item_ids item".to_string(),
            );
        }
        if !stage_seen.insert(id.to_string()) {
            return Err(format!(
                "workflow attestation guard: duplicate covered_required_item_id in {stage}: {id}"
            ));
        }
        let Some(line) = catalog.source_lines_by_id.get(id) else {
            return Err(format!(
                "workflow attestation guard: unknown covered_required_item_id in {stage}: {id}"
            ));
        };
        if *line < expected_range.start || *line > expected_range.end {
            return Err(format!(
                "workflow attestation guard: coverage id assigned to wrong stage: {id}"
            ));
        }
        covered.insert(id.to_string());
    }
    Ok(())
}

fn validate_stage_source_range_object(
    obj: &Map<String, Value>,
    expected: &StageSourceRange,
) -> Result<(), String> {
    require_str(obj, "stage_id", expected.stage)?;
    require_i64(obj, "source_line_start", expected.start)?;
    require_i64(obj, "source_line_end", expected.end)?;
    require_i64(obj, "non_empty_line_count", expected.non_empty_lines)?;
    require_i64(obj, "required_marker_line_count", expected.marker_lines)?;
    validate_evidence_array(obj, "evidence")
}

fn stage_range(stage: &str) -> Result<&'static StageSourceRange, String> {
    STAGE_SOURCE_RANGES
        .iter()
        .find(|range| range.stage == stage)
        .ok_or_else(|| {
            format!("workflow attestation guard: unknown source coverage stage: {stage}")
        })
}

fn validate_required_coverage_endpoints(items: &[Value]) -> Result<(), String> {
    let first = value_obj(
        items
            .first()
            .ok_or_else(|| "workflow attestation guard: empty coverage items".to_string())?,
        "coverage item",
    )?;
    require_str(first, "id", "ai_architect_algorithm")?;
    require_i64(first, "source_line", 1)?;
    require_str(first, "title", "AI ARCHITECT ALGORITHM")?;

    let last = value_obj(
        items
            .last()
            .ok_or_else(|| "workflow attestation guard: empty coverage items".to_string())?,
        "coverage item",
    )?;
    require_str(last, "id", "end_of_ai_architect_algorithm_lifecycle")?;
    require_i64(last, "source_line", SOURCE_LINE_COUNT)?;
    require_str(last, "title", "END OF AI ARCHITECT ALGORITHM LIFECYCLE")?;

    let mut previous_line = 0i64;
    let mut seen = BTreeSet::new();
    for item in items {
        let obj = value_obj(item, "coverage item")?;
        let id = require_non_empty_str(obj, "id")?;
        let line = require_i64_value(obj, "source_line")?;
        require_non_empty_str(obj, "title")?;
        require_str(obj, "status", "covered_by_contract")?;
        validate_evidence_array(obj, "covered_by")?;
        if !seen.insert(id.to_string()) {
            return Err(format!(
                "workflow attestation guard: duplicate coverage id: {id}"
            ));
        }
        if line <= previous_line {
            return Err("workflow attestation guard: coverage lines out of order".to_string());
        }
        previous_line = line;
    }
    Ok(())
}

fn coverage_catalog(items: &[Value]) -> Result<CoverageCatalog, String> {
    let mut ids = BTreeSet::new();
    let mut source_lines_by_id = BTreeMap::new();
    for item in items {
        let obj = value_obj(item, "coverage item")?;
        let id = require_non_empty_str(obj, "id")?.to_string();
        let source_line = require_i64_value(obj, "source_line")?;
        ids.insert(id.clone());
        source_lines_by_id.insert(id, source_line);
    }
    Ok(CoverageCatalog {
        ids,
        source_lines_by_id,
    })
}

fn coverage_items_digest(items: &[Value]) -> Result<u64, String> {
    let mut canonical = String::new();
    for item in items {
        let obj = value_obj(item, "coverage item")?;
        let id = require_non_empty_str(obj, "id")?;
        let line = require_i64_value(obj, "source_line")?;
        let title = require_non_empty_str(obj, "title")?;
        canonical.push_str(id);
        canonical.push('|');
        canonical.push_str(&line.to_string());
        canonical.push('|');
        canonical.push_str(title);
        canonical.push('\n');
    }
    Ok(fnv1a64(&canonical))
}

fn fnv1a64(text: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
