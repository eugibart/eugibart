//! Command-line ECU discovery tool.
//!
//! Probes a real (or simulated) K-line ECU directly — no JPDiag/TuneECU/VDSTS
//! needed to reverse-engineer. Every request/response is captured to a wire
//! trace fixture as it runs, so a single session both answers "what's on
//! local ID 0x0C?" and becomes a permanent regression fixture.
//!
//! Usage:
//!   motodiag-discover --list
//!   motodiag-discover <port> <definition-id> [--trace <path>] [--range START:END]
//!
//! Example (against the built-in simulator, no hardware needed):
//!   cargo run -p motodiag-app-core --bin motodiag-discover -- simulator mv-5sm-brutale-910
//!
//! Example against a real cable on macOS:
//!   cargo run -p motodiag-app-core --bin motodiag-discover -- \
//!       /dev/cu.usbserial-A7043NRK mv-5sm-brutale-910 --trace brutale-discovery.jsonl

use std::time::Duration;

use motodiag_app_core::discovery::{run_discovery, DiscoveryOptions};
use motodiag_app_core::logging::WireTraceRecorder;
use motodiag_app_core::session::ConnectOptions;
use motodiag_ecu_defs::{EcuDefinition, Registry};
use motodiag_kwp2000::init::FastInitConfig;
use motodiag_transport::{mock, serial_vcp::SerialKLine, KLineTransport};

const SIMULATOR_PORT: &str = "simulator";

// Same shipped K-line definitions the desktop app embeds (see
// apps/desktop/src-tauri/src/state.rs) — kept in sync manually since this is
// a small, stable list; a build-time codegen step would be overkill here.
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

fn load_registry() -> Registry {
    let mut registry = Registry::default();
    for (origin, text) in EMBEDDED_DEFINITIONS {
        registry
            .add_toml(text, origin)
            .expect("embedded definitions are validated by the workspace tests");
    }
    registry
}

fn print_usage_and_exit(code: i32) -> ! {
    eprintln!(
        "Usage:\n  \
         motodiag-discover --list\n  \
         motodiag-discover <port> <definition-id> [--trace <path>] [--range START:END]\n\n\
         <port> is a serial device path (e.g. /dev/cu.usbserial-A7043NRK) or '{SIMULATOR_PORT}'.\n\
         START:END are hex bytes, e.g. --range 01:20 to probe only local IDs 0x01-0x20.\n\
         Without --range, the full 0x00-0xFF space is scanned (slower, but thorough)."
    );
    std::process::exit(code);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--list") {
        for def in load_registry().iter() {
            println!(
                "{:<28} {} ({})",
                def.ecu.id, def.ecu.name, def.ecu.manufacturer
            );
        }
        return;
    }

    if args.len() < 2 {
        print_usage_and_exit(1);
    }
    let port = &args[0];
    let definition_id = &args[1];

    let mut trace_path: Option<String> = None;
    let mut range: Option<(u8, u8)> = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--trace" => {
                trace_path = args.get(i + 1).cloned();
                i += 2;
            }
            "--range" => {
                let spec = args.get(i + 1).cloned().unwrap_or_default();
                let (lo, hi) = spec.split_once(':').unwrap_or(("", ""));
                match (u8::from_str_radix(lo, 16), u8::from_str_radix(hi, 16)) {
                    (Ok(lo), Ok(hi)) => range = Some((lo, hi)),
                    _ => {
                        eprintln!("--range expects START:END in hex, e.g. 01:20");
                        print_usage_and_exit(1);
                    }
                }
                i += 2;
            }
            other => {
                eprintln!("unrecognized argument: {other}");
                print_usage_and_exit(1);
            }
        }
    }

    let registry = load_registry();
    let def: EcuDefinition = match registry.get(definition_id) {
        Some(def) => def.clone(),
        None => {
            eprintln!("unknown definition '{definition_id}'. Available:");
            for def in registry.iter() {
                eprintln!("  {}", def.ecu.id);
            }
            std::process::exit(1);
        }
    };

    if !def.ecu.verified {
        eprintln!(
            "NOTE: '{}' is an unverified definition — that's exactly what this run helps fix.",
            def.ecu.id
        );
    }

    let (transport, connect_options): (Box<dyn KLineTransport>, ConnectOptions) =
        if port == SIMULATOR_PORT {
            let (tester, ecu) = mock::pair();
            std::thread::spawn(move || {
                motodiag_ecu_sim::run_on_link(
                    motodiag_ecu_sim::Simulator::new(motodiag_ecu_sim::SimConfig::default()),
                    ecu,
                )
            });
            (
                Box::new(tester),
                ConnectOptions {
                    fast_init: Some(FastInitConfig {
                        idle_before: Duration::from_millis(1),
                        low_time: Duration::from_millis(1),
                        high_time: Duration::from_millis(1),
                    }),
                    ..Default::default()
                },
            )
        } else {
            let baud = def
                .init
                .as_ref()
                .expect("K-line definitions always have an init config")
                .baud;
            match SerialKLine::open(port, baud) {
                Ok(serial) => (Box::new(serial), ConnectOptions::default()),
                Err(e) => {
                    eprintln!("failed to open {port}: {e}");
                    std::process::exit(1);
                }
            }
        };

    let discovery_options = match range {
        Some((lo, hi)) => DiscoveryOptions {
            local_id_range: lo..=hi,
            ..Default::default()
        },
        None => DiscoveryOptions::default(),
    };

    let trace_sink: Option<Box<dyn motodiag_transport::trace::TraceSink>> = match &trace_path {
        Some(path) => match WireTraceRecorder::create(std::path::Path::new(path)) {
            Ok(recorder) => Some(Box::new(recorder)),
            Err(e) => {
                eprintln!("failed to create trace file {path}: {e}");
                std::process::exit(1);
            }
        },
        None => None,
    };

    eprintln!("Connecting to {port} as {definition_id}...");
    match run_discovery(
        transport,
        &def,
        &connect_options,
        &discovery_options,
        trace_sink,
    ) {
        Ok(report) => {
            print!("{}", report.summary());
            if let Some(path) = &trace_path {
                eprintln!("\nWire trace written to {path}");
            }
        }
        Err(e) => {
            eprintln!("discovery failed: {e}");
            std::process::exit(1);
        }
    }
}
