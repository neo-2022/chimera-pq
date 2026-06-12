use std::collections::BTreeSet;

use chimera_core::{ChimeraError, ChimeraResult};
use chimera_session::{FrameKind, validate_sealed_transit_frame};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WeaveNodeCapability {
    LocalIngress,
    PeerIngress,
    LocalEgress,
    PeerTransit,
}

impl WeaveNodeCapability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalIngress => "local_ingress",
            Self::PeerIngress => "peer_ingress",
            Self::LocalEgress => "local_egress",
            Self::PeerTransit => "peer_transit",
        }
    }
}

pub const REQUIRED_WEAVE_NODE_CAPABILITIES: [WeaveNodeCapability; 4] = [
    WeaveNodeCapability::LocalIngress,
    WeaveNodeCapability::PeerIngress,
    WeaveNodeCapability::LocalEgress,
    WeaveNodeCapability::PeerTransit,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaveNodeContract {
    capabilities: BTreeSet<WeaveNodeCapability>,
}

impl WeaveNodeContract {
    pub fn symmetric_mesh_node() -> Self {
        Self::from_capabilities(REQUIRED_WEAVE_NODE_CAPABILITIES)
    }

    pub fn from_capabilities(capabilities: impl IntoIterator<Item = WeaveNodeCapability>) -> Self {
        Self {
            capabilities: capabilities.into_iter().collect(),
        }
    }

    pub fn has_capability(&self, capability: WeaveNodeCapability) -> bool {
        self.capabilities.contains(&capability)
    }

    pub fn capability_names(&self) -> Vec<&'static str> {
        self.capabilities
            .iter()
            .map(|capability| capability.as_str())
            .collect()
    }

    pub fn validate_symmetric(&self) -> ChimeraResult<()> {
        for capability in REQUIRED_WEAVE_NODE_CAPABILITIES {
            if !self.has_capability(capability) {
                return Err(ChimeraError::InvalidConfig(format!(
                    "WEAVE node missing required capability {}",
                    capability.as_str()
                )));
            }
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct WeaveSealedTransitFrame {
    kind: FrameKind,
    packet_number: u64,
    payload_len: usize,
    sealed_bytes: Vec<u8>,
}

impl core::fmt::Debug for WeaveSealedTransitFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WeaveSealedTransitFrame")
            .field("kind", &self.kind)
            .field("packet_number", &self.packet_number)
            .field("payload_len", &self.payload_len)
            .field("sealed_bytes", &"<sealed>")
            .finish()
    }
}

impl WeaveSealedTransitFrame {
    pub fn kind(&self) -> FrameKind {
        self.kind
    }

    pub fn packet_number(&self) -> u64 {
        self.packet_number
    }

    pub fn payload_len(&self) -> usize {
        self.payload_len
    }

    pub fn sealed_bytes(&self) -> &[u8] {
        &self.sealed_bytes
    }

    pub fn into_sealed_bytes(self) -> Vec<u8> {
        self.sealed_bytes
    }
}

pub fn validate_weave_sealed_transit_frame(input: &[u8]) -> ChimeraResult<WeaveSealedTransitFrame> {
    let frame = validate_sealed_transit_frame(input)?;
    Ok(WeaveSealedTransitFrame {
        kind: frame.kind(),
        packet_number: frame.packet_number(),
        payload_len: frame.payload_len(),
        sealed_bytes: frame.into_encoded(),
    })
}

pub fn forward_weave_transit_frame(input: &[u8]) -> ChimeraResult<Vec<u8>> {
    validate_weave_sealed_transit_frame(input).map(WeaveSealedTransitFrame::into_sealed_bytes)
}

#[cfg(test)]
mod tests {
    use super::{
        REQUIRED_WEAVE_NODE_CAPABILITIES, WeaveNodeCapability, WeaveNodeContract,
        forward_weave_transit_frame, validate_weave_sealed_transit_frame,
    };
    use chimera_session::{Frame, FrameKind};

    fn encoded_frame(kind: FrameKind, packet_number: u64, payload: &[u8]) -> Vec<u8> {
        match (Frame {
            kind,
            packet_number,
            payload: payload.to_vec(),
        })
        .encode()
        {
            Ok(encoded) => encoded,
            Err(error) => unreachable!("test frame must encode: {error}"),
        }
    }

    #[test]
    fn symmetric_weave_contract_requires_all_node_capabilities() {
        let contract = WeaveNodeContract::symmetric_mesh_node();

        assert!(contract.validate_symmetric().is_ok());
        for capability in REQUIRED_WEAVE_NODE_CAPABILITIES {
            assert!(contract.has_capability(capability));
        }
        assert_eq!(
            contract.capability_names(),
            vec![
                "local_ingress",
                "peer_ingress",
                "local_egress",
                "peer_transit"
            ]
        );
    }

    #[test]
    fn symmetric_weave_contract_rejects_role_like_partial_node() {
        let partial = WeaveNodeContract::from_capabilities([
            WeaveNodeCapability::LocalIngress,
            WeaveNodeCapability::PeerIngress,
            WeaveNodeCapability::LocalEgress,
        ]);

        let error = match partial.validate_symmetric() {
            Ok(()) => unreachable!("partial WEAVE node must be rejected"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("peer_transit"));
    }

    #[test]
    fn transit_forwarding_preserves_sealed_bytes_without_payload_debug_leak() {
        let payload = b"third-party-secret";
        let encoded = encoded_frame(FrameKind::Data, 42, payload);

        let transit = match validate_weave_sealed_transit_frame(&encoded) {
            Ok(transit) => transit,
            Err(error) => unreachable!("sealed transit frame must validate: {error}"),
        };

        assert_eq!(transit.kind(), FrameKind::Data);
        assert_eq!(transit.packet_number(), 42);
        assert_eq!(transit.payload_len(), payload.len());
        assert_eq!(transit.sealed_bytes(), encoded.as_slice());

        let debug = format!("{transit:?}");
        assert!(debug.contains("<sealed>"));
        assert!(!debug.contains("third-party-secret"));

        let forwarded = match forward_weave_transit_frame(&encoded) {
            Ok(forwarded) => forwarded,
            Err(error) => unreachable!("sealed transit frame must forward: {error}"),
        };
        assert_eq!(forwarded, encoded);
    }

    #[test]
    fn transit_forwarding_accepts_fin_as_sealed_control_frame() {
        let encoded = encoded_frame(FrameKind::Fin, 43, b"");
        let transit = match validate_weave_sealed_transit_frame(&encoded) {
            Ok(transit) => transit,
            Err(error) => unreachable!("sealed FIN frame must validate: {error}"),
        };

        assert_eq!(transit.kind(), FrameKind::Fin);
        assert_eq!(transit.payload_len(), 0);
        let forwarded = match forward_weave_transit_frame(&encoded) {
            Ok(forwarded) => forwarded,
            Err(error) => unreachable!("sealed FIN frame must forward: {error}"),
        };
        assert_eq!(forwarded, encoded);
    }

    #[test]
    fn transit_forwarding_rejects_malformed_envelope() {
        let mut encoded = encoded_frame(FrameKind::Data, 44, b"sealed");
        encoded[1] = 0xff;

        assert!(validate_weave_sealed_transit_frame(&encoded).is_err());
        assert!(forward_weave_transit_frame(&encoded).is_err());
    }
}
