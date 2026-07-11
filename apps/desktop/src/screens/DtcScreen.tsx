import { useEffect, useState } from "react";
import { api, Dtc } from "../ipc";

export default function DtcScreen({ serviceMode }: { serviceMode: boolean }) {
  const [dtcs, setDtcs] = useState<Dtc[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);

  const read = async () => {
    setBusy(true);
    setError(null);
    try {
      setDtcs(await api.readDtcs());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    read();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!confirming) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setConfirming(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [confirming]);

  const clear = async () => {
    setBusy(true);
    setError(null);
    try {
      await api.clearDtcs(true);
      setConfirming(false);
      await read();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="panel">
      <h2>Fault codes</h2>
      {error && <div className="error-box">{error}</div>}

      {dtcs && dtcs.length === 0 && <p className="ok-box">No stored fault codes.</p>}

      {dtcs && dtcs.length > 0 && (
        <div className="dtc-list">
          {dtcs.map((d) => (
            <div className="dtc-card" key={d.code}>
              <div className="dtc-head">
                <span className="mono dtc-code">
                  {d.code.toString(16).toUpperCase().padStart(4, "0")}
                </span>
                <span className="dtc-desc">
                  {d.description ?? <span className="muted">unknown code</span>}
                </span>
                <span className="mono muted small">
                  status 0x{d.status.toString(16).toUpperCase().padStart(2, "0")}
                </span>
              </div>
              {d.causes.length > 0 && (
                <div className="dtc-section">
                  <span className="dtc-section-title">Likely causes</span>
                  <ol className="dtc-items">
                    {d.causes.map((c, i) => (
                      <li key={i}>{c}</li>
                    ))}
                  </ol>
                </div>
              )}
              {d.checks.length > 0 && (
                <div className="dtc-section">
                  <span className="dtc-section-title">What to check</span>
                  <ol className="dtc-items">
                    {d.checks.map((c, i) => (
                      <li key={i}>{c}</li>
                    ))}
                  </ol>
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      <div className="btn-row">
        <button className="btn" onClick={read} disabled={busy}>
          ↻ Re-read
        </button>
        {dtcs && dtcs.length > 0 && !confirming && (
          <button
            className="btn btn-danger"
            onClick={() => setConfirming(true)}
            disabled={busy || !serviceMode}
            title={serviceMode ? "" : "Enable service mode in the Service tab first"}
          >
            Clear fault codes…
          </button>
        )}
      </div>

      {!serviceMode && dtcs && dtcs.length > 0 && (
        <p className="muted small">
          Clearing codes changes ECU state — enable service mode in the Service tab first.
        </p>
      )}

      {confirming && (
        <div className="confirm-box">
          <p>
            Clear all stored fault codes? The fault history will be lost — record the codes
            above first if you need them.
          </p>
          <div className="btn-row">
            <button className="btn btn-danger" onClick={clear} disabled={busy}>
              Yes, clear codes
            </button>
            <button className="btn" onClick={() => setConfirming(false)}>
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
