//! Connection troubleshooting: turn init failures into diagnoses.
//!
//! "I can't connect" is by far the most common failure mode reported by
//! owners using existing K-line tools, and none of them explain *why* a
//! connection failed. This module classifies exactly where the connection
//! attempt died — cable missing, port busy, no bus echo, silent ECU, garbled
//! response — and pairs each cause with the concrete next step a user should
//! take.
//!
//! The classification is a pure mapping over the error types the transport
//! and protocol layers already produce; the interesting design decision is
//! upstream, where `TransportError::NoEcho` was split from `Timeout` so the
//! two very different wiring-vs-ECU problems stay distinguishable here.

use motodiag_ecu_defs::EcuDefinition;
use motodiag_kwp2000::KwpError;
use motodiag_transport::{KLineTransport, TransportError};

use crate::session::{init_kwp_session, ConnectOptions};
use crate::AppError;

/// Where a connection attempt failed, in bus-layer order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailurePoint {
    /// The serial port could not be opened at all.
    PortOpen,
    /// Our own transmission never echoed back from the K-line.
    BusEcho,
    /// The wake-up + StartCommunication got no reply.
    EcuSilent,
    /// Something replied, but not intelligibly.
    GarbledResponse,
    /// The ECU replied with an explicit refusal.
    EcuRefused,
    /// Connected and identified successfully.
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Diagnosis {
    pub failure_point: FailurePoint,
    /// What happened, in plain language.
    pub finding: String,
    /// What the user should do about it.
    pub suggestion: String,
    /// The underlying error text, for bug reports and the curious.
    pub raw_error: Option<String>,
}

/// Attempt a connection and diagnose the outcome. On success the returned
/// diagnosis has `failure_point == None` and `finding` carries the ECU
/// identity string.
pub fn probe_connection(
    transport: Box<dyn KLineTransport>,
    def: &EcuDefinition,
    options: &ConnectOptions,
) -> Diagnosis {
    let mut kwp = match init_kwp_session(transport, def, options) {
        Ok(kwp) => kwp,
        Err(e) => return diagnose_error(&e),
    };

    match kwp.request(&def.identification.request) {
        Ok(response) => {
            let skip = def.identification.skip_bytes.min(response.len());
            let text: String = String::from_utf8_lossy(&response[skip..])
                .chars()
                .map(|c| if c.is_control() { '·' } else { c })
                .collect();
            Diagnosis {
                failure_point: FailurePoint::None,
                finding: format!("Connected. ECU identifies as: {text}"),
                suggestion: "Nothing to fix — you're talking to the ECU.".into(),
                raw_error: None,
            }
        }
        Err(e) => diagnose_error(&AppError::Kwp(e)),
    }
}

/// Map an error from the connect path to a plain-language diagnosis.
pub fn diagnose_error(error: &AppError) -> Diagnosis {
    let raw = Some(error.to_string());
    match error {
        AppError::Kwp(KwpError::Transport(t)) => diagnose_transport(t, raw),
        AppError::Kwp(KwpError::BadChecksum { .. })
        | AppError::Kwp(KwpError::MalformedFrame(_))
        | AppError::Kwp(KwpError::UnexpectedService { .. }) => Diagnosis {
            failure_point: FailurePoint::GarbledResponse,
            finding: "Something on the bus replied, but the response was garbled.".into(),
            suggestion: "Usually a baud-rate or protocol mismatch. Check the definition's baud \
                         (Marelli IAW is 10400) and that you selected the right ECU definition \
                         for this bike. Electrical noise from a running engine can also corrupt \
                         frames — try with the engine off, key on."
                .into(),
            raw_error: raw,
        },
        AppError::Kwp(KwpError::NegativeResponse { .. }) => Diagnosis {
            failure_point: FailurePoint::EcuRefused,
            finding: "The ECU is alive and talking, but refused the request.".into(),
            suggestion: "Good news: wiring and protocol are fine. The ECU rejected this \
                         particular service — the definition may need a different diagnostic \
                         session type or request bytes. Run motodiag-discover to map what this \
                         ECU actually accepts."
                .into(),
            raw_error: raw,
        },
        AppError::Kwp(KwpError::InitFailed(detail)) => Diagnosis {
            failure_point: FailurePoint::GarbledResponse,
            finding: format!("The ECU answered the wake-up, but not as expected: {detail}"),
            suggestion: "The bus works but the init handshake didn't complete. Try the other \
                         init method (fast vs 5-baud) in the definition, and double-check the \
                         ECU address byte."
                .into(),
            raw_error: raw,
        },
        other => Diagnosis {
            failure_point: FailurePoint::PortOpen,
            finding: format!("Connection failed before reaching the bus: {other}"),
            suggestion: "Check that the selected port exists and no other program (another \
                         diagnostic tool, a stale process) is holding it open."
                .into(),
            raw_error: raw,
        },
    }
}

