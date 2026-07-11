import { useEffect, useState } from "react";
import { api } from "../ipc";

export default function LoggingScreen() {
  const [logPath, setLogPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reportPath, setReportPath] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);

  useEffect(() => {
    api
      .connectionStatus()
      .then((s) => setLogPath(s?.csv_log_path ?? null))
      .catch(() => {});
  }, []);

  const start = async () => {
    setError(null);
    try {
      setLogPath(await api.startCsvLog());
    } catch (e) {
      setError(String(e));
    }
  };

  const stop = async () => {
    setError(null);
    try {
      await api.stopCsvLog();
      setLogPath(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const exportReport = async () => {
    setError(null);
    setExporting(true);
    try {
      setReportPath(await api.exportHealthReport());
    } catch (e) {
      setError(String(e));
    } finally {
      setExporting(false);
    }
  };

  return (
    <div className="panel">
      <h2>Data logging</h2>
      {error && (
        <div className="error-box" role="alert">
          {error}
        </div>
      )}

      {logPath ? (
        <>
          <p className="ok-box" role="status">
            Recording live data to <span className="mono">{logPath}</span>
          </p>
          <p className="muted small">
            Keep the Dashboard tab polling (or leave this recording) — every polled reading is
            appended as a CSV row.
          </p>
          <button className="btn btn-danger" onClick={stop}>
            Stop logging
          </button>
        </>
      ) : (
        <>
          <p className="muted">
            Record all live-data readings to a CSV file for later analysis (fueling, temps,
            track-day review).
          </p>
          <button className="btn btn-primary" onClick={start}>
            Start logging
          </button>
        </>
      )}

      <h2 className="section-gap">Bike health report</h2>
      <p className="muted">
        Snapshot the ECU right now — identity, fault codes with likely causes, live data —
        as a single HTML file you can keep, print, or send to a mechanic or a seller.
        Handy for pre-purchase inspections.
      </p>
      {reportPath && (
        <p className="ok-box" role="status">
          Report saved to <span className="mono">{reportPath}</span>
        </p>
      )}
      <button className="btn btn-primary" onClick={exportReport} disabled={exporting}>
        {exporting ? "Reading ECU…" : "Export health report"}
      </button>
    </div>
  );
}
