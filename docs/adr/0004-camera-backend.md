# ADR-0004: nokhwa as the camera backend, behind our own API

- Status: accepted (Linux verified on hardware; Windows and macOS verified to compile only)
- Date: 2026-11-19

## Context

ScopeView needs device enumeration, format listing and a live frame stream on Linux
(V4L2), Windows (Media Foundation) and macOS (AVFoundation), with no system libraries to
install and a single binary.

## Options considered

- **nokhwa 0.10** (`input-native`): one API over V4L2, Media Foundation and AVFoundation.
  Pure Rust bindings except `v4l2-sys`, which needs libclang at build time.
- **Per-OS code against `v4l`, `windows`, `objc2-av-foundation`**: full control, three
  backends to write and maintain.
- **ffmpeg / OpenCV bindings**: large native dependencies, hard to ship as a single binary.

## Decision

Use nokhwa, wrapped by the `scopeview-camera` crate. Nothing outside that crate imports
nokhwa types, so the backend can be replaced later without UI changes.

Two choices inside the wrapper:

- nokhwa's `decoding` feature is **off**. It pulls in `mozjpeg` (C build). MJPEG frames are
  decoded with the `image` crate (pure Rust); YUYV and similar go through nokhwa's own
  converters.
- Each capture runs on its own thread that creates the `Camera` there (the handle is not
  guaranteed `Send` on every OS), keeps only the newest frame, and reconnects by device
  **name** after an unplug, because indexes change.

## Spike results (Linux, built-in UVC webcam)

- Enumeration found two nodes for one camera; the second is the UVC metadata node with no
  formats. It is filtered using sysfs `index == 0`.
- 10 formats listed, from 640x360 to 1920x1080, MJPEG and YUYV.
- Streaming 1280x720 MJPEG: release build reaches the camera's own delivery rate
  (measured 19.9 fps with `v4l2-ctl`, 18.7 fps through the wrapper). Decode costs about
  11.5 ms per 720p frame, so decode is not the bottleneck at 30 fps for 720p.
- Debug builds are much slower at JPEG decode; the workspace sets `opt-level = 3` for
  `image` and `zune-jpeg` in dev profile so `cargo run` stays usable.

## Not verified

- Windows and macOS runtime behavior: CI only compiles for them. `cargo check` for
  `x86_64-pc-windows-msvc` passes locally; for macOS it cannot run here (a dependency compiles
  C against the Apple SDK), so the macOS permission code path is unchecked until CI runs. A hardware pass on both is
  part of the v0.1 release gate.
- macOS needs a camera usage description in an app bundle. Until packaging exists (v1.0
  milestone), a bare binary inherits the terminal's camera permission.

## Consequences

- Building on Linux needs libclang (present on GitHub's Ubuntu runners).
- Format listing opens the device briefly, so it fails on a device already in use.
