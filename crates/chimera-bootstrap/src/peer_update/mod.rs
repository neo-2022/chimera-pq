mod metadata;
mod serve_args;
mod serve_state;
pub mod serve_state_publish;
mod server;

pub use metadata::parse_peer_metadata_file;
pub use serve_args::ServeReleaseOptions;
pub use serve_state_publish::{
    PeerUpdateStateAdvertisement, PeerUpdateStatePublishAction, decide_peer_update_state_publish,
    parse_existing_peer_update_state,
};
pub use server::serve_release;

#[cfg(test)]
mod metadata_tests;

#[cfg(test)]
mod server_auto_port_tests;

#[cfg(test)]
mod server_tests;

pub(super) const RELEASE_ARCHIVE_NAME: &str = "chimera-pq-release.tar.gz";
pub(super) const RELEASE_ARCHIVE_ROUTE: &str = "/chimera-pq-release.tar.gz";
pub(super) const RELEASE_CHECKSUM_NAME: &str = "chimera-pq-release.tar.gz.sha256";
pub(super) const RELEASE_CHECKSUM_ROUTE: &str = "/chimera-pq-release.tar.gz.sha256";
pub(super) const RELEASE_METADATA_ROUTE: &str = "/metadata.json";
pub(super) const DISCOVERY_SNAPSHOT_NAME: &str = "mesh_nodes.discovery.json";
pub(super) const DISCOVERY_SNAPSHOT_ROUTE: &str = "/mesh_nodes.discovery.json";
pub(super) const DISCOVERY_PUBKEY_NAME: &str = "mesh_nodes.discovery.pubkey";
pub(super) const DISCOVERY_PUBKEY_ROUTE: &str = "/mesh_nodes.discovery.pubkey";
