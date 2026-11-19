//! The recording thread: takes frames from the UI, burns in the overlay, encodes JPEG and writes
//! AVI files. Pause, resume, a time limit and splitting by file size are handled here.

use std::fs::{self, File, OpenOptions};
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, RgbaImage};
use scopeview_core::overlay::Primitive;
use scopeview_imaging::{render_overlay, JPEG_QUALITY};

use crate::avi::AviWriter;
use crate::naming::{unique_stamp, video_path};
use crate::pacing::pace;

/// Start a new file once the current one would grow past this many bytes.
pub const SPLIT_BYTES: u64 = 0x8300_0000;

/// Frames waiting for the encoder. More than this and new frames are dropped.
const QUEUE: usize = 4;

/// How often the thread wakes without any message, to check the time limit.
const TICK: Duration = Duration::from_millis(50);

#[derive(Debug, thiserror::Error)]
pub enum RecordError {
    #[error("could not create folder {0}: {1}")]
    Folder(PathBuf, std::io::Error),
    #[error("could not create {0}: {1}")]
    File(PathBuf, std::io::Error),
}

#[derive(Debug, Clone)]
pub struct RecordConfig {
    pub dir: PathBuf,
    /// Time stamp used in the file names, such as `20260101-120000`.
    pub stamp: String,
    pub width: u32,
    pub height: u32,
    /// Constant frame rate of the video.
    pub fps: u32,
    /// Stop on its own after this much recording time (pauses do not count). `None`: no limit.
    pub max_duration: Option<Duration>,
    /// Start a new file past this size. Normally [`SPLIT_BYTES`].
    pub split_bytes: u64,
}

impl RecordConfig {
    pub fn new(dir: PathBuf, stamp: &str, size: (u32, u32), fps: u32) -> Self {
        Self {
            dir,
            stamp: stamp.to_string(),
            width: size.0,
            height: size.1,
            fps,
            max_duration: None,
            split_bytes: SPLIT_BYTES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    User,
    TimeLimit,
    Error,
}

/// What the UI can read while recording and after it ended.
#[derive(Debug, Clone, Default)]
pub struct Status {
    /// Recording time so far, pauses excluded.
    pub active: Duration,
    /// Frames in the video (repeated frames included).
    pub frames: u64,
    /// Frames lost because the encoder could not keep up, or because their size changed.
    pub dropped: u64,
    pub paused: bool,
    /// Files written, in order. The last one is still open until `finished`.
    pub files: Vec<PathBuf>,
    pub finished: bool,
    pub reason: Option<StopReason>,
    pub error: Option<String>,
}

enum Command {
    Frame(Box<(RgbaImage, Vec<Primitive>)>),
    Pause,
    Resume,
    Stop,
}

pub struct Recorder {
    tx: SyncSender<Command>,
    shared: Arc<Mutex<Status>>,
    handle: Option<JoinHandle<()>>,
}

impl Recorder {
    /// Create the folder and the first file, then start the thread.
    pub fn start(config: RecordConfig) -> Result<Self, RecordError> {
        fs::create_dir_all(&config.dir).map_err(|e| RecordError::Folder(config.dir.clone(), e))?;
        let stamp = unique_stamp(&config.dir, &config.stamp);
        let (path, writer) = open_part(&config, &stamp, 1)?;
        let shared = Arc::new(Mutex::new(Status {
            files: vec![path],
            ..Status::default()
        }));
        let (tx, rx) = mpsc::sync_channel(QUEUE);
        let worker = Worker {
            config,
            stamp,
            writer: Some(writer),
            part: 1,
            written: 0,
            shared: Arc::clone(&shared),
        };
        let handle = thread::Builder::new()
            .name("scopeview-recorder".into())
            .spawn(move || worker.run(rx))
            .map_err(|e| {
                RecordError::File(PathBuf::from("recorder thread"), std::io::Error::other(e))
            })?;
        Ok(Self {
            tx,
            shared,
            handle: Some(handle),
        })
    }

    /// Offer a frame with the overlay to burn into it. False when it was not taken: the encoder
    /// is behind, or the recording already ended.
    pub fn submit(&self, image: RgbaImage, primitives: Vec<Primitive>) -> bool {
        match self
            .tx
            .try_send(Command::Frame(Box::new((image, primitives))))
        {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                if let Ok(mut s) = self.shared.lock() {
                    s.dropped += 1;
                }
                false
            }
            Err(TrySendError::Disconnected(_)) => false,
        }
    }

    pub fn pause(&self) {
        let _ = self.tx.send(Command::Pause);
    }

    pub fn resume(&self) {
        let _ = self.tx.send(Command::Resume);
    }

    pub fn status(&self) -> Status {
        self.shared.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Finish the files and return the final status.
    pub fn stop(mut self) -> Status {
        self.shutdown()
    }

    fn shutdown(&mut self) -> Status {
        let _ = self.tx.send(Command::Stop);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        self.status()
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        if self.handle.is_some() {
            self.shutdown();
        }
    }
}

type Writer = AviWriter<BufWriter<File>>;

fn open_part(
    config: &RecordConfig,
    stamp: &str,
    part: u32,
) -> Result<(PathBuf, Writer), RecordError> {
    let path = video_path(&config.dir, stamp, part);
    // `create_new`: never overwrite an existing recording.
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| RecordError::File(path.clone(), e))?;
    let writer = AviWriter::new(
        BufWriter::new(file),
        config.width,
        config.height,
        config.fps,
    )
    .map_err(|e| RecordError::File(path.clone(), e))?;
    Ok((path, writer))
}

/// Recording time that stops while paused.
struct Clock {
    accumulated: Duration,
    running_since: Option<Instant>,
}

impl Clock {
    fn started() -> Self {
        Self {
            accumulated: Duration::ZERO,
            running_since: Some(Instant::now()),
        }
    }

