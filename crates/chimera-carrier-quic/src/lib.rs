#![forbid(unsafe_code)]

use chimera_carrier::Carrier;
use chimera_core::{ChimeraError, ChimeraResult};
use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};

const DEFAULT_MAX_FRAME_LEN: usize = 64 * 1024;

fn quic_bus() -> &'static Mutex<HashMap<String, VecDeque<Vec<u8>>>> {
    static BUS: OnceLock<Mutex<HashMap<String, VecDeque<Vec<u8>>>>> = OnceLock::new();
    BUS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuicCarrierConfig {
    pub server_name: String,
    pub connect_addr: String,
    pub connect_timeout_ms: u64,
}

impl QuicCarrierConfig {
    pub fn validate(&self) -> ChimeraResult<()> {
        if self.server_name.trim().is_empty() {
            return Err(ChimeraError::InvalidConfig(
                "quic carrier server_name is empty".to_string(),
            ));
        }
        if self.connect_addr.trim().is_empty() {
            return Err(ChimeraError::InvalidConfig(
                "quic carrier connect_addr is empty".to_string(),
            ));
        }
        if self.connect_timeout_ms == 0 {
            return Err(ChimeraError::InvalidConfig(
                "quic carrier connect_timeout_ms must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct QuicCarrier {
    config: QuicCarrierConfig,
    max_frame_len: usize,
}

impl QuicCarrier {
    pub fn new(config: QuicCarrierConfig) -> ChimeraResult<Self> {
        config.validate()?;
        Ok(Self {
            config,
            max_frame_len: DEFAULT_MAX_FRAME_LEN,
        })
    }

    pub fn config(&self) -> &QuicCarrierConfig {
        &self.config
    }

    pub fn with_max_frame_len(mut self, max_frame_len: usize) -> ChimeraResult<Self> {
        if max_frame_len == 0 {
            return Err(ChimeraError::InvalidConfig(
                "quic carrier max_frame_len must be > 0".to_string(),
            ));
        }
        self.max_frame_len = max_frame_len;
        Ok(self)
    }
}

impl Carrier for QuicCarrier {
    fn name(&self) -> &'static str {
        "quic"
    }

    fn send(&mut self, frame: Vec<u8>) -> ChimeraResult<()> {
        if frame.len() > self.max_frame_len {
            return Err(ChimeraError::InvalidFrame(
                "quic carrier frame too large".to_string(),
            ));
        }
        let mut guard = quic_bus().lock().map_err(|_| {
            ChimeraError::InvalidFrame("quic carrier bus lock poisoned".to_string())
        })?;
        guard
            .entry(self.config.connect_addr.clone())
            .or_default()
            .push_back(frame);
        Ok(())
    }

    fn recv(&mut self) -> ChimeraResult<Option<Vec<u8>>> {
        let mut guard = quic_bus().lock().map_err(|_| {
            ChimeraError::InvalidFrame("quic carrier bus lock poisoned".to_string())
        })?;
        Ok(guard
            .entry(self.config.connect_addr.clone())
            .or_default()
            .pop_front())
    }
}

#[cfg(test)]
mod tests {
    use super::{QuicCarrier, QuicCarrierConfig};
    use chimera_carrier::Carrier;

    #[test]
    fn config_validation_rejects_zero_timeout() {
        let config = QuicCarrierConfig {
            server_name: "gateway.example.org".to_string(),
            connect_addr: "127.0.0.1:443".to_string(),
            connect_timeout_ms: 0,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn quic_carrier_has_expected_name() {
        let carrier = match QuicCarrier::new(QuicCarrierConfig {
            server_name: "gateway.example.org".to_string(),
            connect_addr: "127.0.0.1:443".to_string(),
            connect_timeout_ms: 1000,
        }) {
            Ok(carrier) => carrier,
            Err(error) => unreachable!("carrier should be created: {error}"),
        };
        assert_eq!(carrier.name(), "quic");
    }

    #[test]
    fn quic_carrier_round_trips_frame_on_same_addr() {
        let cfg = QuicCarrierConfig {
            server_name: "gateway.example.org".to_string(),
            connect_addr: "127.0.0.1:443".to_string(),
            connect_timeout_ms: 1000,
        };
        let mut sender = match QuicCarrier::new(cfg.clone()) {
            Ok(carrier) => carrier,
            Err(error) => unreachable!("sender should be created: {error}"),
        };
        let mut receiver = match QuicCarrier::new(cfg) {
            Ok(carrier) => carrier,
            Err(error) => unreachable!("receiver should be created: {error}"),
        };

        assert!(sender.send(vec![4, 5, 6]).is_ok());
        let recv = match receiver.recv() {
            Ok(Some(frame)) => frame,
            Ok(None) => unreachable!("frame should be available"),
            Err(error) => unreachable!("receive should succeed: {error}"),
        };
        assert_eq!(recv, vec![4, 5, 6]);
    }

    #[test]
    fn quic_carrier_rejects_oversized_frame() {
        let cfg = QuicCarrierConfig {
            server_name: "gateway.example.org".to_string(),
            connect_addr: "127.0.0.1:9443".to_string(),
            connect_timeout_ms: 1000,
        };
        let mut carrier = match QuicCarrier::new(cfg).and_then(|c| c.with_max_frame_len(2)) {
            Ok(carrier) => carrier,
            Err(error) => unreachable!("carrier should be created: {error}"),
        };
        assert!(carrier.send(vec![1, 2, 3]).is_err());
    }
}
