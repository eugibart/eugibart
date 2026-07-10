//! UDS session: request/response exchange over ISO-TP, with the same
//! ResponsePending-tolerant retry loop as `motodiag_kwp2000::session::KwpSession`.

use std::time::{Duration, Instant};

use crate::iso_tp;
use crate::transport::CanTransport;
use crate::uds::{sid, NegativeResponseCode};

#[derive(Debug, thiserror::Error)]
pub enum UdsError {
    #[error(transparent)]
    IsoTp(#[from] iso_tp::IsoTpError),
    #[error("negative response to service {service:02X}: {code}")]
    NegativeResponse {
        service: u8,
        code: NegativeResponseCode,
    },
    #[error("unexpected response: expected SID {expected:02X}, got {got:02X}")]
    UnexpectedService { expected: u8, got: u8 },
    #[error("empty response payload")]
    EmptyResponse,
}

pub type Result<T> = std::result::Result<T, UdsError>;

pub struct UdsSession {
    transport: Box<dyn CanTransport>,
    tx_id: u32,
    rx_id: u32,
    timeout: Duration,
    last_exchange: Instant,
    /// Send TesterPresent when idle longer than this.
    pub keepalive_after: Duration,
    pub max_response_pending: u32,
}

impl UdsSession {
    pub fn new(transport: Box<dyn CanTransport>, tx_id: u32, rx_id: u32) -> Self {
        Self {
            transport,
            tx_id,
            rx_id,
            timeout: Duration::from_millis(200),
            last_exchange: Instant::now(),
            keepalive_after: Duration::from_secs(2),
            max_response_pending: 10,
        }
    }

    pub fn request(&mut self, payload: &[u8]) -> Result<Vec<u8>> {
        let request_sid = *payload.first().ok_or(UdsError::EmptyResponse)?;

        iso_tp::send(
            self.transport.as_mut(),
            self.tx_id,
            self.rx_id,
            payload,
            self.timeout,
        )?;

        let mut pending_waits = 0;
        loop {
            let response = iso_tp::receive(
                self.transport.as_mut(),
                self.tx_id,
                self.rx_id,
                self.timeout,
            )?;
            self.last_exchange = Instant::now();

            match response.as_slice() {
                [nr, _svc, code, ..] if *nr == sid::NEGATIVE_RESPONSE => {
                    let code = NegativeResponseCode::from(*code);
                    if code == NegativeResponseCode::ResponsePending {
                        pending_waits += 1;
                        if pending_waits > self.max_response_pending {
                            return Err(UdsError::NegativeResponse {
                                service: request_sid,
                                code,
                            });
                        }
                        continue;
                    }
                    return Err(UdsError::NegativeResponse {
                        service: request_sid,
                        code,
                    });
                }
                [first, ..] if *first == sid::positive_response(request_sid) => {
                    return Ok(response);
                }
                [first, ..] => {
                    return Err(UdsError::UnexpectedService {
                        expected: sid::positive_response(request_sid),
                        got: *first,
                    });
                }
                [] => return Err(UdsError::EmptyResponse),
            }
        }
    }

    pub fn tester_present(&mut self) -> Result<()> {
        self.request(&[sid::TESTER_PRESENT]).map(|_| ())
    }

    pub fn keepalive_due(&self) -> bool {
        self.last_exchange.elapsed() >= self.keepalive_after
    }
}
