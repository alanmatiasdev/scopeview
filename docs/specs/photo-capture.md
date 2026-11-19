# Photo capture

Status: ready (own decisions; see below). Roadmap item 1.7. Code: `imaging` crate, app Photo button.

## Behavior

- The Photo button saves the newest frame with all measurements burned in, using the same
  primitives as the screen. The image keeps the camera's resolution.
- Format and folder come from settings (JPEG, PNG or BMP; BMP is 24-bit RGB, uncompressed).
- File name: `scopeview-YYYYMMDD-HHMMSS.<ext>` in local time. If the name exists, `-2`, `-3`, ...
  is appended; an existing photo is never overwritten.
- JPEG quality is 90. PNG is lossless.
- Saving runs on a worker thread; the status bar then shows the saved path or the error.
  A second photo cannot start while one is saving.
- No frame yet: the button is disabled.

## Decisions

The file name pattern and the JPEG quality are our own choices.
