use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::{CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType};
use nokhwa::{Buffer, Camera};

use crate::controls::Controller;
use crate::device::{choose_default_format, list_devices, resolve_device, PixelPreference};
use crate::mailbox::Mailbox;
use crate::{CameraError, ControlCommand, ControlInfo, ControlValue, DeviceInfo, FormatInfo};

const RECONNECT_INTERVAL: Duration = Duration::from_millis(1000);
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// One decoded camera frame, tightly packed RGBA8.
#[derive(Debug, Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// Increases by one per captured frame, so consumers can tell frames apart.
    pub sequence: u64,
    /// When the driver captured the frame, as time since the Unix epoch (Linux only for now).
    pub captured_at: Option<Duration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamState {
    /// Opening the device.
    Starting,
    /// Frames are flowing in this format.
    Running(FormatInfo),
    /// The device went away or failed; retrying until it comes back.
    Reconnecting(String),
}

/// What the UI gets on each poll: the newest frame since the last poll (if any) and the state.
#[derive(Debug, Clone)]
pub struct Poll {
    pub frame: Option<Frame>,
    pub state: StreamState,
    /// The device's controls, sent when they were read or changed since the last poll.
    pub controls: Option<Vec<ControlInfo>>,
}

struct Shared {
    latest: Option<Frame>,
    state: StreamState,
    controls: Option<Vec<ControlInfo>>,
}

/// A running capture. Dropping it stops the capture thread.
///
/// Two threads work behind it: one only pulls raw frames from the driver, the other decodes the
/// newest one. Only the newest frame is kept at every step, so neither a slow decode nor a slow
/// UI builds up latency.
pub struct Stream {
    shared: Arc<Mutex<Shared>>,
    stop: Arc<AtomicBool>,
    commands: Sender<ControlCommand>,
    handle: Option<JoinHandle<()>>,
}

