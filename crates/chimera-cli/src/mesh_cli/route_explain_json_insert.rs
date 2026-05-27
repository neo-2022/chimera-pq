pub(crate) fn insert_json(
    object: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: &str,
) {
    object.insert(
        key.to_string(),
        serde_json::Value::String(value.to_string()),
    );
}
