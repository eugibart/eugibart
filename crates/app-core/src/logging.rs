//! Wire tracing and data logging.
//!
//! Every session should be captured: raw traces become replayable regression
//! fixtures (see `fixtures/`), and CSV logs feed later analysis.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use motodiag_kwp2000::session::{Direction, TraceHook};

use crate::live_data::{now_ms, Reading};
use crate::Result;

/// Writes one JSON line per frame:
/// `{"t_ms":1699999999999,"dir":"tx","raw":"81 10 F1 81 03"}`
pub struct TraceLogger {
    out: Arc<Mutex<BufWriter<File>>>,
}

impl TraceLogger {
    pub fn create(path: &Path) -> Result<Self> {
        let file = File::create(path)?;
        Ok(Self {
            out: Arc::new(Mutex::new(BufWriter::new(file))),
        })
    }

    /// Build the hook to plug into `KwpSession::set_trace_hook`.
    pub fn hook(&self) -> TraceHook {
        let out = Arc::clone(&self.out);
        Box::new(move |dir: Direction, raw: &[u8]| {
            let dir = match dir {
                Direction::Sent => "tx",
                Direction::Received => "rx",
            };
            let hex: Vec<String> = raw.iter().map(|b| format!("{b:02X}")).collect();
            let line = serde_json::json!({
                "t_ms": now_ms(),
                "dir": dir,
                "raw": hex.join(" "),
            });
            if let Ok(mut w) = out.lock() {
                let _ = writeln!(w, "{line}");
                let _ = w.flush();
            }
        })
    }
}

/// CSV log of live-data readings: `timestamp_ms,key,name,value,unit`.
pub struct CsvLogger {
    writer: csv::Writer<File>,
}

impl CsvLogger {
    pub fn create(path: &Path) -> Result<Self> {
        let mut writer = csv::Writer::from_writer(File::create(path)?);
        writer
            .write_record(["timestamp_ms", "key", "name", "value", "unit"])
            .map_err(csv_io)?;
        Ok(Self { writer })
    }

    pub fn log(&mut self, reading: &Reading) -> Result<()> {
        self.writer
            .write_record([
                reading.timestamp_ms.to_string(),
                reading.key.clone(),
                reading.name.clone(),
                reading.value.to_string(),
                reading.unit.clone(),
            ])
            .map_err(csv_io)?;
        self.writer.flush()?;
        Ok(())
    }
}

fn csv_io(e: csv::Error) -> crate::AppError {
    crate::AppError::Io(std::io::Error::other(e))
}
