//! CAN bus transport abstraction, plus an in-memory mock bus for tests and
//! the CAN ECU simulator — mirrors `motodiag_transport::KLineTransport`
//! and `motodiag_transport::mock` for the K-line side.

use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use crate::frame::CanFrame;

#[derive(Debug, thiserror::Error)]
pub enum CanError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("timed out waiting for a CAN frame")]
    Timeout,
    #[error("bus closed")]
    Closed,
}

pub type Result<T> = std::result::Result<T, CanError>;

pub trait CanTransport: Send {
    fn send(&mut self, frame: &CanFrame) -> Result<()>;
    fn recv(&mut self, timeout: Duration) -> Result<CanFrame>;
}

/// Tester-side endpoint of an in-memory CAN bus. Every frame sent is visible
/// to the paired [`CanBusEcuEnd`] and vice versa — a real bus is broadcast,
/// but for a two-party tester/ECU simulation a simple duplex pair is enough.
pub struct MockCanBus {
    tx: Sender<CanFrame>,
    rx: Receiver<CanFrame>,
}

pub struct CanBusEcuEnd {
    tx: Sender<CanFrame>,
    rx: Receiver<CanFrame>,
}

pub fn pair() -> (MockCanBus, CanBusEcuEnd) {
    let (t2e_tx, t2e_rx) = channel();
    let (e2t_tx, e2t_rx) = channel();
    (
        MockCanBus {
            tx: t2e_tx,
            rx: e2t_rx,
        },
        CanBusEcuEnd {
            tx: e2t_tx,
            rx: t2e_rx,
        },
    )
}

impl CanTransport for MockCanBus {
    fn send(&mut self, frame: &CanFrame) -> Result<()> {
        self.tx.send(frame.clone()).map_err(|_| CanError::Closed)
    }

    fn recv(&mut self, timeout: Duration) -> Result<CanFrame> {
        self.rx.recv_timeout(timeout).map_err(|e| match e {
            RecvTimeoutError::Timeout => CanError::Timeout,
            RecvTimeoutError::Disconnected => CanError::Closed,
        })
    }
}

impl CanTransport for CanBusEcuEnd {
    fn send(&mut self, frame: &CanFrame) -> Result<()> {
        self.tx.send(frame.clone()).map_err(|_| CanError::Closed)
    }

    fn recv(&mut self, timeout: Duration) -> Result<CanFrame> {
        self.rx.recv_timeout(timeout).map_err(|e| match e {
            RecvTimeoutError::Timeout => CanError::Timeout,
            RecvTimeoutError::Disconnected => CanError::Closed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_flow_both_ways() {
        let (mut tester, mut ecu) = pair();
        tester
            .send(&CanFrame::new(0x7E0, vec![0x02, 0x10, 0x01]))
            .unwrap();
        let received = ecu.recv(Duration::from_millis(50)).unwrap();
        assert_eq!(received.id, 0x7E0);

        ecu.send(&CanFrame::new(0x7E8, vec![0x02, 0x50, 0x01]))
            .unwrap();
        let received = tester.recv(Duration::from_millis(50)).unwrap();
        assert_eq!(received.id, 0x7E8);
    }

    #[test]
    fn recv_times_out_when_idle() {
        let (mut tester, _ecu) = pair();
        assert!(matches!(
            tester.recv(Duration::from_millis(5)),
            Err(CanError::Timeout)
        ));
    }
}
