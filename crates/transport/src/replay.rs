//! Replays a previously recorded [`trace::TraceEvent`] sequence as a
//! transport, so a captured session (real bike or simulator) can be re-run
//! deterministically as a regression test — no hardware, no timing, no
//! flakiness, but the exact same bytes the protocol layer saw originally.

use std::collections::VecDeque;
use std::time::Duration;

use crate::trace::{EventKind, TraceDirection, TraceEvent};
use crate::{KLineTransport, Result, TransportError};

pub struct ReplayTransport {
    events: VecDeque<TraceEvent>,
}

impl ReplayTransport {
    pub fn from_events(events: Vec<TraceEvent>) -> Self {
        Self {
            events: events.into(),
        }
    }

    /// Pop the next event, or `None` if the recording is exhausted.
    fn next_event(&mut self) -> Option<TraceEvent> {
        self.events.pop_front()
    }

    fn mismatch(what: &str, event: Option<&TraceEvent>) -> TransportError {
        TransportError::Io(std::io::Error::other(format!(
            "replay mismatch: expected {what}, but recording had {event:?}"
        )))
    }
}

impl KLineTransport for ReplayTransport {
    fn send(&mut self, bytes: &[u8]) -> Result<()> {
        match self.next_event() {
            Some(TraceEvent {
                dir: TraceDirection::Tx,
                kind: EventKind::Bytes { hex },
                ..
            }) if hex.0 == bytes => Ok(()),
            other => Err(Self::mismatch(&format!("tx {bytes:02X?}"), other.as_ref())),
        }
    }

    fn read_byte(&mut self, _timeout: Duration) -> Result<u8> {
        match self.next_event() {
            Some(TraceEvent {
                dir: TraceDirection::Rx,
                kind: EventKind::Bytes { hex },
                ..
            }) if hex.0.len() == 1 => Ok(hex.0[0]),
            None => Err(TransportError::Timeout),
            other => Err(Self::mismatch("a single rx byte", other.as_ref())),
        }
    }

    fn send_break(&mut self, _duration: Duration) -> Result<()> {
        match self.next_event() {
            Some(TraceEvent {
                dir: TraceDirection::Tx,
                kind: EventKind::Break { .. },
                ..
            }) => Ok(()),
            other => Err(Self::mismatch("a break pulse", other.as_ref())),
        }
    }

    fn set_baud(&mut self, _baud: u32) -> Result<()> {
        match self.next_event() {
            Some(TraceEvent {
                dir: TraceDirection::Tx,
                kind: EventKind::Baud { .. },
                ..
            }) => Ok(()),
            other => Err(Self::mismatch("a baud change", other.as_ref())),
        }
    }

    fn flush_input(&mut self) -> Result<()> {
        // Not part of the recorded trace (see trace.rs); nothing to replay.
        Ok(())
    }

    fn send_5baud_address(&mut self, address: u8, _bit_time: Duration) -> Result<()> {
        match self.next_event() {
            Some(TraceEvent {
                dir: TraceDirection::Tx,
                kind: EventKind::FiveBaudAddress { address: recorded },
                ..
            }) if recorded == address => Ok(()),
            other => Err(Self::mismatch(
                &format!("5-baud address 0x{address:02X}"),
                other.as_ref(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::HexBytes;

    fn bytes_event(dir: TraceDirection, bytes: &[u8]) -> TraceEvent {
        TraceEvent {
            t_ms: 0,
            dir,
            kind: EventKind::Bytes {
                hex: HexBytes(bytes.to_vec()),
            },
        }
    }

    #[test]
    fn replays_recorded_exchange() {
        let events = vec![
            TraceEvent {
                t_ms: 0,
                dir: TraceDirection::Tx,
                kind: EventKind::Break { ms: 25 },
            },
            bytes_event(TraceDirection::Tx, &[0x81, 0x10, 0xF1, 0x81, 0x03]),
            bytes_event(TraceDirection::Rx, &[0xC1]),
            bytes_event(TraceDirection::Rx, &[0xEA]),
        ];
        let mut replay = ReplayTransport::from_events(events);

        replay.send_break(Duration::from_millis(25)).unwrap();
        replay.send(&[0x81, 0x10, 0xF1, 0x81, 0x03]).unwrap();
        assert_eq!(replay.read_byte(Duration::from_millis(10)).unwrap(), 0xC1);
        assert_eq!(replay.read_byte(Duration::from_millis(10)).unwrap(), 0xEA);
    }

    #[test]
    fn mismatched_send_is_an_error() {
        let events = vec![bytes_event(TraceDirection::Tx, &[0x81])];
        let mut replay = ReplayTransport::from_events(events);
        assert!(replay.send(&[0x99]).is_err());
    }

    #[test]
    fn replays_5baud_address_exchange() {
        let events = vec![
            TraceEvent {
                t_ms: 0,
                dir: TraceDirection::Tx,
                kind: EventKind::FiveBaudAddress { address: 0x10 },
            },
            bytes_event(TraceDirection::Rx, &[0x55]),
        ];
        let mut replay = ReplayTransport::from_events(events);
        replay
            .send_5baud_address(0x10, Duration::from_millis(1))
            .unwrap();
        assert_eq!(replay.read_byte(Duration::from_millis(10)).unwrap(), 0x55);
    }

    #[test]
    fn mismatched_5baud_address_is_an_error() {
        let events = vec![TraceEvent {
            t_ms: 0,
            dir: TraceDirection::Tx,
            kind: EventKind::FiveBaudAddress { address: 0x10 },
        }];
        let mut replay = ReplayTransport::from_events(events);
        assert!(replay
            .send_5baud_address(0x55, Duration::from_millis(1))
            .is_err());
    }

    #[test]
    fn exhausted_recording_times_out_on_read() {
        let mut replay = ReplayTransport::from_events(vec![]);
        assert!(matches!(
            replay.read_byte(Duration::from_millis(1)),
            Err(TransportError::Timeout)
        ));
    }
}
