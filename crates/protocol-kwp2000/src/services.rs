//! KWP2000 service identifiers and response classification.

/// Service IDs (requests). A positive response echoes the SID + 0x40.
pub mod sid {
    pub const START_COMMUNICATION: u8 = 0x81;
    pub const STOP_COMMUNICATION: u8 = 0x82;
    pub const START_DIAGNOSTIC_SESSION: u8 = 0x10;
    pub const ECU_RESET: u8 = 0x11;
    pub const CLEAR_DIAGNOSTIC_INFORMATION: u8 = 0x14;
    pub const READ_DTC_BY_STATUS: u8 = 0x18;
    pub const READ_ECU_IDENTIFICATION: u8 = 0x1A;
    pub const READ_DATA_BY_LOCAL_ID: u8 = 0x21;
    pub const INPUT_OUTPUT_CONTROL_BY_LOCAL_ID: u8 = 0x30;
    pub const START_ROUTINE_BY_LOCAL_ID: u8 = 0x31;
    pub const STOP_ROUTINE_BY_LOCAL_ID: u8 = 0x32;
    pub const REQUEST_ROUTINE_RESULTS_BY_LOCAL_ID: u8 = 0x33;
    pub const TESTER_PRESENT: u8 = 0x3E;

    pub const NEGATIVE_RESPONSE: u8 = 0x7F;

    /// Deliberately absent: memory/download services (0x23 ReadMemoryByAddress,
    /// 0x34/0x35/0x36 transfer, 0x3D WriteMemoryByAddress). Map read/write is
    /// out of scope for v1 by design — see docs/SAFETY.md.
    pub fn positive_response(request_sid: u8) -> u8 {
        request_sid | 0x40
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NegativeResponseCode {
    GeneralReject,
    ServiceNotSupported,
    SubFunctionNotSupported,
    BusyRepeatRequest,
    ConditionsNotCorrect,
    RoutineNotComplete,
    RequestOutOfRange,
    SecurityAccessDenied,
    /// ECU needs more time; the tester must keep waiting (P2* window).
    ResponsePending,
    Other(u8),
}

impl From<u8> for NegativeResponseCode {
    fn from(code: u8) -> Self {
        use NegativeResponseCode::*;
        match code {
            0x10 => GeneralReject,
            0x11 => ServiceNotSupported,
            0x12 => SubFunctionNotSupported,
            0x21 => BusyRepeatRequest,
            0x22 => ConditionsNotCorrect,
            0x23 => RoutineNotComplete,
            0x31 => RequestOutOfRange,
            0x33 => SecurityAccessDenied,
            0x78 => ResponsePending,
            other => Other(other),
        }
    }
}

impl NegativeResponseCode {
    pub fn as_u8(self) -> u8 {
        use NegativeResponseCode::*;
        match self {
            GeneralReject => 0x10,
            ServiceNotSupported => 0x11,
            SubFunctionNotSupported => 0x12,
            BusyRepeatRequest => 0x21,
            ConditionsNotCorrect => 0x22,
            RoutineNotComplete => 0x23,
            RequestOutOfRange => 0x31,
            SecurityAccessDenied => 0x33,
            ResponsePending => 0x78,
            Other(code) => code,
        }
    }
}

impl std::fmt::Display for NegativeResponseCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use NegativeResponseCode::*;
        let text = match self {
            GeneralReject => "general reject",
            ServiceNotSupported => "service not supported",
            SubFunctionNotSupported => "sub-function not supported",
            BusyRepeatRequest => "busy, repeat request",
            ConditionsNotCorrect => "conditions not correct",
            RoutineNotComplete => "routine not complete",
            RequestOutOfRange => "request out of range",
            SecurityAccessDenied => "security access denied",
            ResponsePending => "response pending",
            Other(code) => return write!(f, "code 0x{code:02X}"),
        };
        write!(f, "{text} (0x{:02X})", self.as_u8())
    }
}
