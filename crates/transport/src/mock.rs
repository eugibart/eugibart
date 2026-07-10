//! In-memory duplex link used by tests and the ECU simulator.
//!
//! The link carries [`LineEvent`]s rather than raw bytes so that out-of-band
//! bus conditions — break pulses for fast/slow init, baud changes — are
//! visible to the simulated ECU exactly like they would be on a real wire.

use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use crate::{KLineTransport, Result, TransportError};

/// Something observable on the (simulated) K-line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineEvent {
    Byte(u8),
    /// Bus held dominant for the given duration.
    Break(Duration),
    BaudChange(u32),
    /// A resolved 5-baud slow-init address. Stands in for the real bit-banged
    /// waveform (see `KLineTransport::send_5baud_address`), which a passive
    /// listener can't reconstruct since a "1" bit is silent.
    FiveBaudAddress(u8),
}

/// Tester-side endpoint. Implements [`KLineTransport`].
pub struct MockKLine {
    tx: Sender<LineEvent>,
    rx: Receiver<LineEvent>,
}

/// ECU-side endpoint, used by the simulator to observe the bus and reply.
pub struct EcuLink {
    tx: Sender<LineEvent>,
    rx: Receiver<LineEvent>,
}

/// Create a connected (tester, ecu) endpoint pair.
pub fn pair() -> (MockKLine, EcuLink) {
    let (t2e_tx, t2e_rx) = channel();
    let (e2t_tx, e2t_rx) = channel();
    (
        MockKLine {
            tx: t2e_tx,
            rx: e2t_rx,
        },
        EcuLink {
            tx: e2t_tx,
            rx: t2e_rx,
        },
    )
}

impl KLineTransport for MockKLine {
    fn send(&mut self, bytes: &[u8]) -> Result<()> {
        for &b in bytes {
            self.tx
                .send(LineEvent::Byte(b))
                .map_err(|_| TransportError::Closed)?;
        }
        Ok(())
    }

    fn read_byte(&mut self, timeout: Duration) -> Result<u8> {
        // Skip non-byte events: the tester never needs to observe its own
        // break/baud events coming back.
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let remaining = deadline
                .checked_duration_since(std::time::Instant::now())
                .ok_or(TransportError::Timeout)?;
            match self.rx.recv_timeout(remaining) {
                Ok(LineEvent::Byte(b)) => return Ok(b),
                Ok(_) => continue,
                Err(RecvTimeoutError::Timeout) => return Err(TransportError::Timeout),
                Err(RecvTimeoutError::Disconnected) => return Err(TransportError::Closed),
            }
        }
    }

    fn send_break(&mut self, duration: Duration) -> Result<()> {
        self.tx
            .send(LineEvent::Break(duration))
            .map_err(|_| TransportError::Closed)
        // No real-time sleep: simulated time keeps tests instant.
    }

    fn set_baud(&mut self, baud: u32) -> Result<()> {
        self.tx
            .send(LineEvent::BaudChange(baud))
            .map_err(|_| TransportError::Closed)
    }

    fn flush_input(&mut self) -> Result<()> {
        while self.rx.try_recv().is_ok() {}
        Ok(())
    }

    fn send_5baud_address(&mut self, address: u8, _bit_time: Duration) -> Result<()> {
        // No real waveform to simulate: hand the ECU-side listener the
        // resolved address directly (see `LineEvent::FiveBaudAddress`).
        self.tx
            .send(LineEvent::FiveBaudAddress(address))
            .map_err(|_| TransportError::Closed)
    }
}

impl EcuLink {
    /// Wait up to `timeout` for the next bus event.
    pub fn recv_event(&self, timeout: Duration) -> Result<LineEvent> {
        match self.rx.recv_timeout(timeout) {
            Ok(ev) => Ok(ev),
            Err(RecvTimeoutError::Timeout) => Err(TransportError::Timeout),
            Err(RecvTimeoutError::Disconnected) => Err(TransportError::Closed),
        }
    }

    /// Transmit bytes back to the tester.
    pub fn send_bytes(&self, bytes: &[u8]) -> Result<()> {
        for &b in bytes {
            self.tx
                .send(LineEvent::Byte(b))
                .map_err(|_| TransportError::Closed)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_bytes_and_break() {
        let (mut tester, ecu) = pair();
        tester.send_break(Duration::from_millis(25)).unwrap();
        tester.send(&[0x81, 0x10]).unwrap();

        assert_eq!(
            ecu.recv_event(Duration::from_millis(10)).unwrap(),
            LineEvent::Break(Duration::from_millis(25))
        );
        assert_eq!(
            ecu.recv_event(Duration::from_millis(10)).unwrap(),
            LineEvent::Byte(0x81)
        );
        assert_eq!(
            ecu.recv_event(Duration::from_millis(10)).unwrap(),
            LineEvent::Byte(0x10)
        );

        ecu.send_bytes(&[0xC1]).unwrap();
        assert_eq!(tester.read_byte(Duration::from_millis(10)).unwrap(), 0xC1);
    }

    #[test]
    fn read_times_out() {
        let (mut tester, _ecu) = pair();
        assert!(matches!(
            tester.read_byte(Duration::from_millis(5)),
            Err(TransportError::Timeout)
        ));
    }
}
