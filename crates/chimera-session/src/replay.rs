use chimera_core::{ChimeraError, ChimeraResult};

#[derive(Debug, Clone, Default)]
pub struct ReplayWindow {
    highest_seen: Option<u64>,
}

impl ReplayWindow {
    pub fn accept(&mut self, packet_number: u64) -> ChimeraResult<()> {
        if self
            .highest_seen
            .is_some_and(|highest| packet_number <= highest)
        {
            return Err(ChimeraError::ReplayDetected);
        }

        self.highest_seen = Some(packet_number);
        Ok(())
    }
}
