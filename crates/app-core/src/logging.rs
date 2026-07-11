//! Wire tracing and data logging.
//!
//! Every session should be captured: raw traces become replayable regression
//! fixtures (see `fixtures/`), and CSV logs feed later analysis.

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use motodiag_kwp2000::session::{Direction, TraceHook};
use motodiag_transport::trace::{TraceEvent, TraceSink};

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

/// Writes a byte-level [`TraceEvent`] stream (see `motodiag_transport::trace`)
/// to a JSONL file, one event per line. Unlike `TraceLogger` above, wrapping
/// the raw transport in a `TracingTransport` fed by this sink captures the
/// *entire* session including the init handshake, which makes the resulting
/// file replayable end-to-end via `motodiag_transport::replay::ReplayTransport`.
pub struct WireTraceRecorder {
    out: BufWriter<File>,
}

impl WireTraceRecorder {
    pub fn create(path: &Path) -> Result<Self> {
        Ok(Self {
            out: BufWriter::new(File::create(path)?),
        })
    }

    /// Create a trace file whose first line is a metadata header (see
    /// `trace_report::TraceMetadata`) followed by one event per line —
    /// the shareable "motodiag-trace/1" format.
    pub fn create_with_header(path: &Path, header: &str) -> Result<Self> {
        let mut out = BufWriter::new(File::create(path)?);
        writeln!(out, "{header}")?;
        out.flush()?;
        Ok(Self { out })
    }
}

impl TraceSink for WireTraceRecorder {
    fn record(&mut self, event: TraceEvent) {
        if let Ok(line) = serde_json::to_string(&event) {
            let _ = writeln!(self.out, "{line}");
            let _ = self.out.flush();
        }
    }
}

/// Load a wire trace previously written by [`WireTraceRecorder`], e.g. to
/// feed `ReplayTransport::from_events`.
pub fn load_wire_trace(path: &Path) -> Result<Vec<TraceEvent>> {
    let reader = BufReader::new(File::open(path)?);
    reader
        .lines()
        .map(|line| {
            let line = line?;
            serde_json::from_str(&line).map_err(|e| crate::AppError::Io(std::io::Error::other(e)))
        })
        .collect()
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
