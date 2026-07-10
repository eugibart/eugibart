//! UDS (ISO 14229) service identifiers in scope for MotoDiag.
//!
//! Shares the 0x7F negative-response convention and several SIDs with
//! KWP2000, so [`NegativeResponseCode`] is reused from `motodiag-kwp2000`
//! rather than duplicated.

pub use motodiag_kwp2000::services::NegativeResponseCode;

/// Service IDs (requests). A positive response echoes the SID + 0x40, same
/// convention as KWP2000.
pub mod sid {
    pub const DIAGNOSTIC_SESSION_CONTROL: u8 = 0x10;
    pub const CLEAR_DIAGNOSTIC_INFORMATION: u8 = 0x14;
    pub const READ_DTC_INFORMATION: u8 = 0x19;
    pub const READ_DATA_BY_IDENTIFIER: u8 = 0x22;
    pub const INPUT_OUTPUT_CONTROL_BY_IDENTIFIER: u8 = 0x2F;
    pub const ROUTINE_CONTROL: u8 = 0x31;
    pub const TESTER_PRESENT: u8 = 0x3E;

    pub const NEGATIVE_RESPONSE: u8 = 0x7F;

    /// Deliberately absent: SecurityAccess (0x27), WriteDataByIdentifier
    /// (0x2E), and the memory/transfer services (0x23, 0x34-0x37, 0x3D) —
    /// same "no map read/write" posture as the K-line side. See
    /// docs/SAFETY.md.
    pub fn positive_response(request_sid: u8) -> u8 {
        request_sid | 0x40
    }
}

/// RoutineControl (0x31) sub-functions.
pub mod routine_control {
    pub const START: u8 = 0x01;
    pub const STOP: u8 = 0x02;
    pub const REQUEST_RESULTS: u8 = 0x03;
}
