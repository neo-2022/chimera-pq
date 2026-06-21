use std::io::Write;
use std::path::{Path, PathBuf};

use chimera_mesh::MeshPathPlan;

use super::document::{parse_transit_lane_document, render_transit_lane_document};
use super::registration::{
    parse_transit_lane_registrations, render_transit_lane_registrations_from_mesh_plan,
};
use super::transit_lane_document_from_mesh_plan;

pub fn write_transit_lane_registrations_from_mesh_plan(
    plan: &MeshPathPlan,
    path: &str,
) -> Result<usize, String> {
    let contents = render_transit_lane_registrations_from_mesh_plan(plan)?;
    validate_registrations_before_publish(&contents)?;
    write_sensitive_text_file_atomic_replace(Path::new(path), &contents)?;
    Ok(plan.multipath_schedule.carrier_lane_bindings.len())
}

pub fn write_transit_lane_document_from_mesh_plan(
    plan: &MeshPathPlan,
    path: &str,
) -> Result<usize, String> {
    let document = transit_lane_document_from_mesh_plan(plan)?;
    let contents = render_transit_lane_document(&document)?;
    validate_document_before_publish(&contents)?;
    write_sensitive_text_file_atomic_replace(Path::new(path), &contents)?;
    Ok(plan.multipath_schedule.carrier_lane_bindings.len())
}

fn validate_registrations_before_publish(contents: &str) -> Result<(), String> {
    let registrations = parse_transit_lane_registrations(contents)?;
    if registrations.is_empty() {
        return Err("sealed transit lane bindings file has no registrations".to_string());
    }
    Ok(())
}

fn validate_document_before_publish(contents: &str) -> Result<(), String> {
    let document = parse_transit_lane_document(contents)?;
    if document.registrations().is_empty() {
        return Err("sealed transit lane document has no registrations".to_string());
    }
    Ok(())
}

fn write_sensitive_text_file_atomic_replace(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "sealed transit lane bindings path has no parent directory".to_string())?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "sealed transit lane bindings path is not valid utf-8".to_string())?;
    let mut tmp_path = PathBuf::from(parent);
    tmp_path.push(format!(
        ".{file_name}.chimera-tmp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| format!("sealed transit lane bindings clock failed: {error}"))?
            .as_nanos()
    ));

    #[cfg(unix)]
    let mut file = {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&tmp_path)
            .map_err(|error| format!("write sealed transit lane bindings failed: {error}"))?
    };

    #[cfg(not(unix))]
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&tmp_path)
        .map_err(|error| format!("write sealed transit lane bindings failed: {error}"))?;

    file.write_all(contents.as_bytes())
        .map_err(|error| format!("write sealed transit lane bindings failed: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("write sealed transit lane bindings failed: {error}"))?;
    drop(file);

    std::fs::rename(&tmp_path, path).map_err(|error| {
        let _ = std::fs::remove_file(&tmp_path);
        format!("write sealed transit lane bindings failed: {error}")
    })?;
    Ok(())
}
