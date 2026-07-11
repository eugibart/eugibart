/*
 * MotoDiag vacuum gauge firmware ("vacuometro digitale")
 *
 * Reads up to 4 MAP sensors through an ADS1115 ADC and streams absolute
 * manifold pressure over USB serial in the MotoDiag line protocol:
 *
 *   HELLO,motodiag-vac,4          (on boot, then every ~2 s as a heartbeat)
 *   VAC,31.2,31.5,30.9,31.4       (one sample line at ~10 Hz, kPa)
 *
 * Hardware (see docs/VACUOMETRO.md for the full build guide):
 *   - Arduino Uno/Nano (or any 5V board; ESP32 works at 3.3V with ratiometric
 *     correction — see note below)
 *   - ADS1115 4-channel 16-bit I2C ADC (A4=SDA, A5=SCL on an Uno/Nano)
 *   - 4x NXP MPX4250AP absolute pressure sensors (20-250 kPa), one per
 *     cylinder, teed into the throttle body vacuum ports with restrictors
 *     (a ~0.6 mm orifice or a few cm of fuel-filter foam) to damp pulsation
 *
 * MPX4250AP transfer function (Vs = 5.0 V supply):
 *   Vout = Vs * (0.004 * P_kPa - 0.04)   =>   P_kPa = (Vout/Vs + 0.04) / 0.004
 *
 * ESP32 note: the MPX4250AP is ratiometric to its 5V supply. Keep the sensor
 * on 5V, keep the ADS1115 (it tolerates 5V signals when powered at 5V), and
 * only the I2C lines go to the 3.3V board — or use a level shifter.
 */

#include <Wire.h>
#include <Adafruit_ADS1X15.h> // Adafruit ADS1X15 library (install via Library Manager)

static const uint8_t N_CHANNELS = 4;
static const float SUPPLY_VOLTS = 5.0f;
static const unsigned long SAMPLE_INTERVAL_MS = 100; // ~10 Hz
static const unsigned long HELLO_INTERVAL_MS = 2000;
// Average this many raw reads per sample per channel to damp combustion pulses
// (in addition to the mechanical restrictors).
static const uint8_t OVERSAMPLE = 8;

Adafruit_ADS1115 ads;
unsigned long lastSample = 0;
unsigned long lastHello = 0;

void setup() {
  Serial.begin(115200);
  Wire.begin();
  ads.begin(); // default address 0x48
  // GAIN_TWOTHIRDS: +/-6.144 V full scale, safe for a 5 V sensor output.
  ads.setGain(GAIN_TWOTHIRDS);
  sendHello();
}

void sendHello() {
  Serial.print("HELLO,motodiag-vac,");
  Serial.println(N_CHANNELS);
}

float readKpa(uint8_t channel) {
  long sum = 0;
  for (uint8_t i = 0; i < OVERSAMPLE; i++) {
    sum += ads.readADC_SingleEnded(channel);
  }
  float counts = (float)sum / OVERSAMPLE;
  float volts = ads.computeVolts((int16_t)counts);
  // MPX4250AP inverse transfer function.
  float kpa = (volts / SUPPLY_VOLTS + 0.04f) / 0.004f;
  return kpa;
}

void loop() {
  unsigned long now = millis();

  if (now - lastHello >= HELLO_INTERVAL_MS) {
    lastHello = now;
    sendHello();
  }

  if (now - lastSample >= SAMPLE_INTERVAL_MS) {
    lastSample = now;
    Serial.print("VAC");
    for (uint8_t ch = 0; ch < N_CHANNELS; ch++) {
      Serial.print(',');
      Serial.print(readKpa(ch), 1);
    }
    Serial.println();
  }
}
