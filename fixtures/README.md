# Trace fixtures

Byte-level wire captures (JSONL: one `TraceEvent` per line — see
`motodiag_transport::trace`) produced by wrapping a transport in
`TracingTransport`. Unlike the older frame-level trace hook on `KwpSession`,
this captures the *entire* session including the init handshake (break
pulses, StartCommunication), so a fixture here can be replayed end-to-end
with `motodiag_transport::replay::ReplayTransport` — no hardware, no
simulator, no timing, fully deterministic.

## `sim-5sm-full-session.jsonl`

A session against the **built-in ECU simulator** (not a real bike): fast
init, ReadEcuIdentification, ReadDTCByStatus, and one live-data read. Exists
to prove the record → replay pipeline works end-to-end (see
`crates/app-core/tests/replay.rs`) ahead of M1, when the first real-bike
captures will land here alongside it — clearly labeled `sim-` vs. a future
`brutale910-` prefix so it's obvious which fixtures are ground truth and
which are synthetic.

Regenerate it (only if the simulator's session shape changes) with:

```sh
cargo test -p motodiag-app-core --test replay -- --ignored regenerate_fixture
```

## Adding a real-bike fixture (M1+)

1. Wrap the `SerialKLine` transport in `TracingTransport` with a
   `WireTraceRecorder` sink before calling `DiagSession::connect`.
2. Commit the resulting JSONL under a descriptive name
   (`brutale910-ident-dtc-<date>.jsonl`).
3. Add a test in `crates/app-core/tests/` that replays it and asserts the
   expected identity/DTCs/values — this becomes a permanent regression test
   that never needs the bike again.
