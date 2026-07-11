/**
 * Static in-app user guide. Everything here describes what the app actually
 * does today — the same honesty rules as the data layer apply to the copy:
 * no promised features, no glossing over the unverified-values reality, and
 * the safety limits are stated as limits, not fine print.
 *
 * Structure: a table of contents of same-page anchor links, then one
 * section per topic. Every section heading carries tabIndex={-1} so the
 * anchor jump also moves keyboard/screen-reader focus to the heading
 * (WCAG 2.4 bypass/navigation), not just the scroll position.
 */
import { ReactNode } from "react";

const SECTIONS = [
  { id: "help-what", title: "What MotoDiag is" },
  { id: "help-expect", title: "What to expect (read this first)" },
  { id: "help-need", title: "What you need" },
  { id: "help-connect", title: "Connect — garage, wizard & troubleshooting" },
  { id: "help-dashboard", title: "Dashboard — live data, specs & charging test" },
  { id: "help-dtcs", title: "Fault codes" },
  { id: "help-service", title: "Service functions & service mode" },
  { id: "help-sync", title: "Sync — throttle-body balance & compare" },
  { id: "help-logging", title: "Logging — CSV, health report & wire traces" },
  { id: "help-access", title: "Themes, keyboard & accessibility" },
  { id: "help-safety", title: "Safety & deliberate limits" },
] as const;

function Expect({ children }: { children: ReactNode }) {
  return (
    <div className="help-callout">
      <span className="help-callout-label">What to expect</span>
      {children}
    </div>
  );
}

