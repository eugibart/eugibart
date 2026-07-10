//! ISO 14230-2 timing parameters (P1–P4).
//!
//! Defaults follow the standard's normal timing set; ECU definition files may
//! override any of them (Marelli units are generally tolerant, but USB-serial
//! latency means we prefer generous receive deadlines and standard-compliant
//! transmit gaps).

use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimingParams {
    /// P1 max: inter-byte time within an ECU response.
    pub p1_max: Duration,
    /// P2 min/max: time between end of tester request and start of ECU reply.
    pub p2_min: Duration,
    pub p2_max: Duration,
    /// P3 min/max: time between end of ECU reply and next tester request.
    /// `p3_max` is also the keep-alive budget: stay silent longer and the ECU
    /// drops the session.
    pub p3_min: Duration,
    pub p3_max: Duration,
    /// P4 min: inter-byte time within a tester request.
    pub p4_min: Duration,
}

impl Default for TimingParams {
    fn default() -> Self {
        Self {
            p1_max: Duration::from_millis(20),
            p2_min: Duration::from_millis(25),
            p2_max: Duration::from_millis(50),
            p3_min: Duration::from_millis(55),
            p3_max: Duration::from_millis(5000),
            p4_min: Duration::from_millis(5),
        }
    }
}

impl TimingParams {
    /// Deadline for the first byte of a response.
    ///
    /// P2 max is 50 ms on paper, but USB-serial adapters (especially Apple's
    /// FTDI driver with its ~16 ms latency timer) batch arrivals, so we pad
    /// the wait. Being lenient on receive never violates the standard.
    pub fn first_byte_deadline(&self) -> Duration {
        self.p2_max + Duration::from_millis(150)
    }

    /// Deadline for bytes within a response (P1 plus USB slack).
    pub fn inter_byte_deadline(&self) -> Duration {
        self.p1_max + Duration::from_millis(50)
    }

    /// Send a TesterPresent if the bus has been idle this long. Half of P3
    /// max leaves comfortable margin.
    pub fn keepalive_after(&self) -> Duration {
        self.p3_max / 2
    }
}
