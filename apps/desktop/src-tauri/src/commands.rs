//! Tauri commands: the JSON boundary between the UI and app-core.
//!
//! Commands are `async` so their (potentially slow, serial-I/O-bound) bodies
//! run on the async runtime's worker pool instead of blocking the UI thread.

use std::time::Duration;

use motodiag_app_core::dtc::Dtc;
use motodiag_app_core::live_data::Reading;
use motodiag_app_core::logging::CsvLogger;
use motodiag_app_core::{ConnectOptions, DiagSession};
use motodiag_ecu_defs::schema::RiskLevel;
use motodiag_kwp2000::init::FastInitConfig;
use motodiag_transport::{mock, KLineTransport};
use tauri::State;

use crate::state::{
    load_catalog, load_mod_guidance, load_registry, AppState, Connection, VacuumConnection,
};

/// Sentinel "port" that connects to the in-process simulated ECU.
pub const SIMULATOR_PORT: &str = "simulator";

#[derive(serde::Serialize)]
pub struct ChannelSpecInfo {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub target: Option<f64>,
    pub condition: String,
    /// Citation for community-sourced figures (site name + https URL).
    pub source: Option<String>,
    pub source_url: Option<String>,
}

#[derive(serde::Serialize)]
pub struct ChannelInfo {
    pub key: String,
    pub name: String,
    pub unit: String,
    pub verified: bool,
    pub spec: Option<ChannelSpecInfo>,
}

#[derive(serde::Serialize)]
pub struct RoutineInfo {
    pub key: String,
    pub name: String,
    pub description: String,
    /// Full step-by-step procedure; empty when only `description` exists.
    pub procedure: Vec<String>,
    pub risk: String,
    pub verified: bool,
    pub preconditions: Vec<String>,
    /// What to have ready before running ("laptop + KKL cable").
    pub tools: Vec<String>,
}

/// "How do I get to it?" guide — zone is the kebab-case AccessZone name the
/// frontend's diagram keys on.
#[derive(serde::Serialize)]
pub struct AccessGuideInfo {
    pub zone: String,
    pub summary: String,
    pub steps: Vec<String>,
    pub tools: Vec<String>,
    pub verify_note: Option<String>,
    pub source: Option<String>,
    pub source_url: Option<String>,
}

fn access_guide_info(g: &motodiag_ecu_defs::schema::AccessGuide) -> AccessGuideInfo {
    AccessGuideInfo {
        zone: g.zone.as_str().to_string(),
        summary: g.summary.clone(),
        steps: g.steps.clone(),
        tools: g.tools.clone(),
        verify_note: g.verify_note.clone(),
        source: g.source.clone(),
        source_url: g.source_url.clone(),
    }
}

#[derive(serde::Serialize)]
pub struct ChargingTestInfo {
    pub rest_min_v: f64,
    pub charging_min_v: f64,
    pub charging_max_v: f64,
    pub check_rpm_min: f64,
    pub check_rpm_max: f64,
    pub stator_notes: String,
    pub source: String,
    pub source_url: String,
}

#[derive(serde::Serialize)]
pub struct DefinitionInfo {
    pub id: String,
    pub name: String,
    pub manufacturer: String,
    pub models: Vec<String>,
    pub bus: String,
    pub verified: bool,
    pub notes: Option<String>,
    pub channels: Vec<ChannelInfo>,
    pub routines: Vec<RoutineInfo>,
    pub charging: Option<ChargingTestInfo>,
    /// Silhouette variant the access-guide diagrams draw ("naked"/"faired").
    pub body_style: String,
    pub connector_access: Option<AccessGuideInfo>,
    pub vacuum_access: Option<AccessGuideInfo>,
}

#[derive(serde::Serialize)]
pub struct ConnectionInfo {
    pub definition_id: String,
    pub definition_name: String,
    pub identity_text: String,
    pub service_mode: bool,
    pub csv_log_path: Option<String>,
    pub simulated: bool,
}

