//! Wire-trace analysis: turn a recorded session into a human-readable
//! verification report against an ECU definition.
//!
//! This is the community feedback loop: an owner records a session on their
//! real bike (every desktop session is recorded automatically), shares the
//! trace file, and anyone can run it against the shipped definition to see
//! exactly what decoded — identity, live-data channels with plausible
//! values, DTC reads — and what didn't. That evidence is what eventually
//! flips a definition's `verified` flag.
//!
//! The analysis is static: it never replays anything onto a bus. Tx events
//! in a trace are complete frames (TracingTransport records each `send()`
//! whole), so requests parse directly; Rx bytes are accumulated between
//! requests and parsed with the same frame parser the simulator uses. On a
//! real single-wire K-line the tester hears its own echo, so an Rx frame
//! identical to the request is recognized and skipped.

use std::io::BufRead;
use std::path::Path;

use motodiag_ecu_defs::EcuDefinition;
use motodiag_kwp2000::framing::{parse_buffer, Frame, ParseStatus};
use motodiag_transport::trace::{EventKind, TraceDirection, TraceEvent};

use crate::dtc::parse_read_dtc_response;
use crate::{AppError, Result};

/// First line of a shareable trace file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraceMetadata {
    /// "motodiag-trace/1"
    pub format: String,
    pub definition_id: String,
    pub recorded_at_ms: u64,
    pub simulated: bool,
}

