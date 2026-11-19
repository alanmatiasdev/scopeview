//! File names for recordings.

use std::path::{Path, PathBuf};

/// `scopeview-<stamp>.avi`, with `-part<n>` before the extension for the second file onward.
pub fn video_file_name(stamp: &str, part: u32) -> String {
    if part <= 1 {
        format!("scopeview-{stamp}.avi")
    } else {
        format!("scopeview-{stamp}-part{part}.avi")
    }
}

/// A stamp whose first file does not exist in `dir` yet: `stamp`, else `stamp-2`, `stamp-3`, ...
/// An existing recording is never overwritten.
pub fn unique_stamp(dir: &Path, stamp: &str) -> String {
    let mut candidate = stamp.to_string();
    let mut n = 2;
    while dir.join(video_file_name(&candidate, 1)).exists() {
        candidate = format!("{stamp}-{n}");
        n += 1;
    }
    candidate
}

pub fn video_path(dir: &Path, stamp: &str, part: u32) -> PathBuf {
    dir.join(video_file_name(stamp, part))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn names_follow_the_pattern() {
        assert_eq!(
            video_file_name("20260101-120000", 1),
            "scopeview-20260101-120000.avi"
        );
        assert_eq!(
            video_file_name("20260101-120000", 2),
            "scopeview-20260101-120000-part2.avi"
        );
    }

    #[test]
    fn a_taken_stamp_gets_a_suffix() {
        let dir = std::env::temp_dir().join(format!("scopeview-naming-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(unique_stamp(&dir, "s"), "s");
        fs::write(dir.join("scopeview-s.avi"), b"x").unwrap();
        assert_eq!(unique_stamp(&dir, "s"), "s-2");
        fs::write(dir.join("scopeview-s-2.avi"), b"x").unwrap();
        assert_eq!(unique_stamp(&dir, "s"), "s-3");
        let _ = fs::remove_dir_all(&dir);
    }
}
