use chimera_core::{ChimeraError, ChimeraResult};
use chimera_crypto::{TrafficSecret, decrypt_chacha20poly1305, encrypt_chacha20poly1305};

pub const FRAME_VERSION: u8 = 1;
pub const MAX_PAYLOAD_LEN: usize = 16 * 1024;

const HEADER_LEN: usize = 14;
const FRAME_KIND_DATA: u8 = 1;
const FRAME_KIND_FIN: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    Data,
    Fin,
}

impl FrameKind {
    pub(crate) fn encode(self) -> u8 {
        match self {
            Self::Data => FRAME_KIND_DATA,
            Self::Fin => FRAME_KIND_FIN,
        }
    }

    pub(crate) fn decode(value: u8) -> ChimeraResult<Self> {
        match value {
            FRAME_KIND_DATA => Ok(Self::Data),
            FRAME_KIND_FIN => Ok(Self::Fin),
            _ => Err(ChimeraError::InvalidFrame("unknown frame kind".to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub kind: FrameKind,
    pub packet_number: u64,
    pub payload: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SealedTransitFrame {
    kind: FrameKind,
    packet_number: u64,
    payload_len: usize,
    encoded: Vec<u8>,
}

impl core::fmt::Debug for SealedTransitFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SealedTransitFrame")
            .field("kind", &self.kind)
            .field("packet_number", &self.packet_number)
            .field("payload_len", &self.payload_len)
            .field("encoded", &"<sealed>")
            .finish()
    }
}

impl SealedTransitFrame {
    pub fn kind(&self) -> FrameKind {
        self.kind
    }

    pub fn packet_number(&self) -> u64 {
        self.packet_number
    }

    pub fn payload_len(&self) -> usize {
        self.payload_len
    }

    pub fn encoded(&self) -> &[u8] {
        &self.encoded
    }

    pub fn into_encoded(self) -> Vec<u8> {
        self.encoded
    }
}

impl Frame {
    pub fn encode(&self) -> ChimeraResult<Vec<u8>> {
        if self.payload.len() > MAX_PAYLOAD_LEN {
            return Err(ChimeraError::InvalidFrame("payload too large".to_string()));
        }

        let payload_len = u32::try_from(self.payload.len())
            .map_err(|_| ChimeraError::InvalidFrame("payload length overflow".to_string()))?;
        let mut encoded = Vec::with_capacity(HEADER_LEN + self.payload.len());
        encoded.push(FRAME_VERSION);
        encoded.push(self.kind.encode());
        encoded.extend_from_slice(&self.packet_number.to_be_bytes());
        encoded.extend_from_slice(&payload_len.to_be_bytes());
        encoded.extend_from_slice(&self.payload);
        Ok(encoded)
    }

    pub fn decode(input: &[u8]) -> ChimeraResult<Self> {
        if input.len() < HEADER_LEN {
            return Err(ChimeraError::InvalidFrame("frame too short".to_string()));
        }

        if input[0] != FRAME_VERSION {
            return Err(ChimeraError::InvalidFrame(
                "unsupported frame version".to_string(),
            ));
        }

        let kind = FrameKind::decode(input[1])?;
        let packet_number = read_u64(&input[2..10])?;
        let payload_len = read_u32(&input[10..14])? as usize;

        if payload_len > MAX_PAYLOAD_LEN {
            return Err(ChimeraError::InvalidFrame("payload too large".to_string()));
        }

        if input.len() != HEADER_LEN + payload_len {
            return Err(ChimeraError::InvalidFrame(
                "payload length mismatch".to_string(),
            ));
        }

        Ok(Self {
            kind,
            packet_number,
            payload: input[HEADER_LEN..].to_vec(),
        })
    }
}

pub fn encrypt_frame_payload(
    packet_number: u64,
    plaintext: &[u8],
    traffic_secret: &TrafficSecret,
) -> ChimeraResult<Frame> {
    encrypt_frame_payload_with_kind(FrameKind::Data, packet_number, plaintext, traffic_secret)
}

pub fn encrypt_fin_frame_payload(
    packet_number: u64,
    plaintext: &[u8],
    traffic_secret: &TrafficSecret,
) -> ChimeraResult<Frame> {
    encrypt_frame_payload_with_kind(FrameKind::Fin, packet_number, plaintext, traffic_secret)
}

fn encrypt_frame_payload_with_kind(
    kind: FrameKind,
    packet_number: u64,
    plaintext: &[u8],
    traffic_secret: &TrafficSecret,
) -> ChimeraResult<Frame> {
    if plaintext.len() > MAX_PAYLOAD_LEN {
        return Err(ChimeraError::InvalidFrame("payload too large".to_string()));
    }
    let aad = frame_aad(kind, packet_number);
    let payload = encrypt_chacha20poly1305(traffic_secret, packet_number, &aad, plaintext)?;
    Ok(Frame {
        kind,
        packet_number,
        payload,
    })
}

pub fn decrypt_frame_payload(
    frame: &Frame,
    traffic_secret: &TrafficSecret,
) -> ChimeraResult<Vec<u8>> {
    let aad = frame_aad(frame.kind, frame.packet_number);
    decrypt_chacha20poly1305(traffic_secret, frame.packet_number, &aad, &frame.payload)
}

pub fn validate_sealed_transit_frame(input: &[u8]) -> ChimeraResult<SealedTransitFrame> {
    if input.len() < HEADER_LEN {
        return Err(ChimeraError::InvalidFrame("frame too short".to_string()));
    }

    if input[0] != FRAME_VERSION {
        return Err(ChimeraError::InvalidFrame(
            "unsupported frame version".to_string(),
        ));
    }

    let kind = FrameKind::decode(input[1])?;
    let packet_number = read_u64(&input[2..10])?;
    let payload_len = read_u32(&input[10..14])? as usize;
    if payload_len > MAX_PAYLOAD_LEN {
        return Err(ChimeraError::InvalidFrame("payload too large".to_string()));
    }
    if input.len() != HEADER_LEN + payload_len {
        return Err(ChimeraError::InvalidFrame(
            "payload length mismatch".to_string(),
        ));
    }

    Ok(SealedTransitFrame {
        kind,
        packet_number,
        payload_len,
        encoded: input.to_vec(),
    })
}

pub fn forward_sealed_transit_frame(input: &[u8]) -> ChimeraResult<Vec<u8>> {
    validate_sealed_transit_frame(input).map(SealedTransitFrame::into_encoded)
}

fn frame_aad(kind: FrameKind, packet_number: u64) -> [u8; 10] {
    let mut aad = [0_u8; 10];
    aad[0] = FRAME_VERSION;
    aad[1] = kind.encode();
    aad[2..].copy_from_slice(&packet_number.to_be_bytes());
    aad
}

fn read_u64(bytes: &[u8]) -> ChimeraResult<u64> {
    let array: [u8; 8] = bytes
        .try_into()
        .map_err(|_| ChimeraError::InvalidFrame("invalid u64 field".to_string()))?;
    Ok(u64::from_be_bytes(array))
}

fn read_u32(bytes: &[u8]) -> ChimeraResult<u32> {
    let array: [u8; 4] = bytes
        .try_into()
        .map_err(|_| ChimeraError::InvalidFrame("invalid u32 field".to_string()))?;
    Ok(u32::from_be_bytes(array))
}
