# Radiacode (Rust)

Linux-first tooling for [RadiaCode](https://www.radiacode.com/) radiation detectors and spectrometers (RC-10x series). Connect over USB or Bluetooth LE, monitor live readings, capture spectra, and manage device settings from a native desktop app.

This project is developed and tested on Linux. USB permissions, Bluetooth pairing, RSSI reporting, and desktop integration all assume a typical Linux stack (udev, BlueZ, D-Bus, X11/Wayland). Other platforms may compile, but they are not supported targets today.

## Features

**Radiacode Spectrum** (`radiacode-spectrum`) is the main application:

- **Monitor** — live dose rate, count rate, temperature, battery, and alarm state
- **Spectrum** — energy spectrum histogram with calibration overlays
- **Spectrogram** — time–energy waterfall capture, recording, and library playback
- **Dosimeter** — accumulated dose and session duration, cumulative dose chart, alarm lines, and dose reset
- **Settings** — device configuration (alarms, units, display, sound/vibration/light masters, clock sync) and app preferences (poll intervals, auto-connect, PC alarm repeat)

Alarm cards expose sound and vibration per Warn / Danger / OOS; light indication for alarms is firmware-driven. The Signals panel master **Light** toggle follows the official Android control: `LEDS_ON` when the firmware exposes it, otherwise `DEVICE_CTRL` bit 3 (RC-110).

Shared libraries handle device discovery, protocol framing, and transport:

| Crate | Role |
| --- | --- |
| `radiacode-core` | Protocol, VirtSFR commands, spectra, alarms, device config |
| `radiacode-usb` | USB transport via `rusb`, udev rule helpers |
| `radiacode-bluetooth` | BLE transport via `btleplug`, Linux RSSI via BlueZ |
| `radiacode-spectrum` | egui/eframe desktop UI |

## Requirements

- **Rust** 1.85+ (edition 2024)
- **Linux** with:
  - USB: access to RadiaCode USB devices (see below)
  - Bluetooth: BlueZ and a working BLE adapter for wireless use
  - Desktop: X11 or Wayland for the GUI
- **Build deps** (distribution packages vary):
  - `libusb` development headers (for `rusb`)
  - OpenGL/EGL and common GUI libraries (for `eframe`)

## Build

```bash
git clone <repo-url>
cd radiacode
cargo build --release -p radiacode-spectrum
```

Run the app:

```bash
./target/release/radiacode-spectrum
```

Optional desktop entry (adjust paths as needed):

```bash
cp radiacode-spectrum/radiacode-spectrum.desktop ~/.local/share/applications/
```

## USB access on Linux

RadiaCode devices use USB vendor `0483` / product `f123`. Without a udev rule, opening the device may fail with a permission error.

The app can install a rule via `pkexec` when prompted. To install manually:

```bash
sudo cp radiacode.rules /etc/udev/rules.d/99-radiacode.rules
sudo udevadm control --reload
sudo udevadm trigger
```

Unplug and replug the detector after installing the rule.

## License

See [LICENSE](LICENSE).
