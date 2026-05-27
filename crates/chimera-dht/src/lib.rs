#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedDiscoveryRecord {
    pub namespace: String,
    pub node_hint: String,
    pub epoch: u64,
    pub signature: String,
}

impl SignedDiscoveryRecord {
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut namespace = None;
        let mut node_hint = None;
        let mut epoch = None;
        let mut signature = None;

        for part in input.split(';') {
            let mut kv = part.splitn(2, '=');
            let key = match kv.next() {
                Some(value) => value.trim(),
                None => continue,
            };
            let value = match kv.next() {
                Some(v) => v.trim(),
                None => return Err("malformed discovery record field".to_string()),
            };

            match key {
                "namespace" => namespace = Some(value.to_string()),
                "node_hint" => node_hint = Some(value.to_string()),
                "epoch" => {
                    let parsed = match value.parse::<u64>() {
                        Ok(v) => v,
                        Err(_) => return Err("invalid epoch in discovery record".to_string()),
                    };
                    epoch = Some(parsed);
                }
                "signature" => signature = Some(value.to_string()),
                _ => {}
            }
        }

        let namespace = match namespace {
            Some(value) if !value.is_empty() => value,
            _ => return Err("discovery record missing namespace".to_string()),
        };
        let node_hint = match node_hint {
            Some(value) if !value.is_empty() => value,
            _ => return Err("discovery record missing node_hint".to_string()),
        };
        let epoch = match epoch {
            Some(value) => value,
            None => return Err("discovery record missing epoch".to_string()),
        };
        let signature = match signature {
            Some(value) if !value.is_empty() => value,
            _ => return Err("discovery record missing signature".to_string()),
        };

        Ok(Self {
            namespace,
            node_hint,
            epoch,
            signature,
        })
    }

    pub fn basic_verify(&self) -> Result<(), String> {
        if self.signature.len() < 8 {
            return Err("discovery record signature is too short".to_string());
        }
        if self.namespace.len() < 3 {
            return Err("discovery record namespace is too short".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::SignedDiscoveryRecord;

    #[test]
    fn parse_and_verify_valid_record() {
        let parsed = SignedDiscoveryRecord::parse(
            "namespace=cef-public;node_hint=198.51.100.7:443;epoch=42;signature=abcdef123456",
        );
        let record = match parsed {
            Ok(value) => value,
            Err(error) => unreachable!("record should parse: {error}"),
        };
        assert_eq!(record.namespace, "cef-public");
        assert!(record.basic_verify().is_ok());
    }

    #[test]
    fn parse_rejects_missing_epoch() {
        let parsed = SignedDiscoveryRecord::parse(
            "namespace=cef-public;node_hint=198.51.100.7:443;signature=abcdef123456",
        );
        assert!(parsed.is_err());
    }

    #[test]
    fn verify_rejects_short_signature() {
        let parsed = SignedDiscoveryRecord::parse(
            "namespace=cef-public;node_hint=198.51.100.7:443;epoch=42;signature=short",
        );
        let record = match parsed {
            Ok(value) => value,
            Err(error) => unreachable!("record should parse: {error}"),
        };
        assert!(record.basic_verify().is_err());
    }
}
