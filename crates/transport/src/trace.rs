//! Byte-level wire tracing.
//!
//! Unlike `protocol_kwp2000::session`'s frame-level trace hook (which only
//! sees traffic once a `KwpSession` exists), [`TracingTransport`] wraps the
//! raw transport itself, so it captures the *entire* session including the
//! init handshake (break pulses, StartCommunication) — everything needed to
//! reconstruct or replay a session byte-for-byte.

use std::time::Duration;

use crate::{KLineTransport, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceDirection {
    Tx,
    Rx,
}

/// Bytes serialized as a human-readable hex string ("81 10 F1 81 03")
/// instead of a raw JSON number array, so fixtures stay inspectable.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HexBytes(pub Vec<u8>);

impl serde::Serialize for HexBytes {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let hex: Vec<String> = self.0.iter().map(|b| format!("{b:02X}")).collect();
        serializer.serialize_str(&hex.join(" "))
    }
}

impl<'de> serde::Deserialize<'de> for HexBytes {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let bytes = s
            .split_whitespace()
            .map(|tok| u8::from_str_radix(tok, 16).map_err(serde::de::Error::custom))
            .collect::<std::result::Result<Vec<u8>, D::Error>>()?;
        Ok(HexBytes(bytes))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventKind {
    Bytes { hex: HexBytes },
    Break { ms: u64 },
    Baud { baud: u32 },
    FiveBaudAddress { address: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TraceEvent {
    pub t_ms: u64,
    pub dir: TraceDirection,
    pub kind: EventKind,
}

/// Where traced events go. Implement this for a file writer, an in-memory
/// `Vec`, or anything else.
pub trait TraceSink: Send {
    fn record(&mut self, event: TraceEvent);
}

/// A sink that just accumulates events in memory — handy for tests.
#[derive(Default)]
pub struct VecSink(pub Vec<TraceEvent>);

impl TraceSink for VecSink {
    fn record(&mut self, event: TraceEvent) {
        self.0.push(event);
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Wraps any [`KLineTransport`], recording every send/receive/break/baud
/// event to a [`TraceSink`] before forwarding to the inner transport.
pub struct TracingTransport {
    inner: Box<dyn KLineTransport>,
    sink: Box<dyn TraceSink>,
}

impl TracingTransport {
    pub fn new(inner: Box<dyn KLineTransport>, sink: Box<dyn TraceSink>) -> Self {
        Self { inner, sink }
    }

    fn emit(&mut self, dir: TraceDirection, kind: EventKind) {
        self.sink.record(TraceEvent {
            t_ms: now_ms(),
            dir,
            kind,
        });
    }
}

impl KLineTransport for TracingTransport {
    fn send(&mut self, bytes: &[u8]) -> Result<()> {
        self.inner.send(bytes)?;
        // Recorded only after a successful send: a failed transmission never
        // reached the bus, so it shouldn't appear in the trace.
        self.emit(
            TraceDirection::Tx,
            EventKind::Bytes {
                hex: HexBytes(bytes.to_vec()),
            },
        );
        Ok(())
    }

    fn read_byte(&mut self, timeout: Duration) -> Result<u8> {
        let byte = self.inner.read_byte(timeout)?;
        self.emit(
            TraceDirection::Rx,
            EventKind::Bytes {
                hex: HexBytes(vec![byte]),
            },
        );
        Ok(byte)
    }

    fn send_break(&mut self, duration: Duration) -> Result<()> {
        self.inner.send_break(duration)?;
        self.emit(
            TraceDirection::Tx,
            EventKind::Break {
                ms: duration.as_millis() as u64,
            },
        );
        Ok(())
    }

    fn set_baud(&mut self, baud: u32) -> Result<()> {
        self.inner.set_baud(baud)?;
        self.emit(TraceDirection::Tx, EventKind::Baud { baud });
        Ok(())
    }

    fn flush_input(&mut self) -> Result<()> {
        // Not recorded: it discards unread bytes rather than transmitting or
        // receiving anything meaningful for a replay.
        self.inner.flush_input()
    }

    fn send_5baud_address(&mut self, address: u8, bit_time: Duration) -> Result<()> {
        self.inner.send_5baud_address(address, bit_time)?;
        self.emit(TraceDirection::Tx, EventKind::FiveBaudAddress { address });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct SharedSink(Arc<Mutex<Vec<TraceEvent>>>);
    impl TraceSink for SharedSink {
        fn record(&mut self, event: TraceEvent) {
            self.0.lock().unwrap().push(event);
        }
    }

    #[test]
    fn records_send_break_and_receive() {
        let (tester, ecu) = mock::pair();
        let sink = SharedSink::default();
        let mut traced = TracingTransport::new(Box::new(tester), Box::new(sink.clone()));

        traced.send_break(Duration::from_millis(25)).unwrap();
        traced.send(&[0x81, 0x10]).unwrap();
        ecu.send_bytes(&[0xC1]).unwrap();
        let byte = traced.read_byte(Duration::from_millis(50)).unwrap();
        assert_eq!(byte, 0xC1);

        let events = sink.0.lock().unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].dir, TraceDirection::Tx);
        assert_eq!(events[0].kind, EventKind::Break { ms: 25 });
        assert_eq!(events[1].dir, TraceDirection::Tx);
        assert_eq!(
            events[1].kind,
            EventKind::Bytes {
                hex: HexBytes(vec![0x81, 0x10])
            }
        );
        assert_eq!(events[2].dir, TraceDirection::Rx);
        assert_eq!(
            events[2].kind,
            EventKind::Bytes {
                hex: HexBytes(vec![0xC1])
            }
        );
    }

    #[test]
    fn failed_send_is_not_recorded() {
        let (tester, ecu) = mock::pair();
        drop(ecu); // closes the link
        let sink = SharedSink::default();
        let mut traced = TracingTransport::new(Box::new(tester), Box::new(sink.clone()));

        assert!(traced.send(&[0x00]).is_err());
        assert!(sink.0.lock().unwrap().is_empty());
    }

    #[test]
    fn event_roundtrips_through_json_as_readable_hex() {
        let event = TraceEvent {
            t_ms: 42,
            dir: TraceDirection::Tx,
            kind: EventKind::Bytes {
                hex: HexBytes(vec![0x81, 0x10, 0xF1]),
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"81 10 F1\""), "got: {json}");

        let parsed: TraceEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, event);
    }

    #[test]
    fn break_and_baud_events_roundtrip() {
        for kind in [EventKind::Break { ms: 25 }, EventKind::Baud { baud: 10400 }] {
            let event = TraceEvent {
                t_ms: 1,
                dir: TraceDirection::Tx,
                kind: kind.clone(),
            };
            let json = serde_json::to_string(&event).unwrap();
            let parsed: TraceEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, event);
        }
    }
}
