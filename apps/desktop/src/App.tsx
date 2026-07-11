import { useCallback, useEffect, useState } from "react";
import { api, ConnectionInfo } from "./ipc";
import { BikeProfile, recallActiveProfile, rememberActiveProfile, useGarage } from "./garage";
import ConnectScreen from "./screens/connect/ConnectScreen";
import DashboardScreen from "./screens/DashboardScreen";
import DtcScreen from "./screens/DtcScreen";
import ServiceScreen from "./screens/ServiceScreen";
import SyncScreen from "./screens/SyncScreen";
import LoggingScreen from "./screens/LoggingScreen";
import HelpScreen from "./screens/HelpScreen";

type Tab = "connect" | "dashboard" | "dtcs" | "service" | "sync" | "logging" | "help";

const TABS: { id: Tab; label: string; needsConnection: boolean }[] = [
  { id: "connect", label: "Connect", needsConnection: false },
  { id: "dashboard", label: "Dashboard", needsConnection: true },
  { id: "dtcs", label: "Fault codes", needsConnection: true },
  { id: "service", label: "Service", needsConnection: true },
  // The vacuum gauge is its own device — usable without an ECU session.
  { id: "sync", label: "Sync", needsConnection: false },
  { id: "logging", label: "Logging", needsConnection: true },
  { id: "help", label: "Help", needsConnection: false },
];

const SHORTCUTS_KEY = "motodiag-shortcuts";
const THEME_KEY = "motodiag-theme";

type Theme = "dark" | "light";

function initialTheme(): Theme {
  const stored = localStorage.getItem(THEME_KEY);
  if (stored === "light" || stored === "dark") return stored;
  return window.matchMedia?.("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

export default function App() {
  const [tab, setTab] = useState<Tab>("connect");
  const [connection, setConnection] = useState<ConnectionInfo | null>(null);
  const [activeProfileId, setActiveProfileId] = useState<string | null>(null);
  const { profiles } = useGarage();
  // Derived, not stored: re-reads from the live garage store on every
  // render, so an edit (e.g. a spec override saved from the Dashboard)
  // shows up immediately instead of needing a stale snapshot refreshed.
  const activeProfile = profiles.find((p) => p.id === activeProfileId) ?? null;
  // WCAG 2.1.4: single-character shortcuts must be user-disableable.
  const [shortcuts, setShortcuts] = useState(
    () => localStorage.getItem(SHORTCUTS_KEY) !== "off",
  );
  // Follows the OS preference until the user explicitly picks a theme.
  const [theme, setTheme] = useState<Theme>(initialTheme);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  const toggleTheme = () => {
    const next: Theme = theme === "dark" ? "light" : "dark";
    setTheme(next);
    localStorage.setItem(THEME_KEY, next);
  };

  const refreshStatus = useCallback(async () => {
    try {
      const status = await api.connectionStatus();
      setConnection(status);
      // Re-associate with the saved profile after a reload — honest by
      // construction: this looks the id up directly rather than guessing
      // from the definition, so a stale/missing id just means no profile.
      setActiveProfileId(status ? (recallActiveProfile(profiles)?.id ?? null) : null);
    } catch {
      setConnection(null);
      setActiveProfileId(null);
    }
    // profiles is a stable external-store snapshot; refreshStatus only needs
    // to run on mount and on demand, not whenever the garage changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);

  // Keyboard shortcuts: 1-7 switch tabs (skipped while typing in a field,
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

  const handleConnected = (info: ConnectionInfo, profile: BikeProfile | null) => {
    setConnection(info);
    setActiveProfileId(profile?.id ?? null);
    setTab("dashboard");
  };

  const handleDisconnect = async () => {
    await api.disconnect();
    setConnection(null);
    setActiveProfileId(null);
    rememberActiveProfile(null);
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
                Connected: {activeProfile ? `${activeProfile.name} — ` : ""}
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
              <span className="dot dot-off" aria-hidden="true" />
              <span className="conn-text">Not connected</span>
            </>
          )}
          <button
            className="btn btn-small"
            onClick={toggleShortcuts}
            aria-pressed={shortcuts}
            title="Number keys 1-7 switch tabs. Turn off if they conflict with your assistive technology."
          >
            Shortcuts: {shortcuts ? "on" : "off"}
          </button>
          <button className="btn btn-small" onClick={toggleTheme}>
            <span aria-hidden="true">{theme === "dark" ? "☀" : "☾"}</span>{" "}
            {theme === "dark" ? "Light mode" : "Dark mode"}
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
        {tab === "dashboard" && connection && (
          <DashboardScreen connection={connection} activeProfile={activeProfile} />
        )}
        {tab === "dtcs" && connection && (
          <DtcScreen
            serviceMode={connection.service_mode}
            definitionId={connection.definition_id}
            activeProfile={activeProfile}
          />
        )}
        {tab === "service" && connection && (
          <ServiceScreen
            connection={connection}
            onStatusChange={refreshStatus}
            activeProfile={activeProfile}
          />
        )}
        {tab === "sync" && <SyncScreen ecuConnected={connection !== null} />}
        {tab === "logging" && connection && (
          <LoggingScreen activeProfile={activeProfile} />
        )}
        {tab === "help" && <HelpScreen />}
      </main>
    </div>
  );
}
