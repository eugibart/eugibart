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

export default function App() {
  const [tab, setTab] = useState<Tab>("connect");
  const [connection, setConnection] = useState<ConnectionInfo | null>(null);

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

  // Keyboard shortcuts: 1-6 switch tabs (skipped while typing in a field).
  useEffect(() => {
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
  }, [connection]);

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
      <header className="topbar">
        <div className="brand">
          <span className="brand-name">MotoDiag</span>
          <span className="brand-sub">MV Agusta &amp; Ducati diagnostics</span>
        </div>
        <div className="conn-status">
          {connection ? (
            <>
              <span className={`dot ${connection.simulated ? "dot-sim" : "dot-live"}`} />
              <span className="conn-text">
                {connection.definition_name}
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
              <span className="dot dot-off" />
              <span className="conn-text">Not connected</span>
            </>
          )}
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
            title={t.needsConnection && !connection ? "Connect to a bike first" : `Shortcut: ${i + 1}`}
          >
            {t.label}
            <span className="tab-key" aria-hidden="true">{i + 1}</span>
          </button>
        ))}
      </nav>

      <main className="content">
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
