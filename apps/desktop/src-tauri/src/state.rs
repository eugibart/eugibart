//! Shared app state: the (single) diagnostic session and its loggers.
//!
//! ECU definitions are embedded at compile time so the app has no
//! runtime-path dependency; a user-definitions directory can be layered on
//! later.

use std::sync::Mutex;

use motodiag_app_core::logging::CsvLogger;
use motodiag_app_core::DiagSession;
use motodiag_ecu_defs::Registry;

pub struct Connection {
    pub session: DiagSession,
    /// Keeps the in-process simulator thread alive for "Simulator" connections.
    pub sim_thread: Option<std::thread::JoinHandle<motodiag_ecu_sim::Simulator>>,
    pub csv_log: Option<(String, CsvLogger)>,
}

#[derive(Default)]
pub struct AppState {
    pub connection: Mutex<Option<Connection>>,
}

const EMBEDDED_DEFINITIONS: &[(&str, &str)] = &[
    (
        "mv/5sm-brutale-910.toml",
        include_str!("../../../../definitions/mv/5sm-brutale-910.toml"),
    ),
    (
        "ducati/iaw-5am.toml",
        include_str!("../../../../definitions/ducati/iaw-5am.toml"),
    ),
    (
        "ducati/iaw-59m.toml",
        include_str!("../../../../definitions/ducati/iaw-59m.toml"),
    ),
];

pub fn load_registry() -> Registry {
    let mut registry = Registry::default();
    for (origin, text) in EMBEDDED_DEFINITIONS {
        registry
            .add_toml(text, origin)
            .expect("embedded ECU definitions are validated by the workspace tests");
    }
    registry
}
