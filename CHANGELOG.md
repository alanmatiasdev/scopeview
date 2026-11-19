# Changelog

All notable changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [1.0.0] - 2026-11-19

First stable release.

### Added

- Live view from USB (UVC) cameras with device, resolution and format selection, and automatic
  reconnect after an unplug.
- Camera controls: brightness, contrast, saturation, hue, sharpness, gamma, gain, white balance,
  exposure, anti-flicker and backlight compensation, for the controls each device reports.
- Calibration against a reference ruler, with up to ten named profiles (one per lens or zoom level).
- Measurements: line, parallel lines, angle, rectangle, circle, concentric circles and a crosshair
  with ruler ticks. Select, move and edit existing shapes.
- Text notes and freehand drawing.
- Photos in JPEG, PNG or BMP with the measurements burned in, at the camera's resolution.
- Video recording to AVI (MJPEG) with pause, a time limit and automatic file splitting.
- Flip, grayscale, binarization, invert and emboss image modes.
- Recent files panel with thumbnails.
- Six interface languages (English, Portuguese, Spanish, French, German, Italian), keyboard
  shortcuts, light, dark and high-contrast themes, and an adjustable interface size.
- Downloads for Linux (`.tar.gz`, `.deb`, `.rpm`), Windows (installer and portable `.zip`) and
  macOS (`.dmg`).

### Changed

- The camera pipeline pulls frames from the driver and decodes them on separate threads and keeps
  only the newest frame, so the image no longer lags behind when decoding is slower than the
  camera. Development builds now optimize the camera conversion code as well.
