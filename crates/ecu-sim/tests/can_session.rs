//! End-to-end M5 CAN groundwork test: mock CAN bus -> ISO-TP -> UDS session
//! -> simulated CAN ECU. This is the concrete "connect to a CAN bench
//! simulator" exit criterion for milestone M5's protocol/transport layer.
//! Not yet wired to `app-core::DiagSession` or the desktop UI — see the
//! module docs on `motodiag_protocol_can` and `ecu_sim::can`.

use motodiag_ecu_sim::can::{spawn_on_can_bus, CanSimConfig, CanSimulator};
use motodiag_protocol_can::session::UdsSession;
use motodiag_protocol_can::transport::pair;
use motodiag_protocol_can::uds::sid;

fn connect() -> (UdsSession, std::thread::JoinHandle<CanSimulator>) {
    let (tester, ecu) = pair();
    let handle = spawn_on_can_bus(CanSimulator::new(CanSimConfig::default()), ecu);
    let session = UdsSession::new(Box::new(tester), 0x7E0, 0x7E8);
    (session, handle)
}

#[test]
fn identifies_the_simulated_ecu() {
    let (mut session, _handle) = connect();
    let resp = session
        .request(&[0x22, 0xF1, 0x90])
        .expect("read VIN-ish DID");
    assert_eq!(&resp[..3], &[0x62, 0xF1, 0x90]);
    assert_eq!(&resp[3..], b"MC3-SIM-0001");
}

#[test]
fn reads_live_data_channels() {
    let (mut session, _handle) = connect();
    let rpm_resp = session.request(&[0x22, 0xF1, 0x0C]).expect("read rpm DID");
    let raw_rpm = u16::from_be_bytes([rpm_resp[3], rpm_resp[4]]);
    assert_eq!(f64::from(raw_rpm) * 0.25, 1250.0);

    let batt_resp = session
        .request(&[0x22, 0xF1, 0x0D])
        .expect("read battery DID");
    let raw_batt = batt_resp[3];
    assert!((f64::from(raw_batt) * 0.0625 - 12.8).abs() < 0.1);
}

#[test]
fn reads_and_clears_dtcs() {
    let (mut session, _handle) = connect();
    let resp = session
        .request(&[0x19, 0x02, 0xFF])
        .expect("read DTCs over CAN");
    assert_eq!(resp[0], 0x59);
    let dtc_count = (resp.len() - 3) / 4;
    assert_eq!(dtc_count, 1);
    assert_eq!(&resp[3..6], &[0x00, 0x03, 0x35]);

    session
        .request(&[0x14, 0xFF, 0xFF, 0xFF])
        .expect("clear DTCs over CAN");
    let resp = session
        .request(&[0x19, 0x02, 0xFF])
        .expect("re-read DTCs over CAN");
    assert_eq!(resp.len(), 3); // just SID + subfunction + availability mask
}

#[test]
fn runs_an_io_control_routine() {
    let (mut session, handle) = connect();
    let resp = session
        .request(&[sid::INPUT_OUTPUT_CONTROL_BY_IDENTIFIER, 0xF1, 0x50, 0x03])
        .expect("IOControl over CAN");
    assert_eq!(
        resp[0],
        sid::positive_response(sid::INPUT_OUTPUT_CONTROL_BY_IDENTIFIER)
    );

    drop(session);
    let sim = handle.join().unwrap();
    assert_eq!(sim.routine_log.len(), 1);
}

#[test]
fn tester_present_keepalive() {
    let (mut session, _handle) = connect();
    session.request(&[0x22, 0xF1, 0x90]).unwrap();
    assert!(!session.keepalive_due());
    session.tester_present().expect("tester present accepted");
}

#[test]
fn multi_frame_identification_over_can() {
    // The ident string (12 bytes) plus the 3-byte header comfortably exceeds
    // a single CAN frame's 7-byte payload budget, so this exercises the
    // ISO-TP First-Frame/Consecutive-Frame path end to end, not just the
    // single-frame case the other tests happen to hit.
    let (mut session, _handle) = connect();
    let resp = session.request(&[0x22, 0xF1, 0x90]).unwrap();
    assert!(
        resp.len() > 7,
        "response should have required multi-frame ISO-TP"
    );
    assert_eq!(&resp[3..], b"MC3-SIM-0001");
}

#[test]
fn unsupported_service_is_a_negative_response_error() {
    let (mut session, _handle) = connect();
    let err = session.request(&[0x27, 0x01]).unwrap_err();
    assert!(matches!(
        err,
        motodiag_protocol_can::session::UdsError::NegativeResponse { .. }
    ));
}

#[test]
fn times_out_when_ecu_is_gone() {
    // A session with no responder on the other end should time out rather
    // than hang forever.
    let (tester, _ecu_end_dropped_immediately) = pair();
    let mut session = UdsSession::new(Box::new(tester), 0x7E0, 0x7E8);
    let result = session.request(&[0x22, 0xF1, 0x90]);
    assert!(result.is_err());
}
