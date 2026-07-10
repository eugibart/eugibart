//! Proves the M3 fixture pipeline: record a full session (including the
//! fast-init handshake) against the simulated ECU, then replay the recorded
//! bytes with no simulator involved at all and get identical results. This is
//! the mechanism a real captured bike session will use once M1 lands.

use std::path::Path;
use std::time::Duration;

use motodiag_app_core::logging::{load_wire_trace, WireTraceRecorder};
use motodiag_app_core::{ConnectOptions, DiagSession};
use motodiag_ecu_defs::{EcuDefinition, Registry};
use motodiag_ecu_sim::{spawn_on_link, SimConfig, Simulator};
use motodiag_kwp2000::init::FastInitConfig;
use motodiag_transport::trace::TracingTransport;
use motodiag_transport::{mock, replay::ReplayTransport};

fn load_brutale_def() -> EcuDefinition {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../definitions");
    Registry::load_dir(&dir)
        .expect("definitions directory must load")
        .get("mv-5sm-brutale-910")
        .expect("Brutale 910 definition present")
        .clone()
}

fn quick_connect_options() -> ConnectOptions {
    ConnectOptions {
        fast_init: Some(FastInitConfig {
            idle_before: Duration::from_millis(1),
            low_time: Duration::from_millis(1),
            high_time: Duration::from_millis(1),
        }),
        ..Default::default()
    }
}

#[test]
fn recorded_session_replays_identically() {
    let tmp =
        std::env::temp_dir().join(format!("motodiag-replay-test-{}.jsonl", std::process::id()));

    // --- Record: connect to the simulator with the transport wrapped in a
    // tracing layer, then read identity/DTCs/live data.
    {
        let (tester, ecu) = mock::pair();
        let _sim = spawn_on_link(Simulator::new(SimConfig::default()), ecu);
        let recorder = WireTraceRecorder::create(&tmp).expect("create trace file");
        let traced = TracingTransport::new(Box::new(tester), Box::new(recorder));

        let mut session = DiagSession::connect(
            Box::new(traced),
            load_brutale_def(),
            quick_connect_options(),
        )
        .expect("connect while recording");
        assert_eq!(session.identity().text, "IAW 5SM SIM v0.1");
        let dtcs = session.read_dtcs().expect("read DTCs while recording");
        assert_eq!(dtcs.len(), 2);
        let rpm = session
            .read_channel("rpm")
            .expect("read rpm while recording");
        assert!(rpm.value > 0.0);
    }

    // --- Replay: no simulator, no mock link — just the recorded bytes.
    let events = load_wire_trace(&tmp).expect("load recorded trace");
    assert!(!events.is_empty(), "recording should have captured events");
    let replay = ReplayTransport::from_events(events);

    let mut replayed_session = DiagSession::connect(
        Box::new(replay),
        load_brutale_def(),
        quick_connect_options(),
    )
    .expect("connect via replay");
    assert_eq!(replayed_session.identity().text, "IAW 5SM SIM v0.1");
    let dtcs = replayed_session.read_dtcs().expect("read DTCs via replay");
    assert_eq!(dtcs.len(), 2);
    assert_eq!(dtcs[0].code, 0x0115);
    let rpm = replayed_session
        .read_channel("rpm")
        .expect("read rpm via replay");
    assert!(rpm.value > 0.0);

    let _ = std::fs::remove_file(&tmp);
}

/// Regenerates `fixtures/sim-5sm-full-session.jsonl`. Not run by default —
/// the fixture is a checked-in golden file; re-run this deliberately
/// (`cargo test -p motodiag-app-core -- --ignored regenerate_fixture`) if the
/// simulator's session shape changes and the fixture needs updating.
#[test]
#[ignore]
fn regenerate_fixture() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sim-5sm-full-session.jsonl");
    let (tester, ecu) = mock::pair();
    let _sim = spawn_on_link(Simulator::new(SimConfig::default()), ecu);
    let recorder = WireTraceRecorder::create(&path).expect("create fixture file");
    let traced = TracingTransport::new(Box::new(tester), Box::new(recorder));

    let mut session = DiagSession::connect(
        Box::new(traced),
        load_brutale_def(),
        quick_connect_options(),
    )
    .expect("connect while recording fixture");
    session
        .read_dtcs()
        .expect("read DTCs while recording fixture");
    session
        .read_channel("rpm")
        .expect("read rpm while recording fixture");
}

#[test]
fn replaying_a_shipped_fixture_matches_expectations() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sim-5sm-full-session.jsonl");
    let events = load_wire_trace(&fixture).expect("load shipped fixture");
    let replay = ReplayTransport::from_events(events);

    let mut session = DiagSession::connect(
        Box::new(replay),
        load_brutale_def(),
        quick_connect_options(),
    )
    .expect("connect via shipped fixture replay");
    assert_eq!(session.identity().text, "IAW 5SM SIM v0.1");

    let dtcs = session.read_dtcs().expect("read DTCs from fixture");
    assert_eq!(dtcs.len(), 2);
    assert_eq!(dtcs[0].code, 0x0115);
    assert_eq!(dtcs[1].code, 0x0120);

    let rpm = session.read_channel("rpm").expect("read rpm from fixture");
    assert!(rpm.value > 1000.0 && rpm.value < 2000.0);
}
