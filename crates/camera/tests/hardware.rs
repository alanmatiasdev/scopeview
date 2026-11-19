//! Tests that need a real camera. Run with:
//! `cargo test -p scopeview-camera --test hardware -- --ignored --test-threads=1`

use std::time::{Duration, Instant};

use scopeview_camera::{
    list_devices, list_formats, ControlCommand, ControlInfo, DeviceInfo, FormatInfo,
    PixelPreference, Stream, StreamState,
};

fn first_device() -> DeviceInfo {
    list_devices()
        .expect("enumeration failed")
        .into_iter()
        .next()
        .expect("no camera connected")
}

fn wait_for_frames(stream: &Stream, count: usize, timeout: Duration) -> (usize, StreamState) {
    let started = Instant::now();
    let (mut seen, mut last) = (0, StreamState::Starting);
    let mut last_sequence = 0;
    while seen < count && started.elapsed() < timeout {
        let poll = stream.poll();
        last = poll.state;
        if let Some(frame) = poll.frame {
            assert_eq!(
                frame.rgba.len(),
                (frame.width * frame.height * 4) as usize,
                "frame buffer must be tightly packed RGBA"
            );
            assert!(frame.sequence > last_sequence, "sequence must increase");
            last_sequence = frame.sequence;
            seen += 1;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    (seen, last)
}

#[test]
#[ignore = "needs a camera"]
fn stream_delivers_frames() {
    let stream = Stream::start(
        first_device(),
        None,
        Vec::new(),
        PixelPreference::Mjpeg,
        || {},
    );
    let (seen, state) = wait_for_frames(&stream, 10, Duration::from_secs(10));
    assert!(seen >= 10, "only {seen} frames, state {state:?}");
    assert!(matches!(state, StreamState::Running(_)));
}

#[test]
#[ignore = "needs a camera"]
fn switching_format_releases_and_reopens_the_device() {
    let device = first_device();
    let formats = list_formats(&device).expect("formats");
    let pick = |w: u32| -> FormatInfo {
        *formats
            .iter()
            .find(|f| f.width == w)
            .unwrap_or_else(|| panic!("no {w}-wide format in {formats:?}"))
    };
    for width in [640, 1280, 640] {
        let format = pick(width);
        let stream = Stream::start(
            device.clone(),
            Some(format),
            Vec::new(),
            PixelPreference::Mjpeg,
            || {},
        );
        let (seen, state) = wait_for_frames(&stream, 5, Duration::from_secs(10));
        assert!(seen >= 5, "{format}: only {seen} frames, state {state:?}");
        // Dropping joins the capture thread; the next iteration must be able to reopen.
    }
}

/// Regression: a device that lacks the usual 720p MJPEG format (USB microscopes often offer
/// only 640x480 YUYV) must still open with the automatic format.
#[test]
#[ignore = "needs a camera"]
fn every_listed_device_streams_with_automatic_format() {
    let devices = list_devices().expect("enumeration failed");
    assert!(!devices.is_empty(), "no camera connected");
    for device in devices {
        let stream = Stream::start(
            device.clone(),
            None,
            Vec::new(),
            PixelPreference::Mjpeg,
            || {},
        );
        let (seen, state) = wait_for_frames(&stream, 5, Duration::from_secs(10));
        assert!(
            seen >= 5,
            "{}: only {seen} frames, state {state:?}",
            device.name
        );
    }
}

fn wait_for_controls(stream: &Stream, timeout: Duration) -> Option<Vec<ControlInfo>> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if let Some(controls) = stream.poll().controls {
            return Some(controls);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

#[test]
#[ignore = "needs a camera"]
fn controls_are_listed_and_a_written_value_reads_back() {
    // Some cameras have no controls at all, so look at every connected one.
    let mut tested = false;
    for device in list_devices().expect("enumeration failed") {
        let stream = Stream::start(
            device.clone(),
            None,
            Vec::new(),
            PixelPreference::Mjpeg,
            || {},
        );
        let controls = wait_for_controls(&stream, Duration::from_secs(10)).expect("no controls");
        println!("{}: {} controls", device.name, controls.len());
        let Some(target) = controls
            .iter()
            .find(|c| c.active && c.auto != Some(true) && c.max - c.min >= 2 * c.step)
        else {
            continue;
        };
        tested = true;
        let original = target.value;
        let wanted = if original == target.min {
            target.min + target.step
        } else {
            target.min
        };
        stream.control(ControlCommand::Set(target.kind, wanted));
        let after = wait_for_controls(&stream, Duration::from_secs(5)).expect("no update");
        let now = after.iter().find(|c| c.kind == target.kind).unwrap();
        assert_eq!(now.value, wanted, "{:?} did not change", target.kind);

        stream.control(ControlCommand::Set(target.kind, original));
        let restored = wait_for_controls(&stream, Duration::from_secs(5)).expect("no update");
        assert_eq!(
            restored
                .iter()
                .find(|c| c.kind == target.kind)
                .unwrap()
                .value,
            original
        );
    }
    assert!(tested, "no connected camera has an adjustable control");
}
