# Recent files and thumbnails

Status: ready (own decisions marked). Roadmap item 3.6. Code: `core::recent`, `imaging::thumbnail`,
app `recent` module.

## The list

- The latest **5** saved files, photos and videos together, newest first.
- A photo is added when it is saved; a recording is added when it ends (its first file, when it was
  split). Saving a file already in the list moves it to the front.
- Saved in `recent.toml` in the config folder (`files = [...]`). At start, entries whose file no
  longer exists are dropped. A file that does not parse gives an empty list and a warning.

## The panel

The **Recent** toolbar toggle shows a panel on the right. Each entry has its thumbnail, the file name
(the full path as a tooltip), **Open** and **Remove**. Clicking the thumbnail opens the file in the
system's default program (`xdg-open`, `open`, or `explorer`, started without a shell). Remove takes
the entry off the list and never deletes the file. A video without a thumbnail shows a button
labeled "Video".

## Thumbnails

- Kept in a `Thumbnails` folder next to the file: `<folder>/Thumbnails/<name without extension>.jpg`.
- Longest side 128 px, aspect ratio kept, never scaled up, JPEG quality 80.
- A photo's thumbnail is written when the photo is saved (from the image with the overlay burned in).
  A video's is written when recording starts, from the first frame without overlay.
- A photo that has no thumbnail (saved by an older version, or the folder was cleaned) gets one made
  from the photo the first time the panel shows it. A missing thumbnail for a video is looked for
  again every 2 seconds, so one being written shows up on its own.
- A thumbnail that cannot be written never fails the save.
