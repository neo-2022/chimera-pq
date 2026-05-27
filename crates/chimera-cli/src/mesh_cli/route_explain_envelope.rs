use super::route_explain_json_insert::insert_json;
use super::route_explain_meta::ROUTE_EXPLAIN_CONTRACT_FAMILY;

pub(crate) fn insert_route_explain_envelope(
    object: &mut serde_json::Map<String, serde_json::Value>,
    status: &str,
    kind: &str,
    contract_version: &str,
    namespace: &str,
    node: &str,
) {
    insert_json(object, "status", status);
    insert_json(object, "kind", kind);
    insert_json(
        object,
        "route_explain_contract_family",
        ROUTE_EXPLAIN_CONTRACT_FAMILY,
    );
    insert_json(object, "explain_contract_version", contract_version);
    insert_json(object, "namespace", namespace);
    insert_json(object, "node", node);
}

#[cfg(test)]
mod tests {
    use super::insert_route_explain_envelope;
    use crate::mesh_cli::route_explain_meta::{
        ROUTE_EXPLAIN_CONTRACT_FAMILY, ROUTE_EXPLAIN_KIND_ERROR, ROUTE_EXPLAIN_KIND_OK,
        ROUTE_EXPLAIN_STATUS_ERROR, ROUTE_EXPLAIN_STATUS_OK,
    };

    #[test]
    fn route_explain_envelope_success_fields_are_stable() {
        let mut object = serde_json::Map::new();
        insert_route_explain_envelope(
            &mut object,
            ROUTE_EXPLAIN_STATUS_OK,
            ROUTE_EXPLAIN_KIND_OK,
            "mesh_explain_v1",
            "cef-public",
            "node-client",
        );
        let value = serde_json::Value::Object(object);
        assert_eq!(value["status"], ROUTE_EXPLAIN_STATUS_OK);
        assert_eq!(value["kind"], ROUTE_EXPLAIN_KIND_OK);
        assert_eq!(
            value["route_explain_contract_family"],
            ROUTE_EXPLAIN_CONTRACT_FAMILY
        );
        assert_eq!(value["explain_contract_version"], "mesh_explain_v1");
        assert_eq!(value["namespace"], "cef-public");
        assert_eq!(value["node"], "node-client");
    }

    #[test]
    fn route_explain_envelope_error_fields_are_stable() {
        let mut object = serde_json::Map::new();
        insert_route_explain_envelope(
            &mut object,
            ROUTE_EXPLAIN_STATUS_ERROR,
            ROUTE_EXPLAIN_KIND_ERROR,
            "mesh_explain_v1",
            "cef-public",
            "node-client",
        );
        let value = serde_json::Value::Object(object);
        assert_eq!(value["status"], ROUTE_EXPLAIN_STATUS_ERROR);
        assert_eq!(value["kind"], ROUTE_EXPLAIN_KIND_ERROR);
        assert_eq!(
            value["route_explain_contract_family"],
            ROUTE_EXPLAIN_CONTRACT_FAMILY
        );
        assert_eq!(value["explain_contract_version"], "mesh_explain_v1");
        assert_eq!(value["namespace"], "cef-public");
        assert_eq!(value["node"], "node-client");
    }
}
