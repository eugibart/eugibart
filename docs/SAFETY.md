# Safety design

Talking to an ECU can leave a bike unrideable if done carelessly. MotoDiag's
safety posture is enforced in code (`crates/app-core/src/safety.rs`), not
just documentation.

## Principles

1. **Read-only by default.** A new session can read identification, fault
   codes, and live data. Anything that changes ECU state — clearing codes,
   actuator tests, adaptation resets — is refused until the user explicitly
   enables *service mode* for that session.

2. **Per-operation confirmation.** Service mode alone is not enough; every
   state-changing operation additionally requires an explicit confirmation
   flag (surfaced in the UI as a typed confirmation).

3. **Preconditions are checked before any bytes are sent.** Each routine in
   an ECU definition declares its requirements (engine off, battery voltage
   window, notes). app-core measures them from live data and **fails closed**:
   if a precondition can't be measured (no RPM/battery channel), the routine
   is blocked.

4. **No map read/write anywhere in v1.** This is not a hidden feature — it is
   structurally absent:
   - The KWP service layer defines no memory/download services (0x23, 0x34–0x36, 0x3D).
   - The definition-file validator rejects any request using those service IDs.

5. **Unverified means labeled.** Definitions, channels, and routines carry a
   `verified` flag; anything not yet confirmed against a real ECU is shown
   with an "unverified" warning in the UI.

6. **Session loss is handled conservatively.** If the keep-alive lapses or a
   response times out mid-routine, the session surfaces the error rather than
   silently retrying state-changing requests.

## For users

- Bike on a paddock stand, in neutral, kill switch reachable.
- Battery charged (many routines require ≥ 12 V) — a sagging battery mid-write
  is the classic way to corrupt an ECU.
- Never run actuator tests with the engine running unless the procedure
  explicitly calls for it.
