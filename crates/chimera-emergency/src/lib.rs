#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmergencyCarrierKind {
    Ble,
    Lora,
    Audio,
    Qr,
    Nfc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmergencyOffer {
    pub kind: EmergencyCarrierKind,
    pub token: String,
    pub ttl_seconds: u32,
}

impl EmergencyOffer {
    pub fn validate(&self) -> Result<(), String> {
        if self.token.len() < 16 {
            return Err("emergency token is too short".to_string());
        }
        if self.ttl_seconds == 0 {
            return Err("emergency ttl must be greater than zero".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{EmergencyCarrierKind, EmergencyOffer};

    #[test]
    fn valid_offer_passes_validation() {
        let offer = EmergencyOffer {
            kind: EmergencyCarrierKind::Qr,
            token: "abcdef1234567890token".to_string(),
            ttl_seconds: 300,
        };
        assert!(offer.validate().is_ok());
    }

    #[test]
    fn short_token_is_rejected() {
        let offer = EmergencyOffer {
            kind: EmergencyCarrierKind::Ble,
            token: "short".to_string(),
            ttl_seconds: 120,
        };
        assert!(offer.validate().is_err());
    }

    #[test]
    fn zero_ttl_is_rejected() {
        let offer = EmergencyOffer {
            kind: EmergencyCarrierKind::Audio,
            token: "abcdef1234567890token".to_string(),
            ttl_seconds: 0,
        };
        assert!(offer.validate().is_err());
    }
}
