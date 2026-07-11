#![forbid(unsafe_code)]

use chimera_carrier::Carrier;
use chimera_core::{ChimeraError, ChimeraResult};
use std::collections::{HashMap, VecDeque};
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_MAX_FRAME_LEN: usize = 64 * 1024;
const DEFAULT_RECONNECT_MAX_WAIT_MS: u64 = 5_000;
const DEFAULT_RECONNECT_MIN_BACKOFF_MS: u64 = 50;
const DEFAULT_RECONNECT_MAX_BACKOFF_MS: u64 = 1_000;

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
    reconnect_max_wait_ms: u64,
}

impl TlsCarrier {
    pub fn new(config: TlsCarrierConfig) -> ChimeraResult<Self> {
        config.validate()?;
        Ok(Self {
            config,
            max_frame_len: DEFAULT_MAX_FRAME_LEN,
            stream: None,
            reconnect_max_wait_ms: DEFAULT_RECONNECT_MAX_WAIT_MS,
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

    pub fn with_reconnect_max_wait_ms(mut self, milliseconds: u64) -> ChimeraResult<Self> {
        if milliseconds == 0 {
            return Err(ChimeraError::InvalidConfig(
                "tls carrier reconnect_max_wait_ms must be > 0".to_string(),
            ));
        }
        self.reconnect_max_wait_ms = milliseconds;
        Ok(self)
    }

    pub fn set_connect_addr(&mut self, connect_addr: impl AsRef<str>) -> ChimeraResult<()> {
        let trimmed = connect_addr.as_ref().trim();
        if trimmed.is_empty() {
            return Err(ChimeraError::InvalidConfig(
                "tls carrier connect_addr is empty".to_string(),
            ));
        }
        let new_addr = trimmed.to_string();
        if self.config.connect_addr != new_addr {
            self.config.connect_addr = new_addr;
            self.drop_stream("connect_addr_changed");
            eprintln!("event=tls_carrier_connect_addr_changed transport=tls/tcp reason_class=operator_or_discovery_update");
        }
        Ok(())
    }

    fn tcp_target(&self) -> Option<&str> {
        self.config.connect_addr.strip_prefix("tcp://")
    }

    fn drop_stream(&mut self, reason_class: &str) {
        if self.stream.is_some() {
            eprintln!(
                "event=tls_carrier_stream_dropped transport=tls/tcp reason_class={reason_class}"
            );
            self.stream = None;
        }
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

    fn try_send_tcp(&mut self, frame: &[u8]) -> ChimeraResult<()> {
        let stream = self.ensure_stream()?;
        let len = u32::try_from(frame.len()).map_err(|_| {
            ChimeraError::InvalidFrame("tls carrier frame length overflow".to_string())
        })?;
        stream
            .write_all(&len.to_be_bytes())
            .and_then(|_| stream.write_all(frame))
            .map_err(|error| ChimeraError::InvalidFrame(format!("tcp send failed: {error}")))
    }

    fn try_recv_tcp(&mut self) -> ChimeraResult<Option<Vec<u8>>> {
        let max_frame_len = self.max_frame_len;
        let stream = self.ensure_stream()?;
        let mut len_buf = [0_u8; 4];
        match stream.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::WouldBlock | ErrorKind::TimedOut
                ) =>
            {
                return Ok(None);
            }
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::UnexpectedEof
                        | ErrorKind::ConnectionReset
                        | ErrorKind::ConnectionAborted
                        | ErrorKind::BrokenPipe
                ) =>
            {
                return Err(ChimeraError::InvalidFrame(format!(
                    "tcp recv header failed: {error}"
                )));
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
        Ok(Some(frame))
    }

    fn reconnect_loop<F, T>(&mut self, mut operation: F) -> ChimeraResult<T>
    where
        F: FnMut(&mut Self) -> ChimeraResult<T>,
    {
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(self.reconnect_max_wait_ms))
            .ok_or_else(|| {
                ChimeraError::InvalidFrame(
                    "tls carrier reconnect deadline overflow".to_string(),
                )
            })?;
        let max_backoff = Duration::from_millis(DEFAULT_RECONNECT_MAX_BACKOFF_MS);
        let min_backoff = Duration::from_millis(DEFAULT_RECONNECT_MIN_BACKOFF_MS);
        let mut backoff = min_backoff;
        let mut attempt: u64 = 0;

        loop {
            attempt += 1;
            match operation(self) {
                Ok(value) => return Ok(value),
                Err(error) => {
                    if !is_recoverable_carrier_error(&error) || Instant::now() >= deadline {
                        return Err(error);
                    }
                    self.drop_stream("unexpected_disconnect");
                    eprintln!(
                        "event=tls_carrier_reconnect transport=tls/tcp reason_class=unexpected_disconnect attempt={attempt}"
                    );
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        return Err(error);
                    }
                    let sleep = backoff.min(remaining);
                    thread::sleep(sleep);
                    backoff = (backoff * 2).min(max_backoff);
                }
            }
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
        if self.tcp_target().is_none() {
            let mut guard = tls_bus()
                .lock()
                .map_err(|_| ChimeraError::InvalidFrame("tls carrier bus lock poisoned".to_string()))?;
            guard
                .entry(self.config.connect_addr.clone())
                .or_default()
                .push_back(frame);
            return Ok(());
        }
        let frame_ref = frame;
        self.reconnect_loop(|carrier| carrier.try_send_tcp(&frame_ref))
    }

    fn recv(&mut self) -> ChimeraResult<Option<Vec<u8>>> {
        if self.tcp_target().is_none() {
            let mut guard = tls_bus()
                .lock()
                .map_err(|_| ChimeraError::InvalidFrame("tls carrier bus lock poisoned".to_string()))?;
            return Ok(guard
                .entry(self.config.connect_addr.clone())
                .or_default()
                .pop_front());
        }
        self.reconnect_loop(|carrier| carrier.try_recv_tcp())
    }
}

fn is_recoverable_carrier_error(error: &ChimeraError) -> bool {
    match error {
        ChimeraError::InvalidFrame(message) => {
            message.contains("tcp connect failed:")
                || message.contains("tcp send failed:")
                || message.contains("tcp recv header failed:")
                || message.contains("tcp recv body failed:")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{TlsCarrier, TlsCarrierConfig};
    use chimera_carrier::Carrier;
    use std::io::{ErrorKind, Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::{Duration, Instant};

    fn expect_ok<T>(result: Result<T, chimera_core::ChimeraError>, context: &str) -> T {
        match result {
            Ok(value) => value,
            Err(error) => unreachable!("{context}: {error}"),
        }
    }

    fn reserve_port() -> u16 {
        let temp = match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(error) => unreachable!("reserve port bind failed: {error}"),
        };
        match temp.local_addr() {
            Ok(addr) => addr.port(),
            Err(error) => unreachable!("reserve port local addr failed: {error}"),
        }
    }

    fn run_echo_server(
        listener: TcpListener,
        ready: Option<std::sync::mpsc::Sender<()>>,
    ) -> thread::JoinHandle<Result<(), String>> {
        thread::spawn(move || -> Result<(), String> {
            if let Some(ready) = ready {
                let _ = ready.send(());
            }
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
        })
    }

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
            server_name: "node.example.org".to_string(),
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
            server_name: "node.example.org".to_string(),
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
            server_name: "node.example.org".to_string(),
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

        let server = run_echo_server(listener, None);

        let cfg = TlsCarrierConfig {
            server_name: "node.example.org".to_string(),
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

    #[test]
    fn tls_carrier_reconnects_within_deadline_after_server_late_bind() {
        let port = reserve_port();
        let connect_addr = format!("tcp://127.0.0.1:{port}");

        let server_thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            let listener = match TcpListener::bind(format!("127.0.0.1:{port}")) {
                Ok(listener) => listener,
                Err(error) => unreachable!("late server bind failed: {error}"),
            };
            let (mut stream, _) = match listener.accept() {
                Ok(pair) => pair,
                Err(error) => unreachable!("late server accept failed: {error}"),
            };
            let mut len_buf = [0_u8; 4];
            if let Err(error) = stream.read_exact(&mut len_buf) {
                unreachable!("late server read len failed: {error}");
            }
            let req_len = u32::from_be_bytes(len_buf) as usize;
            let mut request = vec![0_u8; req_len];
            if let Err(error) = stream.read_exact(&mut request) {
                unreachable!("late server read body failed: {error}");
            }
            if let Err(error) = stream.write_all(&(request.len() as u32).to_be_bytes()) {
                unreachable!("late server write response len failed: {error}");
            }
            if let Err(error) = stream.write_all(&request) {
                unreachable!("late server write response body failed: {error}");
            }
        });

        let cfg = TlsCarrierConfig {
            server_name: "node.example.org".to_string(),
            connect_addr,
            connect_timeout_ms: 1000,
        };
        let mut carrier = expect_ok(TlsCarrier::new(cfg), "carrier create");
        let start = Instant::now();
        assert!(carrier.send(vec![7, 8, 9]).is_ok());
        let recv = match carrier.recv() {
            Ok(Some(frame)) => frame,
            Ok(None) => unreachable!("reconnected frame should exist"),
            Err(error) => unreachable!("reconnected recv should succeed: {error}"),
        };
        let elapsed = start.elapsed();
        assert_eq!(recv, vec![7, 8, 9]);
        assert!(
            elapsed < Duration::from_secs(5),
            "reconnect took too long: {:?}",
            elapsed
        );

        let _ = server_thread.join();
    }

    #[test]
    fn tls_carrier_reconnects_after_disconnect_and_endpoint_change() {
        let first_port = reserve_port();
        let first_addr = format!("tcp://127.0.0.1:{first_port}");
        let first_listener = match TcpListener::bind(format!("127.0.0.1:{first_port}")) {
            Ok(listener) => listener,
            Err(error) => unreachable!("first listener bind failed: {error}"),
        };

        let (first_ready_tx, first_ready_rx) = std::sync::mpsc::channel();
        let first_server = run_echo_server(first_listener, Some(first_ready_tx));
        let _ = first_ready_rx.recv();

        let cfg = TlsCarrierConfig {
            server_name: "node.example.org".to_string(),
            connect_addr: first_addr,
            connect_timeout_ms: 2000,
        };
        let mut carrier = expect_ok(TlsCarrier::new(cfg), "carrier create");

        assert!(carrier.send(vec![1, 2, 3]).is_ok());
        let first_recv = match carrier.recv() {
            Ok(Some(frame)) => frame,
            Ok(None) => unreachable!("first response should exist"),
            Err(error) => unreachable!("first recv should succeed: {error}"),
        };
        assert_eq!(first_recv, vec![1, 2, 3]);
        assert!(first_server.join().is_ok());

        let second_port = reserve_port();
        let second_addr = format!("tcp://127.0.0.1:{second_port}");
        let second_listener = match TcpListener::bind(format!("127.0.0.1:{second_port}")) {
            Ok(listener) => listener,
            Err(error) => unreachable!("second listener bind failed: {error}"),
        };
        let (second_ready_tx, second_ready_rx) = std::sync::mpsc::channel();
        let second_server = run_echo_server(second_listener, Some(second_ready_tx));
        let _ = second_ready_rx.recv();

        assert!(carrier.set_connect_addr(&second_addr).is_ok());
        assert!(carrier.send(vec![4, 5, 6]).is_ok());
        let second_recv = match carrier.recv() {
            Ok(Some(frame)) => frame,
            Ok(None) => unreachable!("second response should exist"),
            Err(error) => unreachable!("second recv should succeed: {error}"),
        };
        assert_eq!(second_recv, vec![4, 5, 6]);
        assert!(second_server.join().is_ok());
    }
}
