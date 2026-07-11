# Hardware guide

## What you need for the MV Agusta Brutale 910 (K-line)

### 1. FTDI-based KKL K-line USB cable (~€25)

A "VAG-COM KKL 409.1"-style USB interface with a **genuine FTDI FT232RL**
chip. This is the same cable class the JPDiag and GuzziDiag communities use on
Marelli IAW ECUs.

- Buy from a reputable seller (e.g. Lonelec's KKL cable, OBD Innovations
  FT232RL KKL). **Avoid CH340/CH341-based clones** — their break-signal and
  bit timing are unreliable for K-line init.
- macOS: Apple ships a built-in FTDI driver (`AppleUSBFTDI`); the cable
  appears as `/dev/cu.usbserial-XXXX`. No driver install needed.
- Known macOS caveat: the built-in driver's latency timer is fixed at ~16 ms,
  which adds receive jitter. Fine for KWP request/response; if fast-init
  timing ever proves marginal, an FTDI D2XX (`libftd2xx`) transport backend
  can be added behind the same transport trait.

### 2. Adapter to the bike's diagnostic connector

The Brutale has **no OBD-II socket**. Forum reports place a 3-pin diagnostic
connector under the tank (right side) on Brutale models, and a 4-pin
connector under the seat on some bikes. Only three wires matter:

| Signal | KKL cable OBD-II pin |
| ------ | -------------------- |
| K-line | 7                    |
| +12 V  | 16                   |
| GND    | 4                    |

Options:
- **Lonelec "GuzziDiag 3-pin interface cable kit"** — covers the FIAT-style
  3-pin used across Italian bikes; confirm MV fitment with Lonelec first.
- **DIY**: an AMP Superseal (or matching) 3-pin plug wired to the KKL cable's
  OBD pins 7/16/4.

**Before first connection, verify the pinout on the actual bike** with a
multimeter and the workshop manual wiring diagram — see
[MV-PINOUT.md](MV-PINOUT.md).

### 3. Bench/reverse-engineering kit (recommended, ~€40)

Lets you validate timing and sniff other tools without touching the bike:

- 12 V bench power supply.
- L9637D (or SN65HVDA195) K-line transceiver breakout — build a hardware ECU
  simulator or a passive K-line tap.
- Cheap 8-channel logic analyzer (Saleae-clone, ~€15) — verify fast-init
  pulse widths from macOS, capture JPDiag traffic for protocol discovery.

### 4. Optional: digital vacuum gauge for throttle-body sync (~€50)

For the Sync screen: an Arduino + ADS1115 + 4× MPX4250AP MAP sensors on a
second USB port, streaming per-cylinder vacuum next to live ECU data. Full
parts list, wiring, and firmware in [VACUOMETRO.md](VACUOMETRO.md) and
`tools/vacuum-gauge-firmware/`.

## Later — CAN-era Ducatis (milestone M5)

- **OBDLink SX/EX** (STN11xx serial AT command set): proven with Ducati DDA
  tooling (MelcoDiag), appears as a USB serial device on macOS. Software side:
  `crates/protocol-can/src/elm327.rs`.
- **CANable 2.0** (SLCAN firmware): raw CAN frames over serial; cleaner for
  development. Software side: `crates/protocol-can/src/slcan.rs`.
- **Ducati adapters**: 3-pin K-line adapter (pre-~2009 bikes) and 4-pin DDA →
  OBD-II adapter (Multistrada 1200 era onward), e.g. from Lonelec/TunerTools.

Both transports' CAN framing (ISO-TP segmentation, UDS service layer) are
built and tested against an in-memory mock bus — see
`crates/ecu-sim/tests/can_session.rs`. The serial-facing halves of
`SlcanTransport`/`Elm327Transport` themselves are, like `serial_vcp` for
K-line, untested against real hardware until one of these adapters is in hand.

macOS has no SocketCAN; both devices above talk serial, which the transport
layer abstracts anyway.
