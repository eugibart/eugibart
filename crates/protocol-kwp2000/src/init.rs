//! Bus initialization: KWP2000 fast init (ISO 14230-2) and ISO 9141-style
//! 5-baud slow init. Which one an ECU wants comes from its definition file.

use std::time::Duration;

use motodiag_transport::KLineTransport;

use crate::framing::{read_frame, FrameCodec};
use crate::services::sid;
use crate::timing::TimingParams;
use crate::{KwpError, Result};

/// Result of a successful initialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitOutcome {
    /// Key bytes announced by the ECU (define header/timing capabilities).
    pub key_bytes: Option<(u8, u8)>,
    /// Raw positive response payload (fast init) for the trace log.
    pub raw_response: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct FastInitConfig {
    /// Bus must be idle this long before the wake-up pattern (ISO W5, 300 ms).
    pub idle_before: Duration,
    /// Wake-up pattern: line low then high, 25 ms each per the standard.
    pub low_time: Duration,
    pub high_time: Duration,
}

impl Default for FastInitConfig {
    fn default() -> Self {
        Self {
            idle_before: Duration::from_millis(300),
            low_time: Duration::from_millis(25),
            high_time: Duration::from_millis(25),
        }
    }
}

/// KWP2000 fast init: wake-up pulse, then StartCommunication.
pub fn fast_init(
    transport: &mut dyn KLineTransport,
    codec: &FrameCodec,
    timing: &TimingParams,
    cfg: &FastInitConfig,
) -> Result<InitOutcome> {
    transport.flush_input()?;
    std::thread::sleep(cfg.idle_before);

    transport.send_break(cfg.low_time)?;
    std::thread::sleep(cfg.high_time);

    transport.send(&codec.encode(&[sid::START_COMMUNICATION]))?;

    let frame = read_frame(
        transport,
        timing.first_byte_deadline(),
        timing.inter_byte_deadline(),
    )?;
    match frame.payload.first().copied() {
        Some(b) if b == sid::positive_response(sid::START_COMMUNICATION) => {
            let key_bytes = match frame.payload.as_slice() {
                [_, kb1, kb2, ..] => Some((*kb1, *kb2)),
                _ => None,
            };
            Ok(InitOutcome {
                key_bytes,
                raw_response: frame.payload,
            })
        }
        Some(sid::NEGATIVE_RESPONSE) => Err(KwpError::InitFailed(format!(
            "StartCommunication rejected: {:02X?}",
            frame.payload
        ))),
        _ => Err(KwpError::InitFailed(format!(
            "unexpected StartCommunication response: {:02X?}",
            frame.payload
        ))),
    }
}

#[derive(Debug, Clone)]
pub struct SlowInitConfig {
    /// Bit time for the 5-baud address transmission (200 ms at 5 baud).
    pub bit_time: Duration,
    /// How long to wait for the 0x55 synchronization byte.
    pub sync_timeout: Duration,
}

impl Default for SlowInitConfig {
    fn default() -> Self {
        Self {
            bit_time: Duration::from_millis(200),
            sync_timeout: Duration::from_millis(1500),
        }
    }
}

/// ISO 9141 / KWP2000 5-baud slow init.
///
/// The target address is bit-banged at 5 baud (start bit + 8 data bits LSB
/// first + stop bit, 200 ms each) by holding the line low via break for 0-bits.
/// The ECU answers with 0x55 sync at the working baud rate, two key bytes, and
/// expects the inverted second key byte back.
pub fn slow_init_5baud(
    transport: &mut dyn KLineTransport,
    address: u8,
    timing: &TimingParams,
    cfg: &SlowInitConfig,
) -> Result<InitOutcome> {
    transport.flush_input()?;

    // Start bit (dominant/low).
    transport.send_break(cfg.bit_time)?;
    // Data bits, LSB first: 0 = low (break), 1 = high (idle).
    for i in 0..8 {
        if address & (1 << i) == 0 {
            transport.send_break(cfg.bit_time)?;
        } else {
            std::thread::sleep(cfg.bit_time);
        }
    }
    // Stop bit (recessive/high).
    std::thread::sleep(cfg.bit_time);

    let sync = transport.read_byte(cfg.sync_timeout)?;
    if sync != 0x55 {
        return Err(KwpError::InitFailed(format!(
            "expected sync byte 0x55, got 0x{sync:02X}"
        )));
    }
    let kb1 = transport.read_byte(timing.inter_byte_deadline())?;
    let kb2 = transport.read_byte(timing.inter_byte_deadline())?;

    // Acknowledge: send inverted KB2, expect inverted address back.
    transport.send(&[!kb2])?;
    let addr_ack = transport.read_byte(timing.first_byte_deadline())?;
    if addr_ack != !address {
        return Err(KwpError::InitFailed(format!(
            "expected inverted address 0x{:02X}, got 0x{addr_ack:02X}",
            !address
        )));
    }

    Ok(InitOutcome {
        key_bytes: Some((kb1, kb2)),
        raw_response: vec![sync, kb1, kb2, addr_ack],
    })
}
