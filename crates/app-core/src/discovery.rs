//! Brute-force protocol discovery: ask the ECU directly instead of
//! reverse-engineering another tool's traffic.
//!
//! The Brutale 910 (and Ducati IAW) definitions ship with placeholder local
//! identifiers, DTC formats, and identification option bytes because nobody
//! has confirmed them against real hardware yet. Rather than requiring a
//! copy of JPDiag/TuneECU/VDSTS to sniff, this module scans the ECU's
//! request space directly and records everything — including a full wire
//! trace — so the results become both a human-readable report and a
//! replayable regression fixture in one run.

use std::time::Duration;

use motodiag_ecu_defs::EcuDefinition;
use motodiag_kwp2000::services::sid;
use motodiag_transport::trace::TraceSink;
use motodiag_transport::trace::TracingTransport;
use motodiag_transport::KLineTransport;

use crate::session::{init_kwp_session, ConnectOptions};
use crate::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeOutcome {
    /// `Ok` with the positive-response payload, or a description of why the
    /// probe didn't get one (negative response code, timeout, etc).
    pub result: std::result::Result<Vec<u8>, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalIdProbe {
    pub local_id: u8,
    pub outcome: ProbeOutcome,
    /// True if a second read ~200ms later returned different bytes — a
    /// strong hint this local ID is a live sensor rather than static
    /// configuration data (RPM/TPS/temps wander; VIN-type data doesn't).
    pub looks_live: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentificationProbe {
    pub option_byte: u8,
    pub outcome: ProbeOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DtcProbe {
    pub request: Vec<u8>,
    pub outcome: ProbeOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiscoveryReport {
    pub identification: Vec<IdentificationProbe>,
    pub dtc: Vec<DtcProbe>,
    pub local_identifiers: Vec<LocalIdProbe>,
}

impl DiscoveryReport {
    /// Local IDs that got a positive response — the ones worth turning into
    /// `[[channels]]` entries in the definition file.
    pub fn responsive_local_ids(&self) -> impl Iterator<Item = &LocalIdProbe> {
        self.local_identifiers
            .iter()
            .filter(|p| p.outcome.result.is_ok())
    }

    /// A plain-text summary suitable for printing to a terminal or saving
    /// alongside the wire trace fixture.
    pub fn summary(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();

        writeln!(out, "== ReadEcuIdentification option bytes ==").ok();
        for p in &self.identification {
            match &p.outcome.result {
                Ok(bytes) => writeln!(out, "  0x{:02X}: {:02X?}", p.option_byte, bytes).ok(),
                Err(reason) => writeln!(out, "  0x{:02X}: {reason}", p.option_byte).ok(),
            };
        }

        writeln!(out, "== ReadDTCByStatus variants ==").ok();
        for p in &self.dtc {
            match &p.outcome.result {
                Ok(bytes) => writeln!(out, "  {:02X?}: {:02X?}", p.request, bytes).ok(),
                Err(reason) => writeln!(out, "  {:02X?}: {reason}", p.request).ok(),
            };
        }

        let responsive: Vec<&LocalIdProbe> = self.responsive_local_ids().collect();
        writeln!(
            out,
            "== ReadDataByLocalIdentifier: {} of {} IDs responded ==",
            responsive.len(),
            self.local_identifiers.len()
        )
        .ok();
        for p in responsive {
            let bytes = p.outcome.result.as_ref().unwrap();
            let live = if p.looks_live {
                " (changed between polls — likely live)"
            } else {
                ""
            };
            writeln!(out, "  0x{:02X}: {:02X?}{live}", p.local_id, bytes).ok();
        }

        out
    }
}

#[derive(Debug, Clone)]
pub struct DiscoveryOptions {
    /// Inclusive range of local identifiers to probe. Defaults to the full
    /// byte range; narrow it if you already suspect where live data lives.
    pub local_id_range: std::ops::RangeInclusive<u8>,
    /// ReadEcuIdentification option bytes to try. Marelli ECUs commonly
    /// answer to 0x80 ("everything") but some only respond to specific
    /// per-table values.
    pub identification_options: Vec<u8>,
    /// ReadDTCByStatus request payloads to try (SID 0x18 + parameters).
    pub dtc_requests: Vec<Vec<u8>>,
    /// How long to wait between the two live-data polls used to guess
    /// whether a local ID is a live sensor.
    pub live_check_gap: Duration,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            local_id_range: 0x00..=0xFF,
            identification_options: vec![0x80, 0x86, 0x87, 0x88, 0x89, 0x8A],
            dtc_requests: vec![
                vec![sid::READ_DTC_BY_STATUS, 0x02, 0xFF, 0x00],
                vec![sid::READ_DTC_BY_STATUS, 0x00, 0xFF, 0x00],
                vec![sid::READ_DTC_BY_STATUS, 0x02, 0x00, 0x00],
            ],
            live_check_gap: Duration::from_millis(200),
        }
    }
}

/// Connect to `def`'s ECU and probe it. Every request/response — successes
/// and failures alike — flows through `trace_sink` if given, so the whole
/// run doubles as a wire-trace fixture (see `logging::WireTraceRecorder`).
pub fn run_discovery(
    transport: Box<dyn KLineTransport>,
    def: &EcuDefinition,
    connect_options: &ConnectOptions,
    discovery_options: &DiscoveryOptions,
    trace_sink: Option<Box<dyn TraceSink>>,
) -> Result<DiscoveryReport> {
    let transport: Box<dyn KLineTransport> = match trace_sink {
        Some(sink) => Box::new(TracingTransport::new(transport, sink)),
        None => transport,
    };

    let mut kwp = init_kwp_session(transport, def, connect_options)?;
    let mut report = DiscoveryReport::default();

    for &option_byte in &discovery_options.identification_options {
        kwp.transport_mut().flush_input().ok();
        let outcome = probe(kwp.request(&[sid::READ_ECU_IDENTIFICATION, option_byte]));
        report.identification.push(IdentificationProbe {
            option_byte,
            outcome,
        });
    }

    for request in &discovery_options.dtc_requests {
        kwp.transport_mut().flush_input().ok();
        let outcome = probe(kwp.request(request));
        report.dtc.push(DtcProbe {
            request: request.clone(),
            outcome,
        });
    }

    for local_id in discovery_options.local_id_range.clone() {
        kwp.transport_mut().flush_input().ok();
        let first = kwp.request(&[sid::READ_DATA_BY_LOCAL_ID, local_id]);

        let looks_live = if first.is_ok() {
            std::thread::sleep(discovery_options.live_check_gap);
            kwp.transport_mut().flush_input().ok();
            let second = kwp.request(&[sid::READ_DATA_BY_LOCAL_ID, local_id]);
            matches!((&first, &second), (Ok(a), Ok(b)) if a != b)
        } else {
            false
        };

        report.local_identifiers.push(LocalIdProbe {
            local_id,
            outcome: probe(first),
            looks_live,
        });
    }

    Ok(report)
}

fn probe(result: motodiag_kwp2000::Result<Vec<u8>>) -> ProbeOutcome {
    ProbeOutcome {
        result: result.map_err(|e| e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use motodiag_ecu_sim::{spawn_on_link, SimConfig, Simulator};
    use motodiag_kwp2000::init::FastInitConfig;
    use motodiag_transport::mock;

    fn quick_options() -> ConnectOptions {
        ConnectOptions {
            fast_init: Some(FastInitConfig {
                idle_before: Duration::from_millis(1),
                low_time: Duration::from_millis(1),
                high_time: Duration::from_millis(1),
            }),
            ..Default::default()
        }
    }

    fn load_brutale_def() -> EcuDefinition {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../definitions");
        motodiag_ecu_defs::Registry::load_dir(&dir)
            .unwrap()
            .get("mv-5sm-brutale-910")
            .unwrap()
            .clone()
    }

    #[test]
    fn finds_the_simulators_known_local_ids() {
        let (tester, ecu) = mock::pair();
        let _sim = spawn_on_link(Simulator::new(SimConfig::default()), ecu);

        let discovery_options = DiscoveryOptions {
            // Full 0x00..=0xFF would work but is slow in a test (each miss
            // waits out a protocol timeout); the simulator's known channels
            // all live in 0x01..=0x06, so scan a narrow, fast range.
            local_id_range: 0x00..=0x08,
            live_check_gap: Duration::from_millis(1),
            ..Default::default()
        };

        let report = run_discovery(
            Box::new(tester),
            &load_brutale_def(),
            &quick_options(),
            &discovery_options,
            None,
        )
        .expect("discovery run completes");

        let responsive: Vec<u8> = report.responsive_local_ids().map(|p| p.local_id).collect();
        assert_eq!(responsive, vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);

        // Only rpm (0x01) wanders each poll in the simulator; tps/ect/iat
        // /batt/map are held constant, so only rpm should be flagged live.
        let live: Vec<u8> = report
            .responsive_local_ids()
            .filter(|p| p.looks_live)
            .map(|p| p.local_id)
            .collect();
        assert_eq!(live, vec![0x01]);

        assert_eq!(report.local_identifiers.len(), 9); // 0x00..=0x08
        assert!(report.local_identifiers[0].outcome.result.is_err()); // 0x00 unsupported
    }

    #[test]
    fn identification_and_dtc_probes_run() {
        let (tester, ecu) = mock::pair();
        let _sim = spawn_on_link(Simulator::new(SimConfig::default()), ecu);

        let discovery_options = DiscoveryOptions {
            local_id_range: 0x01..=0x01, // don't care about local IDs here
            ..Default::default()
        };

        let report = run_discovery(
            Box::new(tester),
            &load_brutale_def(),
            &quick_options(),
            &discovery_options,
            None,
        )
        .unwrap();

        // The simulator answers 0x80 (and only 0x80) with its ident string.
        let hit = report
            .identification
            .iter()
            .find(|p| p.option_byte == 0x80)
            .unwrap();
        assert_eq!(
            hit.outcome.result.as_ref().unwrap()[2..],
            *b"IAW 5SM SIM v0.1"
        );

        // At least one DTC variant should succeed (the one matching the
        // definition's own read_request shape).
        assert!(report.dtc.iter().any(|p| p.outcome.result.is_ok()));
    }

    #[test]
    fn summary_is_readable_and_non_empty() {
        let (tester, ecu) = mock::pair();
        let _sim = spawn_on_link(Simulator::new(SimConfig::default()), ecu);
        let discovery_options = DiscoveryOptions {
            local_id_range: 0x01..=0x02,
            identification_options: vec![0x80],
            dtc_requests: vec![vec![sid::READ_DTC_BY_STATUS, 0x02, 0xFF, 0x00]],
            live_check_gap: Duration::from_millis(1),
        };
        let report = run_discovery(
            Box::new(tester),
            &load_brutale_def(),
            &quick_options(),
            &discovery_options,
            None,
        )
        .unwrap();

        let summary = report.summary();
        assert!(summary.contains("ReadEcuIdentification"));
        assert!(summary.contains("0x01"));
    }

    #[test]
    fn wire_trace_sink_captures_the_whole_run() {
        use motodiag_transport::trace::{TraceEvent, TraceSink};
        use std::sync::{Arc, Mutex};

        #[derive(Clone, Default)]
        struct SharedSink(Arc<Mutex<Vec<TraceEvent>>>);
        impl TraceSink for SharedSink {
            fn record(&mut self, event: TraceEvent) {
                self.0.lock().unwrap().push(event);
            }
        }

        let (tester, ecu) = mock::pair();
        let _sim = spawn_on_link(Simulator::new(SimConfig::default()), ecu);
        let discovery_options = DiscoveryOptions {
            local_id_range: 0x01..=0x01,
            identification_options: vec![0x80],
            dtc_requests: vec![],
            live_check_gap: Duration::from_millis(1),
        };

        let sink = SharedSink::default();
        run_discovery(
            Box::new(tester),
            &load_brutale_def(),
            &quick_options(),
            &discovery_options,
            Some(Box::new(sink.clone())),
        )
        .expect("discovery with a trace sink attached still succeeds");

        let events = sink.0.lock().unwrap();
        // Init handshake + identification probe + one local-id probe, at
        // minimum: enough to prove the whole run — not just individual
        // requests — flowed through the tracer.
        assert!(
            events.len() > 4,
            "expected multiple traced events, got {}",
            events.len()
        );
    }
}
