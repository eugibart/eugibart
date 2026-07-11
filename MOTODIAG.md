# MotoDiag

Cross-platform desktop diagnostics for MV Agusta and Ducati motorcycles.
First target: **MV Agusta Brutale 910** (Magneti Marelli IAW 5SM, K-line /
ISO 14230 KWP2000). Mac-first, built with Tauri (Rust core + React UI).

> ⚠️ Hobby project, not affiliated with MV Agusta or Ducati. Protocol
> knowledge is community-sourced and reverse-engineered; everything unverified
> is labeled as such. Use at your own risk — read [docs/SAFETY.md](docs/SAFETY.md).

## What it does (v1 scope)

- Connect over a cheap FTDI KKL K-line cable ([hardware guide](docs/HARDWARE.md))
- Identify the ECU, read & clear fault codes, watch live sensor data
- **Guided connection troubleshooting** — every failure is classified (cable
  missing, port busy, no bus echo → wiring, silent ECU → ignition/+12V,
  garbled → baud) with a concrete next step, because "it won't connect" is
  the #1 complaint about every existing K-line tool
- **Plain-language fault codes** — each known DTC lists likely causes and
  what to physically check, not just a hex code
- **Reference specs & procedures** — live-data gauges show a "normal range"
  badge when a spec exists, and service routines carry a full step-by-step
  procedure, not just a one-line description (see the MV model-coverage
  table in [docs/PROTOCOL.md](docs/PROTOCOL.md) for exactly which models
  this applies to today)
- **One-click bike health report** — self-contained HTML snapshot (identity,
  DTCs with guidance, live data) for pre-purchase inspections or sending to
  a mechanic
- Log live data to CSV; capture every session as a replayable wire trace
- **Sync Assistant (vacuometro)** — reads an open-hardware digital vacuum
  gauge (~€50 DIY build, firmware included) on a second USB port and shows
  per-cylinder vacuum, spread, and live ECU RPM on one screen for
  throttle-body sync — the mandatory step before CO adjustment. See
  [docs/VACUOMETRO.md](docs/VACUOMETRO.md)
- Service functions (actuator tests, TPS reset, CO trim) behind hard safety
  interlocks — **ECU map read/write is deliberately out of scope**
- **WCAG 2.2 AA accessible UI** — full keyboard operation, computed contrast,
  pausable live feeds, screen-reader status messages, and light/dark themes
  (each palette validated separately; follows the OS setting until you pick)
  ([docs/ACCESSIBILITY.md](docs/ACCESSIBILITY.md))

## Try it now — no bike or cable needed

The app ships with a built-in simulated ECU:

```sh
# 1. Protocol core: run the full test suite (includes end-to-end sim sessions)
cargo test --workspace

# 2. Desktop app (needs Node 22+; on Linux also webkit2gtk — see CI workflow)
cd apps/desktop
npm install
npm run tauri dev
# then: Connect tab → "Built-in ECU simulator" → Connect
```

There is also a pty-based simulator for testing any serial client:
`cargo run -p motodiag-ecu-sim --bin ecu-sim-pty`.

## Discovering unverified values — no JPDiag/TuneECU/VDSTS required

Every local identifier, DTC format, and identification option byte in the
shipped definitions is a placeholder pending confirmation. `motodiag-discover`
asks the ECU directly instead of requiring a copy of some other tool to sniff:

```sh
# Against the simulator (no hardware needed) — a fast way to see the output shape
cargo run -p motodiag-app-core --bin motodiag-discover -- simulator mv-5sm-brutale-910

# Against the real bike once you have a cable (see docs/HARDWARE.md)
cargo run -p motodiag-app-core --bin motodiag-discover -- \
    /dev/cu.usbserial-A7043NRK mv-5sm-brutale-910 --trace brutale-discovery.jsonl
```

It brute-forces `ReadDataByLocalIdentifier` across the ID space (narrow it
with `--range 01:20` once you have a hunch), tries several
`ReadEcuIdentification`/`ReadDTCByStatus` variants, flags local IDs whose
value changed between two quick polls (a strong hint they're live sensor data
rather than static config), and — with `--trace` — writes the whole run as a
wire-trace fixture in the same format `ReplayTransport` consumes. One run
against the real bike both tells you what's actually on each local ID *and*
becomes a permanent regression fixture.

