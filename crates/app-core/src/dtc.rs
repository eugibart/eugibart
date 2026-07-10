//! Diagnostic trouble code parsing.

use motodiag_ecu_defs::EcuDefinition;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Dtc {
    pub code: u16,
    pub status: u8,
    /// Human-readable meaning from the ECU definition's table, if known.
    pub description: Option<String>,
    /// Likely culprits from the definition's DTC table, most common first.
    pub causes: Vec<String>,
    /// What to physically check, in order.
    pub checks: Vec<String>,
}

impl Dtc {
    pub fn code_string(&self) -> String {
        format!("{:04X}", self.code)
    }
}

/// Parse a ReadDTCByStatus positive response payload:
/// `[SID+0x40, count, (code_hi, code_lo, status)*]`.
pub fn parse_read_dtc_response(payload: &[u8], def: &EcuDefinition) -> Vec<Dtc> {
    let Some(rest) = payload.get(2..) else {
        return Vec::new();
    };
    rest.chunks_exact(3)
        .map(|chunk| {
            let code = u16::from_be_bytes([chunk[0], chunk[1]]);
            let entry = def.dtc.table.iter().find(|e| e.code == code);
            Dtc {
                code,
                status: chunk[2],
                description: entry.map(|e| e.description.clone()),
                causes: entry.map(|e| e.causes.clone()).unwrap_or_default(),
                checks: entry.map(|e| e.checks.clone()).unwrap_or_default(),
            }
        })
        .collect()
}
