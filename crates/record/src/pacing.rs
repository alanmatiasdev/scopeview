//! Keeping the video at a constant frame rate when camera frames arrive unevenly.

use std::time::Duration;

/// How much of a stall is filled by repeating the last frame, in seconds. A longer gap is
/// skipped, so the video is shorter than the wall clock.
pub const MAX_FILL_SECONDS: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pace {
    /// How many copies of the frame to write now (0 when it arrived ahead of schedule).
    pub write: u64,
    /// Frames on the video timeline after writing them.
    pub written: u64,
}

/// Decide what to do with a frame that arrived `active` into the recording (pauses excluded),
/// when `written` frames are in the video so far.
///
/// Frame `n` is due at `n / fps`, and the first frame is due at once. A frame that arrives late
/// is repeated to fill the gap, up to `MAX_FILL_SECONDS`; one that arrives early is dropped.
pub fn pace(written: u64, active: Duration, fps: u32) -> Pace {
    let fps = u64::from(fps.max(1));
    let due = (active.as_secs_f64() * fps as f64).floor() as u64 + 1;
    if due <= written {
        return Pace { write: 0, written };
    }
    let behind = due - written;
    if behind > fps * u64::from(MAX_FILL_SECONDS) {
        // A long stall: write one frame and carry on from the current time.
        return Pace {
            write: 1,
            written: due,
        };
    }
    Pace {
        write: behind,
        written: due,
    }
}

/// Text for the elapsed recording time: `s.t` under a minute, `m:ss` after that.
pub fn format_elapsed(active: Duration) -> String {
    let tenths = active.as_millis() / 100;
    if tenths < 600 {
        format!("{}.{}", tenths / 10, tenths % 10)
    } else {
        format!("{}:{:02}", tenths / 600, (tenths % 600) / 10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn the_first_frame_is_written_at_once() {
        assert_eq!(
            pace(0, ms(0), 30),
            Pace {
                write: 1,
                written: 1
            }
        );
        assert_eq!(
            pace(0, ms(10), 30),
            Pace {
                write: 1,
                written: 1
            }
        );
    }

    #[test]
    fn frames_on_schedule_are_written_one_each() {
        let mut written = 0;
        for t in [0, 34, 67, 100, 134] {
            let p = pace(written, ms(t), 30);
            assert_eq!(p.write, 1, "at {t} ms");
            written = p.written;
        }
        assert_eq!(written, 5);
    }

    #[test]
    fn an_early_frame_is_dropped() {
        // 30 fps: frame 2 is due at 33.3 ms.
        assert_eq!(
            pace(2, ms(40), 30),
            Pace {
                write: 0,
                written: 2
            }
        );
    }

    #[test]
    fn a_late_frame_is_repeated_to_fill_the_gap() {
        // 10 fps, one frame written, next arrives at 400 ms: frames due at 100, 200, 300, 400.
        assert_eq!(
            pace(1, ms(400), 10),
            Pace {
                write: 4,
                written: 5
            }
        );
    }

    #[test]
    fn a_long_stall_is_skipped_not_filled() {
        let p = pace(10, Duration::from_secs(60), 10);
        assert_eq!(p.write, 1);
        assert_eq!(p.written, 601);
        // And the schedule continues from there.
        assert_eq!(pace(p.written, ms(60_100), 10).write, 1);
    }

    #[test]
    fn a_stall_just_under_the_limit_is_still_filled() {
        // 10 fps: 5 s = 50 frames of fill allowed.
        let p = pace(1, ms(5_000), 10);
        assert_eq!(p.write, 50);
    }

    #[test]
    fn elapsed_text_has_the_expected_format() {
        assert_eq!(format_elapsed(ms(0)), "0.0");
        assert_eq!(format_elapsed(ms(1_234)), "1.2");
        assert_eq!(format_elapsed(ms(59_999)), "59.9");
        assert_eq!(format_elapsed(ms(60_000)), "1:00");
        assert_eq!(format_elapsed(ms(125_400)), "2:05");
        assert_eq!(format_elapsed(ms(3_725_000)), "62:05");
    }
}