    fn elapsed(&self) -> Duration {
        self.accumulated + self.running_since.map_or(Duration::ZERO, |t| t.elapsed())
    }

    fn pause(&mut self) {
        if let Some(t) = self.running_since.take() {
            self.accumulated += t.elapsed();
        }
    }

    fn resume(&mut self) {
        if self.running_since.is_none() {
            self.running_since = Some(Instant::now());
        }
    }

    fn paused(&self) -> bool {
        self.running_since.is_none()
    }
}

struct Worker {
    config: RecordConfig,
    stamp: String,
    writer: Option<Writer>,
    part: u32,
    /// Frames on the video timeline (see `pacing`).
    written: u64,
    shared: Arc<Mutex<Status>>,
}

impl Worker {
    fn run(mut self, rx: Receiver<Command>) {
        let mut clock = Clock::started();
        let reason = loop {
            match rx.recv_timeout(TICK) {
                Ok(Command::Frame(job)) => {
                    let (image, primitives) = *job;
                    if clock.paused() {
                        continue;
                    }
                    if let Err(error) = self.handle_frame(image, &primitives, clock.elapsed()) {
                        self.set(|s| s.error = Some(error));
                        break StopReason::Error;
                    }
                }
                Ok(Command::Pause) => clock.pause(),
                Ok(Command::Resume) => clock.resume(),
                Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => break StopReason::User,
                Err(RecvTimeoutError::Timeout) => {}
            }
            let active = clock.elapsed();
            let paused = clock.paused();
            let frames = self.written;
            self.set(|s| {
                s.active = active;
                s.paused = paused;
                s.frames = frames;
            });
            if self.config.max_duration.is_some_and(|max| active >= max) {
                break StopReason::TimeLimit;
            }
        };
        let active = clock.elapsed();
        let closed = self.close_writer();
        let frames = self.written;
        self.set(|s| {
            s.active = active;
            s.frames = frames;
            s.finished = true;
            s.reason = Some(reason);
            if let Err(e) = closed {
                s.error.get_or_insert(e);
            }
        });
    }

    fn set(&self, f: impl FnOnce(&mut Status)) {
        if let Ok(mut s) = self.shared.lock() {
            f(&mut s);
        }
    }

