//! A simulated Magneti Marelli-style KWP2000 ECU.
//!
//! The core (`Simulator`) is a pure state machine: frame payload in, optional
//! response payload out. Wire adapters run it behind the in-memory mock link
//! (tests, in-app "Simulator" connection) or a pty (external tools).

use std::time::Duration;

use motodiag_kwp2000::framing::{parse_buffer, FrameCodec, ParseStatus};
use motodiag_kwp2000::services::sid;
use motodiag_transport::mock::{EcuLink, LineEvent};

/// What the simulated ECU pretends to be.
#[derive(Debug, Clone)]
pub struct SimConfig {
    pub ecu_address: u8,
    pub tester_address: u8,
    /// Identification string returned for ReadEcuIdentification.
    pub ident: String,
    /// Key bytes announced in the StartCommunication response.
    pub key_bytes: (u8, u8),
    /// Stored fault codes (code, status).
    pub dtcs: Vec<(u16, u8)>,
    /// Require a wake-up (break or fresh StartCommunication) before serving
    /// other requests, like a real ECU would.
    pub require_init: bool,
    /// Simulated engine state: idling (~1250 rpm) or off (0 rpm). Safety
    /// interlock tests flip this.
    pub engine_running: bool,
    /// Simulated battery voltage.
    pub battery_v: f64,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            ecu_address: 0x10,
            tester_address: 0xF1,
            ident: "IAW 5SM SIM v0.1".to_string(),
            key_bytes: (0xEA, 0x8F),
            dtcs: vec![(0x0115, 0x60), (0x0120, 0x20)],
            require_init: true,
            engine_running: true,
            battery_v: 12.8,
        }
    }
}

pub struct Simulator {
    config: SimConfig,
    initialized: bool,
    dtcs: Vec<(u16, u8)>,
    /// Routine/actuator requests served so far (for test assertions).
    pub routine_log: Vec<Vec<u8>>,
    /// Synthetic engine state driving the live-data answers.
    pub rpm: f64,
    pub tps_percent: f64,
    pub coolant_c: f64,
    pub intake_c: f64,
    pub battery_v: f64,
    pub map_mbar: f64,
}

impl Simulator {
    pub fn new(config: SimConfig) -> Self {
        let dtcs = config.dtcs.clone();
        let rpm = if config.engine_running { 1250.0 } else { 0.0 };
        let battery_v = config.battery_v;
        Self {
            config,
            initialized: false,
            dtcs,
            routine_log: Vec::new(),
            rpm,
            tps_percent: 2.0,
            coolant_c: 84.0,
            intake_c: 27.0,
            battery_v,
            map_mbar: 1005.0,
        }
    }

    pub fn wake(&mut self) {
        // A break pulse resets protocol state; the ECU then waits for
        // StartCommunication.
        self.initialized = false;
    }

    /// Handle one request payload addressed to us; returns the response
    /// payload, or `None` for stay-silent situations.
    pub fn handle_request(&mut self, payload: &[u8]) -> Option<Vec<u8>> {
        let &request_sid = payload.first()?;

        if request_sid == sid::START_COMMUNICATION {
            self.initialized = true;
            let (kb1, kb2) = self.config.key_bytes;
            return Some(vec![sid::positive_response(request_sid), kb1, kb2]);
        }
        if self.config.require_init && !self.initialized {
            // A real ECU that isn't in a diagnostic session just stays quiet.
            return None;
        }

        let negative =
            |code: u8| -> Option<Vec<u8>> { Some(vec![sid::NEGATIVE_RESPONSE, request_sid, code]) };

        match payload {
            [s] if *s == sid::TESTER_PRESENT => Some(vec![sid::positive_response(*s)]),
            [s, option, ..] if *s == sid::READ_ECU_IDENTIFICATION => {
                let mut out = vec![sid::positive_response(*s), *option];
                out.extend_from_slice(self.config.ident.as_bytes());
                Some(out)
            }
            [s, ..] if *s == sid::READ_DTC_BY_STATUS => {
                let mut out = vec![sid::positive_response(*s), self.dtcs.len() as u8];
                for (code, status) in &self.dtcs {
                    out.push((code >> 8) as u8);
                    out.push(*code as u8);
                    out.push(*status);
                }
                Some(out)
            }
            [s, group @ ..] if *s == sid::CLEAR_DIAGNOSTIC_INFORMATION => {
                self.dtcs.clear();
                let mut out = vec![sid::positive_response(*s)];
                out.extend_from_slice(group);
                Some(out)
            }
            [s, local_id] if *s == sid::READ_DATA_BY_LOCAL_ID => {
                let data = self.live_data(*local_id)?;
                let mut out = vec![sid::positive_response(*s), *local_id];
                out.extend_from_slice(&data);
                Some(out)
            }
            [s, rest @ ..]
                if *s == sid::START_ROUTINE_BY_LOCAL_ID
                    || *s == sid::INPUT_OUTPUT_CONTROL_BY_LOCAL_ID =>
            {
                self.routine_log.push(payload.to_vec());
                let mut out = vec![sid::positive_response(*s)];
                out.extend_from_slice(rest);
                Some(out)
            }
            [s, ..] if *s == sid::START_DIAGNOSTIC_SESSION => {
                Some(vec![sid::positive_response(*s), payload[1]])
            }
            _ => negative(0x11), // service not supported
        }
    }

