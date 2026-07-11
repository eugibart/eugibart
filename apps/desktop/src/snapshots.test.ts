// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { compareSnapshots, loadSnapshots, makeSnapshot, SessionSnapshot } from "./snapshots";

const reading = (key: string, name: string, unit: string, value: number) => ({
  key,
  name,
  unit,
  value,
  timestamp_ms: 0,
});

function snap(label: string, rpm: number, spread: number | null): SessionSnapshot {
  return makeSnapshot(
    label,
    null,
    [reading("rpm", "Engine speed", "rpm", rpm)],
    spread === null
      ? null
      : { channels_kpa: [31, 31 + spread], spread_kpa: spread, deltas_kpa: [0, spread], timestamp_ms: 0 },
  );
}

beforeEach(() => localStorage.clear());

describe("compareSnapshots", () => {
  it("computes per-metric deltas including vacuum", () => {
    const before = snap("before", 1300, 4.5);
    const after = snap("after", 1200, 0.4);
    const rows = compareSnapshots(before, after);

    const rpm = rows.find((r) => r.label === "Engine speed")!;
    expect(rpm.a).toBe(1300);
    expect(rpm.b).toBe(1200);
    expect(rpm.delta).toBe(-100);

    const spread = rows.find((r) => r.label === "Vacuum spread")!;
    expect(spread.delta).toBeCloseTo(-4.1);

    expect(rows.some((r) => r.label === "Cylinder 1 vacuum")).toBe(true);
  });

  it("handles a metric missing on one side without inventing a delta", () => {
    const withVac = snap("a", 1300, 2.0);
    const withoutVac = snap("b", 1250, null);
    const rows = compareSnapshots(withVac, withoutVac);
    const spread = rows.find((r) => r.label === "Vacuum spread")!;
    expect(spread.a).toBe(2.0);
    expect(spread.b).toBeNull();
    expect(spread.delta).toBeNull();
  });
});

describe("snapshot persistence", () => {
  it("starts empty on corrupt storage", () => {
    localStorage.setItem("motodiag-snapshots", "{broken");
    expect(loadSnapshots()).toEqual([]);
  });
});
