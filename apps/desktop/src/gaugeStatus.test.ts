import { describe, expect, it } from "vitest";
import { judgeReading } from "./gaugeStatus";
import { formatDelta, formatValue } from "./format";
import { ResolvedSpec } from "./specResolution";
import { ChargingTestInfo } from "./ipc";

const battSpec: ResolvedSpec = {
  min: 13.5,
  max: 15.0,
  target: null,
  condition: "at ~2000 rpm",
  source: "stock",
  verified: false,
  label: "stock reference",
  note: null,
  citationSite: null,
  citationUrl: null,
};

const charging: ChargingTestInfo = {
  rest_min_v: 12.4,
  charging_min_v: 14.0,
  charging_max_v: 15.0,
  check_rpm_min: 1800,
  check_rpm_max: 2600,
  stator_notes: "",
  source: "mvagusta.net",
  source_url: "https://www.mvagusta.net/",
};

describe("judgeReading — charging-band gating", () => {
  it("defers a low battery reading at idle instead of alarming", () => {
    const j = judgeReading("batt", 12.8, battSpec, charging, 1150);
    expect(j).toEqual({ kind: "deferred", bandText: "1800–2600 rpm" });
  });

  it("judges a low battery reading inside the check band", () => {
    expect(judgeReading("batt", 12.8, battSpec, charging, 2200)).toEqual({ kind: "low" });
  });

  it("flags overvoltage even at idle (regulator faults do not need revs)", () => {
    expect(judgeReading("batt", 15.4, battSpec, charging, 1150)).toEqual({ kind: "high" });
  });

  it("reports ok when in range regardless of rpm", () => {
    expect(judgeReading("batt", 14.2, battSpec, charging, 1150)).toEqual({ kind: "ok" });
  });

  it("judges normally when there is no rpm signal to gate on", () => {
    expect(judgeReading("batt", 12.8, battSpec, charging, null)).toEqual({ kind: "low" });
  });

  it("does not gate non-battery channels or bikes without a charging block", () => {
    expect(judgeReading("rpm", 900, { ...battSpec, min: 1100, max: 1200 }, charging, 900)).toEqual({
      kind: "low",
    });
    expect(judgeReading("batt", 12.8, battSpec, null, 1150)).toEqual({ kind: "low" });
  });

  it("returns null when the spec has no bounds", () => {
    expect(judgeReading("batt", 12.8, { ...battSpec, min: null, max: null }, charging, 1150)).toBeNull();
  });
});

describe("format helpers", () => {
  it("scales precision with magnitude", () => {
    expect(formatValue(1246)).toBe("1246");
    expect(formatValue(84.04)).toBe("84.0");
    expect(formatValue(2.104)).toBe("2.10");
  });

  it("renders sub-precision deltas as an em dash", () => {
    expect(formatDelta(0)).toBe("—");
    expect(formatDelta(0.001)).toBe("—");
    expect(formatDelta(null)).toBe("—");
  });

  it("signs real deltas", () => {
    expect(formatDelta(74)).toBe("+74.0");
    expect(formatDelta(740)).toBe("+740");
    expect(formatDelta(-0.78)).toBe("−0.78");
  });
});
