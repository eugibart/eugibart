import { useCallback, useEffect, useState } from "react";
import { api, ConnectionInfo } from "./ipc";
import ConnectScreen from "./screens/ConnectScreen";
import DashboardScreen from "./screens/DashboardScreen";
import DtcScreen from "./screens/DtcScreen";
import ServiceScreen from "./screens/ServiceScreen";
import LoggingScreen from "./screens/LoggingScreen";

type Tab = "connect" | "dashboard" | "dtcs" | "service" | "logging";

const TABS: { id: Tab; label: string; needsConnection: boolean }[] = [
  { id: "connect", label: "Connect", needsConnection: false },
  { id: "dashboard", label: "Dashboard", needsConnection: true },
  { id: "dtcs", label: "Fault codes", needsConnection: true },
  { id: "service", label: "Service", needsConnection: true },
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

      <nav className="tabs">
        {TABS.map((t) => (
          <button
            key={t.id}
            className={`tab ${tab === t.id ? "tab-active" : ""}`}
            disabled={t.needsConnection && !connection}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </nav>

      <main className="content">
        {tab === "connect" && <ConnectScreen onConnected={handleConnected} />}
        {tab === "dashboard" && connection && <DashboardScreen />}
        {tab === "dtcs" && connection && <DtcScreen serviceMode={connection.service_mode} />}
        {tab === "service" && connection && (
          <ServiceScreen connection={connection} onStatusChange={refreshStatus} />
        )}
        {tab === "logging" && connection && <LoggingScreen />}
      </main>
    </div>
  );
}
