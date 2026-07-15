use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use crate::peer_egress::options_mode::{Mode, mode_name};

pub fn write_resolved_state_file(
    state_file: &str,
    mode: &Mode,
    mesh_node: &str,
    resolved_local_listen: &str,
    resolved_peer_listen: &str,
) -> Result<(), String> {
    let contents = format!(
        "mode={}\nnode_id={}\nresolved_local_listen={}\nresolved_peer_listen={}\n",
        mode_name(mode),
        mesh_node,
        resolved_local_listen,
        resolved_peer_listen
    );
    let path = Path::new(state_file);
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| format!("write state file failed: {error}"))?;
    }
    let tmp_path = path.with_extension("tmp");
    let _ = fs::remove_file(&tmp_path);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&tmp_path)
        .map_err(|error| format!("write state file failed: {error}"))?;
    file.write_all(contents.as_bytes())
        .map_err(|error| format!("write state file failed: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("write state file failed: {error}"))?;
    #[cfg(unix)]
    {
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("write state file failed: {error}"))?;
    }
    fs::rename(&tmp_path, path).map_err(|error| format!("write state file failed: {error}"))?;
    #[cfg(unix)]
    {
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("write state file failed: {error}"))?;
    }
    Ok(())
}
