//! The shipped definition files must always parse and validate.

use std::path::Path;

use motodiag_ecu_defs::Registry;

#[test]
fn shipped_definitions_load_and_validate() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../definitions");
    let registry = Registry::load_dir(&dir).expect("definitions must parse and validate");

    assert!(registry.len() >= 3);
    let brutale = registry
        .get("mv-5sm-brutale-910")
        .expect("Brutale 910 definition present");
    assert_eq!(brutale.ecu.manufacturer, "MV Agusta");
    assert!(
        !brutale.ecu.verified,
        "must stay unverified until confirmed on a real bike"
    );
    assert!(brutale.channels.iter().any(|c| c.key == "rpm"));
    assert!(brutale.channels.iter().any(|c| c.key == "batt"));
    assert!(brutale.routines.iter().any(|r| r.key == "tps_reset"));
    assert!(
        brutale.init.is_some(),
        "K-line ECU must have an init config"
    );
    assert!(
        brutale.can.is_none(),
        "K-line ECU must not have a can config"
    );

    let iaw_5am = registry
        .get("ducati-iaw-5am")
        .expect("Ducati 5AM definition present");
    assert_eq!(iaw_5am.ecu.manufacturer, "Ducati");
    assert!(!iaw_5am.ecu.models.is_empty());
    assert!(iaw_5am.channels.iter().any(|c| c.key == "rpm"));

    let iaw_59m = registry
        .get("ducati-iaw-59m")
        .expect("Ducati 59M definition present");
    assert!(matches!(
        iaw_59m.init.as_ref().unwrap().method,
        motodiag_ecu_defs::schema::InitMethod::Slow5Baud
    ));

    let can_stub = registry
        .get("ducati-mc3-multistrada-can")
        .expect("Ducati CAN stub present");
    assert_eq!(can_stub.ecu.bus, motodiag_ecu_defs::schema::BusKind::Can);
    assert!(
        can_stub.init.is_none(),
        "CAN ECU must not have an init config"
    );
    let can_cfg = can_stub
        .can
        .as_ref()
        .expect("CAN ECU must have a can config");
    assert_eq!(can_cfg.tx_id, 0x7E0);
    assert_eq!(can_cfg.rx_id, 0x7E8);
}
