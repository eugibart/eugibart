import { useEffect, useState } from "react";
import {
  api,
  CatalogBikeInfo,
  ConnectionInfo,
  DefinitionInfo,
  ModGuidanceInfo,
  SIMULATOR_PORT,
  TroubleshootStep,
} from "../../ipc";
import { BikeProfile, rememberActiveProfile, useGarage } from "../../garage";
import GaragePanel from "./GaragePanel";
import BikeWizard from "./BikeWizard";
import HowToPanel from "../../HowToPanel";

export default function ConnectScreen({
  onConnected,
  onShowHelp,
}: {
  onConnected: (info: ConnectionInfo, profile: BikeProfile | null) => void;
  onShowHelp?: () => void;
}) {
  const [definitions, setDefinitions] = useState<DefinitionInfo[]>([]);
  const [catalog, setCatalog] = useState<CatalogBikeInfo[]>([]);
  const [guidance, setGuidance] = useState<ModGuidanceInfo | null>(null);
  const [ports, setPorts] = useState<string[]>([]);
  const [port, setPort] = useState(SIMULATOR_PORT);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [troubleshooting, setTroubleshooting] = useState(false);
  const [steps, setSteps] = useState<TroubleshootStep[] | null>(null);

  const [wizardOpen, setWizardOpen] = useState(false);
  const [editingProfile, setEditingProfile] = useState<BikeProfile | null>(null);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [advancedDefinitionId, setAdvancedDefinitionId] = useState("");
  // Whichever definition was last attempted, garage or Advanced — the
  // troubleshooter tests against it (see docs on the design decision).
  const [troubleshootDefinitionId, setTroubleshootDefinitionId] = useState("");

  const { profiles, addProfile, updateProfile, removeProfile } = useGarage();

  const refresh = async () => {
    const [defs, portList, catalogList, guidanceInfo] = await Promise.all([
      api.listDefinitions(),
      api.listSerialPorts(),
      api.listBikeCatalog(),
      api.listModGuidance(),
    ]);
    setDefinitions(defs);
    setPorts(portList);
    setCatalog(catalogList);
    setGuidance(guidanceInfo);
    if (!advancedDefinitionId && defs.length > 0) {
      const brutale = defs.find((d) => d.id.includes("brutale")) ?? defs[0];
      setAdvancedDefinitionId(brutale.id);
      setTroubleshootDefinitionId(brutale.id);
    }
  };

  useEffect(() => {
    refresh().catch((e) => setError(String(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const connectWith = async (definitionId: string, profile: BikeProfile | null) => {
    setBusy(true);
    setError(null);
    setSteps(null);
    setTroubleshootDefinitionId(definitionId);
    try {
      const info = await api.connect(definitionId, port);
      rememberActiveProfile(profile?.id ?? null);
      onConnected(info, profile);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const troubleshoot = async () => {
    setTroubleshooting(true);
    setSteps(null);
    setError(null);
    try {
      setSteps(await api.troubleshootConnection(troubleshootDefinitionId, port));
    } catch (e) {
      setError(String(e));
    } finally {
      setTroubleshooting(false);
    }
  };

  const saveProfile = (profile: BikeProfile, alsoConnect: boolean) => {
    if (editingProfile) updateProfile(profile);
    else addProfile(profile);
    setWizardOpen(false);
    setEditingProfile(null);
    if (alsoConnect) connectWith(profile.definitionId, profile);
  };

  const advancedDef = definitions.find((d) => d.id === advancedDefinitionId);
  // Whose bike the plug-in guide describes: the Advanced pick when that
  // flow is open, else the first garage bike, else the default definition.
  const guideDef =
    (advancedOpen ? advancedDef : undefined) ??
    definitions.find((d) => d.id === profiles[0]?.definitionId) ??
    advancedDef;

  return (
    <div className="panel panel-wide">
      <h2>Connect to a bike</h2>

      {wizardOpen ? (
        // The wizard owns the screen: no port row, no Advanced picker, no
        // footer competing for attention mid-flow.
        <BikeWizard
          catalog={catalog}
          definitions={definitions}
          guidance={guidance}
          initialProfile={editingProfile ?? undefined}
          onCancel={() => {
            setWizardOpen(false);
            setEditingProfile(null);
          }}
          onSave={saveProfile}
        />
      ) : (
        <>
          <h3>Your garage</h3>
          {profiles.length === 0 ? (
            <div className="empty-state empty-garage">
              <span className="glyph" aria-hidden="true">
                🏍
              </span>
              <p>
                Tell MotoDiag about your bike once — model, year, modifications — and
                reconnecting becomes one click, with reference figures matched to your setup.
              </p>
              <button
                className="btn btn-primary"
                onClick={() => {
                  setEditingProfile(null);
                  setWizardOpen(true);
                }}
              >
                Add your bike
              </button>
              {onShowHelp && (
                <p className="muted small">
                  New here?{" "}
                  <button className="link-button" onClick={onShowHelp}>
                    The Help tab explains what the app does and what to expect.
                  </button>
                </p>
              )}
            </div>
          ) : (
            <>
              <GaragePanel
                profiles={profiles}
                definitions={definitions}
                busy={busy}
                onConnect={(p) => connectWith(p.definitionId, p)}
                onEdit={(p) => {
                  setEditingProfile(p);
                  setWizardOpen(true);
                }}
                onRemove={removeProfile}
              />
              <p className="btn-row">
                {/* Secondary once bikes exist: the garage cards' Connect is
                    this screen's primary action. */}
                <button
                  className="btn"
                  onClick={() => {
                    setEditingProfile(null);
                    setWizardOpen(true);
                  }}
                >
                  Add your bike
                </button>
              </p>
            </>
          )}

          <h3 className="section-gap">Connection</h3>
          <div className="form-row">
            <label htmlFor="connect-port">Port</label>
        <select id="connect-port" value={port} onChange={(e) => setPort(e.target.value)}>
          {ports.map((p) => (
            <option key={p} value={p}>
              {p === SIMULATOR_PORT ? "Built-in ECU simulator (no hardware)" : p}
            </option>
          ))}
        </select>
        <button className="btn btn-small" onClick={() => refresh()}>
          <span aria-hidden="true">↻</span> Refresh
        </button>
        <button
          className="btn btn-small"
          onClick={troubleshoot}
          disabled={troubleshooting || !troubleshootDefinitionId}
          title="Step-by-step check of cable, port, wiring, and ECU handshake"
        >
          {troubleshooting ? "Testing…" : "Troubleshoot connection"}
        </button>
      </div>

      {error && (
        <div className="error-box" role="alert">
          {error}
        </div>
      )}

      {steps && (
        <div className="ts-panel" role="status" aria-live="polite">
          <h3>Connection check</h3>
          {steps.map((s) => (
            <div className={`ts-step ts-${s.status}`} key={s.name}>
              <div className="ts-head">
                <span className="ts-icon">
                  {s.status === "passed" ? "✓" : s.status === "failed" ? "✗" : "○"}
                </span>
                <span className="ts-name">{s.name}</span>
              </div>
              <div className="ts-detail">{s.detail}</div>
              {s.suggestion && <div className="ts-suggestion">→ {s.suggestion}</div>}
            </div>
          ))}
        </div>
      )}

      <details className="section-disclosure section-gap">
        <summary>How to plug in — port location &amp; tools</summary>
        {guideDef && (
          <p className="muted small">
            For <strong>{guideDef.name}</strong>
            {profiles[0] && !advancedOpen ? ` (${profiles[0].name})` : ""} — locations are
            approximate community reports, drawn schematically.
          </p>
        )}
        <HowToPanel
          guide={guideDef?.connector_access ?? null}
          bodyStyle={guideDef?.body_style ?? "naked"}
          fallback={`We don't have a verified connector location for ${
            guideDef?.name ?? "this bike"
          } yet — check the workshop manual wiring diagram. Any FTDI-based KKL cable works once you've found it (docs/HARDWARE.md).`}
        />
      </details>

      <button
        className="btn btn-small section-gap"
        aria-expanded={advancedOpen}
        aria-controls="advanced-connect"
        onClick={() => setAdvancedOpen((v) => !v)}
      >
        {advancedOpen ? "Hide" : "Show"} advanced (pick an ECU definition directly)
      </button>

      {advancedOpen && (
        <div id="advanced-connect" className="def-details">
          <p className="muted small">
            Pick the ECU definition directly if your bike isn't in the catalog, or you just want
            to connect without saving a profile.
          </p>
          <div className="form-row">
            <label htmlFor="connect-definition">ECU definition</label>
            <select
              id="connect-definition"
              value={advancedDefinitionId}
              onChange={(e) => setAdvancedDefinitionId(e.target.value)}
            >
              {definitions.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.manufacturer} — {d.name}
                </option>
              ))}
            </select>
          </div>

          {advancedDef && (
            <div className="def-details">
              <div>
                <span className="muted">Bus:</span> {advancedDef.bus}
                {"  "}
                {advancedDef.verified ? (
                  <span className="badge badge-ok">verified</span>
                ) : (
                  <span className="badge badge-warn">unverified definition</span>
                )}
              </div>
              {advancedDef.models.length > 0 && (
                <div>
                  <span className="muted">Models:</span> {advancedDef.models.join(", ")}
                </div>
              )}
              {advancedDef.notes && <p className="notes">{advancedDef.notes}</p>}
            </div>
          )}

          <div className="btn-row">
            <button
              className="btn btn-primary"
              onClick={() => connectWith(advancedDefinitionId, null)}
              disabled={busy || !advancedDefinitionId}
            >
              {busy ? "Connecting…" : "Connect without saving"}
            </button>
          </div>
        </div>
      )}

          <p className="muted small">
            Connecting is always read-only. Service operations require explicitly enabling
            service mode after connecting.
          </p>
        </>
      )}
    </div>
  );
}
