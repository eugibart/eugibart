//! Bike catalog and community mod guidance.
//!
//! Two data files back the "tell me about your bike" flow:
//!
//! - `definitions/catalog.toml` — a structured brand → model → year map that
//!   resolves to an ECU definition id (or to an honest `gap_note` when we
//!   can't talk to that bike yet). Purely descriptive: nothing in this schema
//!   can make the app send a byte.
//! - `definitions/mods.toml` — editorial community knowledge about modified
//!   bikes (open exhausts, dedicated EPROMs, high-flow filters): adjusted
//!   reference ranges, service-procedure caveats, and DTC likely-causes.
//!
//! Everything in `mods.toml` is unverified by construction: this schema has
//! no `verified` field on purpose, so community folklore can never be
//! promoted to fact by flipping a flag. Conditions must self-describe (the
//! UI additionally labels every use "community reference, unverified").
//!
//! Both files are cross-validated against the loaded [`Registry`]: every
//! `definition_id`, channel key, and routine key must resolve — the same
//! no-dangling-references discipline the service-ID allowlist embodies.

use serde::{Deserialize, Serialize};

use crate::{DefsError, Registry, Result};

/// The bike catalog: which real-world bikes map to which ECU definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BikeCatalog {
    pub bikes: Vec<CatalogBike>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogBike {
    pub brand: String,
    pub model: String,
    /// Display-only trim/variant names ("910R", "910S").
    #[serde(default)]
    pub variants: Vec<String>,
    pub year_from: u16,
    /// `None` = "onwards".
    #[serde(default)]
    pub year_to: Option<u16>,
    /// ECU definition this bike resolves to. `None` for documented gaps.
    #[serde(default)]
    pub definition_id: Option<String>,
    /// Honest explanation when we can't talk to this bike (yet). Required
    /// when `definition_id` is absent; allowed alongside one as a caveat.
    #[serde(default)]
    pub gap_note: Option<String>,
}

/// Community guidance keyed off a bike profile's modifications.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModGuidance {
    #[serde(default)]
    pub adjustments: Vec<SpecAdjustment>,
    #[serde(default)]
    pub procedure_notes: Vec<ProcedureNote>,
    #[serde(default)]
    pub dtc_notes: Vec<DtcNote>,
}

/// Which modification values an entry applies to. An entry matches a profile
/// iff, for every field *present*, the profile's value is in the listed set.
/// Exhaust material (inox vs titanium) is deliberately not matchable — it
/// doesn't change fueling, only sound and weight.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModRequires {
    #[serde(default)]
    pub exhaust: Option<Vec<ExhaustMod>>,
    #[serde(default)]
    pub eprom: Option<Vec<EpromMod>>,
    #[serde(default)]
    pub air_filter: Option<Vec<AirFilterMod>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExhaustMod {
    Stock,
    SlipOnOpen,
    FullSystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EpromMod {
    Stock,
    DealerEprom,
    CustomMap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AirFilterMod {
    Stock,
    HighFlow,
}

/// Mod-combo-driven replacement reference ranges for live-data channels.
/// When several adjustments match the same channel, first-in-file wins —
/// keep the file ordered most-specific first.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecAdjustment {
    pub definition_id: String,
    pub requires: ModRequires,
    pub channel_overrides: Vec<ChannelOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelOverride {
    /// Channel key in the referenced definition.
    pub channel: String,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub target: Option<f64>,
    /// Must self-describe as community-sourced, e.g.
    /// "warm idle, open exhaust on stock EPROM — community reference, unverified".
    pub condition: String,
    /// Plain-language rationale shown alongside the range.
    #[serde(default)]
    pub note: Option<String>,
    /// Where this claim comes from — site name shown in the UI ("mvagusta.net").
    /// REQUIRED: an unsourced community claim cannot ship (see validate()).
    pub source: String,
    /// The specific page/thread backing the claim. Must be https.
    pub source_url: String,
}

/// Extra step/caveat woven into a service routine's procedure for matching
/// mod combos.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcedureNote {
    pub definition_id: String,
    /// Routine key in the referenced definition.
    pub routine: String,
    pub requires: ModRequires,
    pub note: String,
    /// Site name shown in the UI. REQUIRED — see ChannelOverride::source.
    pub source: String,
    /// Must be https.
    pub source_url: String,
}

/// Extra likely-cause/check for a DTC when the bike's mods make it probable.
/// The code does NOT have to exist in the definition's DTC table — a mod
/// note is most useful exactly for codes we haven't documented.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DtcNote {
    pub definition_id: String,
    pub code: u16,
    pub requires: ModRequires,
    pub cause: String,
    pub check: String,
    /// Site name shown in the UI. REQUIRED — see ChannelOverride::source.
    pub source: String,
    /// Must be https.
    pub source_url: String,
}

