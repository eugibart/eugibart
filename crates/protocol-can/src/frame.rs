//! Classic CAN 2.0 frames (CAN-FD's larger payloads are out of scope).

/// A single CAN frame: an 11-bit (standard) or 29-bit (extended) identifier
/// and up to 8 data bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanFrame {
    pub id: u32,
    pub extended: bool,
    pub data: Vec<u8>,
}

impl CanFrame {
    pub fn new(id: u32, data: Vec<u8>) -> Self {
        assert!(data.len() <= 8, "classic CAN frames carry at most 8 bytes");
        Self {
            id,
            extended: id > 0x7FF,
            data,
        }
    }
}
