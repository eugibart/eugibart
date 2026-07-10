//! Serde schema for ECU definition TOML files.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EcuDefinition {
    pub ecu: EcuInfo,
    pub init: InitConfig,
    #[serde(default)]
    pub timing: TimingOverrides,
    #[serde(default)]
    pub identification: Identification,
    #[serde(default)]
    pub channels: Vec<Channel>,
    #[serde(default)]
    pub dtc: DtcConfig,
    #[serde(default)]
    pub routines: Vec<Routine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EcuInfo {
    /// Stable identifier, e.g. "mv-5sm-brutale-910".
    pub id: String,
    /// Human-readable name shown in the UI.
    pub name: String,
    pub manufacturer: String,
    /// Bike models this definition applies to.
    #[serde(default)]
    pub models: Vec<String>,
    pub bus: BusKind,
    /// True once the definition has been exercised against a real ECU.
    /// Unverified definitions are shown with a warning in the UI.
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BusKind {
    KLine,
    Can,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitConfig {
    pub method: InitMethod,
    /// KWP physical address of the ECU (header target byte).
    pub ecu_address: u8,
    /// Tester address (header source byte), conventionally 0xF1.
    #[serde(default = "default_tester_address")]
    pub tester_address: u8,
    pub baud: u32,
    /// Some ECUs require the length in a separate byte instead of packed
    /// into the format byte.
    #[serde(default)]
    pub separate_length_byte: bool,
}

fn default_tester_address() -> u8 {
    0xF1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitMethod {
    #[serde(rename = "fast")]
    Fast,
    #[serde(rename = "slow-5-baud")]
    Slow5Baud,
}

/// Timing overrides in milliseconds; anything unset uses ISO defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimingOverrides {
    pub p1_max_ms: Option<u64>,
    pub p2_min_ms: Option<u64>,
    pub p2_max_ms: Option<u64>,
    pub p3_min_ms: Option<u64>,
    pub p3_max_ms: Option<u64>,
    pub p4_min_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identification {
    /// ReadEcuIdentification request payload.
    pub request: Vec<u8>,
    /// Byte offset into the positive response payload where the ident
    /// text/data begins (skips SID and option bytes).
    #[serde(default = "default_ident_skip")]
    pub skip_bytes: usize,
}

fn default_ident_skip() -> usize {
    2
}

impl Default for Identification {
    fn default() -> Self {
        Self {
            // 0x1A ReadEcuIdentification, option 0x80 = "all of it".
            request: vec![0x1A, 0x80],
            skip_bytes: default_ident_skip(),
        }
    }
}

/// One live-data value: how to request it and how to decode the response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub unit: String,
    /// Request payload, e.g. `[0x21, 0x01]` (ReadDataByLocalIdentifier).
    pub request: Vec<u8>,
    /// Byte offset into the positive response payload (after the response
    /// SID and echoed identifier).
    pub offset: usize,
    /// 1, 2 or 4 bytes, big-endian.
    pub length: usize,
    #[serde(default = "default_scale")]
    pub scale: f64,
    /// Added after scaling: value = raw * scale + offset_value.
    #[serde(default)]
    pub offset_value: f64,
    #[serde(default)]
    pub signed: bool,
    #[serde(default)]
    pub verified: bool,
}

fn default_scale() -> f64 {
    1.0
}

impl Channel {
    /// Decode this channel from a positive response payload.
    pub fn decode(&self, payload: &[u8]) -> Option<f64> {
        let bytes = payload.get(self.offset..self.offset + self.length)?;
        let mut raw: i64 = 0;
        for &b in bytes {
            raw = (raw << 8) | i64::from(b);
        }
        if self.signed {
            let bits = (self.length * 8) as u32;
            let sign_bit = 1i64 << (bits - 1);
            if raw & sign_bit != 0 {
                raw -= 1i64 << bits;
            }
        }
        Some(raw as f64 * self.scale + self.offset_value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DtcConfig {
    /// ReadDTCByStatus request payload.
    pub read_request: Vec<u8>,
    /// ClearDiagnosticInformation request payload.
    pub clear_request: Vec<u8>,
    /// Known code meanings for this ECU.
    #[serde(default)]
    pub table: Vec<DtcEntry>,
}

impl Default for DtcConfig {
    fn default() -> Self {
        Self {
            // 0x18 ReadDTCByStatus: all identified DTCs, whole group.
            read_request: vec![0x18, 0x02, 0xFF, 0x00],
            // 0x14 ClearDiagnosticInformation: whole group.
            clear_request: vec![0x14, 0xFF, 0x00],
            table: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DtcEntry {
    pub code: u16,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Routine {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub risk: RiskLevel,
    /// Request payload sent when the routine runs (e.g. StartRoutineByLocalId
    /// or InputOutputControlByLocalId).
    pub request: Vec<u8>,
    #[serde(default)]
    pub preconditions: Preconditions,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RiskLevel {
    /// Reads or momentary actuator tests.
    Low,
    /// Persists state on the ECU (adaptation resets, trims).
    Medium,
    /// Could leave the bike unrideable if interrupted.
    High,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preconditions {
    #[serde(default)]
    pub engine_must_be_off: bool,
    pub min_battery_v: Option<f64>,
    pub max_battery_v: Option<f64>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Service IDs that definitions may use. Memory/flash services are absent on
/// purpose: a definition file must never be able to reach map read/write.
const ALLOWED_SIDS: &[u8] = &[
    0x10, // StartDiagnosticSession
    0x14, // ClearDiagnosticInformation
    0x18, // ReadDTCByStatus
    0x1A, // ReadEcuIdentification
    0x21, // ReadDataByLocalIdentifier
    0x30, // InputOutputControlByLocalIdentifier
    0x31, // StartRoutineByLocalIdentifier
    0x32, // StopRoutineByLocalIdentifier
    0x33, // RequestRoutineResultsByLocalIdentifier
    0x3E, // TesterPresent
];

impl EcuDefinition {
    pub fn validate(&self) -> Result<(), String> {
        let mut seen = std::collections::HashSet::new();
        for ch in &self.channels {
            if !seen.insert(&ch.key) {
                return Err(format!("duplicate channel key '{}'", ch.key));
            }
            if ch.request.is_empty() {
                return Err(format!("channel '{}' has an empty request", ch.key));
            }
            if !matches!(ch.length, 1 | 2 | 4) {
                return Err(format!(
                    "channel '{}' length must be 1, 2 or 4 bytes",
                    ch.key
                ));
            }
        }

        let mut seen = std::collections::HashSet::new();
        for r in &self.routines {
            if !seen.insert(&r.key) {
                return Err(format!("duplicate routine key '{}'", r.key));
            }
            if r.request.is_empty() {
                return Err(format!("routine '{}' has an empty request", r.key));
            }
        }

        let requests = self
            .channels
            .iter()
            .map(|c| (&c.request, c.key.as_str()))
            .chain(self.routines.iter().map(|r| (&r.request, r.key.as_str())))
            .chain([
                (&self.identification.request, "identification"),
                (&self.dtc.read_request, "dtc.read_request"),
                (&self.dtc.clear_request, "dtc.clear_request"),
            ]);
        for (request, what) in requests {
            match request.first() {
                Some(sid) if ALLOWED_SIDS.contains(sid) => {}
                Some(sid) => {
                    return Err(format!(
                        "'{what}' uses service 0x{sid:02X}, which is not in the allowed set \
                         (memory/flash services are intentionally unsupported)"
                    ))
                }
                None => return Err(format!("'{what}' has an empty request")),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_decode_scales_and_signs() {
        let ch = Channel {
            key: "ect".into(),
            name: "Coolant temp".into(),
            unit: "°C".into(),
            request: vec![0x21, 0x02],
            offset: 2,
            length: 1,
            scale: 1.0,
            offset_value: -40.0,
            signed: false,
            verified: false,
        };
        // payload: [SID+0x40, echoed id, data...]
        assert_eq!(ch.decode(&[0x61, 0x02, 130]), Some(90.0));

        let rpm = Channel {
            key: "rpm".into(),
            name: "RPM".into(),
            unit: "rpm".into(),
            request: vec![0x21, 0x01],
            offset: 2,
            length: 2,
            scale: 0.25,
            offset_value: 0.0,
            signed: false,
            verified: false,
        };
        assert_eq!(rpm.decode(&[0x61, 0x01, 0x2E, 0xE0]), Some(3000.0));
    }

    #[test]
    fn rejects_memory_service_sids() {
        let toml_src = r#"
            [ecu]
            id = "bad"
            name = "Bad"
            manufacturer = "X"
            bus = "k-line"

            [init]
            method = "fast"
            ecu_address = 0x10
            baud = 10400

            [[routines]]
            key = "evil"
            name = "Read memory"
            risk = "high"
            request = [0x23, 0x00, 0x00]
        "#;
        let def: EcuDefinition = toml::from_str(toml_src).unwrap();
        let err = def.validate().unwrap_err();
        assert!(err.contains("0x23"));
    }
}
