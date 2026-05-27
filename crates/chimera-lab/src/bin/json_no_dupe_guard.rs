#![forbid(unsafe_code)]

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use std::collections::BTreeSet;
use std::env;
use std::fmt;
use std::fs;

fn main() {
    let files: Vec<String> = env::args().skip(1).collect();
    if files.is_empty() {
        fail("usage: json_no_dupe_guard <json-file> [<json-file> ...]");
    }
    for path in files {
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|_| fail(&format!("json no-dupe guard: missing file: {path}")));
        parse_no_dupe(&raw).unwrap_or_else(|e| fail(&format!("json no-dupe guard: {path}: {e}")));
    }
    println!("json no-dupe guard: PASS");
}

fn parse_no_dupe(raw: &str) -> Result<(), String> {
    let mut de = serde_json::Deserializer::from_str(raw);
    let _ = NoDupeValue::deserialize(&mut de).map_err(|e| e.to_string())?;
    de.end().map_err(|e| e.to_string())
}

struct NoDupeValue;

impl<'de> Deserialize<'de> for NoDupeValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NoDupeVisitor;

        impl<'de> Visitor<'de> for NoDupeVisitor {
            type Value = NoDupeValue;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("valid JSON value without duplicate object keys")
            }

            fn visit_bool<E>(self, _: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NoDupeValue)
            }
            fn visit_i64<E>(self, _: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NoDupeValue)
            }
            fn visit_u64<E>(self, _: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NoDupeValue)
            }
            fn visit_f64<E>(self, _: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NoDupeValue)
            }
            fn visit_str<E>(self, _: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NoDupeValue)
            }
            fn visit_string<E>(self, _: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NoDupeValue)
            }
            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NoDupeValue)
            }
            fn visit_some<D>(self, d: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                NoDupeValue::deserialize(d)
            }
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NoDupeValue)
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                while seq.next_element::<NoDupeValue>()?.is_some() {}
                Ok(NoDupeValue)
            }
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut seen = BTreeSet::new();
                while let Some(key) = map.next_key::<String>()? {
                    if !seen.insert(key.clone()) {
                        return Err(de::Error::custom(format!("duplicate key: {key}")));
                    }
                    let _ = map.next_value::<NoDupeValue>()?;
                }
                Ok(NoDupeValue)
            }
        }

        deserializer.deserialize_any(NoDupeVisitor)
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}
