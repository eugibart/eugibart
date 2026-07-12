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
    // Access guides: the "where do I plug in?" data the UI draws diagrams
    // from. Both guides must survive schema changes; the connector one must
    // keep pointing at the community-reported under-tank-right zone.
    let conn = brutale
        .connector_access
        .as_ref()
        .expect("5SM connector access guide present");
    assert_eq!(conn.zone.as_str(), "under-tank-right");
    assert!(!conn.steps.is_empty() && !conn.tools.is_empty());
    assert!(conn.verify_note.is_some(), "location honesty note required");
    assert!(brutale.vacuum_access.is_some(), "sync port guide present");
    assert!(
        brutale
            .routines
            .iter()
            .find(|r| r.key == "tps_reset")
            .is_some_and(|r| !r.tools.is_empty()),
        "tps_reset lists its tools"
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

    // The corrected catalog: mis-mapped bikes became documented gaps.
    let hm796 = catalog
        .bikes
        .iter()
        .find(|b| b.model.contains("Hypermotard 796"))
        .expect("Hypermotard 796 entry present");
    assert!(
        hm796.definition_id.is_none()
            && hm796
                .gap_note
                .as_deref()
                .is_some_and(|n| n.contains("Siemens")),
        "Hypermotard 796 must be a Siemens gap entry, not a 5AM claim"
    );
    for model in ["916", "748", "996"] {
        let bike = catalog
            .bikes
            .iter()
            .find(|b| b.model == model)
            .unwrap_or_else(|| panic!("{model} entry present"));
        assert!(
            bike.definition_id.is_none(),
            "{model} runs P8/1.6M — must be a gap entry, not mapped to the 59M"
        );
    }
    // ...while the verified additions resolve to real definitions.
    for (model, def) in [
        ("Monster 695", "ducati-iaw-5am"),
        ("SportClassic GT1000", "ducati-iaw-5am"),
        ("749", "ducati-iaw-59m"),
        ("999", "ducati-iaw-59m"),
        ("ST4S", "ducati-iaw-59m"),
    ] {
        assert!(
            catalog
                .bikes
                .iter()
                .any(|b| b.model == model && b.definition_id.as_deref() == Some(def)),
            "{model} should map to {def}"
        );
    }

    let mods_text = std::fs::read_to_string(dir.join("mods.toml")).expect("mods.toml readable");
    let guidance =
        motodiag_ecu_defs::ModGuidance::from_toml(&mods_text, "mods.toml").expect("mods parses");
    guidance
        .validate(&registry)
        .expect("mod guidance cross-validates against shipped definitions");
    // Adjustments are deliberately empty: research corroborated the earlier
    // numeric bounds' direction but no source quotes the numbers, so they
    // were cut (cited-or-cut policy). Notes carry the knowledge instead.
    assert!(guidance.adjustments.is_empty());
    assert!(!guidance.procedure_notes.is_empty());
    assert!(!guidance.dtc_notes.is_empty());
    // Every shipped community claim must carry a named https source
    // (validate() enforces this; assert it here too so the policy is
    // visible in the test, not just the validator).
    for note in &guidance.procedure_notes {
        assert!(!note.source.trim().is_empty() && note.source_url.starts_with("https://"));
    }
    for note in &guidance.dtc_notes {
        assert!(!note.source.trim().is_empty() && note.source_url.starts_with("https://"));
    }
}
