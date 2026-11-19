# Video recording

Status: ready (own decisions marked). Roadmap items 3.1 and 3.2. Code: `record` crate, app
`recording` module, settings window.

## Behavior

- **Record** starts a recording of the live view; it is enabled once a frame has arrived. While
  recording the toolbar shows **Stop** and **Pause** / **Resume**, and the status bar shows
  `REC <time>` (or `REC paused <time>`).
- The video is the same picture as the screen: flip and image mode applied, and the overlay
  (crosshair, measurements, text, pencil) burned in. Selection handles and the shape being drawn
  are not recorded. The video has the camera's resolution.
- Files go to the save folder of the settings (the one photos use), named
  `scopeview-YYYYMMDD-HHMMSS.avi` in local time. An existing file is never overwritten: the stamp
  gets `-2`, `-3`, ... until the name is free.
- Pause stops the clock and discards frames; resume continues the same file. Elapsed time never
  includes paused time.
- The timer shows `s.t` under one minute and `m:ss` after that.
- Changing the camera or the format stops the recording first and saves it. Closing the app does
  the same.
- When the recording ends, the status bar shows the saved path, or the error.

## Time limit

A setting, `max_record_seconds`, with 0 meaning no limit. The picker offers: unlimited, 10 s, 30 s,
1 min, 5 min, 10 min, 30 min, 1 h, 2 h, 4 h, 8 h. At the limit the recording **stops** and is saved
(it does not continue in a new file). The limit counts recording time, not paused time.

The value is kept in the range 0..=28800 (seconds, so 8 h at most). The default is unlimited.

## Splitting

A file that would pass `0x83000000` bytes (about 2.05 GiB) continues in a new file in the same
recording, with no gap: `scopeview-<stamp>-part2.avi`, `-part3`, ... The clock keeps
running across files.

## File format

AVI 1.0, one video stream: MJPEG (`MJPG`), 24-bit, constant frame rate, with an `idx1` index so
players can seek. No audio. JPEG quality is the same as photos (90). Written by our own code in
`record::avi`; no external encoder. Checked with `ffprobe` and `ffmpeg`.

## Constant frame rate

The frame rate is the one the camera was asked for. Camera frames arrive unevenly, so each frame
is placed on the timeline by the recording clock (`record::pacing`): a late frame is repeated to
fill the gap, an early one is dropped. A stall longer than 5 s (a reconnect) is skipped rather
than filled, so the video is shorter than the wall clock by that gap. The video's length therefore
follows the clock, not the number of frames the camera delivered.

## Threading

Frames are offered to a recorder thread over a bounded queue (4 frames). The thread burns in the
overlay, encodes and writes; if it falls behind, new frames are dropped (counted as `dropped`) and
the UI never waits. A frame whose size differs from the first one is dropped.

## Not done

- Audio.
- Resuming a recording after the camera is switched (it is saved and stopped).
- Real-time clock stamp or the measurement scale in the video.
