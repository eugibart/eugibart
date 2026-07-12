//! Serde schema for ECU definition TOML files.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EcuDefinition {
    pub ecu: EcuInfo,
    /// K-line/KWP2000 bus config. Required when `ecu.bus = "k-line"`, absent
    /// when `ecu.bus = "can"` (see `can` below instead).
    #[serde(default)]
    pub init: Option<InitConfig>,
    /// CAN/UDS bus config. Required when `ecu.bus = "can"`, absent otherwise.
    #[serde(default)]
    pub can: Option<CanBusConfig>,
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
    /// Parameters for the guided charging-system check (community-sourced;
    /// absent when no citable figures exist for this ECU's bikes).
    #[serde(default)]
    pub charging: Option<ChargingTest>,
    /// "Where do I plug in?" — how to physically reach the diagnostic
    /// connector on this ECU's bikes. Absent = we honestly don't know yet;
    /// the UI shows a check-your-manual fallback instead of guessing.
    #[serde(default)]
    pub connector_access: Option<AccessGuide>,
    /// Same, for the vacuum take-off ports used in throttle-body sync.
    #[serde(default)]
    pub vacuum_access: Option<AccessGuide>,
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
    /// Body shape of this definition's bikes — picks which schematic
    /// silhouette the access-guide diagrams draw (marker positions differ).
    #[serde(default)]
    pub body_style: BodyStyle,
    /// True once the definition has been exercised against a real ECU.
    /// Unverified definitions are shown with a warning in the UI.
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BodyStyle {
    #[default]
    Naked,
    Faired,
}

impl BodyStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            BodyStyle::Naked => "naked",
            BodyStyle::Faired => "faired",
        }
    }
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

/// CAN/UDS bus addressing. Segmentation for payloads over 7 bytes is handled
/// by ISO-TP (`crates/protocol-can`), not by this config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanBusConfig {
    /// CAN ID the tester transmits requests on.
    pub tx_id: u32,
    /// CAN ID the ECU responds on.
    pub rx_id: u32,
    #[serde(default)]
    pub extended_ids: bool,
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
    /// Expected normal range under a stated condition, e.g. "warm idle,
    /// neutral". Absent unless the workshop manual (or a verified measurement)
    /// backs it up — never a guess presented as fact.
    #[serde(default)]
    pub spec: Option<ChannelSpec>,
}

fn default_scale() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelSpec {
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// A single target value, when the spec is a point rather than a range
    /// (e.g. a CO trim target percentage).
    pub target: Option<f64>,
    /// The condition this spec applies under — specs are meaningless without
    /// one ("1100-1300 rpm" means nothing without "warm idle, neutral").
    pub condition: String,
    /// Where the figure comes from, when it's community-sourced rather than
    /// factory-manual (site name shown in the UI, e.g. "ducatimonster.org").
    #[serde(default)]
    pub source: Option<String>,
    /// The specific page/thread backing the figure (https).
    #[serde(default)]
    pub source_url: Option<String>,
}

/// Parameters for the guided charging-system test: an interactive
/// rest → idle → revved battery-voltage check driven by live data. The
/// charging system (regulator/rectifier, stator, connectors) is the
/// most notorious real-world failure on both marques, and community
/// threads carry concrete, citable pass/fail figures — this block turns
/// them into a walk-through. Requires both a `batt` and an `rpm` channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChargingTest {
    /// Healthy resting battery voltage (key on, engine off) lower bound.
    pub rest_min_v: f64,
    /// Minimum acceptable voltage at the check RPM — below this, the
    /// charging system is suspect.
    pub charging_min_v: f64,
    /// Maximum acceptable voltage — above this the regulator is
    /// overcharging (which cooks batteries).
    pub charging_max_v: f64,
    /// RPM band the rider holds while the app captures the charging
    /// reading (charging systems often need revs to produce full output).
    pub check_rpm_min: f64,
    pub check_rpm_max: f64,
    /// What to physically check when the verdict points at the stator
    /// vs the regulator/rectifier — the community's diagnostic split.
    pub stator_notes: String,
    /// Community citation (site name + https URL) — required: these are
    /// pass/fail thresholds, they must be attributable.
    pub source: String,
    pub source_url: String,
}

