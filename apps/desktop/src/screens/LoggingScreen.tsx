import { useEffect, useState } from "react";
import { api } from "../ipc";

export default function LoggingScreen() {
  const [logPath, setLogPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

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

  return (
    <div className="panel">
      <h2>Data logging</h2>
      {error && <div className="error-box">{error}</div>}

      {logPath ? (
        <>
          <p className="ok-box">
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
    </div>
  );
}
