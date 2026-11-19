# Units, scale and label formatting

Status: ready. Roadmap item 1.1. Code: `crates/core` (`units`, `scale`, `geometry`, `format`).

## Units

- `Px` (image pixels, used while uncalibrated), `Mm` (millimeters), `Um` (micrometers).
- Display suffixes: `px`, `mm`, `µm`.
- Calibration units are only `Mm` and `Um`. `Px` is never a calibration unit.

## Scale

A scale is `pixels_per_unit` plus a physical unit.

- Built from a calibration: pixel length `P` of the drawn line and entered real length `L`.
  `pixels_per_unit = P / L`.
- Rejected (no scale is produced) unless `P` and `L` are finite and greater than zero, and
  the unit is physical (`Mm` or `Um`).
- A pixel length converts to a real length by dividing by `pixels_per_unit`.
  An area in square pixels divides by `pixels_per_unit` squared.

## Geometry

- All shapes live in image pixel coordinates (camera resolution), as `f64`.
- Segment length is the Euclidean distance between its endpoints.
- A rectangle is defined by two opposite corners given in any order.
  Width and height are the absolute coordinate differences. Its anchor is the top-left corner
  (smallest x, smallest y). Area is width times height.

## Rectangle labels

Two lines, anchored at the top-left corner:

- `"{w:.2}x{h:.2}{unit}"`, for example `3.50x2.00mm`
- `"S:{area:.2}{unit}^2"`, for example `S:7.00mm^2`

With a scale, values are converted and the unit is the scale's unit.
Without a scale, values stay in pixels and the unit is `px` (decision: uncalibrated
measurements always show pixels; no label is hidden).

## Crosshair ruler label

A value `v > 0` with a unit suffix. A small epsilon is added before rounding, and the number of
decimals depends on magnitude:

| Value | Decimals | Epsilon added |
| --- | --- | --- |
| `v >= 0.1` | 1 | 0.01 |
| `0.01 <= v < 0.1` | 2 | 0.001 |
| `v < 0.01` | 3 | 0.0001 |

Example: `0.25mm` is shown as `0.3mm`; `0.05mm` as `0.05mm`.
The step selection rule is in `crosshair.md`.

## Out of scope

Any I/O.

## Straight-line label

One line of text, `"{length:.2}{unit}"`, for example `3.50mm` or `120.00px`, anchored at the
first endpoint (the point where the user started dragging). Values convert with the scale when
there is one, otherwise stay in pixels.

## Milestone 2 labels

- Circle, three lines: `"R:{r:.2}{unit}"`, `"C:{c:.2}{unit}"` (circumference, 2πR), `"S:{a:.2}{unit}^2"`.
- Concentric circles, two lines: `"R1:{r1:.2}{unit}"`, `"R2:{r2:.2}{unit}"`.
- Parallel lines: `"D:{d:.2}{unit}"`, d being the perpendicular distance.
- Angle: `"{deg:.2}°"`.

All use the scale like other lengths; uncalibrated values are in pixels.
