//! Vacuum gauge ("vacuometro") support for throttle-body synchronization.
//!
//! Throttle-body sync is the mandatory precursor to a meaningful CO/idle
//! adjustment: until every cylinder pulls equal vacuum, CO trim chases a
//! moving target. Workshops watch an analog gauge with one eye and the tach
//! with the other; MotoDiag instead reads a cheap open-hardware digital
//! gauge (see `tools/vacuum-gauge-firmware/` and docs/VACUOMETRO.md) over a
//! second USB serial port — fully independent of the K-line session — and
//! shows per-cylinder vacuum next to live ECU data.
//!
//! ## Line protocol
//!
//! The firmware prints one ASCII line per sample at ~10 Hz:
//!
//! ```text
//! HELLO,motodiag-vac,4          on boot / reconnect (channel count)
//! VAC,31.2,31.5,30.9,31.4       one absolute-pressure sample per line, kPa
//! ```
//!
//! Values are absolute manifold pressure in kPa (an idling engine typically
//! sits around 25–40 kPa; ~100 kPa means the port is open to atmosphere).

use std::time::Duration;

use crate::live_data::now_ms;

#[derive(Debug, thiserror::Error)]
pub enum VacuumError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("timed out waiting for a sample")]
    Timeout,
    #[error("gauge disconnected")]
    Closed,
}

pub type Result<T> = std::result::Result<T, VacuumError>;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct VacuumReading {
    /// Absolute manifold pressure per cylinder, in kPa.
    pub channels_kpa: Vec<f64>,
    pub timestamp_ms: u64,
}

impl VacuumReading {
    /// Max−min across cylinders; the number you drive toward zero while
    /// turning the balance screws.
    pub fn spread_kpa(&self) -> f64 {
        let (mut min, mut max) = (f64::INFINITY, f64::NEG_INFINITY);
        for &v in &self.channels_kpa {
            min = min.min(v);
            max = max.max(v);
        }
        if self.channels_kpa.is_empty() {
            0.0
        } else {
            max - min
        }
    }

    /// Difference of each cylinder vs. a reference cylinder (0-based index).
    /// The reference's own delta is 0 by construction.
    pub fn deltas_vs_reference(&self, reference: usize) -> Vec<f64> {
        let base = self
            .channels_kpa
            .get(reference)
            .copied()
            .unwrap_or_default();
        self.channels_kpa.iter().map(|v| v - base).collect()
    }
}

/// One parsed line of gauge output.
#[derive(Debug, Clone, PartialEq)]
pub enum GaugeLine {
    Hello { channels: usize },
    Sample(Vec<f64>),
}

/// Parse one line of the gauge protocol. Returns `None` for anything
/// unrecognized (boot noise, partial lines, debug output) — the reader just
/// skips those, keeping the protocol forgiving of microcontroller chatter.
pub fn parse_line(line: &str) -> Option<GaugeLine> {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix("HELLO,motodiag-vac,") {
        let channels: usize = rest.trim().parse().ok()?;
        if (1..=8).contains(&channels) {
            return Some(GaugeLine::Hello { channels });
        }
        return None;
    }
    if let Some(rest) = line.strip_prefix("VAC,") {
        let values: Option<Vec<f64>> = rest.split(',').map(|v| v.trim().parse().ok()).collect();
        let values = values?;
        if (1..=8).contains(&values.len()) && values.iter().all(|v| v.is_finite()) {
            return Some(GaugeLine::Sample(values));
        }
    }
    None
}

/// A source of vacuum readings.
pub trait VacuumGauge: Send {
    /// Block up to `timeout` for the next sample.
    fn read(&mut self, timeout: Duration) -> Result<VacuumReading>;
}

/// Simulated 4-cylinder gauge for tests and the in-app simulator mode.
///
/// Starts visibly unbalanced and drifts toward balance over successive
/// reads, so the UI demo tells the story of a sync session actually
/// converging (cylinder 3 starts ~4 kPa off, the classic "one screw way
/// out" situation).
pub struct MockVacuumGauge {
    base_kpa: f64,
    offsets: [f64; 4],
    tick: u64,
}

impl Default for MockVacuumGauge {
    fn default() -> Self {
        Self {
            base_kpa: 31.0,
            offsets: [0.0, 0.8, 4.2, -0.6],
            tick: 0,
        }
    }
}

impl VacuumGauge for MockVacuumGauge {
    fn read(&mut self, _timeout: Duration) -> Result<VacuumReading> {
        self.tick += 1;
        // Converge offsets toward zero, with a little deterministic ripple
        // standing in for combustion pulsing.
        let ripple = |t: u64, phase: u64| ((t.wrapping_add(phase) % 7) as f64 - 3.0) * 0.05;
        let channels_kpa = (0..4)
            .map(|i| {
                let decay = 0.98_f64.powi(self.tick as i32);
                self.base_kpa + self.offsets[i] * decay + ripple(self.tick, i as u64 * 3)
            })
            .collect();
        Ok(VacuumReading {
            channels_kpa,
            timestamp_ms: now_ms(),
        })
    }
}

