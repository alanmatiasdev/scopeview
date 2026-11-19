# Image view: flip, image modes and crosshair

Status: ready. Roadmap items 2.2, 3.3 and 2.7 (crosshair, in `crosshair.md`).

## Flip

Two toggles, horizontal and vertical. The flip is applied to each frame right after it is decoded, so
the preview, saved photos and (later) recordings all show the same orientation. Toggling a flip clears
the measurements, because they refer to image pixels that just moved. The setting is not saved.

## Image modes

A mode picker in the toolbar: Normal, Grayscale, Binary, Invert, Emboss. The mode is applied to each
frame after the flip, so the preview, photos and recordings agree. Alpha is untouched. The mode is not
saved between runs, and changing it does not clear measurements (the pixels do not move).

The rules below are ScopeView's own choices.

| Mode | Rule |
| --- | --- |
| Grayscale | Luma `(299 R + 587 G + 114 B) / 1000` written to all three channels |
| Binary | Luma >= 128 becomes 255, otherwise 0 |
| Invert | Each channel becomes `255 - value` |
| Emboss | Luma through the 3x3 kernel `[-2 -1 0; -1 0 1; 0 1 2]`, plus 128, clamped; edges use the nearest pixel. A flat area is mid gray (128) |

## Smooth scaling

The picture is drawn to fit the window, so it is almost never at a 1:1 scale. With smooth scaling
on (the default) the texture is filtered bilinearly. With it off, the nearest pixel is shown, which
is better for judging single pixels at high zoom. Only the preview changes; saved photos and
videos use the camera's pixels as they are. Both defaults are our own.
