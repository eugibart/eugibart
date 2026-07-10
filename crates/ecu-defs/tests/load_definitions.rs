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
}