/// A community claim without provenance is a rumor. Every guidance entry
/// must say where it came from, and the link must be a real https URL.
fn validate_source(source: &str, source_url: &str) -> std::result::Result<(), String> {
    if source.trim().is_empty() {
        return Err("empty source — every community claim needs a named source".into());
    }
    if !source_url.starts_with("https://") {
        return Err(format!(
            "source_url \"{source_url}\" must be an https:// URL"
        ));
    }
    Ok(())
}

impl BikeCatalog {
    pub fn from_toml(text: &str, origin: &str) -> Result<Self> {
        let catalog: Self = toml::from_str(text).map_err(|source| DefsError::Parse {
            path: origin.to_string(),
            source: Box::new(source),
        })?;
        Ok(catalog)
    }

    pub fn validate(&self, registry: &Registry) -> Result<()> {
        for bike in &self.bikes {
            let label = format!("{} {}", bike.brand, bike.model);
            let invalid = |reason: String| DefsError::Invalid {
                id: label.clone(),
                reason,
            };
            if bike.definition_id.is_none() && bike.gap_note.is_none() {
                return Err(invalid(
                    "needs a definition_id or a gap_note — a bike we can neither talk to \
                     nor explain is a silent dead end"
                        .into(),
                ));
            }
            if let Some(id) = &bike.definition_id {
                if registry.get(id).is_none() {
                    return Err(invalid(format!("unknown definition_id \"{id}\"")));
                }
            }
            if let Some(to) = bike.year_to {
                if bike.year_from > to {
                    return Err(invalid(format!(
                        "year_from {} is after year_to {to}",
                        bike.year_from
                    )));
                }
            }
        }
        Ok(())
    }
}

impl ModGuidance {
    pub fn from_toml(text: &str, origin: &str) -> Result<Self> {
        let guidance: Self = toml::from_str(text).map_err(|source| DefsError::Parse {
            path: origin.to_string(),
            source: Box::new(source),
        })?;
        Ok(guidance)
    }

