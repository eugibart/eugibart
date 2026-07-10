//! K-line over a USB virtual COM port (FTDI KKL cable and friends).
//!
//! Timing note for macOS: Apple's built-in FTDI driver uses a fixed ~16 ms
//! latency timer, which adds jitter to byte arrival times. That is acceptable
//! for KWP2000 request/response traffic (P2 is 25–50 ms), but keep it in mind
//! when reading logic-analyzer traces. A `libftd2xx`-based backend can be
//! added behind this same trait if VCP timing proves too coarse for fast init.

use std::io::{Read, Write};
use std::time::{Duration, Instant};

use crate::{KLineTransport, Result, TransportError};

pub struct SerialKLine {
    port: Box<dyn serialport::SerialPort>,
    /// Real K-line hardware echoes every transmitted byte; loopback-style
    /// test rigs (pty pairs) do not. When true, `send` consumes the echo.
    pub consume_echo: bool,
}

impl SerialKLine {
    /// Open `path` at `baud`, 8N1 (KWP2000 on Marelli ECUs is 10400 baud 8N1).
    pub fn open(path: &str, baud: u32) -> Result<Self> {
        let port = serialport::new(path, baud)
            .data_bits(serialport::DataBits::Eight)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .flow_control(serialport::FlowControl::None)
            // Per-call timeouts are enforced in read_byte; keep the port's own
            // timeout short so deadlines stay accurate.
            .timeout(Duration::from_millis(10))
            .open()
            .map_err(|e| TransportError::Io(std::io::Error::other(e)))?;
        Ok(Self {
            port,
            consume_echo: true,
        })
    }

    /// List serial ports visible to the OS.
    pub fn available_ports() -> Vec<String> {
        serialport::available_ports()
            .map(|ports| ports.into_iter().map(|p| p.port_name).collect())
            .unwrap_or_default()
    }
}

/// FTDI's USB vendor ID — genuine FT232-class cables (the kind that work
/// reliably for K-line) enumerate under this.
pub const FTDI_VID: u16 = 0x0403;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsbDetails {
    pub vid: u16,
    pub pid: u16,
    pub product: Option<String>,
    pub manufacturer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortDetails {
    pub name: String,
    /// Present when the OS reports this port as a USB device.
    pub usb: Option<UsbDetails>,
}

impl PortDetails {
    pub fn is_ftdi(&self) -> bool {
        self.usb.as_ref().is_some_and(|u| u.vid == FTDI_VID)
    }
}

/// Enumerate serial ports with USB metadata where the OS provides it (used
/// by the connection troubleshooter to spot FTDI cables vs. CH340 clones).
pub fn port_details() -> Vec<PortDetails> {
    serialport::available_ports()
        .map(|ports| {
            ports
                .into_iter()
                .map(|p| PortDetails {
                    name: p.port_name,
                    usb: match p.port_type {
                        serialport::SerialPortType::UsbPort(info) => Some(UsbDetails {
                            vid: info.vid,
                            pid: info.pid,
                            product: info.product,
                            manufacturer: info.manufacturer,
                        }),
                        _ => None,
                    },
                })
                .collect()
        })
        .unwrap_or_default()
}

impl KLineTransport for SerialKLine {
    fn send(&mut self, bytes: &[u8]) -> Result<()> {
        self.port.write_all(bytes)?;
        self.port.flush()?;
        if self.consume_echo {
            // Give the echo generous time: it should arrive within a few byte
            // times, but USB latency batches arrivals.
            let echoed = self
                .read_exact(
                    bytes.len(),
                    Duration::from_millis(200),
                    Duration::from_millis(50),
                )
                .map_err(|e| match e {
                    // A missing echo is a wiring/cable diagnosis, not a
                    // generic timeout — see TransportError::NoEcho.
                    TransportError::Timeout => TransportError::NoEcho {
                        sent: bytes.to_vec(),
                    },
                    other => other,
                })?;
            if echoed != bytes {
                return Err(TransportError::EchoMismatch {
                    sent: bytes.to_vec(),
                    echoed,
                });
            }
        }
        Ok(())
    }

    fn read_byte(&mut self, timeout: Duration) -> Result<u8> {
        let deadline = Instant::now() + timeout;
        let mut buf = [0u8; 1];
        loop {
            match self.port.read(&mut buf) {
                Ok(1) => return Ok(buf[0]),
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => return Err(e.into()),
            }
            if Instant::now() >= deadline {
                return Err(TransportError::Timeout);
            }
        }
    }

    fn send_break(&mut self, duration: Duration) -> Result<()> {
        self.port
            .set_break()
            .map_err(|e| TransportError::Io(std::io::Error::other(e)))?;
        std::thread::sleep(duration);
        self.port
            .clear_break()
            .map_err(|e| TransportError::Io(std::io::Error::other(e)))?;
        Ok(())
    }

    fn set_baud(&mut self, baud: u32) -> Result<()> {
        self.port
            .set_baud_rate(baud)
            .map_err(|e| TransportError::Io(std::io::Error::other(e)))
    }

    fn flush_input(&mut self) -> Result<()> {
        self.port
            .clear(serialport::ClearBuffer::Input)
            .map_err(|e| TransportError::Io(std::io::Error::other(e)))
    }
}
