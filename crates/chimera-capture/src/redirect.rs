use chimera_core::{ChimeraError, ChimeraResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransparentRedirectPlan {
    pub table_name: String,
    pub chain_name: String,
    pub listen_port: u16,
    pub exempt_uid: u32,
    pub bypass_cidrs_v4: Vec<String>,
    pub capture_cidrs_v4: Vec<String>,
    pub capture_tcp_ports: Vec<u16>,
}

impl TransparentRedirectPlan {
    pub fn validate(&self) -> ChimeraResult<()> {
        validate_identifier(&self.table_name, "table_name")?;
        validate_identifier(&self.chain_name, "chain_name")?;
        if self.listen_port == 0 {
            return Err(ChimeraError::InvalidConfig(
                "listen_port must be > 0".to_string(),
            ));
        }
        for cidr in &self.bypass_cidrs_v4 {
            validate_cidr_v4(cidr)?;
        }
        for cidr in &self.capture_cidrs_v4 {
            validate_cidr_v4(cidr)?;
        }
        for port in &self.capture_tcp_ports {
            if *port == 0 {
                return Err(ChimeraError::InvalidConfig(
                    "capture_tcp_ports must not contain 0".to_string(),
                ));
            }
        }
        Ok(())
    }

    pub fn render_apply_nft(&self) -> ChimeraResult<String> {
        self.validate()?;
        let bypass = render_bypass_rules(&self.bypass_cidrs_v4);
        let redirect = render_redirect_rules(
            &self.capture_cidrs_v4,
            &self.capture_tcp_ports,
            self.listen_port,
        );
        Ok(format!(
            "table inet {table} {{\n  chain {chain} {{\n    type nat hook output priority dstnat; policy accept;\n    meta skuid {uid} return\n    oifname \"lo\" return\n{bypass}{redirect}  }}\n}}\n",
            table = self.table_name,
            chain = self.chain_name,
            uid = self.exempt_uid,
            bypass = bypass,
            redirect = redirect,
        ))
    }

    pub fn render_delete_nft(&self) -> ChimeraResult<String> {
        self.validate()?;
        Ok(format!("delete table inet {}\n", self.table_name))
    }
}

pub fn default_bypass_cidrs_v4() -> Vec<String> {
    vec![
        "0.0.0.0/8".to_string(),
        "10.0.0.0/8".to_string(),
        "127.0.0.0/8".to_string(),
        "169.254.0.0/16".to_string(),
        "172.16.0.0/12".to_string(),
        "192.168.0.0/16".to_string(),
        "224.0.0.0/4".to_string(),
        "240.0.0.0/4".to_string(),
    ]
}

fn render_bypass_rules(cidr_list: &[String]) -> String {
    let mut out = String::new();
    for cidr in cidr_list {
        out.push_str("    ip daddr ");
        out.push_str(cidr);
        out.push_str(" return\n");
    }
    out
}

fn render_redirect_rules(cidr_list: &[String], port_list: &[u16], listen_port: u16) -> String {
    if cidr_list.is_empty() && port_list.is_empty() {
        return format!("    meta l4proto tcp redirect to :{listen_port}\n");
    }
    let mut out = String::new();
    if cidr_list.is_empty() {
        for port in port_list {
            out.push_str("    meta l4proto tcp tcp dport ");
            out.push_str(&port.to_string());
            out.push_str(" redirect to :");
            out.push_str(&listen_port.to_string());
            out.push('\n');
        }
        return out;
    }
    if port_list.is_empty() {
        for cidr in cidr_list {
            out.push_str("    meta l4proto tcp ip daddr ");
            out.push_str(cidr);
            out.push_str(" redirect to :");
            out.push_str(&listen_port.to_string());
            out.push('\n');
        }
        return out;
    }
    for cidr in cidr_list {
        for port in port_list {
            out.push_str("    meta l4proto tcp ip daddr ");
            out.push_str(cidr);
            out.push_str(" tcp dport ");
            out.push_str(&port.to_string());
            out.push_str(" redirect to :");
            out.push_str(&listen_port.to_string());
            out.push('\n');
        }
    }
    out
}

fn validate_identifier(value: &str, name: &str) -> ChimeraResult<()> {
    if value.is_empty() || value.len() > 63 {
        return Err(ChimeraError::InvalidConfig(format!(
            "{name} must be 1..=63 characters"
        )));
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(ChimeraError::InvalidConfig(format!("{name} is empty")));
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(ChimeraError::InvalidConfig(format!(
            "{name} must start with ascii letter or underscore"
        )));
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(ChimeraError::InvalidConfig(format!(
            "{name} contains invalid characters"
        )));
    }
    Ok(())
}

fn validate_cidr_v4(value: &str) -> ChimeraResult<()> {
    let Some((ip, prefix)) = value.split_once('/') else {
        return Err(ChimeraError::InvalidConfig(format!(
            "invalid IPv4 CIDR '{value}'"
        )));
    };
    let octets: Vec<&str> = ip.split('.').collect();
    if octets.len() != 4 {
        return Err(ChimeraError::InvalidConfig(format!(
            "invalid IPv4 CIDR '{value}'"
        )));
    }
    for octet in octets {
        let parsed = octet
            .parse::<u8>()
            .map_err(|_| ChimeraError::InvalidConfig(format!("invalid IPv4 CIDR '{value}'")))?;
        if parsed.to_string() != octet && !(parsed == 0 && octet == "00") {
            return Err(ChimeraError::InvalidConfig(format!(
                "invalid IPv4 CIDR '{value}'"
            )));
        }
    }
    let prefix = prefix
        .parse::<u8>()
        .map_err(|_| ChimeraError::InvalidConfig(format!("invalid IPv4 CIDR '{value}'")))?;
    if prefix > 32 {
        return Err(ChimeraError::InvalidConfig(format!(
            "invalid IPv4 CIDR '{value}'"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{TransparentRedirectPlan, default_bypass_cidrs_v4};

    #[test]
    fn render_apply_contains_loop_exempt_and_redirect() {
        let plan = TransparentRedirectPlan {
            table_name: "chimera_redirect".to_string(),
            chain_name: "output".to_string(),
            listen_port: 18124,
            exempt_uid: 4242,
            bypass_cidrs_v4: default_bypass_cidrs_v4(),
            capture_cidrs_v4: Vec::new(),
            capture_tcp_ports: Vec::new(),
        };
        let nft = plan.render_apply_nft().unwrap_or_else(|error| {
            unreachable!("plan should render: {error}");
        });
        assert!(nft.contains("meta skuid 4242 return"));
        assert!(nft.contains("oifname \"lo\" return"));
        assert!(nft.contains("meta l4proto tcp redirect to :18124"));
    }

    #[test]
    fn render_delete_targets_only_chimera_table() {
        let plan = TransparentRedirectPlan {
            table_name: "chimera_redirect".to_string(),
            chain_name: "output".to_string(),
            listen_port: 18124,
            exempt_uid: 4242,
            bypass_cidrs_v4: Vec::new(),
            capture_cidrs_v4: Vec::new(),
            capture_tcp_ports: Vec::new(),
        };
        let nft = plan.render_delete_nft().unwrap_or_else(|error| {
            unreachable!("plan should render: {error}");
        });
        assert_eq!(nft, "delete table inet chimera_redirect\n");
    }

    #[test]
    fn invalid_identifier_is_rejected() {
        let plan = TransparentRedirectPlan {
            table_name: "bad name".to_string(),
            chain_name: "output".to_string(),
            listen_port: 18124,
            exempt_uid: 4242,
            bypass_cidrs_v4: Vec::new(),
            capture_cidrs_v4: Vec::new(),
            capture_tcp_ports: Vec::new(),
        };
        assert!(plan.render_apply_nft().is_err());
    }

    #[test]
    fn invalid_bypass_cidr_is_rejected() {
        let plan = TransparentRedirectPlan {
            table_name: "chimera_redirect".to_string(),
            chain_name: "output".to_string(),
            listen_port: 18124,
            exempt_uid: 4242,
            bypass_cidrs_v4: vec!["bad".to_string()],
            capture_cidrs_v4: Vec::new(),
            capture_tcp_ports: Vec::new(),
        };
        assert!(plan.render_apply_nft().is_err());
    }

    #[test]
    fn render_apply_can_limit_capture_to_cidr() {
        let plan = TransparentRedirectPlan {
            table_name: "chimera_redirect".to_string(),
            chain_name: "output".to_string(),
            listen_port: 18124,
            exempt_uid: 4242,
            bypass_cidrs_v4: Vec::new(),
            capture_cidrs_v4: vec!["203.0.113.10/32".to_string()],
            capture_tcp_ports: Vec::new(),
        };
        let nft = plan.render_apply_nft().unwrap_or_else(|error| {
            unreachable!("plan should render: {error}");
        });
        assert!(nft.contains("ip daddr 203.0.113.10/32 redirect to :18124"));
        assert!(!nft.contains("meta l4proto tcp redirect to :18124"));
    }

    #[test]
    fn render_apply_can_limit_capture_to_cidr_and_port() {
        let plan = TransparentRedirectPlan {
            table_name: "chimera_redirect".to_string(),
            chain_name: "output".to_string(),
            listen_port: 18124,
            exempt_uid: 4242,
            bypass_cidrs_v4: Vec::new(),
            capture_cidrs_v4: vec!["203.0.113.10/32".to_string()],
            capture_tcp_ports: vec![18143],
        };
        let nft = plan.render_apply_nft().unwrap_or_else(|error| {
            unreachable!("plan should render: {error}");
        });
        assert!(nft.contains("ip daddr 203.0.113.10/32 tcp dport 18143 redirect to :18124"));
    }
}