    pub fn validate(&self, registry: &Registry) -> Result<()> {
        let invalid = |id: &str, reason: String| DefsError::Invalid {
            id: id.to_string(),
            reason,
        };
        let def_for = |id: &str| {
            registry
                .get(id)
                .ok_or_else(|| invalid(id, format!("unknown definition_id \"{id}\"")))
        };

        for adj in &self.adjustments {
            let def = def_for(&adj.definition_id)?;
            if adj.channel_overrides.is_empty() {
                return Err(invalid(
                    &adj.definition_id,
                    "adjustment has no channel_overrides".into(),
                ));
            }
            for over in &adj.channel_overrides {
                if !def.channels.iter().any(|c| c.key == over.channel) {
                    return Err(invalid(
                        &adj.definition_id,
                        format!("adjustment references unknown channel \"{}\"", over.channel),
                    ));
                }
                if over.min.is_none() && over.max.is_none() && over.target.is_none() {
                    return Err(invalid(
                        &adj.definition_id,
                        format!(
                            "channel override \"{}\" sets none of min/max/target",
                            over.channel
                        ),
                    ));
                }
                if over.condition.trim().is_empty() {
                    return Err(invalid(
                        &adj.definition_id,
                        format!(
                            "channel override \"{}\" has an empty condition — a range \
                             without its condition is meaningless",
                            over.channel
                        ),
                    ));
                }
                validate_source(&over.source, &over.source_url)
                    .map_err(|reason| invalid(&adj.definition_id, reason))?;
            }
        }

        for note in &self.procedure_notes {
            let def = def_for(&note.definition_id)?;
            if !def.routines.iter().any(|r| r.key == note.routine) {
                return Err(invalid(
                    &note.definition_id,
                    format!(
                        "procedure note references unknown routine \"{}\"",
                        note.routine
                    ),
                ));
            }
            if note.note.trim().is_empty() {
                return Err(invalid(
                    &note.definition_id,
                    format!("procedure note for \"{}\" is empty", note.routine),
                ));
            }
            validate_source(&note.source, &note.source_url)
                .map_err(|reason| invalid(&note.definition_id, reason))?;
        }

        for note in &self.dtc_notes {
            def_for(&note.definition_id)?;
            if note.cause.trim().is_empty() || note.check.trim().is_empty() {
                return Err(invalid(
                    &note.definition_id,
                    format!(
                        "DTC note for code {:#06X} needs both a cause and a check",
                        note.code
                    ),
                ));
            }
            validate_source(&note.source, &note.source_url)
                .map_err(|reason| invalid(&note.definition_id, reason))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        let mut reg = Registry::default();
        reg.add_toml(
            r#"
            [ecu]
            id = "test-ecu"
            name = "Test ECU"
            manufacturer = "Test"
            bus = "k-line"

            [init]
            method = "fast"
            ecu_address = 0x10
            baud = 10400

            [[channels]]
            key = "rpm"
            name = "Engine speed"
            unit = "rpm"
            request = [0x21, 0x01]
            offset = 2
            length = 2

            [[routines]]
            key = "co_trim"
            name = "CO trim"
            risk = "high"
            request = [0x31, 0x01]
            "#,
            "inline-test",
        )
        .expect("test definition loads");
        reg
    }

    #[test]
    fn catalog_parses_and_validates() {
        let catalog = BikeCatalog::from_toml(
            r#"
            [[bikes]]
            brand = "MV Agusta"
            model = "Brutale 910"
            variants = ["910R"]
            year_from = 2005
            year_to = 2008
            definition_id = "test-ecu"

            [[bikes]]
            brand = "MV Agusta"
            model = "F4 750"
            year_from = 1999
            year_to = 2002
            gap_note = "Uses the Marelli 1.6M, which MotoDiag can't talk to yet."
            "#,
            "test",
        )
        .expect("parses");
        catalog.validate(&registry()).expect("validates");
        assert_eq!(catalog.bikes.len(), 2);
        assert!(catalog.bikes[1].definition_id.is_none());
    }

    #[test]
    fn catalog_rejects_unknown_definition() {
        let catalog = BikeCatalog::from_toml(
            r#"
            [[bikes]]
            brand = "MV Agusta"
            model = "Brutale 910"
            year_from = 2005
            definition_id = "nope"
            "#,
            "test",
        )
        .expect("parses");
        let err = catalog.validate(&registry()).unwrap_err().to_string();
        assert!(err.contains("unknown definition_id"), "{err}");
    }

    #[test]
    fn catalog_rejects_bike_with_neither_definition_nor_gap_note() {
        let catalog = BikeCatalog::from_toml(
            r#"
            [[bikes]]
            brand = "MV Agusta"
            model = "Mystery"
            year_from = 2005
            "#,
            "test",
        )
        .expect("parses");
        assert!(catalog.validate(&registry()).is_err());
    }

    #[test]
    fn catalog_rejects_inverted_years() {
        let catalog = BikeCatalog::from_toml(
            r#"
            [[bikes]]
            brand = "MV Agusta"
            model = "Brutale 910"
            year_from = 2008
            year_to = 2005
            definition_id = "test-ecu"
            "#,
            "test",
        )
        .expect("parses");
        let err = catalog.validate(&registry()).unwrap_err().to_string();
        assert!(err.contains("year_from"), "{err}");
    }

    #[test]
    fn catalog_rejects_unknown_fields() {
        assert!(BikeCatalog::from_toml(
            r#"
            [[bikes]]
            brand = "MV Agusta"
            model = "Brutale 910"
            year_from = 2005
            definition_id = "test-ecu"
            verified = true
            "#,
            "test",
        )
        .is_err());
    }

    fn valid_guidance_toml() -> &'static str {
        r#"
        [[adjustments]]
        definition_id = "test-ecu"
        [adjustments.requires]
        exhaust = ["slip-on-open", "full-system"]
        eprom = ["stock"]
        [[adjustments.channel_overrides]]
        channel = "rpm"
        min = 1200.0
        max = 1400.0
        condition = "warm idle, open exhaust on stock EPROM — community reference, unverified"
        note = "Open pipes on the stock map tend to idle slightly high."
        source = "mvagusta.net"
        source_url = "https://www.mvagusta.net/threads/example.1/"

        [[procedure_notes]]
        definition_id = "test-ecu"
        routine = "co_trim"
        note = "With an open exhaust the stock CO target reads lean at the silencer."
        source = "mvagusta.net"
        source_url = "https://www.mvagusta.net/threads/example.2/"
        [procedure_notes.requires]
        exhaust = ["slip-on-open", "full-system"]

        [[dtc_notes]]
        definition_id = "test-ecu"
        code = 0x0171
        cause = "Lean mixture from an open exhaust on the stock EPROM map"
        check = "Expected with open pipes on the stock map; a dedicated EPROM resolves it"
        source = "mvagusta.net"
        source_url = "https://www.mvagusta.net/threads/example.3/"
        [dtc_notes.requires]
        exhaust = ["slip-on-open", "full-system"]
        eprom = ["stock"]
        "#
    }

