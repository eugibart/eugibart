import { useEffect, useState } from "react";
import { api, BikeReportInfo } from "../ipc";
import { BikeProfile, describeMods } from "../garage";
import { resolveSpecs } from "../specResolution";

export default function LoggingScreen({ activeProfile }: { activeProfile: BikeProfile | null }) {
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
      let bike: BikeReportInfo | null = null;
      if (activeProfile) {
        const [status, defs, guidance] = await Promise.all([
          api.connectionStatus(),
          api.listDefinitions(),
          api.listModGuidance(),
        ]);
        const def = defs.find((d) => d.id === status?.definition_id);
        if (def) {
          const specs = resolveSpecs(def, guidance, activeProfile);
          bike = {
            profile_name: activeProfile.name,
            brand: activeProfile.brand,
            model: activeProfile.model,
            year: activeProfile.year,
            mods_summary: describeMods(activeProfile.mods),
            specs: Object.entries(specs).map(([key, s]) => ({
              key,
              min: s.min,
              max: s.max,
              target: s.target,
              condition: s.condition,
              source: s.source,
              label: s.label,
            })),
          };
        }
      }
      setReportPath(await api.exportHealthReport(bike));
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
