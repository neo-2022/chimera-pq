#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::net::IpAddr;

use chimera_core::{ChimeraError, ChimeraResult};

pub trait Carrier {
    fn name(&self) -> &'static str;
    fn send(&mut self, frame: Vec<u8>) -> ChimeraResult<()>;
    fn recv(&mut self) -> ChimeraResult<Option<Vec<u8>>>;
}

#[derive(Debug, Default)]
pub struct InMemoryCarrier {
    queue: VecDeque<Vec<u8>>,
    max_frame_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarrierEndpoint {
    pub transport: String,
    pub host: String,
    pub port: u16,
}

impl CarrierEndpoint {
    pub fn parse(input: &str) -> ChimeraResult<Self> {
        let raw = input.trim();
        if raw.is_empty() {
            return Err(ChimeraError::InvalidConfig(
                "carrier endpoint is empty".to_string(),
            ));
        }
        let (transport, host_port) = if let Some((left, right)) = raw.split_once('@') {
            (left.trim().to_ascii_lowercase(), right.trim())
        } else {
            ("tcp".to_string(), raw)
        };
        let (host, port_raw) = host_port.rsplit_once(':').ok_or_else(|| {
            ChimeraError::InvalidConfig("carrier endpoint must be host:port".to_string())
        })?;
        let port = port_raw.trim().parse::<u16>().map_err(|_| {
            ChimeraError::InvalidConfig("carrier endpoint has invalid port".to_string())
        })?;
        if host.trim().is_empty() {
            return Err(ChimeraError::InvalidConfig(
                "carrier endpoint has empty host".to_string(),
            ));
        }
        Ok(Self {
            transport,
            host: host.trim().to_string(),
            port,
        })
    }

    pub fn is_self_loop_candidate(&self, local_ips: &[IpAddr]) -> bool {
        if self.host.eq_ignore_ascii_case("localhost") {
            return true;
        }
        if let Ok(ip) = self.host.parse::<IpAddr>() {
            if ip.is_loopback() {
                return true;
            }
            return local_ips.contains(&ip);
        }
        false
    }
}

impl InMemoryCarrier {
    pub fn new(max_frame_len: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            max_frame_len,
        }
    }
}

impl Carrier for InMemoryCarrier {
    fn name(&self) -> &'static str {
        "in-memory"
    }

    fn send(&mut self, frame: Vec<u8>) -> ChimeraResult<()> {
        if frame.len() > self.max_frame_len {
            return Err(ChimeraError::InvalidFrame(
                "carrier frame too large".to_string(),
            ));
        }

        self.queue.push_back(frame);
        Ok(())
    }

    fn recv(&mut self) -> ChimeraResult<Option<Vec<u8>>> {
        Ok(self.queue.pop_front())
    }
}

#[cfg(test)]
mod tests {
    use super::{Carrier, CarrierEndpoint, InMemoryCarrier};
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn in_memory_carrier_round_trips_frame() {
        let mut carrier = InMemoryCarrier::new(1024);
        assert!(carrier.send(vec![1, 2, 3]).is_ok());

        let received = match carrier.recv() {
            Ok(Some(frame)) => frame,
            Ok(None) => unreachable!("frame should be available"),
            Err(error) => unreachable!("carrier should receive: {error}"),
        };

        assert_eq!(received, vec![1, 2, 3]);
    }

    #[test]
    fn in_memory_carrier_rejects_oversized_frame() {
        let mut carrier = InMemoryCarrier::new(2);
        assert!(carrier.send(vec![1, 2, 3]).is_err());
    }

    #[test]
    fn carrier_endpoint_parses_transport_host_port() {
        let ep = CarrierEndpoint::parse("ssh-443@203.0.113.10:443")
            .unwrap_or_else(|error| unreachable!("must parse: {error}"));
        assert_eq!(ep.transport, "ssh-443");
        assert_eq!(ep.host, "203.0.113.10");
        assert_eq!(ep.port, 443);
    }

    #[test]
    fn carrier_endpoint_detects_self_loop_candidates() {
        let local = vec![IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10))];
        let ep1 = CarrierEndpoint::parse("203.0.113.10:22")
            .unwrap_or_else(|error| unreachable!("must parse: {error}"));
        assert!(ep1.is_self_loop_candidate(&local));
        let ep2 = CarrierEndpoint::parse("localhost:22")
            .unwrap_or_else(|error| unreachable!("must parse: {error}"));
        assert!(ep2.is_self_loop_candidate(&local));
        let ep3 = CarrierEndpoint::parse("198.51.100.20:53")
            .unwrap_or_else(|error| unreachable!("must parse: {error}"));
        assert!(!ep3.is_self_loop_candidate(&local));
    }
}
