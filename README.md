# Radiacode (Rust)

Linux-first tooling for [RadiaCode](https://www.radiacode.com/) radiation detectors and spectrometers (RC-10x series). Connect over USB or Bluetooth LE, monitor live readings, capture spectra, and manage device settings from a native desktop app.

This project is developed and tested on Linux. USB permissions, Bluetooth pairing, RSSI reporting, and desktop integration all assume a typical Linux stack (udev, BlueZ, D-Bus, X11/Wayland). Other platforms may compile, but they are not supported targets today.

## Features

**Radiacode Spectrum** (`radiacode-spectrum`) is the main application:

- **Monitor** — live dose rate, count rate, accumulated dose, temperature, battery, and alarm state
- **Spectrum** — energy spectrum histogram with calibration overlays
- **Spectrogram** — time–energy waterfall capture, recording, and library playback
- **Settings** — device configuration (alarms, units, display, sound/vibration/LED signals, clock sync) and app preferences (poll intervals, auto-connect, PC alarm repeat)

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

## Bluetooth on Linux

Wireless connection uses BLE through BlueZ. Pair the detector in your system Bluetooth settings first, then select it in the app’s device list. RSSI is read from BlueZ when available.

## Examples

Low-level transport probes (USB device required):

```bash
cargo run -p radiacode-usb --example scan
cargo run -p radiacode-usb --example connect -- <serial-or-usb-id>
cargo run -p radiacode-usb --example probe_settings
```

## Logging

Set `RUST_LOG` to control verbosity:

```bash
RUST_LOG=radiacode_spectrum=debug,radiacode_core=info ./target/release/radiacode-spectrum
```

## Platform notes

| Area | Linux | Other OS |
| --- | --- | --- |
| USB hot-plug + udev | Supported | Not integrated |
| BLE scan/connect | Supported via BlueZ | Untested |
| RSSI | BlueZ / `btmgmt` | Disabled |
| Desktop app | Primary target | May build; not validated |

Contributions that improve portability are welcome, but the design priority is a reliable Linux desktop experience.

## Related work

Protocol and register layout follow community reverse-engineering, notably [cdump/radiacode](https://github.com/cdump/radiacode). This codebase is an independent Rust implementation with its own UI and device-management model.

## License

See [LICENSE](LICENSE).
