# Protocol notes — KWP2000 on Marelli IAW ECUs

Working notes for the protocol layer (`crates/protocol-kwp2000`). Public
knowledge: ISO 14230-2/-3. Model-specific knowledge: reverse-engineered, and
explicitly marked as verified/unverified in the definition files.

## MV Agusta model coverage (explicit, not implied)

MotoDiag's stated goal is the whole pre-Euro3 MV Agusta range. Coverage
varies by ECU generation, so it's spelled out here rather than left to guess:

| Model | ECU | Status |
| --- | --- | --- |
| Brutale 750 (2003-2007) | Marelli IAW 5SM | Covered by `definitions/mv/5sm-brutale-910.toml` (unverified) |
| Brutale 910/910R/910S | Marelli IAW 5SM | Covered by `definitions/mv/5sm-brutale-910.toml` (unverified) |
| Brutale 989 R | Marelli IAW 5SM | Covered by `definitions/mv/5sm-brutale-910.toml` (unverified) |
| Brutale 1078 RR | Marelli IAW 5SM | Covered by `definitions/mv/5sm-brutale-910.toml` (unverified) |
| F4 750 (2003+) | Marelli IAW 5SM | Covered by `definitions/mv/5sm-brutale-910.toml` (unverified) |
| F4 1000/1000R | Marelli IAW 5SM | Covered by `definitions/mv/5sm-brutale-910.toml` (unverified) |
| F4 312R (2007+) | Marelli IAW **7BM** (different ECU) | Minimal stub only: `definitions/mv/7bm-f4-312r.toml`. Local IDs, DTC format, and routines are unknown — `motodiag-discover` is the starting point, not a working definition |
| F4 750 (1999-~2003) | Marelli **1.6M** (older, different ECU) | **Known gap, not covered at all.** Do not attempt against this era of bike; the protocol assumptions in either file above are for a different ECU generation and could produce misleading results |

The 5SM file being shared across six model/displacement variants is a
genuine "pure reuse" case (same ECU, same protocol) — confirmed via
ECU-tuning and ECU-repair listings for each model, not assumed from the name
similarity. The 312R and pre-2003 F4 750 are explicitly different hardware
and are called out rather than silently mis-mapped onto the 5SM file.

This table's claims are structured, not just prose: `definitions/catalog.toml`
maps each brand/model/year range to its `definitions/*.toml` id (or to a
`gap_note` for the two rows above with no working definition), and is
cross-validated against the real definitions in CI. It's the data source
behind the "tell me about your bike" wizard on the Connect screen.

## Physical layer

- K-line: single wire, bidirectional, 10.4 kbaud, 8N1, idle high (pulled to
  battery voltage), driven low dominant.
- **Echo**: because it's a single wire, every byte the tester transmits is
  also received back. The transport layer consumes and verifies the echo
  before the protocol layer sees any data.

## Initialization

### Fast init (IAW 5SM assumption, to be confirmed in M1)

1. Bus idle ≥ 300 ms (W5).
2. Wake-up pattern: line low 25 ms, high 25 ms.
3. Immediately send StartCommunication:
   `81 10 F1 81 03` (fmt/len, target=ECU 0x10, source=tester 0xF1, SID 0x81, checksum).
4. ECU responds `83 F1 10 C1 <KB1> <KB2> <cs>` — key bytes describe supported
   header/timing formats.

### 5-baud slow init (older IAW, e.g. 59M)

1. Bit-bang the ECU address at 5 baud (200 ms/bit): start bit low, 8 data
   bits LSB-first (low = break), stop bit high.
2. ECU answers `0x55` sync at the working baud, then two key bytes.
3. Tester acks with inverted KB2; ECU responds with inverted address.

## Frames

`[format] [target] [source] [length?] [payload...] [checksum]`

- Format byte: top 2 bits = addressing mode (0b10 physical), low 6 bits =
  payload length (0 → separate length byte follows).
- Checksum: 8-bit wrapping sum of all preceding bytes.

## Timing (ISO defaults; overridable per definition)

| Param | Meaning | Default |
| ----- | ------- | ------- |
| P1    | ECU inter-byte | ≤ 20 ms |
| P2    | request → response gap | 25–50 ms |
| P3    | response → next request | 55 ms – 5 s |
| P4    | tester inter-byte | ≥ 5 ms |