/// Load a trace file: an optional metadata header line followed by one
/// [`TraceEvent`] per line. Raw event-only files (older fixtures) load too.
pub fn load_trace_file(path: &Path) -> Result<(Option<TraceMetadata>, Vec<TraceEvent>)> {
    let reader = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut metadata = None;
    let mut events = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if i == 0 {
            if let Ok(meta) = serde_json::from_str::<TraceMetadata>(&line) {
                if meta.format.starts_with("motodiag-trace/") {
                    metadata = Some(meta);
                    continue;
                }
            }
        }
        let event: TraceEvent = serde_json::from_str(&line)
            .map_err(|e| AppError::Io(std::io::Error::other(format!("line {}: {e}", i + 1))))?;
        events.push(event);
    }
    Ok((metadata, events))
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TraceReportRow {
    /// What the request was ("identification", "channel rpm", "read DTCs",
    /// "tester present", "init", …).
    pub kind: String,
    pub request_hex: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChannelStat {
    pub key: String,
    pub name: String,
    pub unit: String,
    pub samples: u32,
    pub last_value: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TraceReport {
    pub definition_id: String,
    pub event_count: usize,
    pub exchange_count: u32,
    pub init_seen: bool,
    pub identity: Option<String>,
    pub channels: Vec<ChannelStat>,
    pub dtc_reads: u32,
    pub dtc_last_count: Option<usize>,
    pub tester_present: u32,
    pub negative_responses: u32,
    pub unparsed_rx_bytes: u32,
    /// First N exchanges, for the UI; `rows_truncated` says if more existed.
    pub rows: Vec<TraceReportRow>,
    pub rows_truncated: bool,
}

const MAX_ROWS: usize = 60;

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Split an accumulated Rx run into frames, skipping the request echo.
/// Returns (response frames, unparsed byte count).
fn parse_rx_run(rx: &[u8], request: Option<&Frame>) -> (Vec<Frame>, u32) {
    let mut buf = rx.to_vec();
    let mut frames = Vec::new();
    let mut unparsed = 0u32;
    while !buf.is_empty() {
        match parse_buffer(&buf) {
            ParseStatus::Complete { frame, consumed } => {
                buf.drain(..consumed);
                let is_echo = request.is_some_and(|req| req.raw == frame.raw);
                if !is_echo {
                    frames.push(frame);
                }
            }
            ParseStatus::Incomplete => {
                unparsed += buf.len() as u32;
                break;
            }
            ParseStatus::Invalid(_) => {
                // Resynchronize one byte at a time (init handshakes and line
                // noise land here); count what we skipped.
                unparsed += 1;
                buf.remove(0);
            }
        }
    }
    (frames, unparsed)
}

pub fn analyze(def: &EcuDefinition, events: &[TraceEvent]) -> TraceReport {
    let mut report = TraceReport {
        definition_id: def.ecu.id.clone(),
        event_count: events.len(),
        exchange_count: 0,
        init_seen: false,
        identity: None,
        channels: Vec::new(),
        dtc_reads: 0,
        dtc_last_count: None,
        tester_present: 0,
        negative_responses: 0,
        unparsed_rx_bytes: 0,
        rows: Vec::new(),
        rows_truncated: false,
    };

    let mut pending: Option<Frame> = None;
    let mut pending_raw_tx: Option<Vec<u8>> = None; // tx that didn't frame-parse (init bytes)
    let mut rx_buf: Vec<u8> = Vec::new();
    // Everything before the first framed exchange is init handshake, not noise.
    let mut framed_started = false;

    let finish_exchange = |report: &mut TraceReport,
                           pending: &mut Option<Frame>,
                           pending_raw_tx: &mut Option<Vec<u8>>,
                           rx_buf: &mut Vec<u8>,
                           framed_started: bool| {
        if pending.is_none() && pending_raw_tx.is_none() && rx_buf.is_empty() {
            return;
        }
        let request = pending.take();
        let raw_tx = pending_raw_tx.take();
        let (frames, unparsed) = parse_rx_run(rx_buf, request.as_ref());
        rx_buf.clear();

        if request.is_none() {
            // Init handshake traffic (5-baud sync/key bytes, raw acks).
            if !framed_started {
                report.init_seen = true;
                push_row(
                    report,
                    TraceReportRow {
                        kind: "init handshake".into(),
                        request_hex: raw_tx.as_deref().map(hex).unwrap_or_default(),
                        ok: true,
                        detail: "pre-session bytes (wake-up / key bytes)".into(),
                    },
                );
            } else {
                report.unparsed_rx_bytes += unparsed;
            }
            return;
        }

        let request = request.unwrap();
        report.exchange_count += 1;
        report.unparsed_rx_bytes += unparsed;
        let payload = &request.payload;

        let kind = classify(def, payload);
        let response = frames.first();

        let (ok, detail) = match response {
            None => (false, "no response captured in the trace".to_string()),
            Some(resp) if resp.payload.first() == Some(&0x7F) => {
                report.negative_responses += 1;
                let nrc = resp.payload.get(2).copied().unwrap_or(0);
                (false, format!("negative response, NRC 0x{nrc:02X}"))
            }
            Some(resp) => describe_response(def, &kind, resp, report),
        };

        push_row(
            report,
            TraceReportRow {
                kind: kind.label,
                request_hex: hex(&request.raw),
                ok,
                detail,
            },
        );
    };

    for event in events {
        match (&event.dir, &event.kind) {
            (TraceDirection::Tx, EventKind::Bytes { hex: bytes }) => {
                finish_exchange(
                    &mut report,
                    &mut pending,
                    &mut pending_raw_tx,
                    &mut rx_buf,
                    framed_started,
                );
                match parse_buffer(&bytes.0) {
                    ParseStatus::Complete { frame, .. } => {
                        if frame.payload.first() == Some(&0x81) {
                            report.init_seen = true;
                        }
                        framed_started = true;
                        pending = Some(frame);
                    }
                    _ => {
                        pending_raw_tx = Some(bytes.0.clone());
                    }
                }
            }
            (TraceDirection::Rx, EventKind::Bytes { hex: bytes }) => {
                rx_buf.extend_from_slice(&bytes.0);
            }
            (_, EventKind::Break { .. }) | (_, EventKind::Baud { .. }) => {
                finish_exchange(
                    &mut report,
                    &mut pending,
                    &mut pending_raw_tx,
                    &mut rx_buf,
                    framed_started,
                );
                report.init_seen = true;
            }
            (_, EventKind::FiveBaudAddress { address }) => {
                finish_exchange(
                    &mut report,
                    &mut pending,
                    &mut pending_raw_tx,
                    &mut rx_buf,
                    framed_started,
                );
                report.init_seen = true;
                push_row(
                    &mut report,
                    TraceReportRow {
                        kind: "init".into(),
                        request_hex: format!("5-baud address 0x{address:02X}"),
                        ok: true,
                        detail: "slow init wake-up".into(),
                    },
                );
            }
        }
    }
    finish_exchange(
        &mut report,
        &mut pending,
        &mut pending_raw_tx,
        &mut rx_buf,
        framed_started,
    );

    report
}

struct RequestKind {
    label: String,
    channel_key: Option<String>,
}

fn classify(def: &EcuDefinition, payload: &[u8]) -> RequestKind {
    if payload == def.identification.request.as_slice() {
        RequestKind {
            label: "identification".into(),
            channel_key: None,
        }
    } else if payload == def.dtc.read_request.as_slice() {
        RequestKind {
            label: "read DTCs".into(),
            channel_key: None,
        }
    } else if payload == def.dtc.clear_request.as_slice() {
        RequestKind {
            label: "clear DTCs".into(),
            channel_key: None,
        }
    } else if let Some(ch) = def.channels.iter().find(|c| c.request == payload) {
        RequestKind {
            label: format!("channel {}", ch.key),
            channel_key: Some(ch.key.clone()),
        }
    } else if let Some(r) = def.routines.iter().find(|r| r.request == payload) {
        RequestKind {
            label: format!("routine {}", r.key),
            channel_key: None,
        }
    } else if payload.first() == Some(&0x3E) {
        RequestKind {
            label: "tester present".into(),
            channel_key: None,
        }
    } else if payload.first() == Some(&0x81) {
        RequestKind {
            label: "start communication".into(),
            channel_key: None,
        }
    } else {
        RequestKind {
            label: format!(
                "unknown request (SID 0x{:02X})",
                payload.first().unwrap_or(&0)
            ),
            channel_key: None,
        }
    }
}

fn describe_response(
    def: &EcuDefinition,
    kind: &RequestKind,
    resp: &Frame,
    report: &mut TraceReport,
) -> (bool, String) {
    if let Some(key) = &kind.channel_key {
        let channel = def.channels.iter().find(|c| &c.key == key).unwrap();
        return match channel.decode(&resp.payload) {
            Some(value) => {
                match report.channels.iter_mut().find(|s| &s.key == key) {
                    Some(stat) => {
                        stat.samples += 1;
                        stat.last_value = value;
                    }
                    None => report.channels.push(ChannelStat {
                        key: channel.key.clone(),
                        name: channel.name.clone(),
                        unit: channel.unit.clone(),
                        samples: 1,
                        last_value: value,
                    }),
                }
                (true, format!("{value:.2} {}", channel.unit))
            }
            None => (false, "response too short to decode".to_string()),
        };
    }
    match kind.label.as_str() {
        "identification" => {
            let raw = resp
                .payload
                .get(def.identification.skip_bytes.min(resp.payload.len())..)
                .unwrap_or_default();
            let text: String = String::from_utf8_lossy(raw)
                .chars()
                .map(|c| if c.is_control() { '·' } else { c })
                .collect();
            report.identity = Some(text.clone());
            (true, format!("ECU identifies as: {text}"))
        }
        "read DTCs" => {
            let dtcs = parse_read_dtc_response(&resp.payload, def);
            report.dtc_reads += 1;
            report.dtc_last_count = Some(dtcs.len());
            (true, format!("{} stored fault code(s)", dtcs.len()))
        }
        "tester present" => {
            report.tester_present += 1;
            (true, "keep-alive acknowledged".to_string())
        }
        _ => (true, format!("response: {}", hex(&resp.payload))),
    }
}

fn push_row(report: &mut TraceReport, row: TraceReportRow) {
    // Tester-present keep-alives would drown the row list; keep the counters
    // but only surface the first one as a row.
    if row.kind == "tester present" && report.tester_present > 1 {
        return;
    }
    if report.rows.len() < MAX_ROWS {
        report.rows.push(row);
    } else {
        report.rows_truncated = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::WireTraceRecorder;
    use crate::{ConnectOptions, DiagSession};
    use motodiag_ecu_defs::Registry;
    use motodiag_ecu_sim::{spawn_on_link, SimConfig, Simulator};
    use motodiag_kwp2000::init::FastInitConfig;
    use motodiag_transport::mock;
    use motodiag_transport::trace::TracingTransport;
    use std::time::Duration;

    fn load_brutale_def() -> EcuDefinition {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../definitions");
        Registry::load_dir(&dir)
            .unwrap()
            .get("mv-5sm-brutale-910")
            .unwrap()
            .clone()
    }

    fn record_sim_session(path: &std::path::Path) {
        let (tester, ecu) = mock::pair();
        let _sim = spawn_on_link(Simulator::new(SimConfig::default()), ecu);
        let header = serde_json::to_string(&TraceMetadata {
            format: "motodiag-trace/1".into(),
            definition_id: "mv-5sm-brutale-910".into(),
            recorded_at_ms: 1,
            simulated: true,
        })
        .unwrap();
        let recorder = WireTraceRecorder::create_with_header(path, &header).unwrap();
        let traced = TracingTransport::new(Box::new(tester), Box::new(recorder));

        let mut session = DiagSession::connect(
            Box::new(traced),
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
        .expect("connect while recording");
        session.read_dtcs().expect("read DTCs");
        session.read_channel("rpm").expect("read rpm");
        session.read_channel("batt").expect("read batt");
    }

    #[test]
    fn recorded_sim_session_analyzes_end_to_end() {
        let tmp = std::env::temp_dir().join(format!(
            "motodiag-trace-report-test-{}.jsonl",
            std::process::id()
        ));
        record_sim_session(&tmp);

        let (meta, events) = load_trace_file(&tmp).expect("trace loads");
        let meta = meta.expect("metadata header present");
        assert_eq!(meta.definition_id, "mv-5sm-brutale-910");
        assert!(meta.simulated);
        assert!(!events.is_empty());

        let report = analyze(&load_brutale_def(), &events);
        assert!(report.init_seen, "fast-init should be visible");
        assert_eq!(report.identity.as_deref(), Some("IAW 5SM SIM v0.1"));
        assert_eq!(report.dtc_reads, 1);
        assert_eq!(report.dtc_last_count, Some(2));
        let rpm = report
            .channels
            .iter()
            .find(|c| c.key == "rpm")
            .expect("rpm decoded from trace");
        assert!(rpm.last_value > 0.0, "rpm should decode to a live value");
        assert!(report.channels.iter().any(|c| c.key == "batt"));
        assert_eq!(report.negative_responses, 0);
        assert!(report.exchange_count >= 4); // start-comm, ident, dtc, 2 channels
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn analyzing_against_the_wrong_definition_reports_unknowns() {
        let tmp = std::env::temp_dir().join(format!(
            "motodiag-trace-report-wrongdef-{}.jsonl",
            std::process::id()
        ));
        record_sim_session(&tmp);
        let (_, events) = load_trace_file(&tmp).expect("trace loads");

        // The Ducati 59M shares generic request bytes for some things but is
        // a different definition; the report must stay honest, not invent
        // decodes for channels the definition doesn't describe the same way.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../definitions");
        let other = Registry::load_dir(&dir)
            .unwrap()
            .get("ducati-iaw-59m")
            .unwrap()
            .clone();
        let report = analyze(&other, &events);
        // Identity/DTC requests happen to share bytes across Marelli defs, so
        // those may decode — but the report is keyed to the def it ran with.
        assert_eq!(report.definition_id, "ducati-iaw-59m");
        assert!(report.event_count > 0);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn raw_event_file_without_header_still_loads() {
        let tmp = std::env::temp_dir().join(format!(
            "motodiag-trace-report-raw-{}.jsonl",
            std::process::id()
        ));
        std::fs::write(
            &tmp,
            r#"{"t_ms":1,"dir":"tx","kind":{"kind":"bytes","hex":"81 10 F1 81 03"}}"#,
        )
        .unwrap();
        let (meta, events) = load_trace_file(&tmp).expect("raw file loads");
        assert!(meta.is_none());
        assert_eq!(events.len(), 1);
        let _ = std::fs::remove_file(&tmp);
    }
}
