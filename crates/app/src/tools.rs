//! Pure rules for the drawing and editing tools. The UI feeds pointer events in image pixels and
//! gets finished measurements and previews back, so none of this needs egui.

use scopeview_core::edit::{apply_drag, hit_test, Grab};
use scopeview_core::{
    Angle, Circle, Concentric, Measurement, MeasurementList, ParallelLines, Point, RectShape,
    Segment, Stroke,
};

/// Drags shorter than this many image pixels are ignored (accidental clicks).
pub const MIN_DRAG: f64 = 3.0;

/// The pencil skips points closer than this many image pixels to the previous one.
pub const PENCIL_STEP: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    None,
    Select,
    Line,
    Rect,
    Circle,
    Concentric,
    Parallel,
    Angle,
    Text,
    Pencil,
    Calibrate,
}

impl Tool {
    /// Tools that finish with a click after the first drag.
    fn has_second_step(self) -> bool {
        matches!(self, Tool::Concentric | Tool::Parallel | Tool::Angle)
    }

    pub fn draws(self) -> bool {
        !matches!(self, Tool::None | Tool::Select)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Stage {
    Idle,
    /// Button down, dragging the first part (a line, a rectangle, a radius...).
    Dragging {
        start: Point,
        current: Point,
    },
    /// Button down with the pencil: the points passed so far.
    Drawing(Vec<Point>),
    /// First part done; waiting for the click that places the rest.
    Placing {
        start: Point,
        end: Point,
        cursor: Point,
    },
}

/// What a pointer event produced.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Nothing,
    Done(Measurement),
    /// The text tool was clicked; the app asks for the text to put here.
    PlaceText(Point),
    /// The calibration tool finished a line; the app asks for its real length.
    Calibrate(Segment),
}

/// A shape to show while the user is still drawing. `use_scale` is false for the calibration
/// line, which is shown in pixels because its scale is what is being set.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    pub measurement: Measurement,
    pub use_scale: bool,
}

#[derive(Debug, Clone)]
pub struct Builder {
    tool: Tool,
    stage: Stage,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            tool: Tool::None,
            stage: Stage::Idle,
        }
    }
}

impl Builder {
    pub fn tool(&self) -> Tool {
        self.tool
    }

