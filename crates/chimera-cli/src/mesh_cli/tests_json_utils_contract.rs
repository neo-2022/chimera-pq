use super::route_explain_meta::{
    ROUTE_EXPLAIN_CONTRACT_FAMILY, ROUTE_EXPLAIN_CONTRACT_VERSION, ROUTE_EXPLAIN_KIND_ERROR,
    ROUTE_EXPLAIN_KIND_OK, ROUTE_EXPLAIN_NETWORK_STATE_NOT_MODIFIED, ROUTE_EXPLAIN_STATUS_ERROR,
    ROUTE_EXPLAIN_STATUS_OK,
};

pub(crate) fn expected_contract_family() -> &'static str {
    ROUTE_EXPLAIN_CONTRACT_FAMILY
}

pub(crate) fn expected_contract_version() -> &'static str {
    ROUTE_EXPLAIN_CONTRACT_VERSION
}

pub(crate) fn expected_network_state() -> &'static str {
    ROUTE_EXPLAIN_NETWORK_STATE_NOT_MODIFIED
}

pub(crate) fn expected_status_ok() -> &'static str {
    ROUTE_EXPLAIN_STATUS_OK
}

pub(crate) fn expected_status_error() -> &'static str {
    ROUTE_EXPLAIN_STATUS_ERROR
}

pub(crate) fn expected_kind_ok() -> &'static str {
    ROUTE_EXPLAIN_KIND_OK
}

pub(crate) fn expected_kind_error() -> &'static str {
    ROUTE_EXPLAIN_KIND_ERROR
}
