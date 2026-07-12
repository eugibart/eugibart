// Turns a raw in/out-of-range check into what the gauge should actually
// SAY. The one place this differs from plain inRange(): a charging-voltage
// minimum only applies with the engine revving in the definition's check
// band ("13.5–15.0 V @ ~2000 rpm"). At rest or idle a healthy battery sits
// below that minimum, so judging it there paints a fake fault on every
// healthy bike — the #1 way a diagnostic tool loses an owner's trust.
//
// Rules, deliberately asymmetric:
// - value above max  -> "high" always (overvoltage at idle IS a real fault)
// - value below min  -> "low" only when rpm is inside the check band;
//                       otherwise "deferred" (neutral, points at the test)
// - in range         -> "ok"
// - no rpm signal    -> judge normally (nothing to gate on)
import { ChargingTestInfo } from "./ipc";
import { ResolvedSpec } from "./specResolution";

export type GaugeJudgement =
  | { kind: "ok" }
  | { kind: "low" }
  | { kind: "high" }
  | { kind: "deferred"; bandText: string };

export function judgeReading(
  key: string,
  value: number,
  spec: ResolvedSpec,
  charging: ChargingTestInfo | null,
  rpm: number | null,
): GaugeJudgement | null {
  if (spec.min === null && spec.max === null) return null;

  if (spec.max !== null && value > spec.max) return { kind: "high" };

  if (spec.min !== null && value < spec.min) {
    const gated = key === "batt" && charging !== null;
    if (gated && rpm !== null) {
      const inBand = rpm >= charging.check_rpm_min && rpm <= charging.check_rpm_max;
      if (!inBand) {
        return {
          kind: "deferred",
          bandText: `${charging.check_rpm_min.toFixed(0)}–${charging.check_rpm_max.toFixed(0)} rpm`,
        };
      }
    }
    return { kind: "low" };
  }

  return { kind: "ok" };
}
