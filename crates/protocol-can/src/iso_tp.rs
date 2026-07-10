//! ISO 15765-2 (ISO-TP) segmentation over CAN frames.
//!
//! Classic CAN frames carry at most 8 bytes, so anything longer than 7 bytes
//! of payload (1 byte reserved for the PCI header) has to be split into a
//! First Frame + Consecutive Frames, paced by Flow Control from the receiver.
//!
//! Simplification: separation time (STmin) between consecutive frames is not
//! honored (frames are sent back-to-back). This is a real timing detail real
//! hardware may care about; it doesn't affect correctness of the segmentation
//! logic itself, which is what this module is responsible for. Block size
//! (BS) *is* honored, since getting that wrong would actually break framing.

use std::time::Duration;

use crate::frame::CanFrame;
use crate::transport::{CanError, CanTransport};

#[derive(Debug, thiserror::Error)]
pub enum IsoTpError {
    #[error(transparent)]
    Can(#[from] CanError),
    #[error("ISO-TP protocol error: {0}")]
    Protocol(String),
    #[error("flow control reported overflow")]
    Overflow,
}

pub type Result<T> = std::result::Result<T, IsoTpError>;

const PCI_SINGLE_FRAME: u8 = 0x00;
const PCI_FIRST_FRAME: u8 = 0x10;
const PCI_CONSECUTIVE_FRAME: u8 = 0x20;
const PCI_FLOW_CONTROL: u8 = 0x30;

/// Send `data` as a single ISO-TP message (Single Frame if it fits, otherwise
/// First Frame + Consecutive Frames governed by the peer's Flow Control).
pub fn send(
    transport: &mut dyn CanTransport,
    tx_id: u32,
    rx_id: u32,
    data: &[u8],
    timeout: Duration,
) -> Result<()> {
    if data.len() <= 7 {
        let mut payload = vec![PCI_SINGLE_FRAME | data.len() as u8];
        payload.extend_from_slice(data);
        transport.send(&CanFrame::new(tx_id, payload))?;
        return Ok(());
    }

    let len = data.len();
    assert!(
        len <= 0x0FFF,
        "ISO-TP classic addressing caps length at 4095 bytes"
    );
    let mut first = vec![
        PCI_FIRST_FRAME | ((len >> 8) as u8 & 0x0F),
        (len & 0xFF) as u8,
    ];
    first.extend_from_slice(&data[..6]);
    transport.send(&CanFrame::new(tx_id, first))?;

    let mut sent = 6;
    let mut seq = 1u8;
    loop {
        let (block_size, _stmin) = read_flow_control(transport, rx_id, timeout)?;
        let frames_this_block = if block_size == 0 {
            u32::MAX // 0 means "send everything, no further FC needed"
        } else {
            block_size as u32
        };

        let mut sent_this_block = 0;
        while sent < len && sent_this_block < frames_this_block {
            let chunk_len = (len - sent).min(7);
            let mut cf = vec![PCI_CONSECUTIVE_FRAME | (seq & 0x0F)];
            cf.extend_from_slice(&data[sent..sent + chunk_len]);
            transport.send(&CanFrame::new(tx_id, cf))?;
            sent += chunk_len;
            seq = (seq + 1) & 0x0F;
            sent_this_block += 1;
        }

        if sent >= len {
            return Ok(());
        }
        // Block size exhausted with more data left: the peer sends another FC.
    }
}

/// Wait for and validate one Flow Control frame; returns `(block_size, stmin)`.
/// Tolerates a bounded number of "Wait" responses before giving up.
fn read_flow_control(
    transport: &mut dyn CanTransport,
    rx_id: u32,
    timeout: Duration,
) -> Result<(u8, u8)> {
    for _ in 0..8 {
        let frame = transport.recv(timeout)?;
        if frame.id != rx_id {
            continue; // frame from someone else on the (simulated) bus
        }
        let pci = *frame
            .data
            .first()
            .ok_or_else(|| IsoTpError::Protocol("empty flow control frame".into()))?;
        if pci & 0xF0 != PCI_FLOW_CONTROL {
            return Err(IsoTpError::Protocol(format!(
                "expected flow control (0x3_), got PCI 0x{pci:02X}"
            )));
        }
        match pci & 0x0F {
            0 => {
                let block_size = frame.data.get(1).copied().unwrap_or(0);
                let stmin = frame.data.get(2).copied().unwrap_or(0);
                return Ok((block_size, stmin));
            }
            1 => continue, // Wait: try again
            2 => return Err(IsoTpError::Overflow),
            other => return Err(IsoTpError::Protocol(format!("unknown flow status {other}"))),
        }
    }
    Err(IsoTpError::Protocol(
        "peer kept sending Wait; giving up".into(),
    ))
}

/// Receive one ISO-TP message, sending Flow Control (Continue-To-Send, no
/// block limit) as needed for multi-frame messages.
pub fn receive(
    transport: &mut dyn CanTransport,
    tx_id: u32,
    rx_id: u32,
    timeout: Duration,
) -> Result<Vec<u8>> {
    let frame = loop {
        let frame = transport.recv(timeout)?;
        if frame.id == rx_id {
            break frame;
        }
        // Ignore frames not addressed to us (simulated bus can carry others).
    };

    let pci = *frame
        .data
        .first()
        .ok_or_else(|| IsoTpError::Protocol("empty frame".into()))?;

    match pci & 0xF0 {
        PCI_SINGLE_FRAME => {
            let len = (pci & 0x0F) as usize;
            let data = frame
                .data
                .get(1..1 + len)
                .ok_or_else(|| IsoTpError::Protocol("single frame shorter than declared".into()))?;
            Ok(data.to_vec())
        }
        PCI_FIRST_FRAME => {
            let len_hi = (pci & 0x0F) as usize;
            let len_lo = *frame
                .data
                .get(1)
                .ok_or_else(|| IsoTpError::Protocol("truncated first frame".into()))?;
            let total_len = (len_hi << 8) | len_lo as usize;

            let mut data = Vec::with_capacity(total_len);
            data.extend_from_slice(&frame.data[2..]);

            // Continue-To-Send, no block size limit, no separation time.
            transport.send(&CanFrame::new(tx_id, vec![PCI_FLOW_CONTROL, 0x00, 0x00]))?;

            let mut expected_seq = 1u8;
            while data.len() < total_len {
                let cf = transport.recv(timeout)?;
                if cf.id != rx_id {
                    continue;
                }
                let cpci = *cf
                    .data
                    .first()
                    .ok_or_else(|| IsoTpError::Protocol("empty consecutive frame".into()))?;
                if cpci & 0xF0 != PCI_CONSECUTIVE_FRAME {
                    return Err(IsoTpError::Protocol(format!(
                        "expected consecutive frame (0x2_), got PCI 0x{cpci:02X}"
                    )));
                }
                let seq = cpci & 0x0F;
                if seq != expected_seq {
                    return Err(IsoTpError::Protocol(format!(
                        "consecutive frame out of sequence: expected {expected_seq}, got {seq}"
                    )));
                }
                let remaining = total_len - data.len();
                let take = remaining.min(cf.data.len().saturating_sub(1));
                data.extend_from_slice(&cf.data[1..1 + take]);
                expected_seq = (expected_seq + 1) & 0x0F;
            }
            data.truncate(total_len);
            Ok(data)
        }
        other => Err(IsoTpError::Protocol(format!(
            "unexpected PCI 0x{other:02X} starting a message"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::pair;

    const TESTER_ID: u32 = 0x7E0;
    const ECU_ID: u32 = 0x7E8;

    #[test]
    fn single_frame_roundtrip() {
        let (mut tester, mut ecu) = pair();
        let payload = vec![0x22, 0xF1, 0x0C];
        send(
            &mut tester,
            TESTER_ID,
            ECU_ID,
            &payload,
            Duration::from_millis(50),
        )
        .unwrap();
        let received = receive(&mut ecu, ECU_ID, TESTER_ID, Duration::from_millis(50)).unwrap();
        assert_eq!(received, payload);
    }

    #[test]
    fn multi_frame_roundtrip_over_a_background_responder() {
        let (mut tester, ecu) = pair();
        // Responder thread: receive a multi-frame message, echo it straight
        // back (also multi-frame), mirroring the ecu-sim thread pattern.
        let handle = std::thread::spawn(move || {
            let mut ecu = ecu;
            let received =
                receive(&mut ecu, ECU_ID, TESTER_ID, Duration::from_millis(500)).unwrap();
            send(
                &mut ecu,
                ECU_ID,
                TESTER_ID,
                &received,
                Duration::from_millis(500),
            )
            .unwrap();
            received
        });

        let payload: Vec<u8> = (0..20).collect(); // > 7 bytes: forces FF+CF
        send(
            &mut tester,
            TESTER_ID,
            ECU_ID,
            &payload,
            Duration::from_millis(500),
        )
        .unwrap();
        let echoed = receive(&mut tester, TESTER_ID, ECU_ID, Duration::from_millis(500)).unwrap();
        assert_eq!(echoed, payload);

        let received_by_responder = handle.join().unwrap();
        assert_eq!(received_by_responder, payload);
    }

    #[test]
    fn respects_small_block_size() {
        let (mut tester, ecu) = pair();
        let handle = std::thread::spawn(move || {
            let mut ecu = ecu;
            // Manually drive the receive side with a block size of 2 to prove
            // send() asks for another Flow Control mid-message.
            let frame = ecu.recv(Duration::from_millis(500)).unwrap(); // FF
            assert_eq!(frame.id, TESTER_ID);
            let total_len = (((frame.data[0] & 0x0F) as usize) << 8) | frame.data[1] as usize;
            let mut data = frame.data[2..].to_vec();

            // First flow control: block size 2.
            ecu.send(&CanFrame::new(ECU_ID, vec![0x30, 0x02, 0x00]))
                .unwrap();
            for _ in 0..2 {
                let cf = ecu.recv(Duration::from_millis(500)).unwrap();
                data.extend_from_slice(&cf.data[1..]);
            }
            assert!(
                data.len() < total_len,
                "more consecutive frames should remain"
            );

            // A second flow control lets the sender finish.
            ecu.send(&CanFrame::new(ECU_ID, vec![0x30, 0x00, 0x00]))
                .unwrap();
            while data.len() < total_len {
                let cf = ecu.recv(Duration::from_millis(500)).unwrap();
                let remaining = total_len - data.len();
                let take = remaining.min(cf.data.len() - 1);
                data.extend_from_slice(&cf.data[1..1 + take]);
            }
            data.truncate(total_len);
            data
        });

        // 25 bytes = FF(6) + CF(7) + CF(7) + CF(5): with block size 2, the
        // first two CFs don't finish the message, so a second flow control
        // round is genuinely required — unlike a length that divides evenly.
        let payload: Vec<u8> = (0..25).collect();
        send(
            &mut tester,
            TESTER_ID,
            ECU_ID,
            &payload,
            Duration::from_millis(500),
        )
        .unwrap();
        assert_eq!(handle.join().unwrap(), payload);
    }

    #[test]
    fn overflow_flow_control_is_an_error() {
        let (mut tester, ecu) = pair();
        std::thread::spawn(move || {
            let mut ecu = ecu;
            let _ff = ecu.recv(Duration::from_millis(500)).unwrap();
            // PCI 0x32: flow control, flow status 2 (overflow).
            ecu.send(&CanFrame::new(ECU_ID, vec![0x32, 0x00, 0x00]))
                .ok();
        });

        let payload: Vec<u8> = (0..20).collect();
        let err = send(
            &mut tester,
            TESTER_ID,
            ECU_ID,
            &payload,
            Duration::from_millis(200),
        )
        .unwrap_err();
        assert!(matches!(err, IsoTpError::Overflow));
    }
}
