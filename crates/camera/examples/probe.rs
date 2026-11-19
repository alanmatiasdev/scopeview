//! Backend spike: list devices and formats, then capture frames and report the real frame rate.
//!
//! cargo run -p scopeview-camera --example probe -- [device-index] [seconds]

use std::time::{Duration, Instant};

use scopeview_camera::{
    init_permissions, list_devices, list_formats, PixelPreference, Stream, StreamState,
};

fn main() {
    let mut args = std::env::args().skip(1);
    let wanted: Option<u32> = args.next().and_then(|a| a.parse().ok());
    let seconds: u64 = args.next().and_then(|a| a.parse().ok()).unwrap_or(5);

    if let Err(error) = init_permissions() {
        eprintln!("permission: {error}");
        std::process::exit(2);
    }
    let devices = match list_devices() {
        Ok(devices) => devices,
        Err(error) => {
            eprintln!("enumeration failed: {error}");
            std::process::exit(1);
        }
    };
    if devices.is_empty() {
        println!("no cameras found");
        return;
    }
    for device in &devices {
        println!("[{}] {}", device.id.0, device.name);
        match list_formats(device) {
            Ok(formats) => formats.iter().for_each(|f| println!("    {f}")),
            Err(error) => println!("    formats unavailable: {error}"),
        }
    }

    let device = devices
        .iter()
        .find(|d| Some(d.id.0) == wanted)
        .unwrap_or(&devices[0])
        .clone();
    println!(
        "capturing from [{}] {} for {seconds}s",
        device.id.0, device.name
    );

    let stream = Stream::start(device, None, Vec::new(), PixelPreference::Mjpeg, || {});
    let started = Instant::now();
    let (mut frames, mut last_state) = (0u64, None);
    let mut size = (0, 0);
    let mut ages: Vec<f64> = Vec::new();
    while started.elapsed() < Duration::from_secs(seconds) {
        let poll = stream.poll();
        if last_state.as_ref() != Some(&poll.state) {
            println!("state: {:?}", poll.state);
            last_state = Some(poll.state.clone());
        }
        if let Some(frame) = poll.frame {
            frames += 1;
            size = (frame.width, frame.height);
            if let (Some(at), Ok(now)) = (
                frame.captured_at,
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH),
            ) {
                ages.push(now.saturating_sub(at).as_secs_f64() * 1000.0);
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let running = matches!(last_state, Some(StreamState::Running(_)));
    let fps = frames as f64 / started.elapsed().as_secs_f64();
    println!(
        "{frames} frames, {}x{}, ~{fps:.1} fps (polled)",
        size.0, size.1
    );
    if !ages.is_empty() {
        ages.sort_by(|a, b| a.total_cmp(b));
        let mean = ages.iter().sum::<f64>() / ages.len() as f64;
        println!(
            "age at poll (capture -> visible): mean {mean:.0} ms, median {:.0} ms, max {:.0} ms",
            ages[ages.len() / 2],
            ages[ages.len() - 1]
        );
    }
    if !running || frames == 0 {
        std::process::exit(1);
    }
}
