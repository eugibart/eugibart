//! Live sensor readings.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Reading {
    pub key: String,
    pub name: String,
    pub unit: String,
    pub value: f64,
    /// Milliseconds since the Unix epoch, for logging and charting.
    pub timestamp_ms: u64,
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