The CAN/UDS side (M5 groundwork, not yet wired to the desktop UI) has its own
in-memory bench simulator, exercised end-to-end in
`crates/ecu-sim/tests/can_session.rs`.

## Architecture

```
apps/desktop        Tauri v2 shell — React UI, thin command layer
crates/app-core     session orchestration, safety interlocks, logging, wire-trace recording
crates/protocol-kwp2000   ISO 14230: init (fast + 5-baud), framing, timing, session
crates/protocol-can       ISO-TP + UDS (ISO 14229) groundwork: frame, transport, session,
                          SLCAN/ELM327 backends — tested, not yet wired to app-core (M5)
crates/transport    K-line transports: serial VCP, in-memory mock, byte-level tracing, replay
crates/ecu-defs     data-driven ECU definitions (TOML) + validation (K-line and CAN)
crates/ecu-sim      simulated Marelli-style K-line ECU + a simulated CAN/UDS ECU, for dev/tests
definitions/        per-model TOML files — new bike = new file, not new code
docs/               HARDWARE, PROTOCOL, MV-PINOUT, SAFETY
fixtures/           captured wire traces, replayable as regression tests
```

Everything model-specific lives in `definitions/*.toml` (init method,
addresses, live-data channels, DTC tables, service routines with
preconditions — the same shape whether the bus is K-line or CAN). The
definition validator structurally rejects memory/flash service IDs on both
buses.

Every session can be captured byte-for-byte (`TracingTransport`, including
the init handshake) and replayed with no hardware or simulator at all
(`ReplayTransport`) — see `crates/app-core/tests/replay.rs` and
`fixtures/README.md`. This is how real-bike captures from M1 onward become
permanent regression tests.

## Roadmap

- **M0** ✅ workspace, protocol core, ECU simulator, desktop shell, CI
- **M1** *(needs the physical bike — not done)* real K-line connect + ECU
  identification on the Brutale, verify connector pinout, capture first traces
- **M2** *(needs the physical bike)* DTCs + live data validated against the real bike
- **M3** ✅ wire-trace recording + replay transport, regression fixture pipeline
- **M4** *(needs the physical bike + JPDiag/Windows)* service functions discovered
  via JPDiag sniffing (TPS reset first)
- **M5** ✅ groundwork done: Ducati K-line definitions (5AM/59M, pure reuse of
  the existing stack) + CAN/UDS protocol layer (ISO-TP, UDS session, SLCAN/ELM327
  transports, CAN bench simulator), all tested end-to-end. **Not done**: wiring
  the CAN side into `app-core::DiagSession`/the desktop UI, and any real CAN
  hardware/bike validation — both are natural next steps once M5's groundwork
  has a CAN-era bike to test against.

### Forum-driven quality-of-life features (built)

Owner-forum research (mvagusta.net, ducati.ms/.org) drove three additions
beyond the original milestones: the connection troubleshooter (P0 — attacks
the single most-complained-about failure mode of JPDiag/MelcoDiag-era tools),
the plain-language DTC causes/checks library, and the HTML health report.
Still on the researched backlog: log charts, a community definition-exchange
flow, and a scripted charging-system health check.

## Status

M0, M3, and M5's software-buildable groundwork are done and thoroughly tested
(cargo test across the whole workspace, simulator- and mock-bus-driven). M1,
M2, and M4 all require physical access to a real bike (and, for M4, a Windows
box running JPDiag) and haven't been started. All model-specific values in
every definition file — MV (5SM shared range + F4 312R stub), Ducati K-line,
Ducati CAN stub — are educated placeholders flagged `verified = false` until
confirmed on hardware. See the MV model-coverage table in
[docs/PROTOCOL.md](docs/PROTOCOL.md) for exactly which bikes are covered by
a real definition, which are a minimal stub, and which are a known,
documented gap (pre-2003 F4 750, Marelli 1.6M).