    fn handle_frame(
        &mut self,
        mut image: RgbaImage,
        primitives: &[Primitive],
        active: Duration,
    ) -> Result<(), String> {
        if image.dimensions() != (self.config.width, self.config.height) {
            self.set(|s| s.dropped += 1);
            return Ok(());
        }
        let plan = pace(self.written, active, self.config.fps);
        if plan.write == 0 {
            return Ok(());
        }
        render_overlay(&mut image, primitives);
        let jpeg = encode_jpeg(&image)?;
        for _ in 0..plan.write {
            self.write_one(&jpeg)?;
        }
        self.written = plan.written;
        Ok(())
    }

    fn write_one(&mut self, jpeg: &[u8]) -> Result<(), String> {
        let full = self.writer.as_ref().is_some_and(|w| {
            w.frames() > 0 && w.bytes_written() + jpeg.len() as u64 + 8 > self.config.split_bytes
        });
        if full {
            self.close_writer()?;
            self.part += 1;
            let (path, writer) =
                open_part(&self.config, &self.stamp, self.part).map_err(|e| e.to_string())?;
            self.writer = Some(writer);
            self.set(|s| s.files.push(path));
        }
        match &mut self.writer {
            Some(w) => w.write_frame(jpeg).map_err(|e| e.to_string()),
            None => Err("no open file".into()),
        }
    }

