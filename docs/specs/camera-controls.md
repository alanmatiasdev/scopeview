# Camera controls

Status: ready (persistence is our own decision). Roadmap item 2.1. Code: `camera::controls`,
`camera::stream`, `core::camera_settings`, app "Camera controls" window.

## Controls

Brightness, contrast, saturation, hue, sharpness, gamma, gain, white balance, exposure,
anti-flicker and backlight compensation. The window lists **only the controls the device reports**
(with a usable range and not read-only). A device with no controls shows a message instead.

- Each control is an integer slider over the device's own minimum, maximum and step.
- Exposure and white balance have an **Auto** checkbox when the device offers an automatic mode.
  While Auto is on, the slider is disabled.
- Anti-flicker is a list: Off, 50 Hz, 60 Hz and, where the device offers it, Auto.
- Controls the device marks inactive (for example manual exposure while auto exposure is on) are
  disabled.
- **Reset to defaults** puts every control at the device default, turns automatic modes back on, and
  forgets the saved values for that camera.

## How values reach the device

Controls belong to the capture thread, which owns the open camera. The UI sends commands
(`Set`, `SetAuto`, `Reset`); the thread applies them between frames and publishes the controls as
the device now reports them, so the window always shows the device's real values.

## Persistence

Changed values are saved per camera name in `camera-controls.toml` in the config folder and applied
again each time that camera opens, including after an unplug and replug. While a slider is held the
file is not rewritten; it is written when the mouse is released.

A broken file is ignored with a warning in the status bar.

## Platform notes

On Linux, exposure, anti-flicker and the automatic switches come through as raw V4L2 controls and
are recognized by id. Other platforms expose what their capture backend reports, so the list may be
shorter (anti-flicker may be missing).