    #[test]
    fn guidance_parses_and_validates() {
        let guidance = ModGuidance::from_toml(valid_guidance_toml(), "test").expect("parses");
        guidance.validate(&registry()).expect("validates");
        assert_eq!(guidance.adjustments.len(), 1);
        assert_eq!(guidance.procedure_notes.len(), 1);
        assert_eq!(guidance.dtc_notes.len(), 1);
    }

    #[test]
    fn guidance_requires_sources_structurally() {
        // Omitting source entirely fails at PARSE time (required field).
        assert!(ModGuidance::from_toml(
            &valid_guidance_toml().replace("source = \"mvagusta.net\"\n", ""),
            "test",
        )
        .is_err());

        // An empty source name fails validation.
        let guidance = ModGuidance::from_toml(
            &valid_guidance_toml().replace("source = \"mvagusta.net\"", "source = \"  \""),
            "test",
        )
        .expect("parses");
        let err = guidance.validate(&registry()).unwrap_err().to_string();
        assert!(err.contains("empty source"), "{err}");

        // A non-https URL fails validation.
        let guidance = ModGuidance::from_toml(
            &valid_guidance_toml().replace("https://www.mvagusta.net", "http://www.mvagusta.net"),
            "test",
        )
        .expect("parses");
        let err = guidance.validate(&registry()).unwrap_err().to_string();
        assert!(err.contains("https"), "{err}");
    }

    #[test]
    fn guidance_rejects_typoed_mod_value() {
        // "slip-on" is not a valid ExhaustMod — enums make typos parse errors.
        assert!(ModGuidance::from_toml(
            &valid_guidance_toml().replace("slip-on-open", "slip-on"),
            "test",
        )
        .is_err());
    }

    #[test]
    fn guidance_rejects_unknown_channel_and_routine() {
        let guidance = ModGuidance::from_toml(
            &valid_guidance_toml().replace("\"rpm\"", "\"boost\""),
            "test",
        )
        .expect("parses");
        let err = guidance.validate(&registry()).unwrap_err().to_string();
        assert!(err.contains("unknown channel"), "{err}");

        let guidance = ModGuidance::from_toml(
            &valid_guidance_toml().replace("\"co_trim\"", "\"map_write\""),
            "test",
        )
        .expect("parses");
        let err = guidance.validate(&registry()).unwrap_err().to_string();
        assert!(err.contains("unknown routine"), "{err}");
    }

    #[test]
    fn guidance_rejects_override_without_bounds_or_empty_condition() {
        let no_bounds = valid_guidance_toml()
            .replace("min = 1200.0\n", "")
            .replace("max = 1400.0\n", "");
        let guidance = ModGuidance::from_toml(&no_bounds, "test").expect("parses");
        assert!(guidance.validate(&registry()).is_err());

        let empty_cond = valid_guidance_toml().replace(
            "condition = \"warm idle, open exhaust on stock EPROM — community reference, unverified\"",
            "condition = \"  \"",
        );
        let guidance = ModGuidance::from_toml(&empty_cond, "test").expect("parses");
        assert!(guidance.validate(&registry()).is_err());
    }

    #[test]
    fn guidance_has_no_verified_field_by_construction() {
        // Attempting to mark community content as verified must fail to parse.
        assert!(ModGuidance::from_toml(
            &valid_guidance_toml().replace(
                "note = \"Open pipes",
                "verified = true\nnote = \"Open pipes"
            ),
            "test",
        )
        .is_err());
    }
}