/// Vacuum gauge over a USB serial port (the open-hardware firmware in
/// `tools/vacuum-gauge-firmware/`). Line-buffered, skips unparseable lines.
pub struct SerialVacuumGauge {
    port: Box<dyn serialport::SerialPort>,
    buf: Vec<u8>,
    /// Channel count announced by the firmware's HELLO, once seen.
    pub announced_channels: Option<usize>,
}

impl SerialVacuumGauge {
    pub fn open(path: &str) -> Result<Self> {
        let port = serialport::new(path, 115_200)
            .timeout(Duration::from_millis(50))
            .open()
            .map_err(|e| VacuumError::Io(std::io::Error::other(e)))?;
        Ok(Self {
            port,
            buf: Vec::new(),
            announced_channels: None,
        })
    }

    fn read_line(&mut self, deadline: std::time::Instant) -> Result<String> {
        use std::io::Read;
        loop {
            if let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = self.buf.drain(..=pos).collect();
                return Ok(String::from_utf8_lossy(&line).to_string());
            }
            if std::time::Instant::now() >= deadline {
                return Err(VacuumError::Timeout);
            }
            let mut chunk = [0u8; 128];
            match self.port.read(&mut chunk) {
                Ok(0) => {}
                Ok(n) => self.buf.extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => return Err(VacuumError::Io(e)),
            }
        }
    }
}

impl VacuumGauge for SerialVacuumGauge {
    fn read(&mut self, timeout: Duration) -> Result<VacuumReading> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let line = self.read_line(deadline)?;
            match parse_line(&line) {
                Some(GaugeLine::Sample(channels_kpa)) => {
                    return Ok(VacuumReading {
                        channels_kpa,
                        timestamp_ms: now_ms(),
                    })
                }
                Some(GaugeLine::Hello { channels }) => {
                    self.announced_channels = Some(channels);
                }
                None => {} // skip noise
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_samples_and_hello() {
        assert_eq!(
            parse_line("VAC,31.2,31.5,30.9,31.4\n"),
            Some(GaugeLine::Sample(vec![31.2, 31.5, 30.9, 31.4]))
        );
        assert_eq!(
            parse_line("VAC, 28.0 , 28.5"),
            Some(GaugeLine::Sample(vec![28.0, 28.5]))
        );
        assert_eq!(
            parse_line("HELLO,motodiag-vac,4"),
            Some(GaugeLine::Hello { channels: 4 })
        );
    }

    #[test]
    fn rejects_noise_and_malformed_lines() {
        for junk in [
            "",
            "garbage",
            "VAC,",
            "VAC,abc,def",
            "VAC,1,2,3,4,5,6,7,8,9", // too many channels
            "VAC,inf,1.0",
            "HELLO,motodiag-vac,0",
            "HELLO,motodiag-vac,banana",
            "HELLO,other-device,4",
        ] {
            assert_eq!(parse_line(junk), None, "should reject: {junk:?}");
        }
    }

    #[test]
    fn spread_and_deltas() {
        let reading = VacuumReading {
            channels_kpa: vec![31.0, 31.8, 35.2, 30.4],
            timestamp_ms: 0,
        };
        assert!((reading.spread_kpa() - 4.8).abs() < 1e-9);
        let deltas = reading.deltas_vs_reference(0);
        assert_eq!(deltas.len(), 4);
        assert_eq!(deltas[0], 0.0);
        assert!((deltas[2] - 4.2).abs() < 1e-9);
    }

    #[test]
    fn empty_reading_has_zero_spread() {
        let reading = VacuumReading {
            channels_kpa: vec![],
            timestamp_ms: 0,
        };
        assert_eq!(reading.spread_kpa(), 0.0);
        assert!(reading.deltas_vs_reference(0).is_empty());
    }

    #[test]
    fn mock_gauge_converges_toward_balance() {
        let mut gauge = MockVacuumGauge::default();
        let first = gauge.read(Duration::from_millis(1)).unwrap();
        assert_eq!(first.channels_kpa.len(), 4);
        let initial_spread = first.spread_kpa();
        assert!(initial_spread > 3.0, "starts visibly unbalanced");

        let mut last = first;
        for _ in 0..300 {
            last = gauge.read(Duration::from_millis(1)).unwrap();
        }
        assert!(
            last.spread_kpa() < 1.0,
            "converges toward balance, got spread {}",
            last.spread_kpa()
        );
    }
}