#[tauri::command]
pub async fn list_definitions() -> Vec<DefinitionInfo> {
    load_registry()
        .iter()
        .map(|def| DefinitionInfo {
            id: def.ecu.id.clone(),
            name: def.ecu.name.clone(),
            manufacturer: def.ecu.manufacturer.clone(),
            models: def.ecu.models.clone(),
            bus: match def.ecu.bus {
                motodiag_ecu_defs::schema::BusKind::KLine => "K-line".into(),
                motodiag_ecu_defs::schema::BusKind::Can => "CAN".into(),
            },
            verified: def.ecu.verified,
            notes: def.ecu.notes.clone(),
            channels: def
                .channels
                .iter()
                .map(|c| ChannelInfo {
                    key: c.key.clone(),
                    name: c.name.clone(),
                    unit: c.unit.clone(),
                    verified: c.verified,
                    spec: c.spec.as_ref().map(|s| ChannelSpecInfo {
                        min: s.min,
                        max: s.max,
                        target: s.target,
                        condition: s.condition.clone(),
                        source: s.source.clone(),
                        source_url: s.source_url.clone(),
                    }),
                })
                .collect(),
            routines: def
                .routines
                .iter()
                .map(|r| RoutineInfo {
                    key: r.key.clone(),
                    name: r.name.clone(),
                    description: r.description.clone(),
                    procedure: r.procedure.clone(),
                    risk: match r.risk {
                        RiskLevel::Low => "low".into(),
                        RiskLevel::Medium => "medium".into(),
                        RiskLevel::High => "high".into(),
                    },
                    verified: r.verified,
                    preconditions: describe_preconditions(&r.preconditions),
                    tools: r.tools.clone(),
                })
                .collect(),
            charging: def.charging.as_ref().map(|c| ChargingTestInfo {
                rest_min_v: c.rest_min_v,
                charging_min_v: c.charging_min_v,
                charging_max_v: c.charging_max_v,
                check_rpm_min: c.check_rpm_min,
                check_rpm_max: c.check_rpm_max,
                stator_notes: c.stator_notes.clone(),
                source: c.source.clone(),
                source_url: c.source_url.clone(),
            }),
            body_style: def.ecu.body_style.as_str().to_string(),
            connector_access: def.connector_access.as_ref().map(access_guide_info),
            vacuum_access: def.vacuum_access.as_ref().map(access_guide_info),
        })
        .collect()
}

/// The structured bike catalog backing the "tell me about your bike" wizard.
/// Purely descriptive data (see ecu-defs catalog module) — serialized as-is.
#[tauri::command]
pub async fn list_bike_catalog() -> Vec<motodiag_ecu_defs::catalog::CatalogBike> {
    load_catalog().bikes.clone()
}

/// Community mod guidance: spec adjustments, procedure caveats, DTC notes.
/// Unverified by construction; the frontend labels every use.
#[tauri::command]
pub async fn list_mod_guidance() -> motodiag_ecu_defs::ModGuidance {
    load_mod_guidance().clone()
}

fn describe_preconditions(p: &motodiag_ecu_defs::schema::Preconditions) -> Vec<String> {
    let mut out = Vec::new();
    if p.engine_must_be_off {
        out.push("Engine must be off".to_string());
    }
    match (p.min_battery_v, p.max_battery_v) {
        (Some(min), Some(max)) => out.push(format!("Battery {min:.1}–{max:.1} V")),
        (Some(min), None) => out.push(format!("Battery ≥ {min:.1} V")),
        (None, Some(max)) => out.push(format!("Battery ≤ {max:.1} V")),
        (None, None) => {}
    }
    if let Some(notes) = &p.notes {
        out.push(notes.trim().to_string());
    }
    out
}

#[tauri::command]
pub async fn list_serial_ports() -> Vec<String> {
    let mut ports = vec![SIMULATOR_PORT.to_string()];
    ports.extend(
        serialport::available_ports()
            .map(|list| list.into_iter().map(|p| p.port_name).collect::<Vec<_>>())
            .unwrap_or_default(),
    );
    ports
}

