//! Opening a saved file in the system's default program.

use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

/// Ask the desktop to open `path`. Returns once the helper program started, not when it exits.
pub fn open_path(path: &Path) -> io::Result<()> {
    // No shell is involved on any platform, so odd characters in a file name do nothing special.
    #[cfg(target_os = "linux")]
    let mut command = Command::new("xdg-open");
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = Command::new("explorer");
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    let mut command = Command::new("xdg-open");

    let mut child = command
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    // Reap the helper in the background so it does not linger as a zombie.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
