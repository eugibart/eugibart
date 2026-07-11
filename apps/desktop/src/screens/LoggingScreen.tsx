import { useEffect, useState } from "react";
import { api, BikeReportInfo, TraceReport, WireTraceInfo } from "../ipc";
import { BikeProfile, describeMods } from "../garage";
import { resolveSpecs } from "../specResolution";

export default function LoggingScreen({ activeProfile }: { activeProfile: BikeProfile | null }) {
  const [logPath, setLogPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reportPath, setReportPath] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);
  const [trace, setTrace] = useState<WireTraceInfo | null>(null);
  const [traceError, setTraceError] = useState<string | null>(null);
  const [importPath, setImportPath] = useState("");
  const [importing, setImporting] = useState(false);
  const [traceReport, setTraceReport] = useState<TraceReport | null>(null);

  const showTrace = async () => {
    setTraceError(null);
    try {
      setTrace(await api.exportWireTrace());
    } catch (e) {
      setTraceError(String(e));
    }
  };

  const importTrace = async () => {
    setTraceError(null);
    setImporting(true);
    setTraceReport(null);
    try {
      setTraceReport(await api.importWireTrace(importPath.trim(), null));
    } catch (e) {
      setTraceError(String(e));
    } finally {
      setImporting(false);
    }
  };

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
              citation_site: s.citationSite,
              citation_url: s.citationUrl,
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

      <h2 className="section-gap">Wire traces</h2>
      <p className="muted">
        Every session is recorded as a replayable wire trace — the raw bytes between the app
        and the ECU. Sharing your trace is how the community verifies definitions against real
        bikes; importing someone else's shows exactly what decoded.
      </p>
      {traceError && (
        <div className="error-box" role="alert">
          {traceError}
        </div>
      )}
      <div className="btn-row">
        <button className="btn" onClick={showTrace}>
          Show this session's trace
        </button>
      </div>
      {trace && (
        <p className="ok-box" role="status">
          Recording to <span className="mono">{trace.path}</span> — {trace.event_count} events
          so far. Share this file to help verify the definition.
        </p>
      )}

      <h3 className="section-gap">Analyze a shared trace</h3>
      <div className="form-row">
        <label htmlFor="trace-import-path">Trace file path</label>
        <input
          id="trace-import-path"
          className="mono"
          placeholder="/path/to/motodiag-trace-….jsonl"
          value={importPath}
          onChange={(e) => setImportPath(e.target.value)}
        />
        <button
          className="btn"
          onClick={importTrace}
          disabled={importing || importPath.trim() === ""}
        >
          {importing ? "Analyzing…" : "Analyze"}
        </button>
      </div>

      {traceReport && (
        <div className="trace-report" role="status">
          <h3>
            Trace analysis — {traceReport.definition_id}
            {traceReport.negative_responses === 0 && traceReport.identity ? (
              <span className="badge badge-ok">decodes cleanly</span>
            ) : (
              <span className="badge badge-warn">gaps found</span>
            )}
          </h3>
          <ul className="pre-list">
            <li>
              {traceReport.event_count} events, {traceReport.exchange_count} request/response
              exchanges{traceReport.init_seen ? ", init handshake present" : ""}
            </li>
            <li>
              ECU identity: {traceReport.identity ?? "not found in trace"}
            </li>
            <li>
              DTC reads: {traceReport.dtc_reads}
              {traceReport.dtc_last_count !== null &&
                ` (last read: ${traceReport.dtc_last_count} stored code(s))`}
            </li>
            <li>
              Keep-alives: {traceReport.tester_present} · negative responses:{" "}
              {traceReport.negative_responses} · unparsed bytes: {traceReport.unparsed_rx_bytes}
            </li>
          </ul>
          {traceReport.channels.length > 0 && (
            <table className="table">
              <caption className="visually-hidden">Channels decoded from the trace</caption>
              <thead>
                <tr>
                  <th>Channel</th>
                  <th>Samples</th>
                  <th>Last value</th>
                </tr>
              </thead>
              <tbody>
                {traceReport.channels.map((c) => (
                  <tr key={c.key}>
                    <td>{c.name}</td>
                    <td className="mono">{c.samples}</td>
                    <td className="mono">
                      {c.last_value.toFixed(2)} {c.unit}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          <details>
            <summary>
              Exchange log ({traceReport.rows.length}
              {traceReport.rows_truncated ? ", truncated" : ""})
            </summary>
            <ul className="pre-list mono small">
              {traceReport.rows.map((row, i) => (
                <li key={i}>
                  {row.ok ? "✓" : "✗"} {row.kind} — {row.detail}
                </li>
              ))}
            </ul>
          </details>
        </div>
      )}
    </div>
  );
}