impl Stream {
    /// Start capturing from `device`. With `format: None` a format is chosen from the ones the
    /// device offers, close to 1280x720 @ 30 fps. `on_update` is called from the capture thread
    /// whenever a new frame or state is available (use it to wake the UI). `preference` only
    /// matters for the automatic choice; an explicit `format` is used as given.
    /// `controls` are applied each time the device opens, including after a reconnect.
    pub fn start(
        device: DeviceInfo,
        format: Option<FormatInfo>,
        controls: Vec<ControlValue>,
        preference: PixelPreference,
        on_update: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        let shared = Arc::new(Mutex::new(Shared {
            latest: None,
            state: StreamState::Starting,
            controls: None,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let (commands, receiver) = mpsc::channel();
        let handle = {
            let shared = Arc::clone(&shared);
            let stop = Arc::clone(&stop);
            thread::Builder::new()
                .name("camera-capture".into())
                .spawn(move || {
                    let mut controller = Controller::new(controls);
                    let mailbox = Mailbox::new();
                    thread::scope(|scope| {
                        thread::Builder::new()
                            .name("camera-decode".into())
                            .spawn_scoped(scope, || decode_loop(&mailbox, &shared, &on_update))
                            .expect("failed to spawn the decode thread");
                        let link = Link {
                            shared: &shared,
                            stop: &stop,
                            commands: &receiver,
                            on_update: &on_update,
                            mailbox: &mailbox,
                        };
                        capture_loop(
                            device,
                            FormatChoice { format, preference },
                            &link,
                            &mut controller,
                        );
                        mailbox.close();
                    });
                })
                .expect("failed to spawn the camera thread")
        };
        Self {
            shared,
            stop,
            commands,
            handle: Some(handle),
        }
    }

    /// Ask the capture thread to change a camera control.
    pub fn control(&self, command: ControlCommand) {
        let _ = self.commands.send(command);
    }

    pub fn poll(&self) -> Poll {
        let mut shared = lock(&self.shared);
        Poll {
            frame: shared.latest.take(),
            state: shared.state.clone(),
            controls: shared.controls.take(),
        }
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn lock(shared: &Mutex<Shared>) -> std::sync::MutexGuard<'_, Shared> {
    shared
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// What the capture thread talks to: the state shared with the UI, the stop flag, control
/// commands, the wake-up callback and the hand-off to the decode thread.
struct Link<'a> {
    shared: &'a Mutex<Shared>,
    stop: &'a AtomicBool,
    commands: &'a Receiver<ControlCommand>,
    on_update: &'a dyn Fn(),
    mailbox: &'a Mailbox<Buffer>,
}

fn capture_loop(
    mut device: DeviceInfo,
    choice: FormatChoice,
    link: &Link,
    controller: &mut Controller,
) {
    let Link {
        shared,
        stop,
        on_update,
        ..
    } = *link;
    while !stop.load(Ordering::Relaxed) {
        let reason = match open(&device, choice) {
            Ok(mut camera) => {
                let running = FormatInfo::from_backend(camera.camera_format());
                let controls = controller.attach(&mut camera);
                lock(shared).controls = Some(controls);
                set_state(shared, StreamState::Running(running), on_update);
                run_frames(&mut camera, link, controller)
            }
            Err(error) => error.to_string(),
        };
        if stop.load(Ordering::Relaxed) {
            break;
        }
        set_state(shared, StreamState::Reconnecting(reason), on_update);
        wait_interruptibly(stop, RECONNECT_INTERVAL);
        if let Ok(devices) = list_devices() {
            if let Some(found) = resolve_device(&devices, &device.name, device.id) {
                device = found;
            }
        }
    }
}

/// What the caller asked for: an exact format, or the automatic choice with a preference.
#[derive(Clone, Copy)]
struct FormatChoice {
    format: Option<FormatInfo>,
    preference: PixelPreference,
}

fn open(device: &DeviceInfo, choice: FormatChoice) -> Result<Camera, CameraError> {
    let FormatChoice { format, preference } = choice;
    let index = CameraIndex::Index(device.id.0);
    let mut camera = match format {
        Some(format) => Camera::new(
            index,
            RequestedFormat::new::<RgbAFormat>(RequestedFormatType::Exact(format.to_backend())),
        )?,
        None => {
            // Ask the device what it offers and pick from that list. Requesting a fixed format
            // fails on devices that do not have it (e.g. microscopes with only 640x480 YUYV).
            let mut camera = Camera::new(
                index,
                RequestedFormat::new::<RgbAFormat>(RequestedFormatType::None),
            )?;
            let offered: Vec<FormatInfo> = camera
                .compatible_camera_formats()?
                .into_iter()
                .map(FormatInfo::from_backend)
                .collect();
            if let Some(chosen) = choose_default_format(&offered, preference) {
                camera.set_camera_requset(RequestedFormat::new::<RgbAFormat>(
                    RequestedFormatType::Exact(chosen.to_backend()),
                ))?;
            }
            camera
        }
    };
    camera.open_stream()?;
    Ok(camera)
}

/// Returns why capture ended (always an error or a stop request).
fn run_frames(camera: &mut Camera, link: &Link, controller: &mut Controller) -> String {
    let Link {
        shared,
        stop,
        commands,
        on_update,
        mailbox,
    } = *link;
    while !stop.load(Ordering::Relaxed) {
        let pending: Vec<ControlCommand> = commands.try_iter().collect();
        if !pending.is_empty() {
            let controls = controller.handle(camera, &pending);
            lock(shared).controls = Some(controls);
            on_update();
        }
        match camera.frame() {
            Ok(buffer) => mailbox.put(buffer),
            Err(error) => return error.to_string(),
        }
    }
    "stopped".into()
}

/// Decode the newest raw frame each time one arrives, until the mailbox closes.
fn decode_loop(mailbox: &Mailbox<Buffer>, shared: &Mutex<Shared>, on_update: &dyn Fn()) {
    let mut sequence = 0u64;
    while let Some(buffer) = mailbox.take() {
        let captured_at = buffer.capture_timestamp();
        // A corrupt frame (common right after start or on a noisy cable) is skipped.
        if let Ok((width, height, rgba)) = decode(&buffer) {
            sequence += 1;
            lock(shared).latest = Some(Frame {
                width,
                height,
                rgba,
                sequence,
                captured_at,
            });
            on_update();
        }
    }
}

fn decode(buffer: &Buffer) -> Result<(u32, u32, Vec<u8>), CameraError> {
    let image = match buffer.source_frame_format() {
        FrameFormat::MJPEG => {
            image::load_from_memory_with_format(buffer.buffer(), image::ImageFormat::Jpeg)
                .map_err(|e| CameraError::Decode(e.to_string()))?
                .into_rgba8()
        }
        _ => buffer
            .decode_image::<RgbAFormat>()
            .map_err(|e| CameraError::Decode(e.to_string()))?,
    };
    Ok((image.width(), image.height(), image.into_raw()))
}

fn set_state(shared: &Mutex<Shared>, state: StreamState, on_update: &dyn Fn()) {
    lock(shared).state = state;
    on_update();
}

fn wait_interruptibly(stop: &AtomicBool, total: Duration) {
    let mut waited = Duration::ZERO;
    while waited < total && !stop.load(Ordering::Relaxed) {
        thread::sleep(STOP_POLL_INTERVAL);
        waited += STOP_POLL_INTERVAL;
    }
}
