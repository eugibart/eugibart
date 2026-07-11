// Session snapshots: freeze the current readings (and vacuum state, when
// the gauge is connected) so before/after comparisons of sync and CO work
// are evidence, not memory. Persisted in localStorage like the garage.
import { useCallback, useSyncExternalStore } from "react";
import { Reading, VacuumStatus } from "./ipc";

export interface SnapshotReading {
  key: string;
  name: string;
  unit: string;
  value: number;
}

export interface SessionSnapshot {
  id: string;
  label: string;
  atMs: number;
  profileName: string | null;
  readings: SnapshotReading[];
  vacuum: { channels_kpa: number[]; spread_kpa: number } | null;
}

const SNAPSHOTS_KEY = "motodiag-snapshots";
const SNAPSHOTS_VERSION = 1;
/** Bounded: oldest snapshots drop off. Twenty covers a full tuning session. */
const MAX_SNAPSHOTS = 20;

interface Envelope {
  version: number;
  snapshots: SessionSnapshot[];
}

export function loadSnapshots(): SessionSnapshot[] {
  try {
    const raw = localStorage.getItem(SNAPSHOTS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as Envelope;
    if (parsed.version !== SNAPSHOTS_VERSION || !Array.isArray(parsed.snapshots)) return [];
    return parsed.snapshots;
  } catch {
    return [];
  }
}

function persist(snapshots: SessionSnapshot[]): void {
  localStorage.setItem(
    SNAPSHOTS_KEY,
    JSON.stringify({ version: SNAPSHOTS_VERSION, snapshots }),
  );
}

let cache: SessionSnapshot[] | null = null;
const listeners = new Set<() => void>();

function read(): SessionSnapshot[] {
  if (cache === null) cache = loadSnapshots();
  return cache;
}

function write(snapshots: SessionSnapshot[]): void {
  cache = snapshots;
  persist(snapshots);
  for (const l of listeners) l();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function makeSnapshot(
  label: string,
  profileName: string | null,
  readings: Reading[],
  vacuum: VacuumStatus | null,
): SessionSnapshot {
  return {
    id: crypto.randomUUID(),
    label,
    atMs: Date.now(),
    profileName,
    readings: readings.map((r) => ({ key: r.key, name: r.name, unit: r.unit, value: r.value })),
    vacuum: vacuum
      ? { channels_kpa: vacuum.channels_kpa.slice(), spread_kpa: vacuum.spread_kpa }
      : null,
  };
}

export function useSnapshots() {
  const snapshots = useSyncExternalStore(subscribe, read);

  const addSnapshot = useCallback((snapshot: SessionSnapshot) => {
    const next = [...read(), snapshot];
    // Oldest-first eviction, but never silently: the UI shows the cap.
    write(next.slice(Math.max(0, next.length - MAX_SNAPSHOTS)));
  }, []);

  const removeSnapshot = useCallback((id: string) => {
    write(read().filter((s) => s.id !== id));
  }, []);

  return { snapshots, addSnapshot, removeSnapshot, max: MAX_SNAPSHOTS };
}

/** One comparison row: a metric present in either snapshot. */
export interface CompareRow {
  label: string;
  unit: string;
  a: number | null;
  b: number | null;
  delta: number | null;
}

export function compareSnapshots(a: SessionSnapshot, b: SessionSnapshot): CompareRow[] {
  const rows: CompareRow[] = [];
  const keys = new Map<string, { name: string; unit: string }>();
  for (const r of [...a.readings, ...b.readings]) {
    if (!keys.has(r.key)) keys.set(r.key, { name: r.name, unit: r.unit });
  }
  for (const [key, meta] of keys) {
    const va = a.readings.find((r) => r.key === key)?.value ?? null;
    const vb = b.readings.find((r) => r.key === key)?.value ?? null;
    rows.push({
      label: meta.name,
      unit: meta.unit,
      a: va,
      b: vb,
      delta: va !== null && vb !== null ? vb - va : null,
    });
  }
  if (a.vacuum || b.vacuum) {
    rows.push({
      label: "Vacuum spread",
      unit: "kPa",
      a: a.vacuum?.spread_kpa ?? null,
      b: b.vacuum?.spread_kpa ?? null,
      delta:
        a.vacuum && b.vacuum ? b.vacuum.spread_kpa - a.vacuum.spread_kpa : null,
    });
    const cyls = Math.max(a.vacuum?.channels_kpa.length ?? 0, b.vacuum?.channels_kpa.length ?? 0);
    for (let i = 0; i < cyls; i++) {
      const va = a.vacuum?.channels_kpa[i] ?? null;
      const vb = b.vacuum?.channels_kpa[i] ?? null;
      rows.push({
        label: `Cylinder ${i + 1} vacuum`,
        unit: "kPa",
        a: va,
        b: vb,
        delta: va !== null && vb !== null ? vb - va : null,
      });
    }
  }
  return rows;
}
