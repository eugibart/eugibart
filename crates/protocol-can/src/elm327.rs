//! ELM327/STN-class AT-command CAN transport (OBDLink SX/EX and similar).
//!
//! Unlike SLCAN's raw-frame passthrough, ELM327 hides the CAN ID on receive
//! in its default ("headers off") mode — a response is just hex data bytes.
//! This module models that: incoming frames are assumed to come from a
//! single configured `rx_id` set out of band (via `ATSH`/`ATCRA` at
//! bring-up), not parsed from the response text itself. Untested against
//! real hardware; the encode/decode helpers are pure functions with direct
//! unit tests.

/// Format an AT command line (with its trailing `\r`).
pub fn encode_at_command(cmd: &str) -> String {
    format!("{cmd}\r")
}

/// Format a frame's data bytes as the hex line ELM327 expects for a request.
pub fn encode_frame_hex(data: &[u8]) -> String {
    let hex: String = data.iter().map(|b| format!("{b:02X}")).collect();
    format!("{hex}\r")
}

/// Parse one ELM327 response line into raw bytes. Rejects `NO DATA`/`ERROR`
/// and strips the whitespace and `>` prompt ELM327 appends.
pub fn decode_response_hex(line: &str) -> Option<Vec<u8>> {
    let cleaned: String = line
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '>')
        .collect();
    if cleaned.is_empty()
        || cleaned.eq_ignore_ascii_case("NODATA")
        || cleaned.eq_ignore_ascii_case("ERROR")
        || cleaned.eq_ignore_ascii_case("OK")
    {
        return None;
    }
    if !cleaned.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(cleaned.len() / 2);
    for i in (0..cleaned.len()).step_by(2) {
        bytes.push(u8::from_str_radix(&cleaned[i..i + 2], 16).ok()?);
    }
    Some(bytes)
}

#[cfg(feature = "vcp")]
mod serial {
    use std::io::{Read, Write};
    use std::time::{Duration, Instant};

    use super::{decode_response_hex, encode_at_command, encode_frame_hex};
    use crate::frame::CanFrame;
    use crate::transport::{CanError, CanTransport, Result};

    /// ELM327/STN over a USB virtual COM port, addressing a single fixed
    /// tx/rx CAN ID pair (headers configured once at construction).
    pub struct Elm327Transport {
        port: Box<dyn serialport::SerialPort>,
        rx_id: u32,
        read_buf: Vec<u8>,
    }

    impl Elm327Transport {
        pub fn open(path: &str, tx_id: u32, rx_id: u32) -> Result<Self> {
            let mut port = serialport::new(path, 38_400)
                .timeout(Duration::from_millis(200))
                .open()
                .map_err(|e| CanError::Io(std::io::Error::other(e)))?;
            for cmd in [
                "ATZ",                        // reset
                "ATE0",                       // echo off
                "ATH0",                       // headers off (rx is bare data)
                "ATSP6",                      // ISO 15765-4 CAN 11-bit 500kbps
                &format!("ATSH{tx_id:03X}"),  // set our transmit header
                &format!("ATCRA{rx_id:03X}"), // filter responses to this ID
            ] {
                port.write_all(encode_at_command(cmd).as_bytes())
                    .map_err(CanError::Io)?;
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(Self {
                port,
                rx_id,
                read_buf: Vec::new(),
            })
        }

        fn read_line(&mut self, timeout: Duration) -> Result<String> {
            let deadline = Instant::now() + timeout;
            loop {
                if let Some(pos) = self.read_buf.iter().position(|&b| b == b'\r' || b == b'>') {
                    let line: Vec<u8> = self.read_buf.drain(..=pos).collect();
                    return Ok(String::from_utf8_lossy(&line).trim().to_string());
                }
                if Instant::now() >= deadline {
                    return Err(CanError::Timeout);
                }
                let mut chunk = [0u8; 64];
                match self.port.read(&mut chunk) {
                    Ok(0) => {}
                    Ok(n) => self.read_buf.extend_from_slice(&chunk[..n]),
                    Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(e) => return Err(CanError::Io(e)),
                }
            }
        }
    }

    impl CanTransport for Elm327Transport {
        fn send(&mut self, frame: &CanFrame) -> Result<()> {
            self.port
                .write_all(encode_frame_hex(&frame.data).as_bytes())
                .map_err(CanError::Io)
        }

        fn recv(&mut self, timeout: Duration) -> Result<CanFrame> {
            let deadline = Instant::now() + timeout;
            loop {
                let remaining = deadline
                    .checked_duration_since(Instant::now())
                    .ok_or(CanError::Timeout)?;
                let line = self.read_line(remaining)?;
                if let Some(data) = decode_response_hex(&line) {
                    return Ok(CanFrame::new(self.rx_id, data));
                }
            }
        }
    }
}

#[cfg(feature = "vcp")]
pub use serial::Elm327Transport;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_at_command() {
        assert_eq!(encode_at_command("ATZ"), "ATZ\r");
    }

    #[test]
    fn encodes_frame_hex() {
        assert_eq!(encode_frame_hex(&[0x02, 0x10, 0x01]), "021001\r");
    }

    #[test]
    fn decodes_response_hex() {
        assert_eq!(
            decode_response_hex("62F10C1A"),
            Some(vec![0x62, 0xF1, 0x0C, 0x1A])
        );
        assert_eq!(
            decode_response_hex("62 F1 0C 1A\r>"),
            Some(vec![0x62, 0xF1, 0x0C, 0x1A])
        );
    }

    #[test]
    fn rejects_status_lines() {
        assert_eq!(decode_response_hex("NO DATA"), None);
        assert_eq!(decode_response_hex("ERROR"), None);
        assert_eq!(decode_response_hex("OK"), None);
        assert_eq!(decode_response_hex(""), None);
    }

    #[test]
    fn rejects_odd_length_hex() {
        assert_eq!(decode_response_hex("ABC"), None);
    }
}
