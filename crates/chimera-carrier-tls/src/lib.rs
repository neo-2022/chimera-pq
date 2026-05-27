#![forbid(unsafe_code)]

use chimera_carrier::Carrier;
use chimera_core::{ChimeraError, ChimeraResult};
use std::collections::{HashMap, VecDeque};
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

const DEFAULT_MAX_FRAME_LEN: usize = 64 * 1024;

fn tls_bus() -> &'static Mutex<HashMap<String, VecDeque<Vec<u8>>>> {
    static BUS: OnceLock<Mutex<HashMap<String, VecDeque<Vec<u8>>>>> = OnceLock::new();
    BUS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsCarrierConfig {
    pub server_name: String,
    pub connect_addr: String,
    pub connect_timeout_ms: u64,
}

impl TlsCarrierConfig {
    pub fn validate(&self) -> ChimeraResult<()> {
        if self.server_name.trim().is_empty() {
            return Err(ChimeraError::InvalidConfig(
                "tls carrier server_name is empty".to_string(),
            ));
        }
        if self.connect_addr.trim().is_empty() {
            return Err(ChimeraError::InvalidConfig(
                "tls carrier connect_addr is empty".to_string(),
            ));
        }
        if self.connect_timeout_ms == 0 {
            return Err(ChimeraError::InvalidConfig(
                "tls carrier connect_timeout_ms must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct TlsCarrier {
    config: TlsCarrierConfig,
    max_frame_len: usize,
    stream: Option<TcpStream>,
}

impl TlsCarrier {
    pub fn new(config: TlsCarrierConfig) -> ChimeraResult<Self> {
        config.validate()?;
        Ok(Self {
            config,
            max_frame_len: DEFAULT_MAX_FRAME_LEN,
            stream: None,
        })
    }

    pub fn config(&self) -> &TlsCarrierConfig {
        &self.config
    }

    pub fn with_max_frame_len(mut self, max_frame_len: usize) -> ChimeraResult<Self> {
        if max_frame_len == 0 {
            return Err(ChimeraError::InvalidConfig(
                "tls carrier max_frame_len must be > 0".to_string(),
            ));
        }
        self.max_frame_len = max_frame_len;
        Ok(self)
    }

    fn tcp_target(&self) -> Option<&str> {
        self.config.connect_addr.strip_prefix("tcp://")
    }

    fn ensure_stream(&mut self) -> ChimeraResult<&mut TcpStream> {
        if self.stream.is_none() {
            let Some(target) = self.tcp_target() else {
                return Err(ChimeraError::Unsupported(
                    "tls carrier stream requested for non-tcp target".to_string(),
                ));
            };
            let addr = target
                .to_socket_addrs()
                .map_err(|error| {
                    ChimeraError::InvalidConfig(format!("invalid tcp target: {error}"))
                })?
                .next()
                .ok_or_else(|| {
                    ChimeraError::InvalidConfig("tcp target has no addresses".to_string())
                })?;
            let timeout = Duration::from_millis(self.config.connect_timeout_ms);
            let stream = TcpStream::connect_timeout(&addr, timeout).map_err(|error| {
                ChimeraError::InvalidFrame(format!("tcp connect failed: {error}"))
            })?;
            stream.set_read_timeout(Some(timeout)).map_err(|error| {
                ChimeraError::InvalidFrame(format!("set read timeout failed: {error}"))
            })?;
            stream.set_write_timeout(Some(timeout)).map_err(|error| {
                ChimeraError::InvalidFrame(format!("set write timeout failed: {error}"))
            })?;
            self.stream = Some(stream);
        }
        match self.stream.as_mut() {
            Some(stream) => Ok(stream),
            None => Err(ChimeraError::InvalidFrame(
                "tcp stream was not initialized".to_string(),
            )),
        }
    }
}

impl Carrier for TlsCarrier {
    fn name(&self) -> &'static str {
        "tls-tcp"
    }

    fn send(&mut self, frame: Vec<u8>) -> ChimeraResult<()> {
        if frame.len() > self.max_frame_len {
            return Err(ChimeraError::InvalidFrame(
                "tls carrier frame too large".to_string(),
            ));
        }
        if self.tcp_target().is_some() {
            let stream = self.ensure_stream()?;
            let len = u32::try_from(frame.len()).map_err(|_| {
                ChimeraError::InvalidFrame("tls carrier frame length overflow".to_string())
            })?;
            stream
                .write_all(&len.to_be_bytes())
                .and_then(|_| stream.write_all(&frame))
                .map_err(|error| ChimeraError::InvalidFrame(format!("tcp send failed: {error}")))?;
            return Ok(());
        }
        let mut guard = tls_bus()
            .lock()
            .map_err(|_| ChimeraError::InvalidFrame("tls carrier bus lock poisoned".to_string()))?;
        guard
            .entry(self.config.connect_addr.clone())
            .or_default()
            .push_back(frame);
        Ok(())
    }

    fn recv(&mut self) -> ChimeraResult<Option<Vec<u8>>> {
        if self.tcp_target().is_some() {
            let max_frame_len = self.max_frame_len;
            let stream = self.ensure_stream()?;
            let mut len_buf = [0_u8; 4];
            match stream.read_exact(&mut len_buf) {
                Ok(()) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        ErrorKind::WouldBlock
                            | ErrorKind::TimedOut
                            | ErrorKind::UnexpectedEof
                            | ErrorKind::ConnectionReset
                            | ErrorKind::ConnectionAborted
                            | ErrorKind::BrokenPipe
                    ) =>
                {
                    return Ok(None);
                }
                Err(error) => {
                    return Err(ChimeraError::InvalidFrame(format!(
                        "tcp recv header failed: {error}"
                    )));
                }
            }
            let frame_len = u32::from_be_bytes(len_buf) as usize;
            if frame_len > max_frame_len {
                return Err(ChimeraError::InvalidFrame(
                    "tls carrier tcp frame too large".to_string(),
                ));
            }
            let mut frame = vec![0_u8; frame_len];
            stream.read_exact(&mut frame).map_err(|error| {
                ChimeraError::InvalidFrame(format!("tcp recv body failed: {error}"))
            })?;
            return Ok(Some(frame));
        }
        let mut guard = tls_bus()
            .lock()
            .map_err(|_| ChimeraError::InvalidFrame("tls carrier bus lock poisoned".to_string()))?;
        Ok(guard
            .entry(self.config.connect_addr.clone())
            .or_default()
            .pop_front())
    }
}

#[cfg(test)]
mod tests {
    use super::{TlsCarrier, TlsCarrierConfig};
    use chimera_carrier::Carrier;
    use std::io::{Read, Write};

    #[test]
    fn config_validation_rejects_empty_server_name() {
        let config = TlsCarrierConfig {
            server_name: String::new(),
            connect_addr: "127.0.0.1:443".to_string(),
            connect_timeout_ms: 1000,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn tls_carrier_has_expected_name() {
        let carrier = match TlsCarrier::new(TlsCarrierConfig {
            server_name: "gateway.example.org".to_string(),
            connect_addr: "127.0.0.1:443".to_string(),
            connect_timeout_ms: 1000,
        }) {
            Ok(carrier) => carrier,
            Err(error) => unreachable!("carrier should be created: {error}"),
        };
        assert_eq!(carrier.name(), "tls-tcp");
    }

    #[test]
    fn tls_carrier_round_trips_frame_on_same_addr() {
        let cfg = TlsCarrierConfig {
            server_name: "gateway.example.org".to_string(),
            connect_addr: "127.0.0.1:443".to_string(),
            connect_timeout_ms: 1000,
        };
        let mut sender = match TlsCarrier::new(cfg.clone()) {
            Ok(carrier) => carrier,
            Err(error) => unreachable!("sender should be created: {error}"),
        };
        let mut receiver = match TlsCarrier::new(cfg) {
            Ok(carrier) => carrier,
            Err(error) => unreachable!("receiver should be created: {error}"),
        };

        assert!(sender.send(vec![9, 8, 7]).is_ok());
        let recv = match receiver.recv() {
            Ok(Some(frame)) => frame,
            Ok(None) => unreachable!("frame should be available"),
            Err(error) => unreachable!("receive should succeed: {error}"),
        };
        assert_eq!(recv, vec![9, 8, 7]);
    }

    #[test]
    fn tls_carrier_rejects_oversized_frame() {
        let cfg = TlsCarrierConfig {
            server_name: "gateway.example.org".to_string(),
            connect_addr: "127.0.0.1:9443".to_string(),
            connect_timeout_ms: 1000,
        };
        let mut carrier = match TlsCarrier::new(cfg).and_then(|c| c.with_max_frame_len(2)) {
            Ok(carrier) => carrier,
            Err(error) => unreachable!("carrier should be created: {error}"),
        };
        assert!(carrier.send(vec![1, 2, 3]).is_err());
    }

    #[test]
    fn tls_carrier_tcp_send_and_recv_work() {
        use std::io::ErrorKind;
        use std::net::TcpListener;
        use std::thread;

        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(error) if error.kind() == ErrorKind::PermissionDenied => {
                // Some restricted CI/sandbox environments deny local bind.
                return;
            }
            Err(error) => unreachable!("listener should bind: {error}"),
        };
        let addr = match listener.local_addr() {
            Ok(addr) => addr,
            Err(error) => unreachable!("local addr should exist: {error}"),
        };

        let server = thread::spawn(move || -> Result<(), String> {
            let (mut stream, _) = listener
                .accept()
                .map_err(|error| format!("accept failed: {error}"))?;
            let mut len_buf = [0_u8; 4];
            stream
                .read_exact(&mut len_buf)
                .map_err(|error| format!("read request len failed: {error}"))?;
            let req_len = u32::from_be_bytes(len_buf) as usize;
            let mut request = vec![0_u8; req_len];
            stream
                .read_exact(&mut request)
                .map_err(|error| format!("read request body failed: {error}"))?;
            stream
                .write_all(&(request.len() as u32).to_be_bytes())
                .map_err(|error| format!("write response len failed: {error}"))?;
            stream
                .write_all(&request)
                .map_err(|error| format!("write response body failed: {error}"))?;
            Ok(())
        });

        let cfg = TlsCarrierConfig {
            server_name: "gateway.example.org".to_string(),
            connect_addr: format!("tcp://{addr}"),
            connect_timeout_ms: 2000,
        };
        let mut carrier = match TlsCarrier::new(cfg) {
            Ok(carrier) => carrier,
            Err(error) => unreachable!("carrier should be created: {error}"),
        };

        assert!(carrier.send(vec![3, 1, 4, 1, 5]).is_ok());
        let recv = match carrier.recv() {
            Ok(Some(frame)) => frame,
            Ok(None) => unreachable!("response frame should exist"),
            Err(error) => unreachable!("recv should succeed: {error}"),
        };
        assert_eq!(recv, vec![3, 1, 4, 1, 5]);
        let server_result = match server.join() {
            Ok(result) => result,
            Err(_) => unreachable!("server thread should not panic"),
        };
        assert!(server_result.is_ok());
    }
}
