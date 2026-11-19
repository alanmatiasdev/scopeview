#[derive(Debug, thiserror::Error)]
pub enum CameraError {
    #[error("camera backend error: {0}")]
    Backend(String),
    #[error("camera permission was denied")]
    PermissionDenied,
    #[error("failed to decode frame: {0}")]
    Decode(String),
}

impl From<nokhwa::NokhwaError> for CameraError {
    fn from(error: nokhwa::NokhwaError) -> Self {
        Self::Backend(one_line(&error.to_string()))
    }
}

/// Backend errors can span several lines (device dumps); the UI shows them in a status bar.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::one_line;

    #[test]
    fn multiline_backend_errors_collapse_to_one_line() {
        assert_eq!(
            one_line("Could not set\nwidth   : 640\nfourcc : YUYV\n: busy"),
            "Could not set width : 640 fourcc : YUYV : busy"
        );
    }
}
