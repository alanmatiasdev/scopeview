# Calibration

Status: ready. Roadmap items 1.2 and 1.3. Code: `core::calibration`, app calibration window.

## Wizard

1. The user picks the Calibrate tool and drags a line over a segment of known length on a reference
   ruler. Drags shorter than 3 image pixels are ignored.
2. A dialog asks for a name (default `Calibration N`), the real length and the unit (mm or µm).
   The pixel length of the drawn line is shown.
3. Save is accepted only when the length parses to a finite number greater than zero
   (`.` or `,` as decimal mark). Otherwise the dialog stays open with an error.
4. The scale is `pixel_length / entered_length` (see `units-and-scale.md`).
5. The new profile becomes the active one. The tool returns to None.

While dragging, the line is previewed in pixels because the scale is what is being set.

## Profile

| Field | Meaning |
| --- | --- |
| `name` | Free text, trimmed; blank becomes `Calibration` |
| `unit` | `mm` or `um` |
| `pixel_length` | Length of the drawn line in pixels |
| `entered_length` | Real length entered, in `unit` |
| `width`, `height` | Frame size at calibration time |
| `date` | YYYYMMDD, local date |

The scale is derived on load and never stored.

## Profile set

- At most 10 profiles. Adding with an existing name replaces that profile; adding a new name at the
  limit fails with a message.
- One profile is active, or none (measurements then show pixels). The toolbar picker switches it.

## Managing profiles (roadmap 3.7)

The Manage button next to the profile picker opens a window listing every profile with its real
length, unit and calibration resolution.

- A radio button makes a profile the active one.
- Rename trims the text; a blank name becomes `Calibration`. A name used by another profile is
  rejected with a message, so names stay unique (adding a calibration with an existing name still
  replaces that profile).
- Delete asks for an inline confirmation. Deleting the active profile goes back to pixels;
  deleting an earlier one keeps the same profile active.
- Every change is saved to `calibrations.toml` at once. Deleting frees a slot under the
  10-profile cap.

The active profile is never switched automatically when the resolution changes (the warning
stays).

## Resolution mismatch

Measurements are in image pixels. If the live frame size differs from the active profile's
`width`x`height`, the status bar shows a warning. The profile stays active (decision: warn, do not
switch automatically).

## File

`calibrations.toml` in the user config folder (`~/.config/scopeview` on Linux, the platform
equivalent elsewhere): `active` (index) and a `[[profiles]]` array with the fields above.
A file that does not parse loads as empty with a warning; invalid or surplus profiles are dropped
with a warning and the active index follows the surviving profile. Writes go through a temporary file
and a rename.
