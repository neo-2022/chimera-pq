use crate::MeshRuntime;

#[test]
fn runtime_bootstrap_trims_namespace_and_source() {
    let runtime = MeshRuntime::bootstrap("  cef-public  ", "  seed-a  ")
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(runtime.namespace(), "cef-public");
    assert_eq!(runtime.source_count(), 1);
}

#[test]
fn runtime_bootstrap_rejects_invalid_source_name() {
    assert!(MeshRuntime::bootstrap("cef-public", "").is_err());
    assert!(MeshRuntime::bootstrap("cef-public", "seed,a").is_err());
    assert!(MeshRuntime::bootstrap("cef-public", "seed\tbad").is_err());
}
