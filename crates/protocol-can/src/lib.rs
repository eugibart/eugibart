//! CAN/UDS protocol groundwork for MotoDiag (milestone M5).
//!
//! This crate is the CAN-side counterpart to `motodiag-kwp2000` +
//! `motodiag-transport`: a transport abstraction, a segmentation layer
//! (ISO-TP), and a service layer (UDS/ISO 14229), each tested independently
//! via an in-memory mock bus — the same shape the K-line stack had before
//! `app-core` wired it up. That wiring (a `CanDiagSession` mirroring
//! `DiagSession`, plus real CAN hardware transports) is intentionally the
//! next step, not part of this crate.

pub mod elm327;
pub mod frame;
pub mod iso_tp;
pub mod session;
pub mod slcan;
pub mod transport;
pub mod uds;
