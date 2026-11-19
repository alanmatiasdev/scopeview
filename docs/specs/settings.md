# Settings

Status: ready. Roadmap item 1.8. Code: `core::settings`, `app::store`, settings window.

| Key | Default | Notes |
| --- | --- | --- |
| `save_dir` | empty | Empty means `<Pictures>/ScopeView` (home folder if there is no Pictures folder) |
| `line_color` | `[255, 64, 64]` | RGB |
| `line_width` | 2 | Image pixels, clamped to 1..=10 |
| `text_color` | `[255, 255, 255]` | RGB, label text |
| `text_size` | 18 | Image pixels, clamped to 8..=72 |
| `grid_color` | `[255, 220, 64]` | RGB, crosshair and ruler ticks (our own default) |
| `photo_format` | `jpeg` | `jpeg`, `png` or `bmp` |
| `last_device` | none | Camera name; selected again on start if it is present |
| `max_record_seconds` | 0 | Recording time limit in seconds, 0 for none (see `recording.md`) |
| `language` | none | UI language code; none follows the OS (see `i18n.md`) |
| `prefer_mjpeg` | `true` | With the Automatic format, MJPEG wins over raw formats of the same size and rate; with `false` raw wins. Changing it reopens the camera when the format is Automatic. A format picked by hand is never changed |
| `smooth_scaling` | `true` | Filter the image when it is drawn larger or smaller than the camera's pixels; `false` shows hard pixels. Photos and videos are unaffected |
| `theme`, `high_contrast`, `ui_scale` | see `accessibility.md` | |

File: `settings.toml` in the user config folder. Saved whenever a value changes in the settings
window or the camera changes. Missing keys use defaults. A file that does not parse gives defaults
and a warning in the status bar. The photo folder is created when the first photo is saved.
The settings window edits the folder as text (no folder picker yet).
