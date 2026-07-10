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
- Log live data to CSV; capture every session as a replayable wire trace
- Service functions (actuator tests, TPS reset, CO trim) behind hard safety
  interlocks — **ECU map read/write is deliberately out of scope**

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

## Architecture

```
apps/desktop        Tauri v2 shell — React UI, thin command layer
crates/app-core     session orchestration, safety interlocks, logging
crates/protocol-kwp2000   ISO 14230: init (fast + 5-baud), framing, timing, session
crates/transport    K-line transports: serial VCP, in-memory mock (CAN later)
crates/ecu-defs     data-driven ECU definitions (TOML) + validation
crates/ecu-sim      simulated Marelli-style ECU for dev/tests
definitions/        per-model TOML files — new bike = new file, not new code
docs/               HARDWARE, PROTOCOL, MV-PINOUT, SAFETY
fixtures/           captured wire traces (regression tests)
```

Everything model-specific lives in `definitions/*.toml` (init method,
addresses, live-data channels, DTC tables, service routines with
preconditions). The definition validator structurally rejects memory/flash
service IDs.

## Roadmap

- **M0** ✅ workspace, protocol core, ECU simulator, desktop shell, CI
- **M1** real K-line connect + ECU identification on the Brutale (verify
  connector pinout, capture first traces)
- **M2** DTCs + live data validated against the real bike
- **M3** logging polish + trace replay viewer
- **M4** service functions discovered via JPDiag sniffing (TPS reset first)
- **M5** Ducati: K-line models (5AM/59M definitions), then CAN-era (DDA
  connector, SLCAN/STN transports)

## Status

M0. The whole stack works end-to-end against the simulated ECU; nothing has
touched a real bike yet. All model-specific values in the Brutale definition
are educated placeholders flagged `verified = false`.
