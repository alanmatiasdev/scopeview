# Keyboard shortcuts, help and about

Status: ready (own decisions). Roadmap item 4.2. Code: `app::shortcuts` (table), `app::app::help`.

## Shortcuts

`Cmd` on macOS, `Ctrl` elsewhere.

| Keys | Action |
| --- | --- |
| N, V, L, R, C, O, P, A, T, D, K | Tool: none, select, line, rectangle, circle, concentric, parallel, angle, text, pencil, calibrate |
| Space | Take a photo |
| Ctrl+R | Start or stop recording |
| Ctrl+P | Pause or resume the recording |
| Ctrl+Z | Delete the last measurement |
| Ctrl+Shift+Delete | Delete all measurements |
| Ctrl+H, Ctrl+J | Flip horizontally, flip vertically |
| X | Show or hide the crosshair |
| Ctrl+, | Settings |
| F1 | Help |
| Esc | Cancel the shape being drawn (unchanged) |
| Delete | Remove the selected shape (unchanged) |
| Ctrl and + or -, Ctrl+0 | Interface size (built into the UI toolkit) |

Rules:

- The table in `shortcuts.rs` is the only place keys are defined. The same table fills the help
  window and the hover text of the toolbar buttons.
- Shortcuts do nothing while a text field has the keyboard, so typing a name does not change tools.
- A shortcut does what its button does, including being disabled when the button is (no photo
  without an image).
- Tests check that no two actions share keys, that none uses the zoom keys, that every action and
  tool has a shortcut and that every label exists in the catalog.

## Help and About

- **Help** (button or F1): the shortcut table in two groups, tools and actions, and a note about
  Esc, Delete and the interface size keys.
- **About**: name, version (from the package), one-line description, license
  (GPL-3.0-or-later) and a link to the repository.
