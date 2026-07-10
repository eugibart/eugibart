//! ISO 14230 (KWP2000) protocol implementation.
//!
//! This crate is pure protocol: it knows nothing about specific bikes. Which
//! init method an ECU wants, its addresses, timing overrides, and what its
//! data means all come from `motodiag-ecu-defs` definitions at runtime.

pub mod framing;
pub mod init;
pub mod services;
pub mod session;
pub mod timing;

use motodiag_transport::TransportError;

#[derive(Debug, thiserror::Error)]
pub enum KwpError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error("bad checksum: computed {computed:02X}, received {received:02X}")]
    BadChecksum { computed: u8, received: u8 },
    #[error("malformed frame: {0}")]
    MalformedFrame(String),
    #[error("negative response to service {service:02X}: {code}")]
    NegativeResponse {
        service: u8,
        code: services::NegativeResponseCode,
    },
    #[error("unexpected response: expected SID {expected:02X}, got {got:02X}")]
    UnexpectedService { expected: u8, got: u8 },
    #[error("initialization failed: {0}")]
    InitFailed(String),
}

pub type Result<T> = std::result::Result<T, KwpError>;