#[tauri::command]
pub async fn connect(
    state: State<'_, AppState>,
    definition_id: String,
    port: String,
) -> Result<ConnectionInfo, String> {
    let registry = load_registry();
    let def = registry
        .get(&definition_id)
        .ok_or_else(|| format!("unknown ECU definition '{definition_id}'"))?
        .clone();
    let init = def.init.clone().ok_or_else(|| {
        format!(
            "'{definition_id}' is a CAN-bus ECU definition; the desktop app only supports \
             K-line/KWP2000 connections so far (see crates/protocol-can for the CAN groundwork)"
        )
    })?;

    let mut sim_thread = None;
    let transport: Box<dyn KLineTransport> = if port == SIMULATOR_PORT {
        let (tester, ecu) = mock::pair();
        sim_thread = Some(motodiag_ecu_sim::spawn_on_link(
            motodiag_ecu_sim::Simulator::new(motodiag_ecu_sim::SimConfig::default()),
            ecu,
        ));
        Box::new(tester)
    } else {
        Box::new(
            motodiag_transport::serial_vcp::SerialKLine::open(&port, init.baud)
                .map_err(|e| format!("failed to open {port}: {e}"))?,
        )
    };

    let options = if port == SIMULATOR_PORT {
        // The simulator doesn't need real wake-up timing; connect instantly.
        ConnectOptions {
            fast_init: Some(FastInitConfig {
                idle_before: Duration::from_millis(1),
                low_time: Duration::from_millis(1),
                high_time: Duration::from_millis(1),
            }),
            ..Default::default()
        }
    } else {
        ConnectOptions::default()
    };

    // Record every session as a shareable wire trace (community-verification
    // evidence). Best-effort: a trace-file failure never blocks connecting.
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut trace_path = None;
    let transport: Box<dyn KLineTransport> = {
        use motodiag_app_core::logging::WireTraceRecorder;
        use motodiag_app_core::trace_report::TraceMetadata;
        use motodiag_transport::trace::TracingTransport;

        let dir = std::env::temp_dir().join("motodiag-traces");
        let recorder = std::fs::create_dir_all(&dir).ok().and_then(|_| {
            let path = dir.join(format!(
                "motodiag-trace-{}-{}.jsonl",
                definition_id,
                now_ms / 1000
            ));
            let header = serde_json::to_string(&TraceMetadata {
                format: "motodiag-trace/1".into(),
                definition_id: definition_id.clone(),
                recorded_at_ms: now_ms,
                simulated: port == SIMULATOR_PORT,
            })
            .ok()?;
            let recorder = WireTraceRecorder::create_with_header(&path, &header).ok()?;
            trace_path = Some(path);
            Some(recorder)
        });
        match recorder {
            Some(recorder) => Box::new(TracingTransport::new(transport, Box::new(recorder))),
            None => transport,
        }
    };

    let session = DiagSession::connect(transport, def, options).map_err(|e| e.to_string())?;

    let info = ConnectionInfo {
        definition_id: session.definition().ecu.id.clone(),
        definition_name: session.definition().ecu.name.clone(),
        identity_text: session.identity().text.clone(),
        service_mode: false,
        csv_log_path: None,
        simulated: port == SIMULATOR_PORT,
    };

    *state.connection.lock().unwrap() = Some(Connection {
        session,
        sim_thread,
        csv_log: None,
        trace_path,
    });
    Ok(info)
}

#[derive(serde::Serialize)]
pub struct WireTraceInfo {
    pub path: String,
    pub event_count: usize,
}

