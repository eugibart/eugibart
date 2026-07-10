//! ISO 14230-2 frame encoding/decoding.
//!
//! Frame layout:
//! ```text
//! [format] [target] [source] [length?] [payload ...] [checksum]
//! ```
//! - format byte: bits 7..6 = addressing mode (0b10 = physical addressing with
//!   target/source bytes, 0b00 = no address info), bits 5..0 = payload length
//!   (0 means a separate length byte follows).
//! - checksum: 8-bit wrapping sum of every preceding byte.
//!
//! Marelli ECUs (IAW 5SM/5AM/59M) use physical addressing with the length in
//! the format byte, e.g. StartCommunication to ECU 0x10 from tester 0xF1:
//! `81 10 F1 81 03`.

use std::time::Duration;

use motodiag_transport::KLineTransport;

use crate::{KwpError, Result};

pub const ADDR_MODE_MASK: u8 = 0xC0;
pub const ADDR_MODE_NONE: u8 = 0x00;
pub const ADDR_MODE_PHYSICAL: u8 = 0x80;
pub const ADDR_MODE_FUNCTIONAL: u8 = 0xC0;

/// How to build headers for a given ECU. Comes from the ECU definition file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameCodec {
    /// `Some((target, source))` for physical addressing, `None` for headers
    /// without address information.
    pub addressing: Option<(u8, u8)>,
    /// Force a separate length byte even for payloads that fit the format
    /// byte (some ECUs insist on it).
    pub separate_length_byte: bool,
}

impl FrameCodec {
    /// Marelli-style: physical addressing, length in the format byte.
    pub fn physical(target: u8, source: u8) -> Self {
        Self {
            addressing: Some((target, source)),
            separate_length_byte: false,
        }
    }

    pub fn encode(&self, payload: &[u8]) -> Vec<u8> {
        assert!(
            !payload.is_empty() && payload.len() <= 255,
            "payload must be 1..=255 bytes"
        );
        let mut out = Vec::with_capacity(payload.len() + 5);
        let needs_len_byte = self.separate_length_byte || payload.len() > 63;
        let mode = if self.addressing.is_some() {
            ADDR_MODE_PHYSICAL
        } else {
            ADDR_MODE_NONE
        };
        let fmt = if needs_len_byte {
            mode
        } else {
            mode | payload.len() as u8
        };
        out.push(fmt);
        if let Some((target, source)) = self.addressing {
            out.push(target);
            out.push(source);
        }
        if needs_len_byte {
            out.push(payload.len() as u8);
        }
        out.extend_from_slice(payload);
        out.push(checksum(&out));
        out
    }
}

/// 8-bit wrapping sum over all frame bytes before the checksum position.
pub fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

/// A decoded frame as it appeared on the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub target: Option<u8>,
    pub source: Option<u8>,
    pub payload: Vec<u8>,
    /// Complete raw bytes including header and checksum (for trace logs).
    pub raw: Vec<u8>,
}

/// Read one complete frame from the transport.
///
/// `first_byte_timeout` bounds the wait for the format byte (KWP P2);
/// `inter_byte_timeout` bounds gaps inside the frame (KWP P1).
pub fn read_frame(
    transport: &mut dyn KLineTransport,
    first_byte_timeout: Duration,
    inter_byte_timeout: Duration,
) -> Result<Frame> {
    let fmt = transport.read_byte(first_byte_timeout)?;
    let mut raw = vec![fmt];

    let mut next = |raw: &mut Vec<u8>| -> Result<u8> {
        let b = transport.read_byte(inter_byte_timeout)?;
        raw.push(b);
        Ok(b)
    };

    let (target, source) = match fmt & ADDR_MODE_MASK {
        ADDR_MODE_PHYSICAL | ADDR_MODE_FUNCTIONAL => {
            let t = next(&mut raw)?;
            let s = next(&mut raw)?;
            (Some(t), Some(s))
        }
        _ => (None, None),
    };

    let len = match fmt & 0x3F {
        0 => next(&mut raw)? as usize,
        n => n as usize,
    };
    if len == 0 {
        return Err(KwpError::MalformedFrame("zero-length payload".into()));
    }

    let mut payload = Vec::with_capacity(len);
    for _ in 0..len {
        payload.push(next(&mut raw)?);
    }

    let computed = checksum(&raw);
    let received = next(&mut raw)?;
    if computed != received {
        return Err(KwpError::BadChecksum { computed, received });
    }

    Ok(Frame {
        target,
        source,
        payload,
        raw,
    })
}

