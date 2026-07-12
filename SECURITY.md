# Security

MotoDiag talks to vehicle ECUs, so its security posture is deliberately
conservative and documented. This page covers the software threat model;
[docs/SAFETY.md](docs/SAFETY.md) covers the *vehicle* safety interlocks
(read-only defaults, service-mode gating, structural absence of ECU
memory/flash services), which are enforced in code, not policy.

## Threat model & design decisions

**No network access, by design.** MotoDiag makes zero network calls — no
telemetry, no auto-update phone-home, no cloud sync. The only I/O is:
serial ports the user explicitly selects, and files the user explicitly
exports (reports, CSV logs, wire traces). Nothing leaves the machine.

**Webview hardening (Tauri).** All protocol and serial logic lives in the
Rust host process; the webview is a renderer speaking JSON over Tauri's
invoke bridge. The CSP forbids remote content of every kind (`default-src
'self'`; no remote scripts, frames, or objects), so even a compromised
frontend dependency cannot exfiltrate or fetch. The command surface is the
explicit `generate_handler!` list in `src-tauri/src/main.rs` — nothing else
is callable from the webview.

**Input boundaries.** Everything crossing a trust boundary is treated as
untrusted:
- Bytes from the ECU/serial port: length-checked, checksum-verified framing
  with explicit error paths (no panics on malformed traffic — fuzz-shaped
  unit tests cover garbage, truncation, and wrong checksums).
- Definition TOMLs: schema-validated, and the validator structurally rejects
  any request using ECU memory/flash service IDs, so no definition file —
  including a user-supplied one, when that feature lands — can turn the app
  into a flashing tool.
- HTML report output: all interpolated values are escaped (unit-tested), and
  reports generated against the simulator or unverified definitions carry
  non-removable banners saying so.

**Dependencies.** Small, pinned, audited: CI runs `cargo audit` against the
RustSec advisory database and `npm audit` on the frontend lockfile on every
push. The frontend has three runtime dependencies (React, ReactDOM, the
Tauri API); the Rust side avoids heavyweight transitive trees.

**Distribution.** Builds are currently unsigned (pre-release stage).
Before any paid distribution: macOS builds must be signed + notarized,
Windows builds signed, and releases published with checksums. Do not ship
installers to customers before that work is done.

## Reporting a vulnerability

Open a GitHub issue on this repository — or, if the issue is sensitive,
contact the maintainer directly (see the repository profile). Reports about
the ECU-facing safety interlocks are treated with the same priority as
classic security bugs: anything that lets a read-only session write to an
ECU is a critical defect.
