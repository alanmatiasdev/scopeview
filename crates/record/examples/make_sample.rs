//! Writes a short sample recording, for checking the AVI output with external tools.
//! Usage: cargo run -p scopeview-record --example make_sample -- <folder>

use std::path::PathBuf;
use std::time::Duration;

use image::{Rgba, RgbaImage};
use scopeview_core::overlay::Primitive;
use scopeview_core::Point;
use scopeview_record::{RecordConfig, Recorder};

fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).expect("folder"));
    let mut config = RecordConfig::new(dir, "sample", (320, 240), 30);
    config.max_duration = None;
    let recorder = Recorder::start(config).expect("start");
    let begin = std::time::Instant::now();
    let mut n = 0u32;
    while begin.elapsed() < Duration::from_secs(2) {
        let shade = (n * 4 % 256) as u8;
        let frame = RgbaImage::from_pixel(320, 240, Rgba([shade, 80, 255 - shade, 255]));
        let x = f64::from(n % 300);
        let overlay = vec![Primitive::Line {
            a: Point::new(x, 0.0),
            b: Point::new(x, 239.0),
            color: [255, 255, 255, 255],
            width: 3.0,
        }];
        recorder.submit(frame, overlay);
        n += 1;
        std::thread::sleep(Duration::from_millis(33));
    }
    let status = recorder.stop();
    println!("{status:#?}");
}
