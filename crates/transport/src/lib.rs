//! Physical-layer transports for MotoDiag.
//!
//! Everything above this crate speaks in bytes and line events; everything
//! below is a real wire (FTDI K-line cable), an in-memory link for tests and
//! the ECU simulator, or — later — a CAN interface.
//!
//! K-line is a single-wire, half-duplex bus: every byte we transmit is also
//! echoed back at the receiver. Echo consumption is a *transport* concern —
//! backends that talk to a real wire discard the echo before handing bytes to
//! the protocol layer, so protocol code never sees its own transmissions.

pub mod mock;
#[cfg(feature = "vcp")]
pub mod serial_vcp;

use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("timed out waiting for data")]
    Timeout,
    #[error("bus echo mismatch: sent {sent:02X?}, read back {echoed:02X?}")]
    EchoMismatch { sent: Vec<u8>, echoed: Vec<u8> },
    #[error("link closed")]
    Closed,
    #[error("operation not supported by this transport: {0}")]
    Unsupported(&'static str),
}

pub type Result<T> = std::result::Result<T, TransportError>;

/// A K-line-capable byte transport.
///
/// Implementations must be `Send` so a session can run on a worker thread.
pub trait KLineTransport: Send {
    /// Transmit bytes. On real K-line hardware this also consumes and verifies
    /// the bus echo, so on return the receive buffer contains only ECU data.
    fn send(&mut self, bytes: &[u8]) -> Result<()>;

    /// Read a single byte, waiting at most `timeout`.
    fn read_byte(&mut self, timeout: Duration) -> Result<u8>;

    /// Hold the bus dominant (low) for `duration`, then release it.
    /// Used by KWP2000 fast init (25 ms) and 5-baud slow init (~200 ms/bit).
    fn send_break(&mut self, duration: Duration) -> Result<()>;

    /// Change line speed (5-baud init negotiates the working baud rate).
    fn set_baud(&mut self, baud: u32) -> Result<()>;

    /// Discard anything already buffered on the receive side.
    fn flush_input(&mut self) -> Result<()>;

    /// Read exactly `n` bytes. `first_byte_timeout` bounds the wait for the
    /// first byte; `inter_byte_timeout` (KWP P1) bounds each subsequent gap.
    fn read_exact(
        &mut self,
        n: usize,
        first_byte_timeout: Duration,
        inter_byte_timeout: Duration,
    ) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let timeout = if i == 0 {
                first_byte_timeout
            } else {
                inter_byte_timeout
            };
            out.push(self.read_byte(timeout)?);
        }
        Ok(out)
    }
}
