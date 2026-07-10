//! The diagnostic session: transport + ECU definition → operations.

use motodiag_ecu_defs::schema::{Channel, InitMethod};
use motodiag_ecu_defs::EcuDefinition;
use motodiag_kwp2000::framing::FrameCodec;
use motodiag_kwp2000::init::{fast_init, slow_init_5baud, FastInitConfig, SlowInitConfig};
use motodiag_kwp2000::session::{KwpSession, TraceHook};
use motodiag_kwp2000::timing::TimingParams;
use motodiag_transport::KLineTransport;

use crate::dtc::{parse_read_dtc_response, Dtc};
use crate::live_data::{now_ms, Reading};
use crate::safety::{check_preconditions, SafetyViolation};
use crate::{AppError, Result};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct EcuIdentity {
    /// Raw identification payload (after SID and option bytes).
    pub raw: Vec<u8>,
    /// Printable interpretation of the raw bytes.
    pub text: String,
}

/// Knobs for connecting; tests shrink the wake-up delays so suites stay fast.
#[derive(Default)]
pub struct ConnectOptions {
    pub fast_init: Option<FastInitConfig>,
    pub slow_init: Option<SlowInitConfig>,
    pub trace: Option<TraceHook>,
}

pub struct DiagSession {
    kwp: KwpSession,
    def: EcuDefinition,
    identity: EcuIdentity,
    service_mode: bool,
}

/// Run the K-line init sequence from an ECU definition and return a live
/// [`KwpSession`], ready for requests. Shared by [`DiagSession::connect`] and
/// `discovery::run_discovery`, which needs a session but not the rest of
/// `DiagSession`'s definition-shaped API.
pub(crate) fn init_kwp_session(
    mut transport: Box<dyn KLineTransport>,
    def: &EcuDefinition,
    options: &ConnectOptions,
) -> Result<KwpSession> {
    let init = def
        .init
        .clone()
        .ok_or_else(|| AppError::NotKLine(def.ecu.id.clone()))?;
    let codec = FrameCodec {
        addressing: Some((init.ecu_address, init.tester_address)),
        separate_length_byte: init.separate_length_byte,
    };
    let timing = timing_from_def(def);

    match init.method {
        InitMethod::Fast => {
            let cfg = options.fast_init.clone().unwrap_or_default();
            fast_init(transport.as_mut(), &codec, &timing, &cfg)?;
        }
        InitMethod::Slow5Baud => {
            let cfg = options.slow_init.clone().unwrap_or_default();
            slow_init_5baud(transport.as_mut(), init.ecu_address, &timing, &cfg)?;
        }
    }

    Ok(KwpSession::new(transport, codec, timing))
}

impl DiagSession {
    /// Initialize the bus per the ECU definition and read the ECU identity.
    ///
    /// Only K-line/KWP2000 definitions are supported here; a CAN/UDS
    /// definition (`def.init` absent) is rejected with `AppError::NotKLine` —
    /// CAN sessions are a separate, not-yet-wired-up path (see
    /// `crates/protocol-can`, milestone M5 groundwork).
    pub fn connect(
        transport: Box<dyn KLineTransport>,
        def: EcuDefinition,
        options: ConnectOptions,
    ) -> Result<Self> {
        let mut kwp = init_kwp_session(transport, &def, &options)?;
        if let Some(trace) = options.trace {
            kwp.set_trace_hook(trace);
        }

        let response = kwp.request(&def.identification.request)?;
        let raw = response
            .get(def.identification.skip_bytes.min(response.len())..)
            .unwrap_or_default()
            .to_vec();
        let text = String::from_utf8_lossy(&raw)
            .chars()
            .map(|c| if c.is_control() { '·' } else { c })
            .collect();
        let identity = EcuIdentity { raw, text };

        Ok(Self {
            kwp,
            def,
            identity,
            service_mode: false,
        })
    }

    pub fn identity(&self) -> &EcuIdentity {
        &self.identity
    }

    pub fn definition(&self) -> &EcuDefinition {
        &self.def
    }

    // ---- read-only operations -------------------------------------------

    pub fn read_dtcs(&mut self) -> Result<Vec<Dtc>> {
        let response = self.kwp.request(&self.def.dtc.read_request.clone())?;
        Ok(parse_read_dtc_response(&response, &self.def))
    }

