//! End-to-end session tests against the simulated 5SM ECU over the in-memory
//! mock link. This is the M0 exit criterion: the full stack — fast init,
//! framing, session management, definitions, safety interlocks — exercised
//! without any hardware.

use std::path::Path;
use std::time::Duration;

use motodiag_app_core::safety::SafetyViolation;
use motodiag_app_core::{AppError, ConnectOptions, DiagSession};
use motodiag_ecu_defs::{EcuDefinition, Registry};
use motodiag_ecu_sim::{spawn_on_link, SimConfig, Simulator};
use motodiag_kwp2000::init::FastInitConfig;
use motodiag_transport::mock;

fn load_brutale_def() -> EcuDefinition {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../definitions");
    let registry = Registry::load_dir(&dir).expect("definitions directory must load");
    registry
        .get("mv-5sm-brutale-910")
        .expect("Brutale 910 definition present")
        .clone()
}

/// Fast-init timings shrunk so the suite stays quick; the protocol sequence
/// is identical to the real one.
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

fn connect_sim(config: SimConfig) -> (DiagSession, std::thread::JoinHandle<Simulator>) {
    let (tester, ecu) = mock::pair();
    let handle = spawn_on_link(Simulator::new(config), ecu);
    let session = DiagSession::connect(
        Box::new(tester),
        load_brutale_def(),
        quick_connect_options(),
    )
    .expect("connect to simulated ECU");
    (session, handle)
}

#[test]
fn connects_and_reads_identity() {
    let (session, _handle) = connect_sim(SimConfig::default());
    assert_eq!(session.identity().text, "IAW 5SM SIM v0.1");
}

#[test]
fn reads_and_maps_dtcs() {
    let (mut session, _handle) = connect_sim(SimConfig::default());
    let dtcs = session.read_dtcs().expect("read DTCs");
    assert_eq!(dtcs.len(), 2);
    assert_eq!(dtcs[0].code, 0x0115);
    // The definition's DTC table supplies the description plus the
    // plain-language causes/checks lists.
    assert!(dtcs[0]
        .description
        .as_deref()
        .unwrap_or("")
        .contains("Throttle"));
    assert!(
        !dtcs[0].causes.is_empty(),
        "TPS code should list likely causes"
    );
    assert!(dtcs[0].causes[0].contains("connector"));
    assert!(!dtcs[0].checks.is_empty(), "TPS code should list checks");
}

#[test]
fn polls_live_data_channels() {
    let (mut session, _handle) = connect_sim(SimConfig::default());
    let readings: Vec<_> = session
        .poll_all_channels()
        .into_iter()
        .collect::<Result<_, _>>()
        .expect("all channels decode");
    assert_eq!(readings.len(), 6);

    let get = |key: &str| readings.iter().find(|r| r.key == key).unwrap().value;
    assert!(get("rpm") > 1000.0 && get("rpm") < 2000.0, "idle rpm");
    assert!((get("ect") - 84.0).abs() < 1.5, "coolant ~84°C");
    assert!((get("batt") - 12.8).abs() < 0.2, "battery ~12.8V");
}

#[test]
fn write_operations_are_locked_down_by_default() {
    let (mut session, _handle) = connect_sim(SimConfig::default());

    // Read-only session: clearing DTCs is blocked before service mode.
    match session.clear_dtcs(true) {
        Err(AppError::Safety(SafetyViolation::ServiceModeDisabled)) => {}
        other => panic!("expected ServiceModeDisabled, got {other:?}"),
    }

    // Service mode alone is not enough — per-operation confirmation required.
    session.enable_service_mode();
    match session.clear_dtcs(false) {
        Err(AppError::Safety(SafetyViolation::ConfirmationRequired)) => {}
        other => panic!("expected ConfirmationRequired, got {other:?}"),
    }

    session.clear_dtcs(true).expect("clear DTCs once confirmed");
    assert!(session.read_dtcs().expect("re-read").is_empty());
}

#[test]
fn routine_blocked_while_engine_running() {
    // Default sim state: engine idling at ~1250 rpm.
    let (mut session, _handle) = connect_sim(SimConfig::default());
    session.enable_service_mode();

    match session.run_routine("fuel_pump_test", true) {
        Err(AppError::Safety(SafetyViolation::EngineRunning { rpm })) => {
            assert!(rpm > 0.0);
        }
        other => panic!("expected EngineRunning, got {other:?}"),
    }
}

#[test]
fn routine_runs_with_engine_off_and_good_battery() {
    let (mut session, _handle) = connect_sim(SimConfig {
        engine_running: false,
        ..SimConfig::default()
    });
    session.enable_service_mode();

    let response = session
        .run_routine("fuel_pump_test", true)
        .expect("actuator test runs");
    assert_eq!(response[0], 0x70); // positive response to 0x30
}

#[test]
fn rpm_spec_classifies_idle_reading_and_routines_carry_procedures() {
    let (mut session, _handle) = connect_sim(SimConfig::default());
    let def = session.definition().clone();

    let rpm_channel = def
        .channels
        .iter()
        .find(|c| c.key == "rpm")
        .expect("rpm channel present");
    let spec = rpm_channel.spec.as_ref().expect("rpm should carry a spec");

    let rpm = session.read_channel("rpm").expect("read live rpm");
    // The simulator idles around 1150-1450 rpm; the spec's condition string
    // explicitly says "confirm per model" — this just proves the spec is
    // wired through and classifiable, not that the number is verified.
    let classification = spec.in_range(rpm.value);
    assert!(classification.is_some(), "spec has bounds, should classify");

    let tps_reset = def.routines.iter().find(|r| r.key == "tps_reset").unwrap();
    assert!(
        !tps_reset.procedure.is_empty(),
        "tps_reset should carry a step-by-step procedure"
    );
    let co_trim = def.routines.iter().find(|r| r.key == "co_trim").unwrap();
    assert!(!co_trim.procedure.is_empty());
}

#[test]
fn tps_reset_blocked_on_low_battery() {
    let (mut session, _handle) = connect_sim(SimConfig {
        engine_running: false,
        battery_v: 11.2,
        ..SimConfig::default()
    });
    session.enable_service_mode();

    match session.run_routine("tps_reset", true) {
        Err(AppError::Safety(SafetyViolation::BatteryOutOfRange { measured, .. })) => {
            assert!((measured - 11.2).abs() < 0.2);
        }
        other => panic!("expected BatteryOutOfRange, got {other:?}"),
    }
}
