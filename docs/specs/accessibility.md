# Accessibility: themes, contrast and interface size

Status: ready (own decisions). Roadmap item 4.4. Code: `app::theme`, settings window.

## Settings

| Key | Default | Notes |
| --- | --- | --- |
| `theme` | `system` | `system`, `light` or `dark` |
| `high_contrast` | `false` | Applies on top of the light or dark theme |
| `ui_scale` | 1.0 | Interface zoom, clamped to 0.5..=3.0. The settings window offers 75 %, 100 %, 125 %, 150 %, 175 %, 200 % and 300 % |

All three apply at once and are saved. The zoom keys (Ctrl and + or -, Ctrl+0) also change
`ui_scale`, and the new value is saved.

## Contrast

Contrast is the WCAG 2 ratio between the text color and the background it sits on. Unit tests build
the palette for each theme and check these pairs: text on panels, windows and text fields; warning,
error and link colors on panels; button text in the idle, hovered and pressed states; selected items.

- Normal themes: at least 4.5:1 (WCAG AA).
- High contrast: at least 7:1 (WCAG AAA). Backgrounds are pure black or white, text is full
  strength, outlines are 1.5 to 2.5 points wide and hover and keyboard focus use an accent color.

The warning and error colors in the status bar come from the theme, so they stay readable on the
light theme. The hint drawn over the image has a dark box behind it because the picture behind
can be any color.

## Not covered

- Measurement colors (line, text and grid) are chosen by the user in settings and are drawn over
  an arbitrary image, so no contrast rule applies to them.
- Screen reader support depends on the UI toolkit's accessibility layer. It was not tested with a
  screen reader.
