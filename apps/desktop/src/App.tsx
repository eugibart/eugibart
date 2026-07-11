import { useCallback, useEffect, useState } from "react";
import { api, ConnectionInfo } from "./ipc";
import ConnectScreen from "./screens/ConnectScreen";
import DashboardScreen from "./screens/DashboardScreen";
import DtcScreen from "./screens/DtcScreen";
import ServiceScreen from "./screens/ServiceScreen";
import SyncScreen from "./screens/SyncScreen";
import LoggingScreen from "./screens/LoggingScreen";

type Tab = "connect" | "dashboard" | "dtcs" | "service" | "sync" | "logging";

const TABS: { id: Tab; label: string; needsConnection: boolean }[] = [
  { id: "connect", label: "Connect", needsConnection: false },
  { id: "dashboard", label: "Dashboard", needsConnection: true },
  { id: "dtcs", label: "Fault codes", needsConnection: true },
  { id: "service", label: "Service", needsConnection: true },
  // The vacuum gauge is its own device — usable without an ECU session.
  { id: "sync", label: "Sync", needsConnection: false },
  { id: "logging", label: "Logging", needsConnection: true },
];

const SHORTCUTS_KEY = "motodiag-shortcuts";

export default function App() {
  const [tab, setTab] = useState<Tab>("connect");
  const [connection, setConnection] = useState<ConnectionInfo | null>(null);
  // WCAG 2.1.4: single-character shortcuts must be user-disableable.
  const [shortcuts, setShortcuts] = useState(
    () => localStorage.getItem(SHORTCUTS_KEY) !== "off",
  );

  const refreshStatus = useCallback(async () => {
    try {
      setConnection(await api.connectionStatus());
    } catch {
      setConnection(null);
    }
  }, []);

  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);

  // Keyboard shortcuts: 1-6 switch tabs (skipped while typing in a field,
  // and entirely inert when the user has turned shortcuts off).
  useEffect(() => {
    if (!shortcuts) return;
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName)) return;
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      const index = Number.parseInt(e.key, 10) - 1;
      const t = TABS[index];
      if (t && !(t.needsConnection && !connection)) setTab(t.id);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [connection, shortcuts]);

  const toggleShortcuts = () => {
    const next = !shortcuts;
    setShortcuts(next);
    localStorage.setItem(SHORTCUTS_KEY, next ? "on" : "off");
  };

  const handleConnected = (info: ConnectionInfo) => {
    setConnection(info);
    setTab("dashboard");
  };

  const handleDisconnect = async () => {
    await api.disconnect();
    setConnection(null);
    setTab("connect");
  };

  return (
    <div className="app">
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <header className="topbar">
        <div className="brand">
          <h1 className="brand-name">MotoDiag</h1>
          <span className="brand-sub">MV Agusta &amp; Ducati diagnostics</span>
        </div>
        <div className="conn-status" role="status">
          {connection ? (
            <>
              <span
                className={`dot ${connection.simulated ? "dot-sim" : "dot-live"}`}
                aria-hidden="true"
              />
              <span className="conn-text">
                Connected: {connection.definition_name}
                {connection.simulated ? " (simulator)" : ""}
              </span>
              {connection.service_mode && (
                <span className="badge badge-warn">service mode</span>
              )}
              <button className="btn btn-small" onClick={handleDisconnect}>
                Disconnect
              </button>
            </>
          ) : (
            <>
              <span className="dot dot-off" aria-hidden="true" />
              <span className="conn-text">Not connected</span>
            </>
          )}
          <button
            className="btn btn-small"
            onClick={toggleShortcuts}
            aria-pressed={shortcuts}
            title="Number keys 1-6 switch tabs. Turn off if they conflict with your assistive technology."
          >
            Shortcuts: {shortcuts ? "on" : "off"}
          </button>
        </div>
      </header>

      <nav className="tabs" aria-label="Main navigation">
        {TABS.map((t, i) => (
          <button
            key={t.id}
            className={`tab ${tab === t.id ? "tab-active" : ""}`}
            disabled={t.needsConnection && !connection}
            onClick={() => setTab(t.id)}
            aria-current={tab === t.id ? "page" : undefined}
            title={t.needsConnection && !connection ? "Connect to a bike first" : undefined}
          >
            {t.label}
            {shortcuts && (
              <span className="tab-key" aria-hidden="true">
                {i + 1}
              </span>
            )}
          </button>
        ))}
      </nav>

      <main className="content" id="main" tabIndex={-1}>
        {tab === "connect" && <ConnectScreen onConnected={handleConnected} />}
        {tab === "dashboard" && connection && <DashboardScreen connection={connection} />}
        {tab === "dtcs" && connection && <DtcScreen serviceMode={connection.service_mode} />}
        {tab === "service" && connection && (
          <ServiceScreen connection={connection} onStatusChange={refreshStatus} />
        )}
        {tab === "sync" && <SyncScreen ecuConnected={connection !== null} />}
        {tab === "logging" && connection && <LoggingScreen />}
      </main>
    </div>
  );
}
