// Typed wrappers around the Tauri commands.
import { invoke } from "@tauri-apps/api/core";

export interface ChannelInfo {
  key: string;
  name: string;
  unit: string;
  verified: boolean;
}

export interface RoutineInfo {
  key: string;
  name: string;
  description: string;
  risk: "low" | "medium" | "high";
  verified: boolean;
  preconditions: string[];
}

export interface DefinitionInfo {
  id: string;
  name: string;
  manufacturer: string;
  models: string[];
  bus: string;
  verified: boolean;
  notes: string | null;
  channels: ChannelInfo[];
  routines: RoutineInfo[];
}

export interface ConnectionInfo {
  definition_id: string;
  definition_name: string;
  identity_text: string;
  service_mode: boolean;
  csv_log_path: string | null;
  simulated: boolean;
}

export interface Dtc {
  code: number;
  status: number;
  description: string | null;
  causes: string[];
  checks: string[];
}

export interface Reading {
  key: string;
  name: string;
  unit: string;
  value: number;
  timestamp_ms: number;
}

export interface TroubleshootStep {
  name: string;
  status: "passed" | "failed" | "skipped";
  detail: string;
  suggestion: string | null;
}

export interface VacuumInfo {
  port: string;
  channels: number;
  simulated: boolean;
}

export interface VacuumStatus {
  channels_kpa: number[];
  spread_kpa: number;
  deltas_kpa: number[];
  timestamp_ms: number;
}

export const SIMULATOR_PORT = "simulator";

export const api = {
  listDefinitions: () => invoke<DefinitionInfo[]>("list_definitions"),
  listSerialPorts: () => invoke<string[]>("list_serial_ports"),
  connect: (definitionId: string, port: string) =>
    invoke<ConnectionInfo>("connect", { definitionId, port }),
  disconnect: () => invoke<void>("disconnect"),
  connectionStatus: () => invoke<ConnectionInfo | null>("connection_status"),
  readDtcs: () => invoke<Dtc[]>("read_dtcs"),
  clearDtcs: (confirmed: boolean) => invoke<void>("clear_dtcs", { confirmed }),
  pollLiveData: () => invoke<Reading[]>("poll_live_data"),
  enableServiceMode: () => invoke<void>("enable_service_mode"),
  runRoutine: (key: string, confirmed: boolean) =>
    invoke<string>("run_routine", { key, confirmed }),
  startCsvLog: () => invoke<string>("start_csv_log"),
  stopCsvLog: () => invoke<string | null>("stop_csv_log"),
  troubleshootConnection: (definitionId: string, port: string) =>
    invoke<TroubleshootStep[]>("troubleshoot_connection", { definitionId, port }),
  exportHealthReport: () => invoke<string>("export_health_report"),
  connectVacuum: (port: string) => invoke<VacuumInfo>("connect_vacuum", { port }),
  vacuumStatus: () => invoke<VacuumInfo | null>("vacuum_status"),
  pollVacuum: () => invoke<VacuumStatus>("poll_vacuum"),
  disconnectVacuum: () => invoke<void>("disconnect_vacuum"),
};