    pub fn read_channel(&mut self, key: &str) -> Result<Reading> {
        let channel = self
            .def
            .channels
            .iter()
            .find(|c| c.key == key)
            .cloned()
            .ok_or_else(|| AppError::UnknownChannel(key.to_string()))?;
        self.read_channel_inner(&channel)
    }

    /// Poll every defined channel once (the dashboard's refresh tick).
    pub fn poll_all_channels(&mut self) -> Vec<Result<Reading>> {
        let channels: Vec<Channel> = self.def.channels.clone();
        channels
            .iter()
            .map(|c| self.read_channel_inner(c))
            .collect()
    }

    fn read_channel_inner(&mut self, channel: &Channel) -> Result<Reading> {
        let response = self.kwp.request(&channel.request)?;
        let value = channel.decode(&response).ok_or_else(|| {
            AppError::Kwp(motodiag_kwp2000::KwpError::MalformedFrame(format!(
                "response too short for channel '{}'",
                channel.key
            )))
        })?;
        Ok(Reading {
            key: channel.key.clone(),
            name: channel.name.clone(),
            unit: channel.unit.clone(),
            value,
            timestamp_ms: now_ms(),
        })
    }

    /// Keep the session alive during idle periods; call from the UI's tick.
    pub fn keepalive_if_due(&mut self) -> Result<()> {
        if self.kwp.keepalive_due() {
            self.kwp.tester_present()?;
        }
        Ok(())
    }

    // ---- state-changing operations (safety-gated) ------------------------

    pub fn service_mode(&self) -> bool {
        self.service_mode
    }

    /// Explicit opt-in to state-changing operations for this session.
    pub fn enable_service_mode(&mut self) {
        self.service_mode = true;
    }

    /// Clear stored fault codes. Requires service mode and confirmation.
    pub fn clear_dtcs(&mut self, confirmed: bool) -> Result<()> {
        self.require_write_access(confirmed)?;
        self.kwp.request(&self.def.dtc.clear_request.clone())?;
        Ok(())
    }

    /// Run a service routine/actuator test after checking its preconditions
    /// against live measurements.
    pub fn run_routine(&mut self, key: &str, confirmed: bool) -> Result<Vec<u8>> {
        self.require_write_access(confirmed)?;

        let routine = self
            .def
            .routines
            .iter()
            .find(|r| r.key == key)
            .cloned()
            .ok_or_else(|| AppError::UnknownRoutine(key.to_string()))?;

        let needs_rpm = routine.preconditions.engine_must_be_off;
        let needs_battery = routine.preconditions.min_battery_v.is_some()
            || routine.preconditions.max_battery_v.is_some();
        let rpm = if needs_rpm {
            self.try_read_value("rpm")
        } else {
            None
        };
        let battery_v = if needs_battery {
            self.try_read_value("batt")
        } else {
            None
        };
        check_preconditions(&routine.preconditions, rpm, battery_v).map_err(AppError::Safety)?;

        Ok(self.kwp.request(&routine.request)?)
    }

    fn require_write_access(&self, confirmed: bool) -> Result<()> {
        if !self.service_mode {
            return Err(AppError::Safety(SafetyViolation::ServiceModeDisabled));
        }
        if !confirmed {
            return Err(AppError::Safety(SafetyViolation::ConfirmationRequired));
        }
        Ok(())
    }

    /// Best-effort measurement for precondition checks; `None` (fails closed
    /// in the checker) when the channel is missing or unreadable.
    fn try_read_value(&mut self, key: &str) -> Option<f64> {
        self.read_channel(key).ok().map(|r| r.value)
    }
}

fn timing_from_def(def: &EcuDefinition) -> TimingParams {
    use std::time::Duration;
    let mut t = TimingParams::default();
    let o = &def.timing;
    if let Some(ms) = o.p1_max_ms {
        t.p1_max = Duration::from_millis(ms);
    }
    if let Some(ms) = o.p2_min_ms {
        t.p2_min = Duration::from_millis(ms);
    }
    if let Some(ms) = o.p2_max_ms {
        t.p2_max = Duration::from_millis(ms);
    }
    if let Some(ms) = o.p3_min_ms {
        t.p3_min = Duration::from_millis(ms);
    }
    if let Some(ms) = o.p3_max_ms {
        t.p3_max = Duration::from_millis(ms);
    }
    if let Some(ms) = o.p4_min_ms {
        t.p4_min = Duration::from_millis(ms);
    }
    t
}
