mod metadata;
mod serve_args;
mod serve_state;
mod server;

pub(crate) use metadata::parse_peer_metadata_file;
pub(crate) use serve_args::ServeReleaseOptions;
pub(crate) use server::serve_release;

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
