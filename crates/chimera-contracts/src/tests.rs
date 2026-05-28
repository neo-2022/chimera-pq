use super::*;

#[test]
fn sample_bundle_validates_and_roundtrips() {
    let bundle = ManifestCatalogBundle::sample();
    assert!(bundle.validate().is_ok(), "sample bundle must validate");

    let json_result = serde_json::to_string_pretty(&bundle);
    assert!(json_result.is_ok(), "sample bundle must serialize");
    let json = json_result.unwrap_or_default();

    let restored_result: Result<ManifestCatalogBundle, _> = serde_json::from_str(&json);
    assert!(restored_result.is_ok(), "sample bundle must deserialize");
    let restored = restored_result.unwrap_or_else(|_| ManifestCatalogBundle::sample());

    assert!(restored.validate().is_ok(), "restored bundle must validate");
}

#[test]
fn envelope_rejects_bad_times() {
    let mut envelope = PublishEnvelope::sample();
    envelope.expires_at_unix = Some(envelope.issued_at_unix);
    let err_result = envelope.validate();
    assert!(err_result.is_err(), "bad envelope must fail");
    let err = err_result.err().unwrap_or_default();
    assert!(err.contains("expires_at_unix"));
}

#[test]
fn persistence_document_validates() {
    let doc = CatalogPersistenceDocument::sample();
    assert!(doc.validate().is_ok(), "persistence document must validate");
}
