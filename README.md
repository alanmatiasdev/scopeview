# ScopeView

[![CI](https://github.com/alanmatiasdev/scopeview/actions/workflows/ci.yml/badge.svg)](https://github.com/alanmatiasdev/scopeview/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/alanmatiasdev/scopeview)](https://github.com/alanmatiasdev/scopeview/releases/latest)
[![License: GPL v3+](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](LICENSE)

A viewer and measurement tool for USB microscopes, written in Rust. It runs on Linux, Windows and macOS as a single binary, with no drivers or extra libraries to install.

<!-- screenshots: add docs/screenshots/measuring.png, then uncomment
![ScopeView measuring a sample](docs/screenshots/measuring.png)
-->

## Features

- **Live image** from USB/UVC cameras, with device, resolution and format selection, and automatic reconnect
- **Camera controls:** brightness, contrast, saturation, hue, sharpness, gamma, gain, white balance, exposure, anti-flicker and backlight compensation
- **Flip** horizontally and vertically, and image modes: grayscale, binarization, invert, emboss
- **Calibration** from a reference ruler, with up to ten saved profiles (per lens or zoom level)
- **Measurement** on the image: straight line, parallel lines, angle, rectangle, circle, concentric circles, and a crosshair with ruler ticks. Select, move and edit shapes after drawing them
- **Text notes** and **freehand drawing** over the image
- **Photos** in JPEG, PNG or BMP with the measurements burned in, at the camera's full resolution
- **Video recording** to AVI with pause, a maximum duration and automatic file splitting
- **Interface** in English, Portuguese, Spanish, French, German and Italian, keyboard shortcuts (press F1), light, dark and high-contrast themes, and an adjustable interface size

<!-- screenshots: add calibration.png and settings.png, then uncomment
| Calibration | Settings |
| --- | --- |
| ![Calibration](docs/screenshots/calibration.png) | ![Settings](docs/screenshots/settings.png) |
-->

## Download

Get the latest build from the [Releases page](https://github.com/alanmatiasdev/scopeview/releases/latest).

| System | File | Install |
| --- | --- | --- |
| Windows | `scopeview-<version>-setup.exe` or the portable `.zip` | Run the installer. The files are not code signed, so SmartScreen may show a warning: choose **More info > Run anyway** |
| macOS | `ScopeView-<version>-macos-universal.dmg` | Drag ScopeView to Applications. The app is not notarized: the first time, right-click it and choose **Open** |
| Debian, Ubuntu | `scopeview_<version>_amd64.deb` (or `arm64`) | `sudo apt install ./scopeview_*.deb` |
| Fedora, openSUSE | `scopeview-<version>-1.x86_64.rpm` (or `aarch64`) | `sudo dnf install ./scopeview-*.rpm` |
| Arch, Manjaro, others | `scopeview-<version>-x86_64-linux.tar.gz` (or `aarch64`) | Extract it and run `./install.sh` (per user) or `sudo ./install.sh /usr/local` |

Every release has a `SHA256SUMS` file to verify the downloads.

**Linux:** your user needs access to the camera: `sudo usermod -aG video $USER`, then log in again.
**macOS:** the first run asks for camera access.

## Usage

1. Connect the USB microscope and open ScopeView. Pick the device and resolution in the toolbar.
2. To measure, calibrate first: place a reference ruler under the lens, draw a line over a segment of known length and enter its value.
3. Pick a measurement tool and draw on the image. Take photos or record video into the configured folder.

Press **F1** for the keyboard shortcuts.

## Building from source

You need the latest stable [Rust](https://www.rust-lang.org/tools/install). On Linux you also need `libclang` (Arch: `clang`, Debian/Ubuntu: `libclang-dev`) and, on Debian/Ubuntu, `libxkbcommon-dev` and `libwayland-dev`.

```sh
git clone https://github.com/alanmatiasdev/scopeview
cd scopeview
cargo run -p scopeview --release
```

`cargo build --release -p scopeview` writes the binary to `target/release/scopeview` (`scopeview.exe` on Windows). It can be copied anywhere; it has no runtime files to install.

Useful commands while developing:

```sh
cargo test --workspace                      # unit tests, no camera needed
cargo fmt --all --check                     # formatting
cargo clippy --workspace --all-targets -- -D warnings

# List cameras and formats, then capture for 5 seconds and report frame rate and delay
cargo run --release -p scopeview-camera --example probe -- 0 5

# Tests that need a connected camera
cargo test -p scopeview-camera --test hardware -- --ignored --test-threads=1
```

## Project structure

- `crates/core`: units, calibration, measurement and overlay logic (no I/O).
- `crates/imaging`: overlay rasterization and photo saving.
- `crates/record`: AVI video recording.
- `crates/i18n`: translation layer (Fluent catalogs in `locales/`).
- `crates/camera`: camera enumeration and live frame streaming.
- `crates/app`: the `scopeview` desktop application.
- `assets/`: application icon. `packaging/`: Linux, Windows and macOS packaging files.
- `docs/`: behavior specs (`specs/`), architecture decisions (`adr/`) and the release process.

## Contributing

Contributions are welcome, including translation fixes. Read [CONTRIBUTING.md](CONTRIBUTING.md) first, and please open an issue to discuss larger changes. Changes are listed in [CHANGELOG.md](CHANGELOG.md).

## Author

Developed by [Alan Matias](https://github.com/alanmatiasdev).

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
