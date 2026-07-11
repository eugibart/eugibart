//! The Sync Assistant's core promise: a vacuum gauge and the ECU session run
//! side by side — two independent "devices" feeding one workflow. This test
//! proves the dual-connection story end-to-end with no hardware: the mock
//! vacuum gauge converges toward balance while the simulated ECU keeps
//! serving live RPM, exactly what the Sync screen polls.

use std::path::Path;
use std::time::Duration;

use motodiag_app_core::vacuum::{MockVacuumGauge, VacuumGauge};
use motodiag_app_core::{ConnectOptions, DiagSession};
use motodiag_ecu_defs::{EcuDefinition, Registry};
use motodiag_ecu_sim::{spawn_on_link, SimConfig, Simulator};
use motodiag_kwp2000::init::FastInitConfig;
use motodiag_transport::mock;

fn load_brutale_def() -> EcuDefinition {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../definitions");
    Registry::load_dir(&dir)
        .unwrap()
        .get("mv-5sm-brutale-910")
        .unwrap()
        .clone()
}

#[test]
fn vacuum_gauge_and_ecu_session_run_side_by_side() {
    // ECU session on "port 1".
    let (tester, ecu) = mock::pair();
    let _sim = spawn_on_link(Simulator::new(SimConfig::default()), ecu);
    let mut session = DiagSession::connect(
        Box::new(tester),
        load_brutale_def(),
        ConnectOptions {
            fast_init: Some(FastInitConfig {
                idle_before: Duration::from_millis(1),
                low_time: Duration::from_millis(1),
                high_time: Duration::from_millis(1),
            }),
            ..Default::default()
        },
    )
    .expect("connect to simulated ECU");

    // Vacuum gauge on "port 2".
    let mut gauge = MockVacuumGauge::default();

    // Interleave polls the way the Sync screen does: vacuum + rpm each tick.
    let mut spreads = Vec::new();
    for _ in 0..40 {
        let vac = gauge
            .read(Duration::from_millis(10))
            .expect("vacuum sample");
        assert_eq!(vac.channels_kpa.len(), 4);
        spreads.push(vac.spread_kpa());

        let rpm = session.read_channel("rpm").expect("live rpm during sync");
        assert!(rpm.value > 1000.0, "engine idling while syncing");
    }

    // The mock gauge tells a converging story — the spread must shrink over
    // the session, mirroring what a real balancing job looks like.
    let early: f64 = spreads[..5].iter().sum::<f64>() / 5.0;
    let late: f64 = spreads[spreads.len() - 5..].iter().sum::<f64>() / 5.0;
    assert!(
        late < early,
        "spread should converge (early avg {early:.2} kPa, late avg {late:.2} kPa)"
    );
}
