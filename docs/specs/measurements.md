# Measurements and overlay

Status: ready. Roadmap items 1.4, 1.5 and 1.6. Code: `core::measure`, `core::overlay`, app canvas.

## Tools

None (no drawing), Select, Line, Rectangle, Circle, Concentric, Parallel, Angle, Calibrate.
Shapes are created by dragging on the image. Parallel, Angle and Concentric finish with a second click.
The start and end points are clamped to the image. Shapes smaller than 3 image pixels
(any side, for rectangles) are discarded. Escape cancels a drag in progress.

## Coordinates

All shapes are stored in image pixels (camera resolution). Line width and text size are also in
image pixels, so a saved photo looks like the screen at any window size.
Measurements are cleared when the frame size changes (new format or device), since they would
refer to different pixels.

## Shapes added in milestone 2

| Tool | Gesture | Drawn | Label lines | Label anchor |
| --- | --- | --- | --- | --- |
| Circle | drag from the center to the edge | radius line and circle | `R:` radius, `C:` circumference, `S:` area | center |
| Concentric | drag center to the first edge, then click the second edge | radius line to the second edge and both circles | `R1:`, `R2:` | second edge point |
| Parallel | drag the first line, then click a point | first line, a second line centered on the click with the same length and direction, and the perpendicular between them | `D:` perpendicular distance | the clicked point |
| Angle | drag the first ray from the vertex, then click the end of the second ray | both rays | angle in degrees, `{:.2}°` | vertex |

Values use the same rules as lines: calibrated units, or pixels when uncalibrated. Areas use the
unit squared. An angle has no unit and ignores calibration. While the second click is pending a
preview follows the pointer; Escape cancels. A final click closer than 3 image pixels is ignored.

Decision: the angle is the smaller one between the rays (0 to 180 degrees), never the reflex angle.

## Selecting and editing

The Select tool picks the newest shape whose outline or handle is within 6 screen points of the
pointer. A selected shape shows a 10 px square handle on each of its points. A handle wins over the
outline, and the selected shape's handles stay reachable under shapes drawn later.

- Dragging the outline moves the whole shape.
- Dragging a handle reshapes it: a line end moves; a rectangle corner moves with the opposite corner
  fixed; a circle's edge handle changes the radius and its center handle moves the circle; for
  concentric circles each edge handle sets its own radius; for parallel lines the first line's ends
  move it and the third handle changes the distance; an angle's vertex and ray ends move one by one.
- Handle drags stay inside the image.
- Delete removes the selected shape; Escape or a click on empty space deselects.
- Handles are shown on screen only, never in photos.

## Labels

Defined in `units-and-scale.md`.

- Line: one label at the first endpoint (where the drag started).
- Rectangle: two lines (size, area) at the top-left corner.
- With an active calibration values use its unit; without one they are in pixels.
- Label text is white on a translucent black box, so it stays readable on any image.

## Style

Line color and width come from settings (default red, 2 px). A rectangle is drawn as a frame with
its stroke inside the corners.

## Deleting

- Delete last removes the newest measurement.
- Delete all removes every measurement.
- Neither can be undone. Both are disabled when there is nothing to delete.

## Overlay primitives

`core::overlay` turns measurements, scale and style into primitives (`Line`, `RectOutline`, `Text`).
The app paints them with egui; `imaging` rasterizes the same primitives into photos.

## Annotations (roadmap 3.4 and 3.5)

Two more items in the same list, with the same select, move, delete and burn-in behavior. Neither
measures anything, so they have no scale label.

### Text

- The Text tool: click where the text goes, type it in the small window, press Enter or Place. Empty
  text is discarded. Esc or closing the window cancels.
- The text is one line, drawn at the clicked point (its top-left) in the text color and text size from
  the settings, on the same translucent backing as measurement labels. It follows later changes to
  those settings.
- Select grabs it inside its box. One handle marks the selection; dragging the handle or the body
  moves the note. The box used for grabbing is an estimate (0.55 x size per character wide, 1.2 x size
  tall) because the core crate has no font; it only has to be close.
- The note is placed by the click and moved afterwards with Select.

### Pencil

- The Pencil tool: press, drag, release. Points closer than 1 image pixel to the previous one are
  skipped. A stroke needs at least two points; a plain click draws nothing. Esc drops a stroke in
  progress.
- Drawn as straight segments between the points in the line color and line width.
- Select grabs it within the usual tolerance of any segment. One handle (the first point) marks the
  selection; a stroke moves as a whole and cannot be reshaped.
