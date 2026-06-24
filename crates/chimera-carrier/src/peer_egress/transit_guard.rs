use std::net::TcpStream;
use std::time::Duration;

pub const DEFAULT_TRANSIT_MAX_FRAMES_PER_DIRECTION: u64 = 1_000_000;
pub const DEFAULT_TRANSIT_MAX_BYTES_PER_DIRECTION: u64 = 64 * 1024 * 1024 * 1024;
pub const DEFAULT_TRANSIT_IDLE_TIMEOUT_MS: u64 = 120_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitRelayLimits {
    pub max_frames_per_direction: u64,
    pub max_bytes_per_direction: u64,
    pub idle_timeout_ms: u64,
}

impl Default for TransitRelayLimits {
    fn default() -> Self {
        Self {
            max_frames_per_direction: DEFAULT_TRANSIT_MAX_FRAMES_PER_DIRECTION,
            max_bytes_per_direction: DEFAULT_TRANSIT_MAX_BYTES_PER_DIRECTION,
            idle_timeout_ms: DEFAULT_TRANSIT_IDLE_TIMEOUT_MS,
        }
    }
}

impl TransitRelayLimits {
    pub fn new(
        max_frames_per_direction: u64,
        max_bytes_per_direction: u64,
        idle_timeout_ms: u64,
    ) -> Result<Self, String> {
        let limits = Self {
            max_frames_per_direction,
            max_bytes_per_direction,
            idle_timeout_ms,
        };
        limits.validate()?;
        Ok(limits)
    }

    pub fn validate(self) -> Result<(), String> {
        if self.max_frames_per_direction == 0 {
            return Err("sealed transit max frames per direction must be positive".to_string());
        }
        if self.max_bytes_per_direction == 0 {
            return Err("sealed transit max bytes per direction must be positive".to_string());
        }
        if self.idle_timeout_ms == 0 {
            return Err("sealed transit idle timeout ms must be positive".to_string());
        }
        Ok(())
    }

    pub(crate) fn idle_timeout(self) -> Duration {
        Duration::from_millis(self.idle_timeout_ms)
    }
}

pub(crate) struct TransitRelayGuard {
    limits: TransitRelayLimits,
    forwarded_frames: u64,
    forwarded_bytes: u64,
}

impl TransitRelayGuard {
    pub(crate) fn new(limits: TransitRelayLimits) -> Self {
        Self {
            limits,
            forwarded_frames: 0,
            forwarded_bytes: 0,
        }
    }

    pub(crate) fn record_frame(&mut self, sealed_wire_bytes: usize) -> Result<(), String> {
        let sealed_wire_bytes = u64::try_from(sealed_wire_bytes)
            .map_err(|_| "sealed transit frame length overflow".to_string())?;
        let next_frames = self
            .forwarded_frames
            .checked_add(1)
            .ok_or_else(|| "sealed transit frame budget overflow".to_string())?;
        if next_frames > self.limits.max_frames_per_direction {
            return Err("sealed transit frame budget exceeded".to_string());
        }
        let next_bytes = self
            .forwarded_bytes
            .checked_add(sealed_wire_bytes)
            .ok_or_else(|| "sealed transit byte budget overflow".to_string())?;
        if next_bytes > self.limits.max_bytes_per_direction {
            return Err("sealed transit byte budget exceeded".to_string());
        }
        self.forwarded_frames = next_frames;
        self.forwarded_bytes = next_bytes;
        Ok(())
    }
}

pub(crate) fn apply_transit_stream_limits(
    stream: &TcpStream,
    limits: TransitRelayLimits,
) -> Result<(), String> {
    let timeout = limits.idle_timeout();
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| format!("set sealed transit read timeout failed: {error}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| format!("set sealed transit write timeout failed: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;

    use super::{TransitRelayGuard, TransitRelayLimits, apply_transit_stream_limits};

    #[test]
    fn relay_limits_reject_zero_values() {
        assert!(TransitRelayLimits::new(0, 1, 1).is_err());
        assert!(TransitRelayLimits::new(1, 0, 1).is_err());
        assert!(TransitRelayLimits::new(1, 1, 0).is_err());
    }

    #[test]
    fn relay_guard_bounds_frames_and_bytes() -> Result<(), String> {
        let limits = TransitRelayLimits::new(2, 12, 1_000)?;
        let mut guard = TransitRelayGuard::new(limits);

        guard.record_frame(6)?;
        guard.record_frame(6)?;
        assert!(guard.record_frame(1).is_err());

        let mut byte_guard = TransitRelayGuard::new(limits);
        byte_guard.record_frame(7)?;
        assert!(byte_guard.record_frame(6).is_err());
        Ok(())
    }

    #[test]
    fn relay_stream_limits_apply_idle_timeouts() -> Result<(), String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("bind test listener failed: {error}"))?;
        let addr = listener
            .local_addr()
            .map_err(|error| format!("resolve test listener addr failed: {error}"))?;
        let client = TcpStream::connect(addr)
            .map_err(|error| format!("connect test stream failed: {error}"))?;
        let (server, _) = listener
            .accept()
            .map_err(|error| format!("accept test stream failed: {error}"))?;

        let limits = TransitRelayLimits::new(1, 1, 25)?;
        apply_transit_stream_limits(&client, limits)?;
        apply_transit_stream_limits(&server, limits)?;

        assert_eq!(
            client.read_timeout().map_err(|error| error.to_string())?,
            Some(Duration::from_millis(25))
        );
        assert_eq!(
            client.write_timeout().map_err(|error| error.to_string())?,
            Some(Duration::from_millis(25))
        );
        assert_eq!(
            server.read_timeout().map_err(|error| error.to_string())?,
            Some(Duration::from_millis(25))
        );
        assert_eq!(
            server.write_timeout().map_err(|error| error.to_string())?,
            Some(Duration::from_millis(25))
        );
        Ok(())
    }
}