    pub fn set_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.stage = Stage::Idle;
    }

    /// Drop whatever is being drawn (Escape).
    pub fn cancel(&mut self) {
        self.stage = Stage::Idle;
    }

    pub fn is_busy(&self) -> bool {
        self.stage != Stage::Idle
    }

    pub fn is_placing(&self) -> bool {
        matches!(self.stage, Stage::Placing { .. })
    }

    /// The pointer moved (with or without a button down).
    pub fn pointer_move(&mut self, p: Point) {
        match &mut self.stage {
            Stage::Drawing(points) => {
                if points
                    .last()
                    .is_none_or(|last| Segment::new(*last, p).length() >= PENCIL_STEP)
                {
                    points.push(p);
                }
            }
            Stage::Dragging { current, .. } => *current = p,
            Stage::Placing { cursor, .. } => *cursor = p,
            Stage::Idle => {}
        }
    }

    /// Button pressed. Starts a drag, or places the last point of a multi-step shape.
    pub fn pointer_down(&mut self, p: Point) -> Outcome {
        if !self.tool.draws() {
            return Outcome::Nothing;
        }
        if self.tool == Tool::Text {
            return Outcome::PlaceText(p);
        }
        if self.tool == Tool::Pencil {
            self.stage = Stage::Drawing(vec![p]);
            return Outcome::Nothing;
        }
        match self.stage.clone() {
            Stage::Idle | Stage::Dragging { .. } | Stage::Drawing(_) => {
                self.stage = Stage::Dragging {
                    start: p,
                    current: p,
                };
                Outcome::Nothing
            }
            Stage::Placing { start, end, .. } => match self.finish_second(start, end, p) {
                Some(m) => {
                    self.stage = Stage::Idle;
                    Outcome::Done(m)
                }
                // Too close to be meaningful: keep waiting for a better click.
                None => Outcome::Nothing,
            },
        }
    }

    /// Button released after a drag.
    pub fn pointer_up(&mut self, p: Point) -> Outcome {
        if matches!(self.stage, Stage::Drawing(_)) {
            let Stage::Drawing(mut points) = std::mem::replace(&mut self.stage, Stage::Idle) else {
                return Outcome::Nothing;
            };
            if points
                .last()
                .is_none_or(|last| Segment::new(*last, p).length() >= PENCIL_STEP)
            {
                points.push(p);
            }
            return if points.len() >= 2 {
                Outcome::Done(Measurement::Pencil(Stroke::new(points)))
            } else {
                Outcome::Nothing
            };
        }
        let Stage::Dragging { start, .. } = self.stage else {
            return Outcome::Nothing;
        };
        self.stage = Stage::Idle;
        let segment = Segment::new(start, p);
        let rect = RectShape::new(start, p);
        match self.tool {
            Tool::Line if segment.length() >= MIN_DRAG => Outcome::Done(Measurement::Line(segment)),
            Tool::Rect if rect.width() >= MIN_DRAG && rect.height() >= MIN_DRAG => {
                Outcome::Done(Measurement::Rect(rect))
            }
            Tool::Circle if segment.length() >= MIN_DRAG => {
                Outcome::Done(Measurement::Circle(Circle::new(start, p)))
            }
            Tool::Calibrate if segment.length() >= MIN_DRAG => Outcome::Calibrate(segment),
            t if t.has_second_step() && segment.length() >= MIN_DRAG => {
                self.stage = Stage::Placing {
                    start,
                    end: p,
                    cursor: p,
                };
                Outcome::Nothing
            }
            _ => Outcome::Nothing,
        }
    }

    /// The finished shape for a multi-step tool, or `None` if `third` is too close to be useful.
    fn finish_second(&self, start: Point, end: Point, third: Point) -> Option<Measurement> {
        match self.tool {
            Tool::Concentric => {
                let c = Concentric::new(start, end, third);
                (c.radius2() >= MIN_DRAG).then_some(Measurement::Concentric(c))
            }
            Tool::Parallel => {
                let p = ParallelLines::new(Segment::new(start, end), third);
                (p.distance() >= MIN_DRAG).then_some(Measurement::Parallel(p))
            }
            Tool::Angle => {
                let a = Angle::new(start, end, third);
                (Segment::new(start, third).length() >= MIN_DRAG).then_some(Measurement::Angle(a))
            }
            _ => None,
        }
    }

    pub fn preview(&self) -> Option<Preview> {
        let shown = |measurement| {
            Some(Preview {
                measurement,
                use_scale: true,
            })
        };
        match (self.stage.clone(), self.tool) {
            (Stage::Idle, _) => None,
            (Stage::Drawing(points), _) => shown(Measurement::Pencil(Stroke::new(points))),
            (Stage::Dragging { start, current }, tool) => match tool {
                Tool::Rect => shown(Measurement::Rect(RectShape::new(start, current))),
                Tool::Circle | Tool::Concentric => {
                    shown(Measurement::Circle(Circle::new(start, current)))
                }
                Tool::Calibrate => Some(Preview {
                    measurement: Measurement::Line(Segment::new(start, current)),
                    use_scale: false,
                }),
                Tool::Line | Tool::Parallel | Tool::Angle => {
                    shown(Measurement::Line(Segment::new(start, current)))
                }
                Tool::None | Tool::Select | Tool::Text | Tool::Pencil => None,
            },
            (Stage::Placing { start, end, cursor }, Tool::Concentric) => {
                shown(Measurement::Concentric(Concentric::new(start, end, cursor)))
            }
            (Stage::Placing { start, end, cursor }, Tool::Parallel) => shown(
                Measurement::Parallel(ParallelLines::new(Segment::new(start, end), cursor)),
            ),
            (Stage::Placing { start, end, cursor }, Tool::Angle) => {
                shown(Measurement::Angle(Angle::new(start, end, cursor)))
            }
            (Stage::Placing { .. }, _) => None,
        }
    }
}

struct EditDrag {
    index: usize,
    grab: Grab,
    original: Measurement,
    from: Point,
}

/// Selection and editing of existing measurements.
#[derive(Default)]
pub struct Selector {
    selected: Option<usize>,
    drag: Option<EditDrag>,
}

impl Selector {
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    pub fn reset(&mut self) {
        self.selected = None;
        self.drag = None;
    }