Session keep-alive: TesterPresent (0x3E) sent when idle > P3max/2.
USB-serial latency (macOS FTDI ≈ 16 ms batches) is absorbed by padding the
receive deadlines — never by shrinking transmit gaps.

## Services used (v1 scope)

| SID  | Service |
| ---- | ------- |
| 0x81 | StartCommunication |
| 0x10 | StartDiagnosticSession |
| 0x3E | TesterPresent |
| 0x1A | ReadEcuIdentification |
| 0x18 | ReadDTCByStatus |
| 0x14 | ClearDiagnosticInformation |
| 0x21 | ReadDataByLocalIdentifier |
| 0x30 | InputOutputControlByLocalIdentifier |
| 0x31/0x32/0x33 | Start/Stop/RequestResults RoutineByLocalIdentifier |

Memory/flash services are intentionally unimplemented (docs/SAFETY.md).

## Trace capture and replay (built in M3)

Wrapping any `KLineTransport` in `motodiag_transport::trace::TracingTransport`
records every send/receive/break/baud event to a `TraceSink` — a byte-level
capture of the *whole* session, init handshake included. `app-core`'s
`WireTraceRecorder` writes these as JSONL under `fixtures/`, and
`motodiag_transport::replay::ReplayTransport` plays a recording back as a
transport, so `DiagSession::connect` and every read/write path can be
exercised against a fixture with no hardware and no timing dependency —
see `crates/app-core/tests/replay.rs`.

This is the mechanism M1/M4 will use to turn captures from the real bike
into permanent regression tests.

## Discovery plan (M1/M4)

**Primary path — ask the ECU directly, no other tool required.** Run
`motodiag-discover` against the real bike:

```sh
cargo run -p motodiag-app-core --bin motodiag-discover -- \
    /dev/cu.usbserial-XXXX mv-5sm-brutale-910 --trace brutale-discovery.jsonl
```

It brute-forces `ReadDataByLocalIdentifier` (flagging IDs whose value changes
between two quick polls — a strong hint they're live sensor data), tries
several `ReadEcuIdentification`/`ReadDTCByStatus` variants, and writes the
whole run as a wire-trace fixture in the same run. See
`crates/app-core/src/discovery.rs` and `crates/app-core/src/bin/discover.rs`.

**Secondary — cross-reference against another tool's traffic**, useful once
`motodiag-discover` has narrowed down candidate local IDs and you want to
confirm a scaling, or for service routines (TPS reset, CO trim) that need a
specific byte sequence rather than just "what's on this ID":

1. Sniff JPDiag or TuneECU (Windows) talking to the real 5SM through a
   passive K-line tap + logic analyzer; record init, ident, DTC, live-data
   and TPS-reset exchanges.
2. Wrap the serial transport in `TracingTransport` while doing your own M1/M4
   testing and commit interesting sessions under `fixtures/` (see
   `fixtures/README.md`) — they double as regression tests via
   `ReplayTransport`.

**Either way**: update `definitions/mv/5sm-brutale-910.toml` local
IDs/scalings and flip `verified` flags as facts land.

## UDS over CAN (M5 groundwork)

Newer Ducatis (~2010+) use CAN with UDS (ISO 14229) instead of K-line KWP2000.
The two protocols share enough structure (0x7F negative response, several
identical SIDs) that `crates/protocol-can` reuses `protocol-kwp2000`'s
`NegativeResponseCode`. See `crates/protocol-can/src/uds.rs` for the service
IDs in scope, and the note in docs/SAFETY.md on which UDS services are
deliberately excluded.

Transport-wise, CAN frames are limited to 8 bytes, so multi-byte payloads are
segmented with ISO-TP (ISO 15765-2): Single Frame for ≤7 bytes, First
Frame + Consecutive Frames with Flow Control for longer ones. See
`crates/protocol-can/src/iso_tp.rs`.

This layer is protocol/transport groundwork only — it is not yet wired into
`app-core::DiagSession` or the desktop UI (that pairing, plus a real CAN
adapter, is future work once a CAN-era bike is available to validate against).

## Open questions

- Does the 5SM want StartDiagnosticSession (0x10) after StartCommunication,
  and with which session byte?
- Actual local identifiers for live data (current values are placeholders).
- DTC code table for the Brutale 910 (workshop manual appendix).
- Exact byte sequences for TPS reset / CO trim (sniff from JPDiag).
