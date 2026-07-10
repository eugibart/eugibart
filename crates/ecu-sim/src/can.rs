//! A simulated CAN/UDS ECU — the CAN-side counterpart to [`crate::Simulator`]
//! (the K-line ECU simulator). Independent of `ecu-defs`, same as the K-line
//! simulator: request/response mapping is hardcoded here rather than loaded
//! from a definition file, matching how a protocol-level "device under test"
//! should work. Satisfies the M5 exit criterion of connecting to a CAN bench
//! simulator — see `crates/ecu-sim/tests/can_session.rs`.

use std::time::Duration;

use motodiag_protocol_can::iso_tp::{self, IsoTpError};
use motodiag_protocol_can::transport::{CanBusEcuEnd, CanError};
use motodiag_protocol_can::uds::sid;

#[derive(Debug, Clone)]
pub struct CanSimConfig {
    /// CAN ID the tester transmits requests on (what this simulator listens
    /// for).
    pub tester_tx_id: u32,
    /// CAN ID this simulator transmits responses on.
    pub ecu_tx_id: u32,
    /// Bytes returned for the identification DID (0xF190).
    pub ident: Vec<u8>,
    /// Stored fault codes: (3-byte UDS DTC masked into a u32, status byte).
    pub dtcs: Vec<(u32, u8)>,
    pub rpm: f64,
    pub battery_v: f64,
}

impl Default for CanSimConfig {
    fn default() -> Self {
        Self {
            tester_tx_id: 0x7E0,
            ecu_tx_id: 0x7E8,
            ident: b"MC3-SIM-0001".to_vec(),
            dtcs: vec![(0x000335, 0x28)],
            rpm: 1250.0,
            battery_v: 12.8,
        }
    }
}

pub struct CanSimulator {
    config: CanSimConfig,
    dtcs: Vec<(u32, u8)>,
    /// IOControl/RoutineControl requests served so far (for test assertions).
    pub routine_log: Vec<Vec<u8>>,
}

impl CanSimulator {
    pub fn new(config: CanSimConfig) -> Self {
        let dtcs = config.dtcs.clone();
        Self {
            config,
            dtcs,
            routine_log: Vec::new(),
        }
    }

    /// Handle one UDS request payload; returns the response payload, or
    /// `None` for stay-silent situations (UDS has no equivalent of KWP's
    /// "not yet initialized" — a session-less request just gets a normal
    /// negative response instead).
    pub fn handle_request(&mut self, payload: &[u8]) -> Option<Vec<u8>> {
        let &request_sid = payload.first()?;
        let negative =
            |code: u8| -> Option<Vec<u8>> { Some(vec![sid::NEGATIVE_RESPONSE, request_sid, code]) };

        match payload {
            [s] if *s == sid::TESTER_PRESENT => Some(vec![sid::positive_response(*s)]),
            [s, sub] if *s == sid::DIAGNOSTIC_SESSION_CONTROL => {
                Some(vec![sid::positive_response(*s), *sub])
            }
            [s, did_hi, did_lo] if *s == sid::READ_DATA_BY_IDENTIFIER => {
                let did = u16::from_be_bytes([*did_hi, *did_lo]);
                let data = self.read_data_by_identifier(did)?;
                let mut out = vec![sid::positive_response(*s), *did_hi, *did_lo];
                out.extend(data);
                Some(out)
            }
            [s, 0x02, _mask] if *s == sid::READ_DTC_INFORMATION => {
                let mut out = vec![sid::positive_response(*s), 0x02, 0xFF];
                for (code, status) in &self.dtcs {
                    out.push((code >> 16) as u8);
                    out.push((code >> 8) as u8);
                    out.push(*code as u8);
                    out.push(*status);
                }
                Some(out)
            }
            [s, rest @ ..] if *s == sid::CLEAR_DIAGNOSTIC_INFORMATION => {
                self.dtcs.clear();
                let mut out = vec![sid::positive_response(*s)];
                out.extend_from_slice(rest);
                Some(out)
            }
            [s, rest @ ..] if *s == sid::INPUT_OUTPUT_CONTROL_BY_IDENTIFIER => {
                self.routine_log.push(payload.to_vec());
                let mut out = vec![sid::positive_response(*s)];
                out.extend_from_slice(rest);
                Some(out)
            }
            _ => negative(0x11), // service not supported
        }
    }

    fn read_data_by_identifier(&self, did: u16) -> Option<Vec<u8>> {
        match did {
            0xF190 => Some(self.config.ident.clone()),
            0xF10C => Some(((self.config.rpm / 0.25) as u16).to_be_bytes().to_vec()),
            0xF10D => Some(vec![(self.config.battery_v / 0.0625) as u8]),
            _ => None,
        }
    }

    pub fn dtc_count(&self) -> usize {
        self.dtcs.len()
    }
}

/// Run a simulator behind the ECU end of a CAN bus until it closes (or stays
/// idle past a generous number of timeouts is not a factor — only genuine
/// bus-closed errors end the loop). Returns the simulator so tests can
/// inspect its state.
pub fn run_on_can_bus(mut sim: CanSimulator, mut ecu_end: CanBusEcuEnd) -> CanSimulator {
    loop {
        let result = iso_tp::receive(
            &mut ecu_end,
            sim.config.ecu_tx_id,
            sim.config.tester_tx_id,
            Duration::from_millis(500),
        );
        match result {
            Ok(request) => {
                if let Some(response) = sim.handle_request(&request) {
                    let sent = iso_tp::send(
                        &mut ecu_end,
                        sim.config.ecu_tx_id,
                        sim.config.tester_tx_id,
                        &response,
                        Duration::from_millis(500),
                    );
                    if sent.is_err() {
                        return sim;
                    }
                }
            }
            Err(IsoTpError::Can(CanError::Timeout)) => continue,
            Err(_) => return sim,
        }
    }
}

/// Spawn `run_on_can_bus` on a background thread.
pub fn spawn_on_can_bus(
    sim: CanSimulator,
    ecu_end: CanBusEcuEnd,
) -> std::thread::JoinHandle<CanSimulator> {
    std::thread::spawn(move || run_on_can_bus(sim, ecu_end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_and_reads_live_data() {
        let mut sim = CanSimulator::new(CanSimConfig::default());
        let resp = sim.handle_request(&[0x22, 0xF1, 0x90]).unwrap();
        assert_eq!(&resp[..3], &[0x62, 0xF1, 0x90]);
        assert_eq!(&resp[3..], b"MC3-SIM-0001");

        let resp = sim.handle_request(&[0x22, 0xF1, 0x0C]).unwrap();
        assert_eq!(resp, vec![0x62, 0xF1, 0x0C, 0x13, 0x88]); // 1250 / 0.25 = 5000 = 0x1388
    }

    #[test]
    fn dtc_read_and_clear() {
        let mut sim = CanSimulator::new(CanSimConfig::default());
        let resp = sim.handle_request(&[0x19, 0x02, 0xFF]).unwrap();
        assert_eq!(resp[0], 0x59);
        assert_eq!(&resp[3..6], &[0x00, 0x03, 0x35]);

        let resp = sim.handle_request(&[0x14, 0xFF, 0xFF, 0xFF]).unwrap();
        assert_eq!(resp[0], 0x54);
        assert_eq!(sim.dtc_count(), 0);
    }

    #[test]
    fn unknown_service_gets_negative_response() {
        let mut sim = CanSimulator::new(CanSimConfig::default());
        let resp = sim.handle_request(&[0x27, 0x01]).unwrap();
        assert_eq!(resp, vec![0x7F, 0x27, 0x11]);
    }
}