    /// Synthetic sensor bytes for a live-data local identifier, matching the
    /// scalings in `definitions/mv/5sm-brutale-910.toml`.
    fn live_data(&mut self, local_id: u8) -> Option<Vec<u8>> {
        // Wander the engine state a little so dashboards visibly update.
        if self.config.engine_running {
            self.rpm = 1150.0 + (self.rpm - 1150.0 + 37.0) % 300.0;
        }
        match local_id {
            0x01 => {
                let raw = (self.rpm / 0.25) as u16;
                Some(raw.to_be_bytes().to_vec())
            }
            0x02 => {
                let raw = (self.tps_percent / 0.0015259) as u16;
                Some(raw.to_be_bytes().to_vec())
            }
            0x03 => Some(vec![(self.coolant_c + 40.0) as u8]),
            0x04 => Some(vec![(self.intake_c + 40.0) as u8]),
            0x05 => Some(vec![(self.battery_v / 0.0625) as u8]),
            0x06 => {
                let raw = self.map_mbar as u16;
                Some(raw.to_be_bytes().to_vec())
            }
            _ => None,
        }
    }

    pub fn dtc_count(&self) -> usize {
        self.dtcs.len()
    }
}

/// Run a simulator behind the ECU end of an in-memory mock link until the
/// link closes. Returns the simulator so tests can inspect its state.
pub fn run_on_link(mut sim: Simulator, link: EcuLink) -> Simulator {
    let response_codec = FrameCodec::physical(sim.config.tester_address, sim.config.ecu_address);
    let mut buf: Vec<u8> = Vec::new();

    loop {
        match link.recv_event(Duration::from_millis(200)) {
            Ok(LineEvent::Break(_)) => {
                sim.wake();
                buf.clear();
            }
            Ok(LineEvent::BaudChange(_)) => buf.clear(),
            Ok(LineEvent::Byte(b)) => {
                buf.push(b);
                match parse_buffer(&buf) {
                    ParseStatus::Incomplete => {}
                    ParseStatus::Invalid(_) => buf.clear(),
                    ParseStatus::Complete { frame, consumed } => {
                        buf.drain(..consumed);
                        if frame.target.is_none_or(|t| t == sim.config.ecu_address) {
                            if let Some(response) = sim.handle_request(&frame.payload) {
                                let encoded = response_codec.encode(&response);
                                if link.send_bytes(&encoded).is_err() {
                                    return sim;
                                }
                            }
                        }
                    }
                }
            }
            Err(motodiag_transport::TransportError::Timeout) => continue,
            Err(_) => return sim,
        }
    }
}

/// Spawn `run_on_link` on a background thread.
pub fn spawn_on_link(sim: Simulator, link: EcuLink) -> std::thread::JoinHandle<Simulator> {
    std::thread::spawn(move || run_on_link(sim, link))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_communication_then_ident() {
        let mut sim = Simulator::new(SimConfig::default());
        assert_eq!(sim.handle_request(&[0x1A, 0x80]), None); // not initialized

        let resp = sim.handle_request(&[0x81]).unwrap();
        assert_eq!(resp, vec![0xC1, 0xEA, 0x8F]);

        let resp = sim.handle_request(&[0x1A, 0x80]).unwrap();
        assert_eq!(&resp[..2], &[0x5A, 0x80]);
        assert_eq!(&resp[2..], b"IAW 5SM SIM v0.1");
    }

    #[test]
    fn dtc_read_and_clear() {
        let mut sim = Simulator::new(SimConfig::default());
        sim.handle_request(&[0x81]).unwrap();

        let resp = sim.handle_request(&[0x18, 0x02, 0xFF, 0x00]).unwrap();
        assert_eq!(resp[0], 0x58);
        assert_eq!(resp[1], 2); // two stored DTCs

        let resp = sim.handle_request(&[0x14, 0xFF, 0x00]).unwrap();
        assert_eq!(resp[0], 0x54);
        assert_eq!(sim.dtc_count(), 0);
    }

    #[test]
    fn unknown_service_gets_negative_response() {
        let mut sim = Simulator::new(SimConfig::default());
        sim.handle_request(&[0x81]).unwrap();
        let resp = sim.handle_request(&[0x27, 0x01]).unwrap();
        assert_eq!(resp, vec![0x7F, 0x27, 0x11]);
    }
}