    /// Press: select the newest measurement under `p` and start dragging what was grabbed.
    /// Pressing on nothing clears the selection.
    ///
    /// `text_size` is the label size in image pixels, which sets the box of a text note.
    pub fn press(&mut self, list: &MeasurementList, p: Point, tolerance: f64, text_size: f64) {
        self.drag = None;
        // The selected shape is tested first so its handles stay reachable under other shapes.
        let hit = self
            .selected
            .and_then(|i| {
                list.get(i)
                    .and_then(|m| hit_test(m, p, tolerance, text_size))
                    .map(|g| (i, g))
            })
            .or_else(|| list.hit(p, tolerance, text_size));
        match hit {
            Some((index, grab)) => {
                self.selected = Some(index);
                self.drag = list.get(index).map(|m| EditDrag {
                    index,
                    grab,
                    original: m.clone(),
                    from: p,
                });
            }
            None => self.selected = None,
        }
    }

    /// Drag: reshape or move the grabbed measurement. Handle drags use `clamped` (kept inside
    /// the image); body drags use the free position so the shape moves by the pointer delta.
    pub fn drag(&mut self, list: &mut MeasurementList, free: Point, clamped: Point) {
        let Some(d) = &self.drag else {
            return;
        };
        let to = match d.grab {
            Grab::Body => free,
            Grab::Handle(_) => clamped,
        };
        list.replace(d.index, apply_drag(&d.original, d.grab, d.from, to));
    }

    pub fn release(&mut self) {
        self.drag = None;
    }

    /// Remove the selected measurement; true if there was one.
    pub fn delete_selected(&mut self, list: &mut MeasurementList) -> bool {
        let removed = self.selected.is_some_and(|i| list.remove(i).is_some());
        self.reset();
        removed
    }

