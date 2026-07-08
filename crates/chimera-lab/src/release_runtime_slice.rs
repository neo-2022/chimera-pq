use std::fs;

const PROOF_FILE_PREFIX: &str = "WORKFLOW_ATTESTATION_GITHUB_RELEASE_RUNTIME_GATE_";
const PROOF_FILE_SUFFIX: &str = ".md";

const REQUIRED_STATUS_MARKERS: &[&str] = &[
    "pass_after_ssh_proof_scope=github_latest_release_runtime_slice_only",
    "full_mvp_pass=false",
    "prod_ready=false",
    "Status: PASS for the GitHub Latest SSH release/runtime slice.",
];

const REQUIRED_TRUE_MARKERS: &[&str] = &[
    "remote_stand_used=true",
    "ssh_ok=true",
    "github_latest_ok=true",
    "github_one_command_install_ok=true",
    "github_one_command_update_ok=true",
    "install_without_cargo_ok=true",
    "update_without_cargo_ok=true",
    "no_cargo_called=true",
    "version_ok=true",
    "checksum_ok=true",
    "installed_checksum_matches_github_asset=true",
    "start_ok=true",
    "status_after_start_ok=true",
    "restart_ok=true",
    "status_after_restart_ok=true",
    "stop_ok=true",
    "status_after_stop_ok=true",
    "rebind_ok=true",
    "reconnect_ok=true",
    "old_endpoint_closed_ok=true",
    "rollback_ok=true",
    "doctor_fail_closed_ok=true",
    "diagnostics_redacted_ok=true",
    "logs_secret_marker_absent=true",
    "logs_ipv4_absent=true",
    "logs_path_absent=true",
    "raw_payload_absent=true",
];

const REQUIRED_CONTRACT_MARKERS: &[&str] = &[
    "doctor_ok=false",
    "doctor_reason=node_endpoint_unconfigured",
];

const REQUIRED_BOUNDARY_MARKERS: &[&str] = &[
    "proof_failures_present=false",
    "- full MVP production PASS;",
    "- full two-host transparent app traffic proof;",
    "- sealed transit datapath real-world proof;",
    "- long-run soak/performance proof.",
];

pub(crate) fn detect_github_release_ssh_runtime_slice_proven() -> bool {
    let paths = discover_github_release_ssh_runtime_slice_proof_paths("docs");
    detect_github_release_ssh_runtime_slice_proven_from_paths(&paths)
}

pub(crate) fn discover_github_release_ssh_runtime_slice_proof_paths(dir: &str) -> Vec<String> {
    let mut paths: Vec<String> = fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .filter_map(|entry| {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if file_name.starts_with(PROOF_FILE_PREFIX) && file_name.ends_with(PROOF_FILE_SUFFIX) {
                Some(entry.path().to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();
    paths.sort();
    paths.reverse();
    paths
}

pub(crate) fn detect_github_release_ssh_runtime_slice_proven_from_paths(paths: &[String]) -> bool {
    let Some(path) = paths.iter().find(|path| fs::metadata(path).is_ok()) else {
        return false;
    };
    fs::read_to_string(path)
        .map(|content| github_release_ssh_runtime_slice_text_is_proven(&content))
        .unwrap_or(false)
}

pub(crate) fn github_release_ssh_runtime_slice_text_is_proven(content: &str) -> bool {
    REQUIRED_STATUS_MARKERS
        .iter()
        .chain(REQUIRED_TRUE_MARKERS)
        .chain(REQUIRED_CONTRACT_MARKERS)
        .chain(REQUIRED_BOUNDARY_MARKERS)
        .all(|marker| content.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::{
        detect_github_release_ssh_runtime_slice_proven_from_paths,
        github_release_ssh_runtime_slice_text_is_proven,
    };
    use std::{
        env, fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn valid_text() -> String {
        let mut text = String::new();
        for marker in super::REQUIRED_STATUS_MARKERS
            .iter()
            .chain(super::REQUIRED_TRUE_MARKERS)
            .chain(super::REQUIRED_CONTRACT_MARKERS)
            .chain(super::REQUIRED_BOUNDARY_MARKERS)
        {
            text.push_str(marker);
            text.push('\n');
        }
        text
    }

    #[test]
    fn accepts_complete_redacted_slice_proof() {
        assert!(github_release_ssh_runtime_slice_text_is_proven(
            &valid_text()
        ));
    }

    #[test]
    fn rejects_slice_without_boundary_against_full_datapath_claim() {
        let text = valid_text().replace("- sealed transit datapath real-world proof;", "");
        assert!(!github_release_ssh_runtime_slice_text_is_proven(&text));
    }

    #[test]
    fn rejects_slice_with_failed_proof_marker() {
        let text = valid_text().replace(
            "proof_failures_present=false",
            "proof_failures_present=true",
        );
        assert!(!github_release_ssh_runtime_slice_text_is_proven(&text));
    }

    #[test]
    fn rejects_slice_with_legacy_doctor_reason() {
        let text = valid_text().replace(
            "doctor_reason=node_endpoint_unconfigured",
            "doctor_reason=client_endpoint_unconfigured",
        );
        assert!(!github_release_ssh_runtime_slice_text_is_proven(&text));
    }

    #[test]
    fn detect_uses_latest_proof_only() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = env::temp_dir().join(format!("chimera_release_runtime_slice_{unique}"));
        fs::create_dir_all(&temp_dir).unwrap();
        let latest_path = temp_dir.join("latest.md");
        let older_path = temp_dir.join("older.md");
        fs::write(
            &latest_path,
            valid_text().replace(
                "doctor_reason=node_endpoint_unconfigured",
                "doctor_reason=client_endpoint_unconfigured",
            ),
        )
        .unwrap();
        fs::write(&older_path, valid_text()).unwrap();
        let paths = vec![
            latest_path.to_string_lossy().to_string(),
            older_path.to_string_lossy().to_string(),
        ];
        assert!(!detect_github_release_ssh_runtime_slice_proven_from_paths(
            &paths
        ));
        let _ = fs::remove_file(latest_path);
        let _ = fs::remove_file(older_path);
        let _ = fs::remove_dir(temp_dir);
    }
}
