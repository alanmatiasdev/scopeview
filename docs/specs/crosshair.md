# Crosshair with ruler ticks

Status: ready. Roadmap item 2.7. Code: `core::crosshair`, drawn in the grid color from settings.

A toggle in the tools row. The crosshair is part of the scene, so it is burned into saved photos too.

## Axes

A horizontal line across the full width at half the height, and a vertical line across the full
height at half the width. Lines are 1 image pixel wide.

## Uncalibrated

A tick every 10 px outward from the center on both axes. The 5th, 10th, ... tick is long (9 px) and
the others are short (4 px). Ticks are centered on the axis.

## Calibrated

The major step is chosen among 1, 2 and 5 times a power of ten (from 0.0001 to 10 in the calibration
unit): the candidate whose length on screen is closest to 100 px wins, and the smaller value wins a tie.
The step length is rounded to whole pixels. If it rounds to zero, only the axes are drawn.

- Major ticks every step from the center in both directions, 20 px long.
- Nine minor ticks between majors, 8 px long, 14 px for the middle one.
- One label with the step value and unit, to the right of the center at the first major tick, just
  under the horizontal axis. Number format as in `units-and-scale.md`.

## Decisions

- Ticks are centered on the axis (own choice).
- The label's left edge sits on the first major tick instead of being centered on it (own choice).
