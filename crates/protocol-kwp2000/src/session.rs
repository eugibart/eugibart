//! KWP2000 session: request/response exchange, P-timing enforcement,
//! ResponsePending handling, tester-present keep-alive, and wire tracing.

use std::time::Instant;

use motodiag_transport::KLineTransport;

use crate::framing::{read_frame, FrameCodec};
use crate::services::{sid, NegativeResponseCode};
use crate::timing::TimingParams;
use crate::{KwpError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Sent,
    Received,
}

/// Observes every raw frame on the wire; app-core plugs the trace logger in
/// here so captures become replayable fixtures.
pub type TraceHook = Box<dyn FnMut(Direction, &[u8]) + Send>;

pub struct KwpSession {
    transport: Box<dyn KLineTransport>,
    codec: FrameCodec,
    timing: TimingParams,
    last_exchange: Instant,
    trace: Option<TraceHook>,
    /// How many ResponsePending (0x78) waits to tolerate per request.
    pub max_response_pending: u32,
}

impl KwpSession {
    pub fn new(
        transport: Box<dyn KLineTransport>,
        codec: FrameCodec,
        timing: TimingParams,
    ) -> Self {
        Self {
            transport,
            codec,
            timing,
            last_exchange: Instant::now(),
            trace: None,
            max_response_pending: 10,
        }
    }

    pub fn set_trace_hook(&mut self, hook: TraceHook) {
        self.trace = Some(hook);
    }

    pub fn transport_mut(&mut self) -> &mut dyn KLineTransport {
        self.transport.as_mut()
    }

    pub fn codec(&self) -> &FrameCodec {
        &self.codec
    }

    pub fn timing(&self) -> &TimingParams {
        &self.timing
    }

    /// Send a request payload (starting with the SID) and return the positive
    /// response payload (starting with SID+0x40).
    pub fn request(&mut self, payload: &[u8]) -> Result<Vec<u8>> {
        let request_sid = *payload
            .first()
            .ok_or_else(|| KwpError::MalformedFrame("empty request payload".into()))?;

        // Honor P3 min: don't transmit too soon after the previous exchange.
        let since_last = self.last_exchange.elapsed();
        if since_last < self.timing.p3_min {
            std::thread::sleep(self.timing.p3_min - since_last);
        }

        let frame = self.codec.encode(payload);
        if let Some(trace) = &mut self.trace {
            trace(Direction::Sent, &frame);
        }
        self.transport.send(&frame)?;

        let mut pending_waits = 0;
        loop {
            let response = read_frame(
                self.transport.as_mut(),
                self.timing.first_byte_deadline(),
                self.timing.inter_byte_deadline(),
            )?;
            if let Some(trace) = &mut self.trace {
                trace(Direction::Received, &response.raw);
            }
            self.last_exchange = Instant::now();

            match response.payload.as_slice() {
                [nr, _svc, code, ..] if *nr == sid::NEGATIVE_RESPONSE => {
                    let code = NegativeResponseCode::from(*code);
                    if code == NegativeResponseCode::ResponsePending {
                        pending_waits += 1;
                        if pending_waits > self.max_response_pending {
                            return Err(KwpError::NegativeResponse {
                                service: request_sid,
                                code,
                            });
                        }
                        continue; // ECU is working; wait for the real answer.
                    }
                    return Err(KwpError::NegativeResponse {
                        service: request_sid,
                        code,
                    });
                }
                [first, ..] if *first == sid::positive_response(request_sid) => {
                    return Ok(response.payload);
                }
                [first, ..] => {
                    return Err(KwpError::UnexpectedService {
                        expected: sid::positive_response(request_sid),
                        got: *first,
                    });
                }
                [] => return Err(KwpError::MalformedFrame("empty response payload".into())),
            }
        }
    }

    pub fn tester_present(&mut self) -> Result<()> {
        self.request(&[sid::TESTER_PRESENT]).map(|_| ())
    }

    /// True when the bus has been idle long enough that a keep-alive is due.
    pub fn keepalive_due(&self) -> bool {
        self.last_exchange.elapsed() >= self.timing.keepalive_after()
    }
}
