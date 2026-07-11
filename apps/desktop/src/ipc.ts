// Typed wrappers around the Tauri commands.
import { invoke } from "@tauri-apps/api/core";

export interface ChannelSpecInfo {
  min: number | null;
  max: number | null;
  target: number | null;
  condition: string;
  /** Citation for community-sourced figures (site name + https URL). */
  source: string | null;
  source_url: string | null;
}

export interface ChannelInfo {
  key: string;
  name: string;
  unit: string;
  verified: boolean;
  spec: ChannelSpecInfo | null;
}

export interface RoutineInfo {
  key: string;
  name: string;
  description: string;
  procedure: string[];
  risk: "low" | "medium" | "high";
  verified: boolean;
  preconditions: string[];
}

/** Community-sourced thresholds for the guided charging-system test. */
export interface ChargingTestInfo {
  rest_min_v: number;
  charging_min_v: number;
  charging_max_v: number;
  check_rpm_min: number;
  check_rpm_max: number;
  stator_notes: string;
  source: string;
  source_url: string;
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
  charging: ChargingTestInfo | null;
}

/** One bike in the structured catalog (brand → model → year → definition). */
export interface CatalogBikeInfo {
  brand: string;
  model: string;
  variants: string[];
  year_from: number;
  year_to: number | null;
  /** null = documented gap: MotoDiag can't talk to this bike yet. */
  definition_id: string | null;
  gap_note: string | null;
}

// Modification enums — kebab-case values mirror the Rust catalog schema.
export type ExhaustMod = "stock" | "slip-on-open" | "full-system";
export type ExhaustMaterial = "inox" | "titanium";
export type EpromMod = "stock" | "dealer-eprom" | "custom-map";
export type AirFilterMod = "stock" | "high-flow";

/** Match predicate: an entry applies iff, per field present, the profile's
 *  value is in the listed set. */
export interface ModRequires {
  exhaust: ExhaustMod[] | null;
  eprom: EpromMod[] | null;
  air_filter: AirFilterMod[] | null;
}

export interface ChannelOverrideInfo {
  channel: string;
  min: number | null;
  max: number | null;
  target: number | null;
  condition: string;
  note: string | null;
  /** Required by the schema: every community claim carries its citation. */
  source: string;
  source_url: string;
}

export interface SpecAdjustmentInfo {
  definition_id: string;
  requires: ModRequires;
  channel_overrides: ChannelOverrideInfo[];
}

export interface ProcedureNoteInfo {
  definition_id: string;
  routine: string;
  requires: ModRequires;
  note: string;
  source: string;
  source_url: string;
}

export interface DtcNoteInfo {
  definition_id: string;
  code: number;
  requires: ModRequires;
  cause: string;
  check: string;
  source: string;
  source_url: string;
}

/** Community knowledge about modified bikes — unverified by construction. */
export interface ModGuidanceInfo {
  adjustments: SpecAdjustmentInfo[];
  procedure_notes: ProcedureNoteInfo[];
  dtc_notes: DtcNoteInfo[];
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

/** Health-report payload for a connected garage profile — already resolved
 *  by specResolution.ts; Rust only renders, never re-resolves. */
export interface BikeReportSpecInfo {
  key: string;
  min: number | null;
  max: number | null;
  target: number | null;
  condition: string;
  source: string;
  label: string;
  citation_site: string | null;
  citation_url: string | null;
}

export interface BikeReportInfo {
  profile_name: string;
  brand: string;
  model: string;
  year: number | null;
  mods_summary: string;
  specs: BikeReportSpecInfo[];
}

/** This session's wire-trace recording (path + how many events so far). */
export interface WireTraceInfo {
  path: string;
  event_count: number;
}

export interface TraceReportRow {
  kind: string;
  request_hex: string;
  ok: boolean;
  detail: string;
}

export interface TraceChannelStat {
  key: string;
  name: string;
  unit: string;
  samples: number;
  last_value: number;
}

/** What decoded when a shared trace was analyzed against a definition. */
export interface TraceReport {
  definition_id: string;
  event_count: number;
  exchange_count: number;
  init_seen: boolean;
  identity: string | null;
  channels: TraceChannelStat[];
  dtc_reads: number;
  dtc_last_count: number | null;
  tester_present: number;
  negative_responses: number;
  unparsed_rx_bytes: number;
  rows: TraceReportRow[];
  rows_truncated: boolean;
}

export const SIMULATOR_PORT = "simulator";

export const api = {
  listDefinitions: () => invoke<DefinitionInfo[]>("list_definitions"),
  listBikeCatalog: () => invoke<CatalogBikeInfo[]>("list_bike_catalog"),
  listModGuidance: () => invoke<ModGuidanceInfo>("list_mod_guidance"),
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
  exportHealthReport: (bike: BikeReportInfo | null) =>
    invoke<string>("export_health_report", { bike }),
  exportWireTrace: () => invoke<WireTraceInfo>("export_wire_trace"),
  importWireTrace: (path: string, definitionId: string | null) =>
    invoke<TraceReport>("import_wire_trace", { path, definitionId }),
  connectVacuum: (port: string) => invoke<VacuumInfo>("connect_vacuum", { port }),
  vacuumStatus: () => invoke<VacuumInfo | null>("vacuum_status"),
  pollVacuum: () => invoke<VacuumStatus>("poll_vacuum"),
  disconnectVacuum: () => invoke<void>("disconnect_vacuum"),
};