/// "How do I get to it?" — where a port physically is on the bike, what
/// tools the job needs, and the steps to reach it. Rendered as a schematic
/// side-view diagram in the UI; `zone` picks the marker position. Locations
/// are approximate by design — `verify_note` carries the honest caveat.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessGuide {
    pub zone: AccessZone,
    /// One-sentence answer ("3-pin connector under the tank, right side").
    pub summary: String,
    /// Ordered steps to physically reach the port.
    #[serde(default)]
    pub steps: Vec<String>,
    /// What to have ready before starting.
    #[serde(default)]
    pub tools: Vec<String>,
    /// Caveat shown with the guide ("varies by model year — verify against
    /// the wiring diagram before first connection").
    #[serde(default)]
    pub verify_note: Option<String>,
    /// Citation (site + https URL) when the location claim is
    /// community-sourced; both or neither.
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccessZone {
    UnderSeat,
    UnderTankLeft,
    UnderTankRight,
    UnderTankCenter,
    TailSection,
    DashArea,
    SidePanelLeft,
    SidePanelRight,
}

impl AccessZone {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessZone::UnderSeat => "under-seat",
            AccessZone::UnderTankLeft => "under-tank-left",
            AccessZone::UnderTankRight => "under-tank-right",
            AccessZone::UnderTankCenter => "under-tank-center",
            AccessZone::TailSection => "tail-section",
            AccessZone::DashArea => "dash-area",
            AccessZone::SidePanelLeft => "side-panel-left",
            AccessZone::SidePanelRight => "side-panel-right",
        }
    }
}