/// Result of attempting to parse a frame from an accumulating byte buffer
/// (used by the ECU simulator, which receives bytes rather than pulling them).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseStatus {
    /// Not enough bytes yet.
    Incomplete,
    /// A complete, checksum-valid frame; `consumed` bytes should be drained.
    Complete { frame: Frame, consumed: usize },
    /// The buffer cannot be a valid frame; the caller should resynchronize
    /// (typically by clearing the buffer).
    Invalid(String),
}

/// Try to parse one frame from the front of `buf`.
pub fn parse_buffer(buf: &[u8]) -> ParseStatus {
    let Some(&fmt) = buf.first() else {
        return ParseStatus::Incomplete;
    };
    let has_addr = matches!(
        fmt & ADDR_MODE_MASK,
        ADDR_MODE_PHYSICAL | ADDR_MODE_FUNCTIONAL
    );
    let header_len = 1 + if has_addr { 2 } else { 0 };

    let len_bits = (fmt & 0x3F) as usize;
    let (payload_len, len_bytes) = if len_bits == 0 {
        match buf.get(header_len) {
            Some(&n) => (n as usize, 1),
            None => return ParseStatus::Incomplete,
        }
    } else {
        (len_bits, 0)
    };
    if payload_len == 0 {
        return ParseStatus::Invalid("zero-length payload".into());
    }

    let total = header_len + len_bytes + payload_len + 1;
    if buf.len() < total {
        return ParseStatus::Incomplete;
    }

    let computed = checksum(&buf[..total - 1]);
    let received = buf[total - 1];
    if computed != received {
        return ParseStatus::Invalid(format!(
            "checksum mismatch: computed {computed:02X}, received {received:02X}"
        ));
    }

    let (target, source) = if has_addr {
        (Some(buf[1]), Some(buf[2]))
    } else {
        (None, None)
    };
    let payload_start = header_len + len_bytes;
    ParseStatus::Complete {
        frame: Frame {
            target,
            source,
            payload: buf[payload_start..payload_start + payload_len].to_vec(),
            raw: buf[..total].to_vec(),
        },
        consumed: total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use motodiag_transport::mock;

    #[test]
    fn encodes_marelli_start_communication() {
        let codec = FrameCodec::physical(0x10, 0xF1);
        // Known-good KWP fast-init request shape: 81 10 F1 81 03
        assert_eq!(codec.encode(&[0x81]), vec![0x81, 0x10, 0xF1, 0x81, 0x03]);
    }

    #[test]
    fn encodes_separate_length_byte() {
        let codec = FrameCodec {
            addressing: Some((0x10, 0xF1)),
            separate_length_byte: true,
        };
        let frame = codec.encode(&[0x3E]);
        assert_eq!(
            frame,
            vec![0x80, 0x10, 0xF1, 0x01, 0x3E, checksum(&frame[..5])]
        );
    }

    #[test]
    fn decode_roundtrip() {
        let codec = FrameCodec::physical(0xF1, 0x10); // ECU -> tester direction
        let encoded = codec.encode(&[0xC1, 0xE9, 0x8F]);

        let (mut tester, ecu) = mock::pair();
        ecu.send_bytes(&encoded).unwrap();
        let frame = read_frame(
            &mut tester,
            Duration::from_millis(50),
            Duration::from_millis(20),
        )
        .unwrap();
        assert_eq!(frame.target, Some(0xF1));
        assert_eq!(frame.source, Some(0x10));
        assert_eq!(frame.payload, vec![0xC1, 0xE9, 0x8F]);
        assert_eq!(frame.raw, encoded);
    }

    #[test]
    fn detects_bad_checksum() {
        let codec = FrameCodec::physical(0xF1, 0x10);
        let mut encoded = codec.encode(&[0xC1]);
        *encoded.last_mut().unwrap() ^= 0xFF;

        let (mut tester, ecu) = mock::pair();
        ecu.send_bytes(&encoded).unwrap();
        let err = read_frame(
            &mut tester,
            Duration::from_millis(50),
            Duration::from_millis(20),
        )
        .unwrap_err();
        assert!(matches!(err, KwpError::BadChecksum { .. }));
    }
}