export default function HelpScreen() {
  return (
    <div className="panel panel-wide help">
      <h2>Help &amp; user guide</h2>

      <nav aria-label="Help contents" className="help-toc">
        <ul>
          {SECTIONS.map((s) => (
            <li key={s.id}>
              <a href={`#${s.id}`}>{s.title}</a>
            </li>
          ))}
        </ul>
      </nav>

      <section aria-labelledby="help-what">
        <h3 id="help-what" tabIndex={-1}>
          What MotoDiag is
        </h3>
        <p>
          MotoDiag talks to the engine ECU of pre-Euro3 MV Agusta and K-line-era Ducati
          motorcycles over a cheap serial cable, using the same diagnostic protocol
          (ISO&nbsp;14230 “KWP2000” over K-line) the factory tools use. It reads what the ECU
          knows — identity, stored fault codes, live sensor values — and runs the official
          service routines (TPS reset, CO trim, actuator tests) with their preconditions
          spelled out.
        </p>
        <p>
          The bikes it targets: MV Agusta Brutale 750/910/989/1078 and F4 750 (2003-on) /
          1000 / 312R, plus the Ducatis that share the same Magneti Marelli ECU families
          (IAW&nbsp;5AM and 5.9M — Monsters, SuperSports, ST4S, 749/999, SportClassics and
          more). The Connect tab’s bike catalog is the authoritative list: bikes MotoDiag
          cannot talk to yet (earlier P8/1.6M ECUs, later Siemens or CAN-bus bikes) are
          listed there too, with an honest “can’t connect to this yet” note instead of a
          guess.
        </p>
        <p>
          It is a <strong>diagnostic</strong> tool: it reads data, clears codes, and runs
          the ECU’s own service routines. It does not remap, reflash, or read out ECU
          memory — deliberately (see{" "}
          <a className="help-jump" href="#help-safety">
            Safety &amp; deliberate limits
          </a>
          ).
        </p>
      </section>

      <section aria-labelledby="help-expect">
        <h3 id="help-expect" tabIndex={-1}>
          What to expect (read this first)
        </h3>
        <p>
          MotoDiag’s protocol knowledge is community-sourced and reverse-engineered — there
          is no official MV Agusta or Ducati specification behind it. The app is built to be
          honest about that everywhere:
        </p>
        <ul className="help-list">
          <li>
            <strong>“unverified” badges are real.</strong> Every model-specific value
            (sensor addresses, fault-code tables, routine numbers) ships flagged
            unverified until it has been confirmed against a real bike. An unverified
            routine may simply do nothing, or return an error — the ECU rejects requests it
            doesn’t recognize; it doesn’t half-execute them.
          </li>
          <li>
            <strong>Community figures carry their source.</strong> Reference ranges and mod
            guidance that come from forum knowledge show a clickable “Source: …&nbsp;↗”
            citation to the exact thread or article. If a claim has no source, it isn’t in
            the app — the data format itself refuses uncited entries.
          </li>
          <li>
            <strong>Modified bikes are never judged against stock numbers silently.</strong>{" "}
            When your garage profile lists mods, every affected range is labeled with where
            it came from: factory spec, community guidance for your mod combo (always
            marked unverified), or a target you typed in yourself — which always wins.
          </li>
          <li>
            <strong>Expect the first connection to a real bike to be the hard part.</strong>{" "}
            Ignition on, kill switch to run, correct port, correct wiring. When it fails,
            use <em>Troubleshoot connection</em> — it tells you <em>which layer</em> failed
            (cable, port, wiring, ECU) instead of a generic error.
          </li>
          <li>
            <strong>No bike needed to learn the app.</strong> The port list always includes
            “Built-in ECU simulator (no hardware)” — a simulated Marelli-style ECU with
            live data, fault codes, and service routines. Every screen works against it.
          </li>
        </ul>
      </section>

      <section aria-labelledby="help-need">
        <h3 id="help-need" tabIndex={-1}>
          What you need
        </h3>
        <ul className="help-list">
          <li>
            <strong>A K-line interface cable</strong> — an FTDI-based “KKL” /
            VAG-COM-409-style USB cable (about €15–25). Cheap CH340/PL2303 clones cause
            most “it won’t connect” misery; the hardware guide shipped with the project
            (docs/HARDWARE.md) explains what to buy and how to check it.
          </li>
          <li>
            <strong>The right connector on the bike</strong> — the MV diagnostic connector
            location and pinout are documented in docs/MV-PINOUT.md. Ignition must be on
            (kill switch to run) for the ECU to answer.
          </li>
          <li>
            <strong>For throttle-body sync only:</strong> the open-hardware digital vacuum
            gauge (a ~€50 DIY build; firmware and build guide in docs/VACUOMETRO.md) on a
            second USB port. Everything else needs no extra hardware.
          </li>
        </ul>
        <Expect>
          <p>
            Nothing to buy to evaluate the app: pick the built-in simulator on the Connect
            tab and every feature below is fully usable.
          </p>
        </Expect>
      </section>

      <section aria-labelledby="help-connect">
        <h3 id="help-connect" tabIndex={-1}>
          Connect — garage, wizard &amp; troubleshooting
        </h3>
        <p>
          The Connect tab is bike-first. <strong>Add your bike</strong> starts a short
          wizard: brand, model and year (from the built-in catalog), then your
          modifications — exhaust (stock, open slip-ons, full system; steel or titanium),
          ECU/EPROM (stock, dedicated chip, kit ECU), air filter, and free-text notes.
          Saved bikes appear as garage cards for one-click reconnect, and you can keep
          several.
        </p>
        <p>
          Why the mods matter: a bike with open pipes and a kit EPROM does not idle or trim
          like a stock one. Your answers select which community guidance applies — adjusted
          reference ranges, procedure notes, and fault-code notes all key off the mod combo
          you describe, and each one shows its citation.
        </p>
        <ul className="help-list">
          <li>
            <strong>Port</strong> — pick your USB cable, or “Built-in ECU simulator”.
            <em> Refresh</em> re-scans after plugging in.
          </li>
          <li>
            <strong>Troubleshoot connection</strong> — a step-by-step check (cable present →
            port opens → bus echo → ECU handshake), each step with a pass/fail and a
            concrete suggestion. Run it whenever a connection fails.
          </li>
          <li>
            <strong>Show advanced</strong> — pick an ECU definition directly, without
            saving a profile. Useful when your bike isn’t in the catalog but you know (from
            the label on the ECU itself) which family it is.
          </li>
        </ul>
        <Expect>
          <p>
            Catalog entries for unsupported bikes (pre-2003 F4 750, Siemens-era Ducatis,
            CAN-bus 848/1098…) explain the gap instead of offering a connection that can’t
            work. A successful connect lands you on the Dashboard with the ECU’s reported
            identity in the header.
          </p>
        </Expect>
      </section>

      <section aria-labelledby="help-dashboard">
        <h3 id="help-dashboard" tabIndex={-1}>
          Dashboard — live data, specs &amp; charging test
        </h3>
        <p>
          Live gauges for every channel the ECU definition knows — rpm, coolant and air
          temperature, throttle position, battery voltage, injection/ignition values —
          each with a sparkline of recent history. Polling is continuous;{" "}
          <strong>Pause updates</strong> freezes the whole screen so you can read values
          calmly (and stops the motion for accessibility).
        </p>
        <ul className="help-list">
          <li>
            <strong>Reference ranges.</strong> Where a spec exists, the gauge shows the
            expected range and an in/out-of-range status. The line under the gauge tells
            you the provenance: factory/community stock spec (with its citation link),
            community-adjusted for your mods (labeled unverified, cited), or your own
            target.
          </li>
          <li>
            <strong>Your targets.</strong> An editor for per-channel personal targets —
            e.g. the idle figure your tuner set after a dyno session. A saved target
            overrides everything else and is stored in the bike’s garage profile.
          </li>
          <li>
            <strong>Snapshots.</strong> Type a label (“before sync”) and press{" "}
            <em>Snapshot</em> to save the current readings. Compare any two snapshots
            side-by-side on the Sync tab.
          </li>
          <li>
            <strong>Charging system test (guided).</strong> Regulator/rectifier and stator
            failures are the most notorious real fault on both marques. This walk-through
            measures battery voltage in three states — engine off, idling, and held at the
            check rpm — capturing automatically when the rpm settles in each band (or via{" "}
            <em>Capture now</em>). The verdict separates “healthy”, “battery weak”,
            “overcharging → regulator”, and “won’t rise with revs → stator/connectors”,
            with the community source for the thresholds cited. It appears when the
            connected bike’s definition carries a cited charging spec.
          </li>
        </ul>
        <Expect>
          <p>
            On the simulator the values wander realistically and the charging test can be
            walked end-to-end. On a real bike, expect a channel to read implausibly until
            its scaling has been verified — that is what the unverified flag warns about;
            the Logging tab’s wire traces are how those get confirmed.
          </p>
        </Expect>
      </section>

      <section aria-labelledby="help-dtcs">
        <h3 id="help-dtcs" tabIndex={-1}>
          Fault codes
        </h3>
        <p>
          Reads the ECU’s stored diagnostic trouble codes. Known codes come with a
          plain-language description plus <em>Likely causes</em> and{" "}
          <em>What to check</em> lists — written for someone with a multimeter, not a
          factory manual. Unknown codes are shown honestly as “unknown code” with their raw
          hex, rather than a guessed meaning.
        </p>
        <p>
          If your garage profile lists mods, applicable community notes appear on matching
          codes (e.g. lean-mixture codes on an open-exhaust bike), labeled{" "}
          <em>“For your mods — community, unverified”</em> with their source link.
        </p>
        <Expect>
          <p>
            <strong>Clear fault codes…</strong> requires service mode (Service tab) and a
            typed confirmation, because clearing erases the fault history a mechanic might
            want. Codes with a live underlying fault typically return as soon as the ECU
            re-detects the condition — that’s the ECU’s behavior, not a failed clear.
          </p>
        </Expect>
      </section>

      <section aria-labelledby="help-service">
        <h3 id="help-service" tabIndex={-1}>
          Service functions &amp; service mode
        </h3>
        <p>
          Every session starts <strong>read-only</strong>. Anything that changes ECU state —
          clearing codes, actuator tests, TPS reset, CO trim — is locked behind{" "}
          <strong>service mode</strong>, which you enable per session and which announces
          itself with a badge in the header.
        </p>
        <ul className="help-list">
          <li>
            Each routine shows a <strong>risk badge</strong> (low/medium/high), its
            preconditions (engine off, warm engine, throttle closed…), and — where the
            community has documented it — the full step-by-step procedure, not a one-liner.
          </li>
          <li>
            Running a routine asks for a typed confirmation naming the routine, so a
            mis-click can’t fire an actuator.
          </li>
          <li>
            Mod-specific procedure notes (e.g. how CO trim behaves with open pipes, or that
            5SM/5AM ECUs have <em>no physical trimmer</em> — trim is software-only) appear
            on the routine they describe, cited.
          </li>
        </ul>
        <Expect>
          <p>
            Routines marked unverified may be rejected by a real ECU (a “negative response”
            error) — that is the safe failure mode. Follow the preconditions literally:
            bike on a stand, in neutral, and never run actuator tests with the engine
            running unless the procedure says to.
          </p>
        </Expect>
      </section>

      <section aria-labelledby="help-sync">
        <h3 id="help-sync" tabIndex={-1}>
          Sync — throttle-body balance &amp; compare
        </h3>
        <p>
          Throttle-body synchronization is the mandatory step before any CO/idle work, and
          doing it by eye on bouncing analog clocks is miserable. The Sync tab reads the
          open-hardware digital vacuum gauge (see <em>What you need</em>) and shows
          per-cylinder vacuum, the spread between cylinders, and — when the ECU is also
          connected — live rpm on the same screen. The written procedure sits below the
          live display.
        </p>
        <p>
          This tab works <strong>without</strong> an ECU connection: the gauge is its own
          USB device. ECU rpm simply appears when both are connected.
        </p>
        <p>
          <strong>Before / after compare:</strong> snapshots taken here or on the Dashboard
          (readings, and vacuum state when the gauge is connected) can be compared in a
          side-by-side delta table — pick two snapshots and see exactly what changed, per
          channel and per cylinder. Up to 20 snapshots are kept; the oldest is dropped
          first.
        </p>
        <Expect>
          <p>
            Expect the spread number to be what matters, not the absolute vacuum values —
            the procedure text explains the target. “It feels smoother” becomes a number
            you can screenshot.
          </p>
        </Expect>
      </section>

      <section aria-labelledby="help-logging">
        <h3 id="help-logging" tabIndex={-1}>
          Logging — CSV, health report &amp; wire traces
        </h3>
        <ul className="help-list">
          <li>
            <strong>CSV logging.</strong> <em>Start logging</em> streams every live-data
            poll to a timestamped CSV — one column per channel — for spreadsheets or
            plotting tools. The file path is shown while recording.
          </li>
          <li>
            <strong>Bike health report.</strong> One click reads identity, fault codes
            (with their causes/checks guidance) and current live data, and writes a
            self-contained HTML file — made for pre-purchase inspections or sending to a
            mechanic. If your profile has mods, the report says which reference figures are
            community-sourced or your own targets, citations included.
          </li>
          <li>
            <strong>Wire traces.</strong> Every session is automatically recorded,
            byte-for-byte, as a replayable trace file — <em>Show this session’s trace</em>{" "}
            reveals its path so you can share it. <em>Analyze a shared trace</em> decodes
            anyone’s trace file against an ECU definition and reports what actually
            happened on the wire: identity read, channels with decoded values, fault-code
            reads, and any negative responses or undecodable bytes. This is how the
            community confirms unverified definitions — a trace from a real bike either
            decodes cleanly or shows exactly where the definition is wrong.
          </li>
        </ul>
        <Expect>
          <p>
            Traces contain only the diagnostic bytes exchanged with the ECU plus bike/ECU
            identity — no personal data — but treat them like any file you share. The
            Logging tab needs an active connection.
          </p>
        </Expect>
      </section>

      <section aria-labelledby="help-access">
        <h3 id="help-access" tabIndex={-1}>
          Themes, keyboard &amp; accessibility
        </h3>
        <ul className="help-list">
          <li>
            <strong>Light/dark theme</strong> — the header button switches instantly; the
            app follows your OS preference until you pick one. Both palettes meet WCAG 2.2
            AA contrast, checked automatically in CI.
          </li>
          <li>
            <strong>Keyboard shortcuts</strong> — number keys 1–7 switch tabs (shown on
            each tab). They’re inert while you’re typing in a field, and the{" "}
            <em>Shortcuts</em> header button turns them off entirely if they conflict with
            your assistive technology.
          </li>
          <li>
            <strong>Fully keyboard-operable</strong> — every control is reachable and
            usable without a mouse; a “Skip to content” link is the first Tab stop; live
            feeds can be paused; status changes are announced to screen readers.
          </li>
        </ul>
        <p className="muted small">
          The full accessibility statement, including known limitations, is in
          docs/ACCESSIBILITY.md.
        </p>
      </section>

      <section aria-labelledby="help-safety">
        <h3 id="help-safety" tabIndex={-1}>
          Safety &amp; deliberate limits
        </h3>
        <ul className="help-list">
          <li>
            <strong>No ECU map reading, writing, or flashing — ever.</strong> This is not a
            missing feature: the definition format structurally rejects memory- and
            flash-access commands, so no definition file anyone writes can turn MotoDiag
            into a flasher. A bricked ECU on these bikes is expensive and often
            irreplaceable; remapping belongs to dedicated tools.
          </li>
          <li>
            Service mode is per-session and off by default; destructive actions need typed
            confirmation; risk levels and preconditions are always shown.
          </li>
          <li>
            MotoDiag is a hobby project, not affiliated with MV Agusta or Ducati. You use
            it on your own bike at your own risk — docs/SAFETY.md is the full statement.
            When in doubt, especially with service functions, don’t: read-only diagnostics
            can’t hurt anything.
          </li>
        </ul>
      </section>
    </div>
  );
}
