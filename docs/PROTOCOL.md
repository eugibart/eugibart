# Protocol notes — KWP2000 on Marelli IAW ECUs

Working notes for the protocol layer (`crates/protocol-kwp2000`). Public
knowledge: ISO 14230-2/-3. Model-specific knowledge: reverse-engineered, and
explicitly marked as verified/unverified in the definition files.

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

## Discovery plan (M1/M4)

1. Sniff JPDiag (Windows) talking to the real 5SM through a passive K-line
   tap + logic analyzer; record init, ident, DTC, live-data and TPS-reset
   exchanges.
2. Store captures under `fixtures/` (JSONL trace format produced by
   `app-core`'s TraceLogger) — they double as regression tests via a replay
   transport.
3. Update `definitions/mv/5sm-brutale-910.toml` local IDs/scalings and flip
   `verified` flags as facts land.

## Open questions

- Does the 5SM want StartDiagnosticSession (0x10) after StartCommunication,
  and with which session byte?
- Actual local identifiers for live data (current values are placeholders).
- DTC code table for the Brutale 910 (workshop manual appendix).
- Exact byte sequences for TPS reset / CO trim (sniff from JPDiag).
