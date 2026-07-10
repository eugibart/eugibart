//! SLCAN (Lawicel ASCII CAN) transport, as spoken by CANable/candleLight-class
//! USB-CAN adapters over a virtual COM port.
//!
//! Frame format: `t<id:3hex><dlc:1hex><data hex>\r` for standard IDs,
//! `T<id:8hex><dlc:1hex><data hex>\r` for extended. The encode/decode
//! functions are pure and unit-tested directly; the serial-backed transport
//! (behind the `vcp` feature) is untested against real hardware, same as
//! `motodiag_transport::serial_vcp` for K-line.

use crate::frame::CanFrame;

/// Encode a frame as one SLCAN line, including the trailing `\r`.
pub fn encode_frame(frame: &CanFrame) -> String {
    let mut s = String::new();
    if frame.extended {
        s.push('T');
        s.push_str(&format!("{:08X}", frame.id));
    } else {
        s.push('t');
        s.push_str(&format!("{:03X}", frame.id));
    }
    s.push_str(&format!("{:X}", frame.data.len()));
    for b in &frame.data {
        s.push_str(&format!("{b:02X}"));
    }
    s.push('\r');
    s
}

/// Decode one SLCAN line (a `t...`/`T...` frame). Non-frame lines (acks,
/// errors, other Lawicel commands) return `None`.
pub fn decode_frame(line: &str) -> Option<CanFrame> {
    let line = line.trim();
    let mut chars = line.chars();
    let kind = chars.next()?;
    let (extended, id_len) = match kind {
        't' => (false, 3),
        'T' => (true, 8),
        _ => return None,
    };
    let rest: String = chars.collect();
    if rest.len() < id_len + 1 {
        return None;
    }
    let id = u32::from_str_radix(&rest[..id_len], 16).ok()?;
    let dlc = rest[id_len..id_len + 1].chars().next()?.to_digit(16)? as usize;
    if dlc > 8 {
        return None;
    }
    let data_hex = &rest[id_len + 1..];
    if data_hex.len() < dlc * 2 {
        return None;
    }
    let mut data = Vec::with_capacity(dlc);
    for i in 0..dlc {
        data.push(u8::from_str_radix(&data_hex[i * 2..i * 2 + 2], 16).ok()?);
    }
    Some(CanFrame { id, extended, data })
}

#[cfg(feature = "vcp")]
mod serial {
    use std::io::{Read, Write};
    use std::time::{Duration, Instant};

    use super::{decode_frame, encode_frame};
    use crate::frame::CanFrame;
    use crate::transport::{CanError, CanTransport, Result};

    /// SLCAN over a USB virtual COM port. Construction opens the port and
    /// sends the standard Lawicel bring-up sequence (close any existing
    /// session, set the bit rate, open the channel).
    pub struct SlcanTransport {
        port: Box<dyn serialport::SerialPort>,
        read_buf: Vec<u8>,
    }

    impl SlcanTransport {
        /// `bitrate_code` is a Lawicel `S`-command index (e.g. `6` = 500 kbit/s,
        /// the common CAN bus speed on OBD-II-style diagnostic connectors).
        pub fn open(path: &str, bitrate_code: u8) -> Result<Self> {
            let mut port = serialport::new(path, 115_200)
                .timeout(Duration::from_millis(100))
                .open()
                .map_err(|e| CanError::Io(std::io::Error::other(e)))?;
            port.write_all(b"C\r").ok(); // close, in case already open
            std::thread::sleep(Duration::from_millis(20));
            port.write_all(format!("S{bitrate_code}\r").as_bytes())
                .map_err(CanError::Io)?;
            port.write_all(b"O\r").map_err(CanError::Io)?; // open channel
            Ok(Self {
                port,
                read_buf: Vec::new(),
            })
        }

        fn read_line(&mut self, timeout: Duration) -> Result<String> {
            let deadline = Instant::now() + timeout;
            loop {
                if let Some(pos) = self.read_buf.iter().position(|&b| b == b'\r') {
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

    impl CanTransport for SlcanTransport {
        fn send(&mut self, frame: &CanFrame) -> Result<()> {
            self.port
                .write_all(encode_frame(frame).as_bytes())
                .map_err(CanError::Io)
        }

        fn recv(&mut self, timeout: Duration) -> Result<CanFrame> {
            let deadline = Instant::now() + timeout;
            loop {
                let remaining = deadline
                    .checked_duration_since(Instant::now())
                    .ok_or(CanError::Timeout)?;
                let line = self.read_line(remaining)?;
                if let Some(frame) = decode_frame(&line) {
                    return Ok(frame);
                }
                // Not a frame line (ack/error/other) — keep reading.
            }
        }
    }
}

#[cfg(feature = "vcp")]
pub use serial::SlcanTransport;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_standard_frame() {
        let frame = CanFrame::new(0x7E0, vec![0x02, 0x10, 0x01]);
        assert_eq!(encode_frame(&frame), "t7E03021001\r");
    }

    #[test]
    fn encodes_extended_frame() {
        let frame = CanFrame::new(0x1FFFFFFF, vec![0xAA]);
        assert_eq!(encode_frame(&frame), "T1FFFFFFF1AA\r");
    }

    #[test]
    fn decode_roundtrips_with_encode() {
        let frame = CanFrame::new(0x7E8, vec![0x04, 0x62, 0xF1, 0x0C, 0x1A]);
        let line = encode_frame(&frame);
        assert_eq!(decode_frame(&line), Some(frame));
    }

    #[test]
    fn decode_rejects_non_frame_lines() {
        assert_eq!(decode_frame("z"), None);
        assert_eq!(decode_frame(""), None);
        assert_eq!(decode_frame("\r"), None);
    }

    #[test]
    fn decode_rejects_truncated_data() {
        // Claims dlc=8 but only provides 2 data bytes.
        assert_eq!(decode_frame("t7E081234"), None);
    }
}