fn diagnose_transport(error: &TransportError, raw: Option<String>) -> Diagnosis {
    match error {
        TransportError::NoEcho { .. } => Diagnosis {
            failure_point: FailurePoint::BusEcho,
            finding: "Bytes were transmitted but never echoed back from the K-line.".into(),
            suggestion: "On a single-wire K-line your own transmission always loops back when \
                         the wiring is right — no echo means the cable's K-line pin isn't \
                         reaching the bus. Check: K-line wired to the correct connector pin \
                         (OBD pin 7 on a KKL cable), connector seated, and that the cable is a \
                         genuine FTDI KKL (CH340 clones often fail here)."
                .into(),
            raw_error: raw,
        },
        TransportError::EchoMismatch { .. } => Diagnosis {
            failure_point: FailurePoint::BusEcho,
            finding: "The bus echo came back corrupted.".into(),
            suggestion: "Something else is driving the K-line or the baud rate is wrong. Make \
                         sure no other device/tool is connected to the diagnostic bus, the \
                         battery isn't sagging, and the definition's baud matches the ECU \
                         (10400 for Marelli IAW)."
                .into(),
            raw_error: raw,
        },
        TransportError::Timeout => Diagnosis {
            failure_point: FailurePoint::EcuSilent,
            finding: "The wake-up went out on the bus, but the ECU never answered.".into(),
            suggestion: "The cable and K-line wiring look fine (our bytes echoed back), so the \
                         ECU end is the problem. Check: ignition ON, kill switch to RUN, the \
                         connector's +12V pin actually live (some diagnostic connectors are \
                         only powered with ignition on), and the ECU address / init method in \
                         the definition (fast vs 5-baud slow init)."
                .into(),
            raw_error: raw,
        },
        TransportError::Io(e) => Diagnosis {
            failure_point: FailurePoint::PortOpen,
            finding: format!("Serial port I/O failed: {e}"),
            suggestion: "The port disappeared or refused access mid-session. Check the USB \
                         cable is still plugged in, and that no other program grabbed the \
                         port. On macOS, prefer the /dev/cu.* device over /dev/tty.*."
                .into(),
            raw_error: raw,
        },
        TransportError::Closed | TransportError::Unsupported(_) => Diagnosis {
            failure_point: FailurePoint::PortOpen,
            finding: format!("Transport error: {error}"),
            suggestion: "Reconnect the interface and try again.".into(),
            raw_error: raw,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use motodiag_ecu_sim::{spawn_on_link, SimConfig, Simulator};
    use motodiag_kwp2000::init::FastInitConfig;
    use motodiag_transport::mock;
    use std::time::Duration;

    fn load_brutale_def() -> EcuDefinition {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../definitions");
        motodiag_ecu_defs::Registry::load_dir(&dir)
            .unwrap()
            .get("mv-5sm-brutale-910")
            .unwrap()
            .clone()
    }

    fn quick_options() -> ConnectOptions {
        ConnectOptions {
            fast_init: Some(FastInitConfig {
                idle_before: Duration::from_millis(1),
                low_time: Duration::from_millis(1),
                high_time: Duration::from_millis(1),
            }),
            ..Default::default()
        }
    }

    /// A transport that fails every operation with a cloneable error recipe.
    struct FailingTransport(fn() -> TransportError);
    impl KLineTransport for FailingTransport {
        fn send(&mut self, _bytes: &[u8]) -> motodiag_transport::Result<()> {
            Err((self.0)())
        }
        fn read_byte(&mut self, _t: Duration) -> motodiag_transport::Result<u8> {
            Err((self.0)())
        }
        fn send_break(&mut self, _d: Duration) -> motodiag_transport::Result<()> {
            Ok(())
        }
        fn set_baud(&mut self, _b: u32) -> motodiag_transport::Result<()> {
            Ok(())
        }
        fn flush_input(&mut self) -> motodiag_transport::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn healthy_connection_reports_identity() {
        let (tester, ecu) = mock::pair();
        let _sim = spawn_on_link(Simulator::new(SimConfig::default()), ecu);
        let diagnosis = probe_connection(Box::new(tester), &load_brutale_def(), &quick_options());
        assert_eq!(diagnosis.failure_point, FailurePoint::None);
        assert!(diagnosis.finding.contains("IAW 5SM SIM v0.1"));
    }

    #[test]
    fn missing_echo_points_at_wiring() {
        let transport = FailingTransport(|| TransportError::NoEcho { sent: vec![0x81] });
        let diagnosis =
            probe_connection(Box::new(transport), &load_brutale_def(), &quick_options());
        assert_eq!(diagnosis.failure_point, FailurePoint::BusEcho);
        assert!(diagnosis.suggestion.contains("K-line pin"));
    }

    #[test]
    fn silent_ecu_points_at_ignition_and_power() {
        // No simulator on the other end: send succeeds (mock doesn't echo),
        // then the response read times out — the classic "ECU silent" case.
        let (tester, _ecu) = mock::pair();
        let diagnosis = probe_connection(Box::new(tester), &load_brutale_def(), &quick_options());
        assert_eq!(diagnosis.failure_point, FailurePoint::EcuSilent);
        assert!(diagnosis.suggestion.contains("ignition"));
    }

    #[test]
    fn garbage_on_the_bus_points_at_baud() {
        // An "ECU" that answers every request with a noise burst shaped like
        // a plausible frame start (0x81 fmt byte, so the parser commits to
        // reading a frame and then fails its checksum).
        let (tester, ecu) = mock::pair();
        std::thread::spawn(move || loop {
            match ecu.recv_event(Duration::from_millis(200)) {
                Ok(motodiag_transport::mock::LineEvent::Byte(_)) => {
                    if ecu.send_bytes(&[0x81, 0xAA, 0x00, 0xFF, 0x13]).is_err() {
                        return;
                    }
                }
                Ok(_) => continue,
                Err(motodiag_transport::TransportError::Timeout) => continue,
                Err(_) => return,
            }
        });
        let diagnosis = probe_connection(Box::new(tester), &load_brutale_def(), &quick_options());
        assert_eq!(diagnosis.failure_point, FailurePoint::GarbledResponse);
        assert!(diagnosis.suggestion.contains("baud"));
    }

    #[test]
    fn refusing_ecu_is_reported_as_alive() {
        let error = AppError::Kwp(KwpError::NegativeResponse {
            service: 0x1A,
            code: motodiag_kwp2000::services::NegativeResponseCode::ServiceNotSupported,
        });
        let diagnosis = diagnose_error(&error);
        assert_eq!(diagnosis.failure_point, FailurePoint::EcuRefused);
        assert!(diagnosis.finding.contains("alive"));
    }

    #[test]
    fn checksum_errors_point_at_baud_mismatch() {
        let error = AppError::Kwp(KwpError::BadChecksum {
            computed: 0x12,
            received: 0x34,
        });
        let diagnosis = diagnose_error(&error);
        assert_eq!(diagnosis.failure_point, FailurePoint::GarbledResponse);
        assert!(diagnosis.suggestion.contains("baud"));
    }
}
