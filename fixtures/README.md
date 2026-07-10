# Trace fixtures

Captured K-line sessions (JSONL: one `{"t_ms", "dir", "raw"}` object per
frame, produced by app-core's `TraceLogger`).

Every real-bike session should be captured and the interesting ones committed
here — they are the ground truth for the protocol layer and will feed a
replay transport for regression tests.

Currently empty: no hardware sessions have been run yet (milestone M1).
