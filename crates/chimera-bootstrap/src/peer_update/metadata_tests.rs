use super::metadata::parse_peer_metadata_file;
use std::fs;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn temp_dir(prefix: &str) -> TestResult<std::path::PathBuf> {
    let mut base = std::env::temp_dir();
    let unique = format!(
        "{}-{}-{}",
        prefix,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    base.push(unique);
    fs::create_dir_all(&base)?;
    Ok(base)
}

fn write_metadata(root: &std::path::Path, body: &str) -> TestResult<std::path::PathBuf> {
    let metadata_path = root.join("metadata.json");
    fs::write(&metadata_path, body)?;
    Ok(metadata_path)
}

#[test]
fn parse_peer_metadata_accepts_same_origin_urls() -> TestResult {
    let root = temp_dir("chimera-peer-metadata-test")?;
    let metadata_path = write_metadata(
        &root,
        "{\"status\":\"ok\",\"kind\":\"chimera_peer_update_metadata\",\"version\":\"1.2.3\",\"archive\":\"http://node.example:18179/chimera-pq-release.tar.gz\",\"checksum\":\"http://node.example:18179/chimera-pq-release.tar.gz.sha256\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\"}\n",
    )?;

    let metadata =
        parse_peer_metadata_file(&metadata_path, "http://node.example:18179/metadata.json")?;

    assert_eq!(metadata.version, "1.2.3");
    assert_eq!(
        metadata.archive,
        "http://node.example:18179/chimera-pq-release.tar.gz"
    );
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn parse_peer_metadata_rejects_cross_origin_archive() -> TestResult {
    let root = temp_dir("chimera-peer-metadata-test")?;
    let metadata_path = write_metadata(
        &root,
        "{\"status\":\"ok\",\"kind\":\"chimera_peer_update_metadata\",\"version\":\"1.2.3\",\"archive\":\"http://evil.example/chimera-pq-release.tar.gz\",\"checksum\":\"http://node.example:18179/chimera-pq-release.tar.gz.sha256\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\"}\n",
    )?;

    assert!(
        parse_peer_metadata_file(&metadata_path, "http://node.example:18179/metadata.json")
            .is_err()
    );
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn parse_peer_metadata_rejects_userinfo_and_query() -> TestResult {
    let root = temp_dir("chimera-peer-metadata-test")?;
    let metadata_path = write_metadata(
        &root,
        "{\"status\":\"ok\",\"kind\":\"chimera_peer_update_metadata\",\"version\":\"1.2.3\",\"archive\":\"http://user@node.example:18179/chimera-pq-release.tar.gz\",\"checksum\":\"http://node.example:18179/chimera-pq-release.tar.gz.sha256?x=1\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\"}\n",
    )?;

    assert!(
        parse_peer_metadata_file(&metadata_path, "http://node.example:18179/metadata.json")
            .is_err()
    );
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn parse_peer_metadata_rejects_duplicate_keys() -> TestResult {
    let root = temp_dir("chimera-peer-metadata-test")?;
    let metadata_path = write_metadata(
        &root,
        "{\"status\":\"ok\",\"status\":\"ok\",\"kind\":\"chimera_peer_update_metadata\",\"version\":\"1.2.3\",\"archive\":\"http://node.example:18179/chimera-pq-release.tar.gz\",\"checksum\":\"http://node.example:18179/chimera-pq-release.tar.gz.sha256\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\"}\n",
    )?;

    assert!(
        parse_peer_metadata_file(&metadata_path, "http://node.example:18179/metadata.json")
            .is_err()
    );
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn parse_peer_metadata_rejects_unknown_fields() -> TestResult {
    let root = temp_dir("chimera-peer-metadata-test")?;
    let metadata_path = write_metadata(
        &root,
        "{\"status\":\"ok\",\"kind\":\"chimera_peer_update_metadata\",\"version\":\"1.2.3\",\"archive\":\"http://node.example:18179/chimera-pq-release.tar.gz\",\"checksum\":\"http://node.example:18179/chimera-pq-release.tar.gz.sha256\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"extra\":\"unexpected\"}\n",
    )?;

    assert!(
        parse_peer_metadata_file(&metadata_path, "http://node.example:18179/metadata.json")
            .is_err()
    );
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn parse_peer_metadata_rejects_missing_or_non_string_fields() -> TestResult {
    let root = temp_dir("chimera-peer-metadata-test")?;
    let missing_path = write_metadata(
        &root,
        "{\"status\":\"ok\",\"kind\":\"chimera_peer_update_metadata\",\"version\":\"1.2.3\",\"archive\":\"http://node.example:18179/chimera-pq-release.tar.gz\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\"}\n",
    )?;
    assert!(
        parse_peer_metadata_file(&missing_path, "http://node.example:18179/metadata.json").is_err()
    );

    let typed_path = write_metadata(
        &root,
        "{\"status\":\"ok\",\"kind\":\"chimera_peer_update_metadata\",\"version\":123,\"archive\":\"http://node.example:18179/chimera-pq-release.tar.gz\",\"checksum\":\"http://node.example:18179/chimera-pq-release.tar.gz.sha256\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\"}\n",
    )?;
    assert!(
        parse_peer_metadata_file(&typed_path, "http://node.example:18179/metadata.json").is_err()
    );
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn parse_peer_metadata_rejects_bad_status_kind_sha_and_path() -> TestResult {
    let root = temp_dir("chimera-peer-metadata-test")?;
    let cases = [
        "{\"status\":\"hold\",\"kind\":\"chimera_peer_update_metadata\",\"version\":\"1.2.3\",\"archive\":\"http://node.example:18179/chimera-pq-release.tar.gz\",\"checksum\":\"http://node.example:18179/chimera-pq-release.tar.gz.sha256\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\"}\n",
        "{\"status\":\"ok\",\"kind\":\"wrong\",\"version\":\"1.2.3\",\"archive\":\"http://node.example:18179/chimera-pq-release.tar.gz\",\"checksum\":\"http://node.example:18179/chimera-pq-release.tar.gz.sha256\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\"}\n",
        "{\"status\":\"ok\",\"kind\":\"chimera_peer_update_metadata\",\"version\":\"1.2.3\",\"archive\":\"http://node.example:18179/chimera-pq-release.tar.gz\",\"checksum\":\"http://node.example:18179/chimera-pq-release.tar.gz.sha256\",\"sha256\":\"bad\"}\n",
        "{\"status\":\"ok\",\"kind\":\"chimera_peer_update_metadata\",\"version\":\"1.2.3\",\"archive\":\"http://node.example:18179/other.tar.gz\",\"checksum\":\"http://node.example:18179/chimera-pq-release.tar.gz.sha256\",\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\"}\n",
    ];
    for (idx, body) in cases.iter().enumerate() {
        let metadata_path = write_metadata(&root, body)?;
        assert!(
            parse_peer_metadata_file(&metadata_path, "http://node.example:18179/metadata.json")
                .is_err(),
            "accepted bad metadata case {idx}"
        );
    }
    fs::remove_dir_all(root)?;
    Ok(())
}
