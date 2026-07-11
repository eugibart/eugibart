import { useEffect, useRef, useState } from "react";
import { ChargingTestInfo, Reading } from "./ipc";
import SourceLink from "./SourceLink";

/**
 * Guided charging-system test: rest → idle → revved, captured from live
 * data. The charging system (stator, regulator/rectifier, connectors) is
 * the most notorious real-world failure on both marques; the thresholds and
 * the stator-vs-regulator diagnostic split come from cited community
 * threads (see the definition's [charging] block).
 *
 * Auto-capture: each step waits until the rpm signal has sat in the step's
 * band for a few consecutive polls, then averages the battery voltage. A
 * manual "Capture now" button covers bikes whose rpm reading is unreliable.
 */

type Step = "intro" | "rest" | "idle" | "revved" | "verdict";

interface Captured {
  rest: number | null;
  idle: number | null;
  revved: number | null;
}

const STABLE_SAMPLES = 4;

export default function ChargingTestPanel({
  charging,
  readings,
}: {
  charging: ChargingTestInfo;
  readings: Reading[];
}) {
  const [step, setStep] = useState<Step>("intro");
  const [captured, setCaptured] = useState<Captured>({ rest: null, idle: null, revved: null });
  const window_ = useRef<number[]>([]);

  const rpm = readings.find((r) => r.key === "rpm")?.value ?? null;
  const batt = readings.find((r) => r.key === "batt")?.value ?? null;

  const inBand = (): boolean => {
    if (rpm === null) return false;
    if (step === "rest") return rpm < 100;
    if (step === "idle") return rpm >= 500 && rpm < charging.check_rpm_min;
    if (step === "revved") return rpm >= charging.check_rpm_min && rpm <= charging.check_rpm_max;
    return false;
  };

  const capture = (value: number) => {
    window_.current = [];
    setCaptured((c) =>
      step === "rest"
        ? { ...c, rest: value }
        : step === "idle"
          ? { ...c, idle: value }
          : { ...c, revved: value },
    );
    setStep(step === "rest" ? "idle" : step === "idle" ? "revved" : "verdict");
  };

  // Auto-capture: average the battery voltage once rpm has held the band
  // for STABLE_SAMPLES consecutive polls.
  useEffect(() => {
    if (!["rest", "idle", "revved"].includes(step) || batt === null) return;
    if (inBand()) {
      window_.current.push(batt);
      if (window_.current.length >= STABLE_SAMPLES) {
        const avg = window_.current.reduce((a, b) => a + b, 0) / window_.current.length;
        capture(avg);
      }
    } else {
      window_.current = [];
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [readings]);

  const reset = () => {
    window_.current = [];
    setCaptured({ rest: null, idle: null, revved: null });
    setStep("intro");
  };

  const fmt = (v: number | null) => (v === null ? "—" : `${v.toFixed(2)} V`);

  const verdict = (): { ok: boolean; title: string; detail: string } => {
    const { rest, idle, revved } = captured;
    if (revved === null) return { ok: false, title: "Incomplete", detail: "Test not finished." };
    const battery_weak = rest !== null && rest < charging.rest_min_v;
    if (revved > charging.charging_max_v) {
      return {
        ok: false,
        title: "Overcharging — regulator suspect",
        detail: `${revved.toFixed(2)} V at revs is above the healthy ceiling of ${charging.charging_max_v.toFixed(1)} V. An overcharging regulator cooks batteries — replace it before it takes the battery with it.`,
      };
    }
    if (revved >= charging.charging_min_v) {
      return {
        ok: true,
        title: "Charging system healthy",
        detail:
          `${revved.toFixed(2)} V in the ${charging.check_rpm_min.toFixed(0)}–${charging.check_rpm_max.toFixed(0)} rpm band is within the community-healthy window (${charging.charging_min_v.toFixed(1)}–${charging.charging_max_v.toFixed(1)} V).` +
          (battery_weak
            ? ` But the resting voltage (${fmt(rest)}) was below ${charging.rest_min_v.toFixed(1)} V — the charging side is fine, the battery itself may be on the way out.`
            : ""),
      };
    }
    const wont_climb = idle !== null && revved - idle < 0.3;
    if (wont_climb) {
      return {
        ok: false,
        title: "Voltage won't climb with revs — stator/connectors suspect",
        detail: `Idle ${fmt(idle)} → revved ${fmt(revved)}: the classic failure signature. ${charging.stator_notes}`,
      };
    }
    return {
      ok: false,
      title: "Undercharging — regulator/rectifier or stator",
      detail: `${revved.toFixed(2)} V at revs is below the ${charging.charging_min_v.toFixed(1)} V minimum. ${charging.stator_notes}`,
    };
  };

  const stepInstruction: Record<Step, string> = {
    intro: "",
    rest: "Key ON, engine OFF. The app captures the resting battery voltage automatically once it sees 0 rpm for a couple of seconds.",
    idle: "Start the engine and let it idle. The app captures the idle voltage automatically.",
    revved: `Hold the engine between ${charging.check_rpm_min.toFixed(0)} and ${charging.check_rpm_max.toFixed(0)} rpm — charging systems need revs to make full output. The app captures automatically once the rpm holds the band.`,
    verdict: "",
  };

  const v = step === "verdict" ? verdict() : null;

  return (
    <details className="section-gap">
      <summary>Charging system test (guided)</summary>
      <div className="charging-panel">
        {step === "intro" && (
          <>
            <p className="muted small">
              A three-step check of the battery, regulator/rectifier, and stator using live
              readings — the most common real fault on these bikes. Thresholds are
              community-sourced: healthy is {charging.charging_min_v.toFixed(1)}–
              {charging.charging_max_v.toFixed(1)} V measured at{" "}
              {charging.check_rpm_min.toFixed(0)}–{charging.check_rpm_max.toFixed(0)} rpm, not at
              idle. <SourceLink site={charging.source} url={charging.source_url} />
            </p>
            <button className="btn btn-primary" onClick={() => setStep("rest")}>
              Start test
            </button>
          </>
        )}

        {["rest", "idle", "revved"].includes(step) && (
          <>
            <p role="status">
              <strong>
                Step {step === "rest" ? 1 : step === "idle" ? 2 : 3} of 3
                {step === "rest" ? " — resting voltage" : step === "idle" ? " — idle" : " — revved"}
              </strong>
              <br />
              {stepInstruction[step]}
            </p>
            <p className="mono">
              Live: {batt !== null ? `${batt.toFixed(2)} V` : "— V"} ·{" "}
              {rpm !== null ? `${rpm.toFixed(0)} rpm` : "— rpm"}
              {inBand() && " · capturing…"}
            </p>
            <div className="btn-row">
              <button
                className="btn btn-small"
                onClick={() => batt !== null && capture(batt)}
                disabled={batt === null}
              >
                Capture now
              </button>
              <button className="btn btn-small" onClick={reset}>
                Cancel
              </button>
            </div>
          </>
        )}

        {step === "verdict" && v && (
          <>
            <div className={v.ok ? "ok-box" : "error-box"} role="status">
              <strong>{v.title}</strong>
              <p style={{ marginBottom: 0 }}>{v.detail}</p>
            </div>
            <table className="table">
              <thead>
                <tr>
                  <th>Step</th>
                  <th>Captured</th>
                  <th>Reference</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>Resting (engine off)</td>
                  <td className="mono">{fmt(captured.rest)}</td>
                  <td className="muted">≥ {charging.rest_min_v.toFixed(1)} V</td>
                </tr>
                <tr>
                  <td>Idle</td>
                  <td className="mono">{fmt(captured.idle)}</td>
                  <td className="muted">rises toward charging voltage</td>
                </tr>
                <tr>
                  <td>
                    At {charging.check_rpm_min.toFixed(0)}–{charging.check_rpm_max.toFixed(0)} rpm
                  </td>
                  <td className="mono">{fmt(captured.revved)}</td>
                  <td className="muted">
                    {charging.charging_min_v.toFixed(1)}–{charging.charging_max_v.toFixed(1)} V
                  </td>
                </tr>
              </tbody>
            </table>
            <p className="muted small">
              Community reference, unverified —{" "}
              <SourceLink site={charging.source} url={charging.source_url} />
            </p>
            <button className="btn btn-small" onClick={reset}>
              Run again
            </button>
          </>
        )}
      </div>
    </details>
  );
}