/// Where this session's wire trace lives (recorded continuously since
/// connect) — the file an owner shares for community verification.
#[tauri::command]
pub async fn export_wire_trace(state: State<'_, AppState>) -> Result<WireTraceInfo, String> {
    let guard = state.connection.lock().unwrap();
    let conn = guard.as_ref().ok_or("not connected")?;
    let path = conn
        .trace_path
        .as_ref()
        .ok_or("this session has no wire trace (recording failed at connect)")?;
    let (_, events) = motodiag_app_core::trace_report::load_trace_file(path)
        .map_err(|e| format!("trace unreadable: {e}"))?;
    Ok(WireTraceInfo {
        path: path.display().to_string(),
        event_count: events.len(),
    })
}

/// Analyze a shared wire-trace file against an ECU definition and report
/// what decoded — the receiving end of the community verification loop.
/// `definition_id` overrides the trace's own metadata when provided.
#[tauri::command]
pub async fn import_wire_trace(
    path: String,
    definition_id: Option<String>,
) -> Result<motodiag_app_core::trace_report::TraceReport, String> {
    let (metadata, events) =
        motodiag_app_core::trace_report::load_trace_file(std::path::Path::new(&path))
            .map_err(|e| format!("could not read trace: {e}"))?;
    let def_id = definition_id
        .or(metadata.map(|m| m.definition_id))
        .ok_or("trace has no definition metadata — pick a definition to analyze against")?;
    let def = load_registry()
        .get(&def_id)
        .ok_or_else(|| format!("unknown ECU definition '{def_id}'"))?;
    Ok(motodiag_app_core::trace_report::analyze(def, &events))
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<(), String> {
    // Dropping the connection closes the link; the simulator thread exits on
    // link close and the JoinHandle drop detaches it harmlessly.
    *state.connection.lock().unwrap() = None;
    Ok(())
}

#[tauri::command]
pub async fn connection_status(
    state: State<'_, AppState>,
) -> Result<Option<ConnectionInfo>, String> {
    let guard = state.connection.lock().unwrap();
    Ok(guard.as_ref().map(|conn| ConnectionInfo {
        definition_id: conn.session.definition().ecu.id.clone(),
        definition_name: conn.session.definition().ecu.name.clone(),
        identity_text: conn.session.identity().text.clone(),
        service_mode: conn.session.service_mode(),
        csv_log_path: conn.csv_log.as_ref().map(|(path, _)| path.clone()),
        simulated: conn.sim_thread.is_some(),
    }))
}

fn with_connection<T>(
    state: &State<'_, AppState>,
    f: impl FnOnce(&mut Connection) -> Result<T, String>,
) -> Result<T, String> {
    let mut guard = state.connection.lock().unwrap();
    let conn = guard.as_mut().ok_or("not connected")?;
    f(conn)
}

#[tauri::command]
pub async fn read_dtcs(state: State<'_, AppState>) -> Result<Vec<Dtc>, String> {
    with_connection(&state, |conn| {
        conn.session.read_dtcs().map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub async fn clear_dtcs(state: State<'_, AppState>, confirmed: bool) -> Result<(), String> {
    with_connection(&state, |conn| {
        conn.session
            .clear_dtcs(confirmed)
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
pub async fn poll_live_data(state: State<'_, AppState>) -> Result<Vec<Reading>, String> {
    with_connection(&state, |conn| {
        let readings: Vec<Reading> = conn
            .session
            .poll_all_channels()
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();
        if let Some((_, logger)) = conn.csv_log.as_mut() {
            for reading in &readings {
                let _ = logger.log(reading);
            }
        }
        Ok(readings)
    })
}

#[tauri::command]
pub async fn enable_service_mode(state: State<'_, AppState>) -> Result<(), String> {
    with_connection(&state, |conn| {
        conn.session.enable_service_mode();
        Ok(())
    })
}

#[tauri::command]
pub async fn run_routine(
    state: State<'_, AppState>,
    key: String,
    confirmed: bool,
) -> Result<String, String> {
    with_connection(&state, |conn| {
        let response = conn
            .session
            .run_routine(&key, confirmed)
            .map_err(|e| e.to_string())?;
        Ok(response
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" "))
    })
}

#[tauri::command]
pub async fn start_csv_log(state: State<'_, AppState>) -> Result<String, String> {
    with_connection(&state, |conn| {
        let dir = std::env::temp_dir().join("motodiag-logs");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = dir.join(format!("motodiag-{stamp}.csv"));
        let logger = CsvLogger::create(&path).map_err(|e| e.to_string())?;
        let path_str = path.display().to_string();
        conn.csv_log = Some((path_str.clone(), logger));
        Ok(path_str)
    })
}

/// Read fresh DTCs + live data from the connected ECU and write a
/// self-contained HTML health report. Returns the file path. `bike` is the
/// frontend's already-resolved profile (see specResolution.ts) — `None`
/// produces a byte-identical legacy report with no Bike section.
#[tauri::command]
pub async fn export_health_report(
    state: State<'_, AppState>,
    bike: Option<motodiag_app_core::report::BikeReportSection>,
) -> Result<String, String> {
    use motodiag_app_core::report::{render_html, ReportInput};

    with_connection(&state, |conn| {
        let dtcs = conn.session.read_dtcs().map_err(|e| e.to_string())?;
        let readings: Vec<Reading> = conn
            .session
            .poll_all_channels()
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let html = render_html(&ReportInput {
            def: conn.session.definition(),
            identity: conn.session.identity(),
            dtcs: &dtcs,
            readings: &readings,
            simulated: conn.sim_thread.is_some(),
            generated_at_ms: now_ms,
            bike: bike.as_ref(),
        });

        let dir = std::env::temp_dir().join("motodiag-reports");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!(
            "motodiag-report-{}-{}.html",
            conn.session.definition().ecu.id,
            now_ms / 1000
        ));
        std::fs::write(&path, html).map_err(|e| e.to_string())?;
        Ok(path.display().to_string())
    })
}

#[derive(serde::Serialize)]
pub struct VacuumInfo {
    pub port: String,
    pub channels: usize,
    pub simulated: bool,
}

#[derive(serde::Serialize)]
pub struct VacuumStatus {
    pub channels_kpa: Vec<f64>,
    /// Max−min across cylinders — drive this toward zero while balancing.
    pub spread_kpa: f64,
    /// Per-cylinder difference vs cylinder 1.
    pub deltas_kpa: Vec<f64>,
    pub timestamp_ms: u64,
}

/// Connect a vacuum gauge (Sync Assistant). Independent of the ECU session —
/// its own port, its own lifecycle.
#[tauri::command]
pub async fn connect_vacuum(
    state: State<'_, AppState>,
    port: String,
) -> Result<VacuumInfo, String> {
    use motodiag_app_core::vacuum::{MockVacuumGauge, SerialVacuumGauge, VacuumGauge};

    let simulated = port == SIMULATOR_PORT;
    let mut gauge: Box<dyn VacuumGauge> = if simulated {
        Box::new(MockVacuumGauge::default())
    } else {
        Box::new(SerialVacuumGauge::open(&port).map_err(|e| format!("failed to open {port}: {e}"))?)
    };

    // Confirm we're actually receiving samples before declaring success.
    let first = gauge
        .read(Duration::from_millis(1500))
        .map_err(|e| format!("no vacuum samples arriving on {port}: {e}"))?;

    let info = VacuumInfo {
        port: port.clone(),
        channels: first.channels_kpa.len(),
        simulated,
    };
    *state.vacuum.lock().unwrap() = Some(VacuumConnection { port, gauge });
    Ok(info)
}

#[tauri::command]
pub async fn vacuum_status(state: State<'_, AppState>) -> Result<Option<VacuumInfo>, String> {
    let guard = state.vacuum.lock().unwrap();
    Ok(guard.as_ref().map(|v| VacuumInfo {
        port: v.port.clone(),
        channels: 0, // unknown between reads; the UI uses poll data anyway
        simulated: v.port == SIMULATOR_PORT,
    }))
}

#[tauri::command]
pub async fn poll_vacuum(state: State<'_, AppState>) -> Result<VacuumStatus, String> {
    let reading = {
        let mut guard = state.vacuum.lock().unwrap();
        let vac = guard.as_mut().ok_or("no vacuum gauge connected")?;
        vac.gauge
            .read(Duration::from_millis(500))
            .map_err(|e| e.to_string())?
    };

    // If a CSV log is running on the ECU session, vacuum flows into it too —
    // one timeline for everything.
    if let Some(conn) = state.connection.lock().unwrap().as_mut() {
        if let Some((_, logger)) = conn.csv_log.as_mut() {
            for (i, kpa) in reading.channels_kpa.iter().enumerate() {
                let _ = logger.log(&Reading {
                    key: format!("vac{}", i + 1),
                    name: format!("Vacuum cyl {}", i + 1),
                    unit: "kPa".into(),
                    value: *kpa,
                    timestamp_ms: reading.timestamp_ms,
                });
            }
        }
    }

    Ok(VacuumStatus {
        spread_kpa: reading.spread_kpa(),
        deltas_kpa: reading.deltas_vs_reference(0),
        channels_kpa: reading.channels_kpa,
        timestamp_ms: reading.timestamp_ms,
    })
}

#[tauri::command]
pub async fn disconnect_vacuum(state: State<'_, AppState>) -> Result<(), String> {
    *state.vacuum.lock().unwrap() = None;
    Ok(())
}

#[derive(serde::Serialize)]
pub struct TroubleshootStep {
    pub name: String,
    /// "passed" | "failed" | "skipped"
    pub status: String,
    pub detail: String,
    pub suggestion: Option<String>,
}

impl TroubleshootStep {
    fn passed(name: &str, detail: String) -> Self {
        Self {
            name: name.into(),
            status: "passed".into(),
            detail,
            suggestion: None,
        }
    }
    fn failed(name: &str, detail: String, suggestion: String) -> Self {
        Self {
            name: name.into(),
            status: "failed".into(),
            detail,
            suggestion: Some(suggestion),
        }
    }
    fn skipped(name: &str, detail: String) -> Self {
        Self {
            name: name.into(),
            status: "skipped".into(),
            detail,
            suggestion: None,
        }
    }
}

/// Staged connection diagnosis: cable → port → bus/ECU handshake. Runs
/// without touching the active session (it opens its own transport), so it's
/// safe to run while disconnected — which is exactly when you need it.
#[tauri::command]
pub async fn troubleshoot_connection(
    definition_id: String,
    port: String,
) -> Result<Vec<TroubleshootStep>, String> {
    use motodiag_app_core::troubleshoot::{probe_connection, FailurePoint};
    use motodiag_transport::serial_vcp::{port_details, FTDI_VID};

    let registry = load_registry();
    let def = registry
        .get(&definition_id)
        .ok_or_else(|| format!("unknown ECU definition '{definition_id}'"))?
        .clone();
    let init = def.init.clone().ok_or_else(|| {
        format!("'{definition_id}' is a CAN-bus definition; troubleshooting supports K-line only")
    })?;

    let mut steps = Vec::new();

    // Step 1: interface cable presence.
    let simulated = port == SIMULATOR_PORT;
    if simulated {
        steps.push(TroubleshootStep::passed(
            "Interface cable",
            "Using the built-in simulator — no cable involved.".into(),
        ));
    } else {
        let details = port_details();
        match details.iter().find(|d| d.name == port) {
            Some(d) if d.is_ftdi() => steps.push(TroubleshootStep::passed(
                "Interface cable",
                format!(
                    "{port} is an FTDI device ({}) — the right chip family for K-line.",
                    d.usb
                        .as_ref()
                        .and_then(|u| u.product.clone())
                        .unwrap_or_else(|| "unnamed".into())
                ),
            )),
            Some(d) => {
                let (vid, product) = d
                    .usb
                    .as_ref()
                    .map(|u| {
                        (
                            format!("{:04X}", u.vid),
                            u.product.clone().unwrap_or_default(),
                        )
                    })
                    .unwrap_or_else(|| ("not USB".into(), String::new()));
                steps.push(TroubleshootStep {
                    name: "Interface cable".into(),
                    status: "passed".into(),
                    detail: format!("{port} found ({product}, vendor {vid})."),
                    suggestion: Some(format!(
                        "This is not an FTDI device (vendor {vid}, FTDI is {FTDI_VID:04X}). \
                         CH340-class clones often fail at K-line init timing — if the handshake \
                         step below fails, a genuine FTDI KKL cable is the first thing to try."
                    )),
                });
            }
            None => {
                let available: Vec<String> = details.into_iter().map(|d| d.name).collect();
                steps.push(TroubleshootStep::failed(
                    "Interface cable",
                    format!("{port} is not present on this system."),
                    if available.is_empty() {
                        "No serial ports found at all. Plug the cable in, then hit Refresh. On \
                         macOS an FTDI cable should appear as /dev/cu.usbserial-XXXX with no \
                         driver install needed."
                            .into()
                    } else {
                        format!(
                            "Available ports: {}. Pick one of those, or re-plug the cable.",
                            available.join(", ")
                        )
                    },
                ));
                steps.push(TroubleshootStep::skipped(
                    "Open serial port",
                    "No cable to open.".into(),
                ));
                steps.push(TroubleshootStep::skipped(
                    "Bus & ECU handshake",
                    "No cable to test.".into(),
                ));
                return Ok(steps);
            }
        }
    }

    // Step 2: open the port / create the transport.
    let transport: Box<dyn KLineTransport> = if simulated {
        let (tester, ecu) = mock::pair();
        // Detached simulator thread; exits when the link drops at the end of
        // the probe.
        std::thread::spawn(move || {
            motodiag_ecu_sim::run_on_link(
                motodiag_ecu_sim::Simulator::new(motodiag_ecu_sim::SimConfig::default()),
                ecu,
            )
        });
        steps.push(TroubleshootStep::passed(
            "Open serial port",
            "Simulator link created.".into(),
        ));
        Box::new(tester)
    } else {
        match motodiag_transport::serial_vcp::SerialKLine::open(&port, init.baud) {
            Ok(serial) => {
                steps.push(TroubleshootStep::passed(
                    "Open serial port",
                    format!("{port} opened at {} baud.", init.baud),
                ));
                Box::new(serial)
            }
            Err(e) => {
                steps.push(TroubleshootStep::failed(
                    "Open serial port",
                    format!("Could not open {port}: {e}"),
                    "Another program may be holding the port (a previous session, a different \
                     diagnostic tool). Close other tools, unplug/replug the cable, and try \
                     again."
                        .into(),
                ));
                steps.push(TroubleshootStep::skipped(
                    "Bus & ECU handshake",
                    "Port could not be opened.".into(),
                ));
                return Ok(steps);
            }
        }
    };

    // Step 3: wake-up + init + identification, classified.
    let options = if simulated {
        ConnectOptions {
            fast_init: Some(FastInitConfig {
                idle_before: Duration::from_millis(1),
                low_time: Duration::from_millis(1),
                high_time: Duration::from_millis(1),
            }),
            ..Default::default()
        }
    } else {
        ConnectOptions::default()
    };
    let diagnosis = probe_connection(transport, &def, &options);
    if diagnosis.failure_point == FailurePoint::None {
        steps.push(TroubleshootStep::passed(
            "Bus & ECU handshake",
            diagnosis.finding,
        ));
    } else {
        steps.push(TroubleshootStep::failed(
            "Bus & ECU handshake",
            diagnosis.finding,
            diagnosis.suggestion,
        ));
    }

    Ok(steps)
}

#[tauri::command]
pub async fn stop_csv_log(state: State<'_, AppState>) -> Result<Option<String>, String> {
    with_connection(&state, |conn| Ok(conn.csv_log.take().map(|(path, _)| path)))
}