    fn close_writer(&mut self) -> Result<(), String> {
        match self.writer.take() {
            Some(w) => w.finish().map(|_| ()).map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }
}

fn encode_jpeg(image: &RgbaImage) -> Result<Vec<u8>, String> {
    let rgb = DynamicImage::ImageRgba8(image.clone()).to_rgb8();
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode_image(&rgb)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::avi::read;
    use image::Rgba;
    use scopeview_core::Point;
    use std::path::Path;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scopeview-rec-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn frame() -> RgbaImage {
        RgbaImage::from_pixel(64, 48, Rgba([0, 0, 255, 255]))
    }

    fn red_line() -> Vec<Primitive> {
        vec![Primitive::Line {
            a: Point::new(0.0, 10.0),
            b: Point::new(63.0, 10.0),
            color: [255, 0, 0, 255],
            width: 4.0,
        }]
    }

    fn config(dir: &Path, name: &str) -> RecordConfig {
        RecordConfig::new(dir.to_path_buf(), name, (64, 48), 20)
    }

    fn feed(rec: &Recorder, count: usize, gap_ms: u64) {
        for _ in 0..count {
            rec.submit(frame(), red_line());
            thread::sleep(Duration::from_millis(gap_ms));
        }
    }

    #[test]
    fn records_a_playable_file_with_the_overlay_burned_in() {
        let dir = temp_dir("basic");
        let rec = Recorder::start(config(&dir, "t1")).unwrap();
        feed(&rec, 6, 30);
        let status = rec.stop();
        assert!(status.finished && status.error.is_none(), "{status:?}");
        assert_eq!(status.reason, Some(StopReason::User));
        assert_eq!(status.files.len(), 1);
        assert_eq!(status.files[0].file_name().unwrap(), "scopeview-t1.avi");

        let parsed = read::parse(&fs::read(&status.files[0]).unwrap());
        assert_eq!((parsed.width, parsed.height, parsed.fps), (64, 48, 20));
        assert_eq!(parsed.total_frames as u64, status.frames);
        assert!(parsed.total_frames >= 3, "got {}", parsed.total_frames);
        let first = image::load_from_memory(&parsed.frames[0])
            .unwrap()
            .to_rgb8();
        assert_eq!(first.dimensions(), (64, 48));
        let on = first.get_pixel(30, 10).0;
        let off = first.get_pixel(30, 40).0;
        assert!(on[0] > 200 && on[2] < 90, "line pixel was {on:?}");
        assert!(off[2] > 200 && off[0] < 90, "background pixel was {off:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn video_length_follows_the_clock_not_the_frame_arrivals() {
        let dir = temp_dir("pace");
        let rec = Recorder::start(config(&dir, "t2")).unwrap();
        // Only a few frames, spread over ~0.6 s: gaps are filled by repeating.
        feed(&rec, 3, 200);
        let status = rec.stop();
        let parsed = read::parse(&fs::read(&status.files[0]).unwrap());
        // 20 fps over ~0.6 s is about 12 frames; accept a wide band for a loaded machine.
        assert!(
            (8..=16).contains(&parsed.total_frames),
            "got {}",
            parsed.total_frames
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_time_limit_stops_the_recording() {
        let dir = temp_dir("limit");
        let mut cfg = config(&dir, "t3");
        cfg.max_duration = Some(Duration::from_millis(200));
        let rec = Recorder::start(cfg).unwrap();
        feed(&rec, 20, 30);
        let status = rec.status();
        assert!(status.finished, "should have stopped itself: {status:?}");
        assert_eq!(status.reason, Some(StopReason::TimeLimit));
        assert!(!rec.submit(frame(), vec![]) || rec.status().finished);
        let status = rec.stop();
        let parsed = read::parse(&fs::read(&status.files[0]).unwrap());
        // About 0.2 s at 20 fps.
        assert!(
            (2..=8).contains(&parsed.total_frames),
            "got {}",
            parsed.total_frames
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn pausing_stops_the_clock_and_ignores_frames() {
        let dir = temp_dir("pause");
        let rec = Recorder::start(config(&dir, "t4")).unwrap();
        feed(&rec, 4, 30);
        rec.pause();
        thread::sleep(Duration::from_millis(100));
        let before = rec.status();
        feed(&rec, 6, 100); // 0.6 s paused
        let during = rec.status();
        assert!(during.paused);
        assert!(
            during.active.saturating_sub(before.active) < Duration::from_millis(150),
            "clock ran while paused: {before:?} -> {during:?}"
        );
        rec.resume();
        feed(&rec, 3, 30);
        let status = rec.stop();
        let parsed = read::parse(&fs::read(&status.files[0]).unwrap());
        // About 0.3 s of recording at 20 fps, nowhere near the 1 s of wall clock.
        assert!(parsed.total_frames < 14, "got {}", parsed.total_frames);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_full_file_continues_in_a_new_one() {
        let dir = temp_dir("split");
        let mut cfg = config(&dir, "t5");
        // Room for the header and about two frames.
        cfg.split_bytes = 224 + 2 * 1500;
        let rec = Recorder::start(cfg).unwrap();
        feed(&rec, 12, 50);
        let status = rec.stop();
        assert!(status.files.len() >= 2, "files: {:?}", status.files);
        assert_eq!(
            status.files[1].file_name().unwrap(),
            "scopeview-t5-part2.avi"
        );
        let total: u32 = status
            .files
            .iter()
            .map(|p| read::parse(&fs::read(p).unwrap()).total_frames)
            .sum();
        assert_eq!(total as u64, status.frames);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn frames_of_another_size_are_dropped() {
        let dir = temp_dir("size");
        let rec = Recorder::start(config(&dir, "t6")).unwrap();
        rec.submit(RgbaImage::new(10, 10), vec![]);
        feed(&rec, 3, 30);
        let status = rec.stop();
        assert_eq!(status.dropped, 1);
        assert!(status.frames >= 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_existing_recording_is_never_overwritten() {
        let dir = temp_dir("clash");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("scopeview-t7.avi"), b"keep me").unwrap();
        let rec = Recorder::start(config(&dir, "t7")).unwrap();
        let status = rec.stop();
        assert_eq!(status.files[0].file_name().unwrap(), "scopeview-t7-2.avi");
        assert_eq!(fs::read(dir.join("scopeview-t7.avi")).unwrap(), b"keep me");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unwritable_folder_is_an_error() {
        let dir = temp_dir("blocked");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("file");
        fs::write(&file, b"x").unwrap();
        let err = Recorder::start(config(&file.join("sub"), "t8"));
        assert!(matches!(err, Err(RecordError::Folder(..))));
        let _ = fs::remove_dir_all(&dir);
    }
}
