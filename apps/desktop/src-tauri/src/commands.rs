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

use crate::state::{load_registry, AppState, Connection};

/// Sentinel "port" that connects to the in-process simulated ECU.
pub const SIMULATOR_PORT: &str = "simulator";

#[derive(serde::Serialize)]
pub struct ChannelInfo {
    pub key: String,
    pub name: String,
    pub unit: String,
    pub verified: bool,
}

#[derive(serde::Serialize)]
pub struct RoutineInfo {
    pub key: String,
    pub name: String,
    pub description: String,
    pub risk: String,
    pub verified: bool,
    pub preconditions: Vec<String>,
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
                })
                .collect(),
            routines: def
                .routines
                .iter()
                .map(|r| RoutineInfo {
                    key: r.key.clone(),
                    name: r.name.clone(),
                    description: r.description.clone(),
                    risk: match r.risk {
                        RiskLevel::Low => "low".into(),
                        RiskLevel::Medium => "medium".into(),
                        RiskLevel::High => "high".into(),
                    },
                    verified: r.verified,
                    preconditions: describe_preconditions(&r.preconditions),
                })
                .collect(),
        })
        .collect()
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
            motodiag_transport::serial_vcp::SerialKLine::open(&port, def.init.baud)
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
    });
    Ok(info)
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

#[tauri::command]
pub async fn stop_csv_log(state: State<'_, AppState>) -> Result<Option<String>, String> {
    with_connection(&state, |conn| Ok(conn.csv_log.take().map(|(path, _)| path)))
}