impl ChannelSpec {
    /// `None` when there's nothing to check against (no min/max set) or the
    /// value can't be classified; `Some(true)` when in range.
    pub fn in_range(&self, value: f64) -> Option<bool> {
        if self.min.is_none() && self.max.is_none() {
            return None;
        }
        let above_min = self.min.is_none_or(|m| value >= m);
        let below_max = self.max.is_none_or(|m| value <= m);
        Some(above_min && below_max)
    }
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
    /// Likely culprits, most common first ("TPS connector corrosion",
    /// "throttle cable out of adjustment"). Plain language for the UI.
    #[serde(default)]
    pub causes: Vec<String>,
    /// What to physically check/do, in order.
    #[serde(default)]
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Routine {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Full step-by-step procedure, workshop-manual-shaped. Empty means only
    /// `description` is available — the UI falls back to that.
    #[serde(default)]
    pub procedure: Vec<String>,
    pub risk: RiskLevel,
    /// Request payload sent when the routine runs (e.g. StartRoutineByLocalId
    /// or InputOutputControlByLocalId).
    pub request: Vec<u8>,
    #[serde(default)]
    pub preconditions: Preconditions,
    /// What to have ready before running this routine ("laptop + KKL cable",
    /// "battery charger connected"). Shown as chips on the routine card.
    #[serde(default)]
    pub tools: Vec<String>,
    /// Citation for a community-sourced procedure (site + https URL, paired).
    /// The Service UI shows it as a clickable link under the steps — same
    /// treatment specs and mod-notes get, so an enriched procedure says
    /// where it came from instead of burying it in the prose.
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
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
/// Covers both KWP2000 (K-line) and UDS/ISO 14229 (CAN) — the two protocols
/// share several identical SIDs (0x10, 0x14, 0x3E) and the 0x7F negative
/// response convention; where they differ, both are listed. Notably absent:
/// UDS WriteDataByIdentifier (0x2E) and SecurityAccess (0x27) — see
/// docs/SAFETY.md.
const ALLOWED_SIDS: &[u8] = &[
    0x10, // StartDiagnosticSession / DiagnosticSessionControl
    0x14, // ClearDiagnosticInformation
    0x18, // ReadDTCByStatus (KWP2000)
    0x19, // ReadDTCInformation (UDS)
    0x1A, // ReadEcuIdentification (KWP2000)
    0x21, // ReadDataByLocalIdentifier (KWP2000)
    0x22, // ReadDataByIdentifier (UDS)
    0x2F, // InputOutputControlByIdentifier (UDS)
    0x30, // InputOutputControlByLocalIdentifier (KWP2000)
    0x31, // StartRoutineByLocalIdentifier / RoutineControl
    0x32, // StopRoutineByLocalIdentifier (KWP2000)
    0x33, // RequestRoutineResultsByLocalIdentifier (KWP2000)
    0x3E, // TesterPresent
];

impl EcuDefinition {
    pub fn validate(&self) -> Result<(), String> {
        match self.ecu.bus {
            BusKind::KLine => {
                if self.init.is_none() {
                    return Err("bus = \"k-line\" requires an [init] table".to_string());
                }
                if self.can.is_some() {
                    return Err("bus = \"k-line\" must not have a [can] table".to_string());
                }
            }
            BusKind::Can => {
                if self.can.is_none() {
                    return Err("bus = \"can\" requires a [can] table".to_string());
                }
                if self.init.is_some() {
                    return Err("bus = \"can\" must not have an [init] table".to_string());
                }
            }
        }

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

        if let Some(charging) = &self.charging {
            if !(charging.rest_min_v < charging.charging_min_v
                && charging.charging_min_v < charging.charging_max_v)
            {
                return Err(
                    "[charging] needs rest_min_v < charging_min_v < charging_max_v".to_string(),
                );
            }
            if charging.check_rpm_min >= charging.check_rpm_max || charging.check_rpm_min <= 0.0 {
                return Err("[charging] needs 0 < check_rpm_min < check_rpm_max".to_string());
            }
            for key in ["batt", "rpm"] {
                if !self.channels.iter().any(|c| c.key == key) {
                    return Err(format!(
                        "[charging] requires a '{key}' channel — the guided test reads it live"
                    ));
                }
            }
            if charging.source.trim().is_empty() || !charging.source_url.starts_with("https://") {
                return Err(
                    "[charging] thresholds are community claims — they need a named source \
                     and an https source_url"
                        .to_string(),
                );
            }
        }

        for (label, guide) in [
            ("connector_access", &self.connector_access),
            ("vacuum_access", &self.vacuum_access),
        ] {
            let Some(g) = guide else { continue };
            if g.summary.trim().is_empty() {
                return Err(format!("[{label}] needs a non-empty summary"));
            }
            if g.steps.iter().any(|s| s.trim().is_empty())
                || g.tools.iter().any(|t| t.trim().is_empty())
            {
                return Err(format!("[{label}] has an empty step or tool entry"));
            }
            match (&g.source, &g.source_url) {
                (None, None) => {}
                (Some(s), Some(u)) => {
                    if s.trim().is_empty() || !u.starts_with("https://") {
                        return Err(format!(
                            "[{label}] citation needs a named source and an https source_url"
                        ));
                    }
                }
                _ => {
                    return Err(format!(
                        "[{label}] source and source_url go together — set both or neither"
                    ))
                }
            }
        }

        for r in &self.routines {
            if r.tools.iter().any(|t| t.trim().is_empty()) {
                return Err(format!("routine '{}' has an empty tools entry", r.key));
            }
            match (&r.source, &r.source_url) {
                (None, None) => {}
                (Some(s), Some(u)) => {
                    if s.trim().is_empty() || !u.starts_with("https://") {
                        return Err(format!(
                            "routine '{}' citation needs a named source and an https source_url",
                            r.key
                        ));
                    }
                }
                _ => {
                    return Err(format!(
                        "routine '{}' source and source_url go together — set both or neither",
                        r.key
                    ))
                }
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
            spec: None,
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
            spec: None,
        };
        assert_eq!(rpm.decode(&[0x61, 0x01, 0x2E, 0xE0]), Some(3000.0));
    }

    #[test]
    fn access_guide_parses_and_validates() {
        let toml = r#"
            [ecu]
            id = "x"
            name = "X"
            manufacturer = "Y"
            bus = "k-line"
            body_style = "faired"

            [init]
            method = "fast"
            ecu_address = 0x10
            baud = 10400

            [identification]
            request = [0x1A, 0x80]

            [connector_access]
            zone = "under-tank-right"
            summary = "3-pin connector under the tank, right side."
            steps = ["Key OFF", "Prop the tank per the manual"]
            tools = ["FTDI KKL cable"]
            verify_note = "Varies by model year."
            source = "mvagusta.net"
            source_url = "https://www.mvagusta.net/threads/1/"
        "#;
        let def: EcuDefinition = toml::from_str(toml).unwrap();
        def.validate().unwrap();
        let guide = def.connector_access.as_ref().unwrap();
        assert_eq!(guide.zone, AccessZone::UnderTankRight);
        assert_eq!(guide.zone.as_str(), "under-tank-right");
        assert_eq!(def.ecu.body_style, BodyStyle::Faired);
        assert!(def.vacuum_access.is_none());

        // Bad zone is a parse error, not a silent fallback.
        assert!(toml::from_str::<EcuDefinition>(
            &toml.replace("under-tank-right", "behind-the-headlight")
        )
        .is_err());

        // Citation must be https and paired.
        let http = toml.replace("https://www.mvagusta.net/threads/1/", "http://x.com/");
        let def: EcuDefinition = toml::from_str(&http).unwrap();
        assert!(def.validate().unwrap_err().contains("https"));
        let unpaired = toml.replace("source = \"mvagusta.net\"\n", "");
        let def: EcuDefinition = toml::from_str(&unpaired).unwrap();
        assert!(def.validate().unwrap_err().contains("go together"));

        // Empty steps entries are rejected.
        let blank = toml.replace("\"Key OFF\"", "\"  \"");
        let def: EcuDefinition = toml::from_str(&blank).unwrap();
        assert!(def.validate().unwrap_err().contains("empty step"));
    }

    #[test]
    fn k_line_bus_requires_init_and_forbids_can() {
        let missing_init = r#"
            [ecu]
            id = "x"
            name = "X"
            manufacturer = "Y"
            bus = "k-line"
        "#;
        let def: EcuDefinition = toml::from_str(missing_init).unwrap();
        assert!(def.validate().unwrap_err().contains("[init]"));

        let both = r#"
            [ecu]
            id = "x"
            name = "X"
            manufacturer = "Y"
            bus = "k-line"

            [init]
            method = "fast"
            ecu_address = 0x10
            baud = 10400

            [can]
            tx_id = 0x7E0
            rx_id = 0x7E8
        "#;
        let def: EcuDefinition = toml::from_str(both).unwrap();
        assert!(def.validate().unwrap_err().contains("[can]"));
    }

    #[test]
    fn can_bus_requires_can_and_forbids_init() {
        let missing_can = r#"
            [ecu]
            id = "x"
            name = "X"
            manufacturer = "Y"
            bus = "can"
        "#;
        let def: EcuDefinition = toml::from_str(missing_can).unwrap();
        assert!(def.validate().unwrap_err().contains("[can]"));

        let valid = r#"
            [ecu]
            id = "x"
            name = "X"
            manufacturer = "Y"
            bus = "can"

            [can]
            tx_id = 0x7E0
            rx_id = 0x7E8
        "#;
        let def: EcuDefinition = toml::from_str(valid).unwrap();
        assert!(def.validate().is_ok());
    }

    #[test]
    fn channel_spec_in_range_classification() {
        let idle_rpm = ChannelSpec {
            min: Some(1100.0),
            max: Some(1300.0),
            target: None,
            condition: "warm idle, neutral".into(),
            source: None,
            source_url: None,
        };
        assert_eq!(idle_rpm.in_range(1200.0), Some(true));
        assert_eq!(idle_rpm.in_range(900.0), Some(false));
        assert_eq!(idle_rpm.in_range(1300.0), Some(true)); // inclusive bound

        let no_bounds = ChannelSpec {
            min: None,
            max: None,
            target: Some(2.0),
            condition: "any".into(),
            source: None,
            source_url: None,
        };
        assert_eq!(no_bounds.in_range(2.0), None);

        let min_only = ChannelSpec {
            min: Some(11.5),
            max: None,
            target: None,
            condition: "engine off".into(),
            source: None,
            source_url: None,
        };
        assert_eq!(min_only.in_range(12.8), Some(true));
        assert_eq!(min_only.in_range(9.0), Some(false));
    }

    #[test]
    fn channel_spec_and_routine_procedure_parse_from_toml() {
        let toml_src = r#"
            [ecu]
            id = "x"
            name = "X"
            manufacturer = "Y"
            bus = "k-line"

            [init]
            method = "fast"
            ecu_address = 0x10
            baud = 10400

            [[channels]]
            key = "rpm"
            name = "Engine speed"
            request = [0x21, 0x01]
            offset = 2
            length = 2
            scale = 0.25
            [channels.spec]
            min = 1100.0
            max = 1300.0
            condition = "warm idle, neutral"

            [[routines]]
            key = "tps_reset"
            name = "TPS reset"
            risk = "medium"
            request = [0x31, 0x01]
            procedure = ["Step one", "Step two", "Step three"]
        "#;
        let def: EcuDefinition = toml::from_str(toml_src).unwrap();
        def.validate().unwrap();

        let rpm = &def.channels[0];
        let spec = rpm.spec.as_ref().expect("spec present");
        assert_eq!(spec.min, Some(1100.0));
        assert_eq!(spec.condition, "warm idle, neutral");

        assert_eq!(def.routines[0].procedure.len(), 3);
        assert_eq!(def.routines[0].procedure[0], "Step one");
    }

    #[test]
    fn routine_citation_parses_and_validates() {
        let base = r#"
            [ecu]
            id = "x"
            name = "X"
            manufacturer = "Y"
            bus = "k-line"

            [init]
            method = "fast"
            ecu_address = 0x10
            baud = 10400

            [identification]
            request = [0x1A, 0x80]

            [[routines]]
            key = "tps_reset"
            name = "TPS reset"
            risk = "medium"
            request = [0x31, 0x01]
            procedure = ["Step one"]
            source = "revlimiter.it"
            source_url = "https://www.revlimiter.it/forum/viewtopic.php?t=1"
        "#;
        let def: EcuDefinition = toml::from_str(base).unwrap();
        def.validate().unwrap();
        assert_eq!(def.routines[0].source.as_deref(), Some("revlimiter.it"));

        // Non-https is rejected.
        let http = base.replace("https://www.revlimiter.it", "http://www.revlimiter.it");
        let def: EcuDefinition = toml::from_str(&http).unwrap();
        assert!(def.validate().unwrap_err().contains("https"));

        // Source without url (or vice versa) is rejected.
        let unpaired = base.replace("source = \"revlimiter.it\"\n", "");
        let def: EcuDefinition = toml::from_str(&unpaired).unwrap();
        assert!(def.validate().unwrap_err().contains("go together"));
    }

    #[test]
    fn spec_and_procedure_are_optional() {
        // A definition with neither field must still parse and validate,
        // same as before this feature existed.
        let toml_src = r#"
            [ecu]
            id = "x"
            name = "X"
            manufacturer = "Y"
            bus = "k-line"

            [init]
            method = "fast"
            ecu_address = 0x10
            baud = 10400

            [[routines]]
            key = "r"
            name = "R"
            risk = "low"
            request = [0x3E]
        "#;
        let def: EcuDefinition = toml::from_str(toml_src).unwrap();
        def.validate().unwrap();
        assert!(def.routines[0].procedure.is_empty());
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
