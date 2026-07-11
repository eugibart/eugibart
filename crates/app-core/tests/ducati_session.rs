//! Proves the M5 "pure reuse" claim: the K-line transport, KWP2000 protocol
//! layer, and app-core orchestration all work unmodified against the Ducati
//! definitions — only new TOML data was added, no new code.

use std::path::Path;
use std::time::Duration;

use motodiag_app_core::safety::SafetyViolation;
use motodiag_app_core::{AppError, ConnectOptions, DiagSession};
use motodiag_ecu_defs::{EcuDefinition, Registry};
use motodiag_ecu_sim::{spawn_on_link, SimConfig, Simulator};
use motodiag_kwp2000::init::{FastInitConfig, SlowInitConfig};
use motodiag_transport::mock;

fn load_def(id: &str) -> EcuDefinition {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../definitions");
    Registry::load_dir(&dir)
        .expect("definitions directory must load")
        .get(id)
        .unwrap_or_else(|| panic!("definition '{id}' present"))
        .clone()
}

fn quick_options_for(def: &EcuDefinition) -> ConnectOptions {
    use motodiag_ecu_defs::schema::InitMethod;
    match def
        .init
        .as_ref()
        .expect("K-line def has init config")
        .method
    {
        InitMethod::Fast => ConnectOptions {
            fast_init: Some(FastInitConfig {
                idle_before: Duration::from_millis(1),
                low_time: Duration::from_millis(1),
                high_time: Duration::from_millis(1),
            }),
            ..Default::default()
        },
        InitMethod::Slow5Baud => ConnectOptions {
            slow_init: Some(SlowInitConfig {
                bit_time: Duration::from_millis(1),
                sync_timeout: Duration::from_millis(200),
            }),
            ..Default::default()
        },
    }
}

fn connect_sim(def: EcuDefinition, sim_config: SimConfig) -> DiagSession {
    let (tester, ecu) = mock::pair();
    let _sim = spawn_on_link(Simulator::new(sim_config), ecu);
    let options = quick_options_for(&def);
    DiagSession::connect(Box::new(tester), def, options).expect("connect to simulated ECU")
}

#[test]
fn iaw_5am_fast_init_full_session() {
    let def = load_def("ducati-iaw-5am");
    assert_eq!(def.ecu.manufacturer, "Ducati");
    let ecu_address = def.init.as_ref().unwrap().ecu_address;
    let tester_address = def.init.as_ref().unwrap().tester_address;

    let mut session = connect_sim(
        def,
        SimConfig {
            ecu_address,
            tester_address,
            ident: "IAW 5AM SIM v0.1".to_string(),
            ..SimConfig::default()
        },
    );
    assert_eq!(session.identity().text, "IAW 5AM SIM v0.1");

    let dtcs = session.read_dtcs().expect("read DTCs");
    assert_eq!(dtcs.len(), 2);
    assert!(dtcs[0]
        .description
        .as_deref()
        .unwrap_or("")
        .contains("Throttle"));
    // Parity fix: Ducati DTCs must carry causes/checks too, not just MV's.
    assert!(!dtcs[0].causes.is_empty());
    assert!(!dtcs[0].checks.is_empty());

    let readings: Vec<_> = session
        .poll_all_channels()
        .into_iter()
        .collect::<Result<_, _>>()
        .expect("all 5AM channels decode");
    assert_eq!(readings.len(), 4); // rpm, tps, ect, batt

    let rpm_spec = session
        .definition()
        .channels
        .iter()
        .find(|c| c.key == "rpm")
        .and_then(|c| c.spec.as_ref())
        .expect("rpm should carry a reference spec");
    assert!(rpm_spec.min.is_some() && rpm_spec.max.is_some());

    session.enable_service_mode();
    match session.run_routine("tps_reset", true) {
        Err(AppError::Safety(SafetyViolation::EngineRunning { .. })) => {}
        other => panic!("expected EngineRunning (engine idling by default), got {other:?}"),
    }
}

#[test]
fn iaw_59m_slow_init_full_session() {
    let def = load_def("ducati-iaw-59m");
    let ecu_address = def.init.as_ref().unwrap().ecu_address;
    let tester_address = def.init.as_ref().unwrap().tester_address;

    let mut session = connect_sim(
        def,
        SimConfig {
            ecu_address,
            tester_address,
            ident: "IAW 59M SIM v0.1".to_string(),
            engine_running: false,
            ..SimConfig::default()
        },
    );
    assert_eq!(session.identity().text, "IAW 59M SIM v0.1");

    let dtcs = session.read_dtcs().expect("read DTCs");
    assert_eq!(dtcs.len(), 2);

    session
        .clear_dtcs(true)
        .expect_err("clear should be blocked before service mode");
    session.enable_service_mode();
    session
        .clear_dtcs(true)
        .expect("clear DTCs once in service mode");
    assert!(session.read_dtcs().expect("re-read").is_empty());

    // Engine off in this run, so the low-risk actuator test should succeed.
    let response = session
        .run_routine("fuel_pump_test", true)
        .expect("fuel pump test runs with engine off");
    assert_eq!(response[0], 0x70);
}
