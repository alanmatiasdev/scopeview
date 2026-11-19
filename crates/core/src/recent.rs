//! The list of the files saved most recently (photos and videos), newest first.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// How many files the list keeps.
pub const MAX_RECENT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Photo,
    Video,
}

impl FileKind {
    /// By extension: `.avi` is a video, anything else a photo.
    pub fn of(path: &Path) -> Self {
        match path.extension().and_then(|e| e.to_str()) {
            Some(e) if e.eq_ignore_ascii_case("avi") => FileKind::Video,
            _ => FileKind::Photo,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RecentFiles {
    #[serde(default)]
    files: Vec<PathBuf>,
}

impl RecentFiles {
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Put `path` first. An entry already in the list moves to the front; the oldest entries past
    /// [`MAX_RECENT`] fall off.
    pub fn push(&mut self, path: PathBuf) {
        self.files.retain(|p| *p != path);
        self.files.insert(0, path);
        self.files.truncate(MAX_RECENT);
    }

    pub fn remove(&mut self, path: &Path) {
        self.files.retain(|p| p != path);
    }

    /// Keep only the entries `keep` accepts (for example, files that still exist).
    pub fn retain(&mut self, mut keep: impl FnMut(&Path) -> bool) {
        self.files.retain(|p| keep(p));
    }

    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("recent files always serialize")
    }

    /// Parse a saved file. A broken file gives an empty list and a warning; extra entries are cut.
    pub fn from_toml_lossy(text: &str) -> (Self, Option<String>) {
        match toml::from_str::<RecentFiles>(text) {
            Ok(mut list) => {
                list.files.truncate(MAX_RECENT);
                (list, None)
            }
            Err(error) => (
                Self::default(),
                Some(format!("recent files ignored: {error}")),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(name: &str) -> PathBuf {
        PathBuf::from(name)
    }

    #[test]
    fn newest_goes_first_and_the_list_is_capped() {
        let mut list = RecentFiles::default();
        for i in 0..8 {
            list.push(p(&format!("f{i}.jpg")));
        }
        assert_eq!(list.files().len(), MAX_RECENT);
        assert_eq!(list.files()[0], p("f7.jpg"));
        assert_eq!(list.files()[MAX_RECENT - 1], p("f3.jpg"));
    }

    #[test]
    fn pushing_a_known_file_moves_it_to_the_front_without_a_duplicate() {
        let mut list = RecentFiles::default();
        list.push(p("a"));
        list.push(p("b"));
        list.push(p("c"));
        list.push(p("a"));
        assert_eq!(list.files(), [p("a"), p("c"), p("b")]);
    }

    #[test]
    fn remove_and_retain() {
        let mut list = RecentFiles::default();
        list.push(p("a.jpg"));
        list.push(p("b.avi"));
        list.push(p("c.png"));
        list.remove(&p("b.avi"));
        assert_eq!(list.files(), [p("c.png"), p("a.jpg")]);
        list.retain(|f| f != Path::new("a.jpg"));
        assert_eq!(list.files(), [p("c.png")]);
    }

    #[test]
    fn toml_round_trip() {
        let mut list = RecentFiles::default();
        list.push(p("/photos/one.jpg"));
        list.push(p("/photos/two.avi"));
        let (loaded, warning) = RecentFiles::from_toml_lossy(&list.to_toml());
        assert_eq!(warning, None);
        assert_eq!(loaded, list);
    }

    #[test]
    fn broken_or_long_files_are_handled() {
        let (list, warning) = RecentFiles::from_toml_lossy("files = [1, 2");
        assert!(list.is_empty() && warning.is_some());
        let many = format!(
            "files = [{}]",
            (0..20)
                .map(|i| format!("\"f{i}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let (list, warning) = RecentFiles::from_toml_lossy(&many);
        assert_eq!(warning, None);
        assert_eq!(list.files().len(), MAX_RECENT);
        assert_eq!(RecentFiles::from_toml_lossy("").0, RecentFiles::default());
    }

    #[test]
    fn kind_comes_from_the_extension() {
        assert_eq!(FileKind::of(Path::new("a.avi")), FileKind::Video);
        assert_eq!(FileKind::of(Path::new("a.AVI")), FileKind::Video);
        assert_eq!(FileKind::of(Path::new("a.jpg")), FileKind::Photo);
        assert_eq!(FileKind::of(Path::new("noext")), FileKind::Photo);
    }
}