    /// Call after the list changed in ways the selector did not do (delete last, clear).
    pub fn validate(&mut self, list: &MeasurementList) {
        if self.selected.is_some_and(|i| i >= list.len()) {
            self.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn drag(b: &mut Builder, from: Point, to: Point) -> Outcome {
        b.pointer_down(from);
        b.pointer_move(to);
        b.pointer_up(to)
    }

    fn builder(tool: Tool) -> Builder {
        let mut b = Builder::default();
        b.set_tool(tool);
        b
    }

    #[test]
    fn line_and_rect_need_a_real_drag() {
        let mut b = builder(Tool::Line);
        assert_eq!(drag(&mut b, p(0.0, 0.0), p(1.0, 1.0)), Outcome::Nothing);
        assert!(matches!(
            drag(&mut b, p(0.0, 0.0), p(30.0, 40.0)),
            Outcome::Done(Measurement::Line(_))
        ));
        let mut b = builder(Tool::Rect);
        // Thin in one direction: discarded.
        assert_eq!(drag(&mut b, p(0.0, 0.0), p(50.0, 1.0)), Outcome::Nothing);
        assert!(matches!(
            drag(&mut b, p(0.0, 0.0), p(50.0, 20.0)),
            Outcome::Done(Measurement::Rect(_))
        ));
    }

    #[test]
    fn circle_is_center_then_edge() {
        let mut b = builder(Tool::Circle);
        let Outcome::Done(Measurement::Circle(c)) = drag(&mut b, p(10.0, 10.0), p(40.0, 50.0))
        else {
            panic!("no circle")
        };
        assert_eq!(c.center, p(10.0, 10.0));
        assert_eq!(c.radius(), 50.0);
    }

    #[test]
    fn calibrate_returns_the_segment() {
        let mut b = builder(Tool::Calibrate);
        assert!(matches!(
            drag(&mut b, p(0.0, 0.0), p(100.0, 0.0)),
            Outcome::Calibrate(s) if s.length() == 100.0
        ));
    }

    #[test]
    fn parallel_needs_a_second_click() {
        let mut b = builder(Tool::Parallel);
        assert_eq!(drag(&mut b, p(0.0, 0.0), p(100.0, 0.0)), Outcome::Nothing);
        assert!(b.is_placing());
        // Preview follows the cursor.
        b.pointer_move(p(50.0, 25.0));
        assert!(matches!(
            b.preview(),
            Some(Preview { measurement: Measurement::Parallel(pl), .. }) if pl.distance() == 25.0
        ));
        // A click too close to the first line does nothing and keeps waiting.
        assert_eq!(b.pointer_down(p(50.0, 1.0)), Outcome::Nothing);
        assert!(b.is_placing());
        let Outcome::Done(Measurement::Parallel(pl)) = b.pointer_down(p(50.0, 25.0)) else {
            panic!("no parallel")
        };
        assert_eq!(pl.distance(), 25.0);
        assert!(!b.is_busy());
    }

    #[test]
    fn angle_and_concentric_finish_on_the_second_click() {
        let mut b = builder(Tool::Angle);
        drag(&mut b, p(0.0, 0.0), p(100.0, 0.0));
        let Outcome::Done(Measurement::Angle(a)) = b.pointer_down(p(0.0, 100.0)) else {
            panic!("no angle")
        };
        assert!((a.degrees() - 90.0).abs() < 1e-9);

        let mut b = builder(Tool::Concentric);
        drag(&mut b, p(50.0, 50.0), p(60.0, 50.0));
        let Outcome::Done(Measurement::Concentric(c)) = b.pointer_down(p(50.0, 90.0)) else {
            panic!("no concentric")
        };
        assert_eq!((c.radius1(), c.radius2()), (10.0, 40.0));
    }

    #[test]
    fn escape_and_tool_change_discard_the_work() {
        let mut b = builder(Tool::Angle);
        drag(&mut b, p(0.0, 0.0), p(100.0, 0.0));
        b.cancel();
        assert!(!b.is_busy());
        assert_eq!(b.pointer_down(p(0.0, 100.0)), Outcome::Nothing);
        b.cancel();
        drag(&mut b, p(0.0, 0.0), p(100.0, 0.0));
        b.set_tool(Tool::Line);
        assert!(!b.is_busy());
    }

    #[test]
    fn none_and_select_tools_never_draw() {
        for tool in [Tool::None, Tool::Select] {
            let mut b = builder(tool);
            assert_eq!(drag(&mut b, p(0.0, 0.0), p(100.0, 100.0)), Outcome::Nothing);
            assert!(b.preview().is_none());
        }
    }

    #[test]
    fn previews_show_the_work_in_progress() {
        let mut b = builder(Tool::Calibrate);
        b.pointer_down(p(0.0, 0.0));
        b.pointer_move(p(10.0, 0.0));
        assert!(matches!(
            b.preview(),
            Some(Preview {
                use_scale: false,
                ..
            })
        ));
        let mut b = builder(Tool::Concentric);
        b.pointer_down(p(0.0, 0.0));
        b.pointer_move(p(10.0, 0.0));
        assert!(matches!(
            b.preview(),
            Some(Preview {
                measurement: Measurement::Circle(_),
                use_scale: true
            })
        ));
    }

    fn list_with_line_and_circle() -> MeasurementList {
        let mut list = MeasurementList::default();
        list.push(Measurement::Line(Segment::new(p(0.0, 0.0), p(100.0, 0.0))));
        list.push(Measurement::Circle(Circle::new(
            p(200.0, 200.0),
            p(230.0, 200.0),
        )));
        list
    }

    #[test]
    fn press_selects_and_empty_press_deselects() {
        let list = list_with_line_and_circle();
        let mut s = Selector::default();
        s.press(&list, p(50.0, 2.0), 5.0, 18.0);
        assert_eq!(s.selected(), Some(0));
        s.press(&list, p(200.0, 170.0), 5.0, 18.0);
        assert_eq!(s.selected(), Some(1));
        s.press(&list, p(500.0, 500.0), 5.0, 18.0);
        assert_eq!(s.selected(), None);
        assert!(!s.is_dragging());
    }

    #[test]
    fn dragging_the_body_moves_the_shape_without_drift() {
        let mut list = list_with_line_and_circle();
        let mut s = Selector::default();
        s.press(&list, p(50.0, 0.0), 5.0, 18.0);
        // Many intermediate moves must end the same as one move.
        for x in [60.0, 70.0, 80.0] {
            s.drag(&mut list, p(x, 10.0), p(x, 10.0));
        }
        s.release();
        assert_eq!(
            list.get(0),
            Some(&Measurement::Line(Segment::new(
                p(30.0, 10.0),
                p(130.0, 10.0)
            )))
        );
    }

    #[test]
    fn dragging_a_handle_edits_and_is_clamped_by_the_caller() {
        let mut list = list_with_line_and_circle();
        let mut s = Selector::default();
        s.press(&list, p(100.0, 0.0), 5.0, 18.0);
        s.drag(&mut list, p(900.0, 900.0), p(640.0, 480.0));
        assert_eq!(
            list.get(0),
            Some(&Measurement::Line(Segment::new(
                p(0.0, 0.0),
                p(640.0, 480.0)
            )))
        );
    }

    #[test]
    fn selected_handles_win_over_shapes_on_top() {
        let mut list = MeasurementList::default();
        list.push(Measurement::Line(Segment::new(p(0.0, 0.0), p(100.0, 0.0))));
        // A newer circle whose ring passes over the line's right end.
        list.push(Measurement::Circle(Circle::new(
            p(100.0, 50.0),
            p(100.0, 0.0),
        )));
        let mut s = Selector::default();
        s.press(&list, p(1.0, 0.0), 5.0, 18.0);
        assert_eq!(s.selected(), Some(0));
        s.press(&list, p(100.0, 0.0), 5.0, 18.0);
        assert_eq!(s.selected(), Some(0), "selected shape keeps its handle");
    }

    #[test]
    fn delete_selected_and_validate() {
        let mut list = list_with_line_and_circle();
        let mut s = Selector::default();
        assert!(!s.delete_selected(&mut list));
        s.press(&list, p(50.0, 0.0), 5.0, 18.0);
        assert!(s.delete_selected(&mut list));
        assert_eq!(list.len(), 1);
        assert_eq!(s.selected(), None);
        s.press(&list, p(200.0, 170.0), 5.0, 18.0);
        assert_eq!(s.selected(), Some(0));
        list.clear();
        s.validate(&list);
        assert_eq!(s.selected(), None);
    }

    #[test]
    fn text_tool_asks_the_app_for_text_at_the_click() {
        let mut b = builder(Tool::Text);
        assert_eq!(
            b.pointer_down(p(30.0, 40.0)),
            Outcome::PlaceText(p(30.0, 40.0))
        );
        assert!(!b.is_busy());
    }

    #[test]
    fn pencil_collects_points_and_skips_near_duplicates() {
        let mut b = builder(Tool::Pencil);
        b.pointer_down(p(0.0, 0.0));
        b.pointer_move(p(0.3, 0.0)); // closer than one pixel: skipped
        b.pointer_move(p(5.0, 0.0));
        b.pointer_move(p(5.0, 5.0));
        assert!(matches!(
            b.preview(),
            Some(Preview { measurement: Measurement::Pencil(s), .. }) if s.points.len() == 3
        ));
        let out = b.pointer_up(p(10.0, 5.0));
        let Outcome::Done(Measurement::Pencil(stroke)) = out else {
            panic!("expected a stroke, got {out:?}");
        };
        assert_eq!(
            stroke.points,
            vec![p(0.0, 0.0), p(5.0, 0.0), p(5.0, 5.0), p(10.0, 5.0)]
        );
        assert!(!b.is_busy());
    }

    #[test]
    fn pencil_click_without_movement_draws_nothing() {
        let mut b = builder(Tool::Pencil);
        b.pointer_down(p(7.0, 7.0));
        assert_eq!(b.pointer_up(p(7.0, 7.0)), Outcome::Nothing);
        assert!(!b.is_busy());
    }

    #[test]
    fn escape_drops_a_stroke_in_progress() {
        let mut b = builder(Tool::Pencil);
        b.pointer_down(p(0.0, 0.0));
        b.pointer_move(p(9.0, 9.0));
        b.cancel();
        assert_eq!(b.pointer_up(p(20.0, 20.0)), Outcome::Nothing);
    }

    #[test]
    fn text_note_can_be_selected_and_moved() {
        let mut list = MeasurementList::default();
        list.push(Measurement::Text(scopeview_core::TextNote::new(
            p(10.0, 10.0),
            "abcd",
        )));
        let mut s = Selector::default();
        s.press(&list, p(30.0, 20.0), 5.0, 20.0);
        assert_eq!(s.selected(), Some(0));
        s.drag(&mut list, p(40.0, 30.0), p(40.0, 30.0));
        s.release();
        let Some(Measurement::Text(t)) = list.get(0) else {
            panic!("text expected")
        };
        assert_eq!(t.pos, p(20.0, 20.0));
    }
}
