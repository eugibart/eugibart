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
    // Shared across the whole pre-Euro3 5SM range, not just the 910.
    assert!(brutale.ecu.models.iter().any(|m| m.contains("750")));
    assert!(brutale.ecu.models.iter().any(|m| m.contains("1078")));
    assert!(brutale.ecu.models.iter().any(|m| m.contains("F4")));
    let rpm = brutale
        .channels
        .iter()
        .find(|c| c.key == "rpm")
        .expect("rpm channel");
    assert!(rpm.spec.is_some(), "rpm should carry a reference spec");
    let tps_reset = brutale
        .routines
        .iter()
        .find(|r| r.key == "tps_reset")
        .unwrap();
    assert!(
        !tps_reset.procedure.is_empty(),
        "tps_reset should have a step-by-step procedure"
    );

    let f4_312r = registry
        .get("mv-7bm-f4-312r")
        .expect("F4 312R (IAW 7BM) stub present");
    assert_eq!(f4_312r.ecu.manufacturer, "MV Agusta");
    assert!(!f4_312r.ecu.verified);

    let iaw_5am = registry
        .get("ducati-iaw-5am")
        .expect("Ducati 5AM definition present");
    assert_eq!(iaw_5am.ecu.manufacturer, "Ducati");
    assert!(!iaw_5am.ecu.models.is_empty());
    assert!(iaw_5am.channels.iter().any(|c| c.key == "rpm"));
    // Parity: causes/checks must exist here too, not just on the MV file.
    assert!(iaw_5am
        .dtc
        .table
        .iter()
        .any(|e| !e.causes.is_empty() && !e.checks.is_empty()));

    let iaw_59m = registry
        .get("ducati-iaw-59m")
        .expect("Ducati 59M definition present");
    assert!(matches!(
        iaw_59m.init.as_ref().unwrap().method,
        motodiag_ecu_defs::schema::InitMethod::Slow5Baud
    ));
    assert!(iaw_59m
        .dtc
        .table
        .iter()
        .any(|e| !e.causes.is_empty() && !e.checks.is_empty()));

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

/// The shipped bike catalog and mod guidance must parse, validate, and keep
/// referential integrity against the shipped definitions — this is the CI
/// gate that keeps the wizard's data honest.
#[test]
fn shipped_catalog_and_mod_guidance_load_and_cross_validate() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../definitions");
    let registry = Registry::load_dir(&dir).expect("definitions must parse and validate");

    let catalog_text =
        std::fs::read_to_string(dir.join("catalog.toml")).expect("catalog.toml readable");
    let catalog = motodiag_ecu_defs::BikeCatalog::from_toml(&catalog_text, "catalog.toml")
        .expect("catalog parses");
    catalog
        .validate(&registry)
        .expect("catalog cross-validates against shipped definitions");

    // Every claim in the coverage table shows up structurally.
    assert!(catalog.bikes.len() >= 10);
    let brands: Vec<&str> = catalog.bikes.iter().map(|b| b.brand.as_str()).collect();
    assert!(brands.contains(&"MV Agusta") && brands.contains(&"Ducati"));
    let gap = catalog
        .bikes
        .iter()
        .find(|b| b.model.contains("pre-2003"))
        .expect("pre-2003 F4 750 gap entry present");
    assert!(gap.definition_id.is_none());
    assert!(gap.gap_note.as_deref().is_some_and(|n| n.contains("1.6M")));
    assert!(catalog.bikes.iter().any(
        |b| b.definition_id.as_deref() == Some("mv-5sm-brutale-910") && b.model.contains("910")
    ));

    let mods_text = std::fs::read_to_string(dir.join("mods.toml")).expect("mods.toml readable");
    let guidance =
        motodiag_ecu_defs::ModGuidance::from_toml(&mods_text, "mods.toml").expect("mods parses");
    guidance
        .validate(&registry)
        .expect("mod guidance cross-validates against shipped definitions");
    assert!(!guidance.adjustments.is_empty());
    assert!(!guidance.procedure_notes.is_empty());
    assert!(!guidance.dtc_notes.is_empty());
    // Community content must self-describe as unverified in its conditions.
    for adj in &guidance.adjustments {
        for over in &adj.channel_overrides {
            assert!(
                over.condition.contains("unverified"),
                "community condition must self-describe: {}",
                over.condition
            );
        }
    }
}
