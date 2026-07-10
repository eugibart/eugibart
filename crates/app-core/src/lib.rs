//! Session orchestration for MotoDiag.
//!
//! This crate ties the layers together: it takes a transport and an ECU
//! definition, runs the right init, and exposes the operations the UI needs —
//! always through the safety interlocks in [`safety`].

pub mod discovery;
pub mod dtc;
pub mod live_data;
pub mod logging;
pub mod safety;
pub mod session;

pub use session::{ConnectOptions, DiagSession, EcuIdentity};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Kwp(#[from] motodiag_kwp2000::KwpError),
    #[error(transparent)]
    Defs(#[from] motodiag_ecu_defs::DefsError),
    #[error("blocked by safety interlock: {0}")]
    Safety(#[from] safety::SafetyViolation),
    #[error("unknown channel '{0}' for this ECU")]
    UnknownChannel(String),
    #[error("unknown routine '{0}' for this ECU")]
    UnknownRoutine(String),
    #[error("'{0}' is a CAN-bus ECU definition; DiagSession only supports K-line/KWP2000 so far")]
    NotKLine(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;
