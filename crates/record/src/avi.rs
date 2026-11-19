//! A minimal AVI 1.0 writer for one MJPEG video stream (no audio).
//!
//! Layout: `RIFF AVI` > `LIST hdrl` (main header, stream header, stream format), `LIST movi`
//! with one `00dc` chunk per frame, then an `idx1` index. Sizes are patched when the file is
//! finished, so the output needs `Seek`. Offsets are 32 bit, so one file stays below 4 GiB.

use std::io::{self, Seek, SeekFrom, Write};

const HEADER_LEN: u64 = 224; // everything before the first frame chunk
const RIFF_SIZE_AT: u64 = 4;
const TOTAL_FRAMES_AT: u64 = 48;
const STREAM_LENGTH_AT: u64 = 140;
const MOVI_SIZE_AT: u64 = 216;
/// Offset of the `movi` fourcc; index offsets count from here.
const MOVI_TAG_AT: u64 = 220;

const AVIF_HASINDEX: u32 = 0x10;
const AVIIF_KEYFRAME: u32 = 0x10;

fn u32le(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

struct IndexEntry {
    offset: u32,
    size: u32,
}

pub struct AviWriter<W: Write + Seek> {
    out: W,
    width: u32,
    height: u32,
    index: Vec<IndexEntry>,
    /// Bytes of chunk data written after the header (including chunk headers and padding).
    movi_bytes: u64,
    max_frame: u32,
}

impl<W: Write + Seek> AviWriter<W> {
    /// Start a file for `width`x`height` video at a constant `fps` (frames per second, at least 1).
    pub fn new(mut out: W, width: u32, height: u32, fps: u32) -> io::Result<Self> {
        let fps = fps.max(1);
        let mut h: Vec<u8> = Vec::with_capacity(HEADER_LEN as usize);
        h.extend_from_slice(b"RIFF");
        h.extend_from_slice(&u32le(0)); // patched in finish
        h.extend_from_slice(b"AVI ");
        h.extend_from_slice(b"LIST");
        h.extend_from_slice(&u32le(192));
        h.extend_from_slice(b"hdrl");
        // Main header.
        h.extend_from_slice(b"avih");
        h.extend_from_slice(&u32le(56));
        h.extend_from_slice(&u32le(1_000_000 / fps)); // microseconds per frame
        h.extend_from_slice(&u32le(0)); // max bytes per second
        h.extend_from_slice(&u32le(0)); // padding granularity
        h.extend_from_slice(&u32le(AVIF_HASINDEX));
        h.extend_from_slice(&u32le(0)); // total frames, patched
        h.extend_from_slice(&u32le(0)); // initial frames
        h.extend_from_slice(&u32le(1)); // streams
        h.extend_from_slice(&u32le(0)); // suggested buffer size
        h.extend_from_slice(&u32le(width));
        h.extend_from_slice(&u32le(height));
        h.extend_from_slice(&[0; 16]); // reserved
                                       // Stream list.
        h.extend_from_slice(b"LIST");
        h.extend_from_slice(&u32le(116));
        h.extend_from_slice(b"strl");
        h.extend_from_slice(b"strh");
        h.extend_from_slice(&u32le(56));
        h.extend_from_slice(b"vids");
        h.extend_from_slice(b"MJPG");
        h.extend_from_slice(&u32le(0)); // flags
        h.extend_from_slice(&[0; 4]); // priority, language
        h.extend_from_slice(&u32le(0)); // initial frames
        h.extend_from_slice(&u32le(1)); // scale
        h.extend_from_slice(&u32le(fps)); // rate
        h.extend_from_slice(&u32le(0)); // start
        h.extend_from_slice(&u32le(0)); // length in frames, patched
        h.extend_from_slice(&u32le(0)); // suggested buffer size
        h.extend_from_slice(&u32le(u32::MAX)); // quality: default
        h.extend_from_slice(&u32le(0)); // sample size
        h.extend_from_slice(&0u16.to_le_bytes()); // frame rectangle: left, top
        h.extend_from_slice(&0u16.to_le_bytes());
        h.extend_from_slice(&(width.min(u32::from(u16::MAX)) as u16).to_le_bytes());
        h.extend_from_slice(&(height.min(u32::from(u16::MAX)) as u16).to_le_bytes());
        h.extend_from_slice(b"strf");
        h.extend_from_slice(&u32le(40));
        h.extend_from_slice(&u32le(40)); // BITMAPINFOHEADER size
        h.extend_from_slice(&u32le(width));
        h.extend_from_slice(&u32le(height));
        h.extend_from_slice(&1u16.to_le_bytes()); // planes
        h.extend_from_slice(&24u16.to_le_bytes()); // bit count
        h.extend_from_slice(b"MJPG");
        h.extend_from_slice(&u32le(width.saturating_mul(height).saturating_mul(3)));
        h.extend_from_slice(&[0; 16]); // pixels per meter, colors used and important
                                       // Start of the movie list.
        h.extend_from_slice(b"LIST");
        h.extend_from_slice(&u32le(0)); // patched in finish
        h.extend_from_slice(b"movi");
        debug_assert_eq!(h.len() as u64, HEADER_LEN);
        out.write_all(&h)?;
        Ok(Self {
            out,
            width,
            height,
            index: Vec::new(),
            movi_bytes: 0,
            max_frame: 0,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn frames(&self) -> u32 {
        self.index.len() as u32
    }

    /// Size the file has now, not counting the index written at the end.
    pub fn bytes_written(&self) -> u64 {
        HEADER_LEN + self.movi_bytes
    }

    /// Append one JPEG-encoded frame.
    pub fn write_frame(&mut self, jpeg: &[u8]) -> io::Result<()> {
        let size = u32::try_from(jpeg.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "frame too large"))?;
        let offset = u32::try_from(MOVI_TAG_AT.abs_diff(HEADER_LEN + self.movi_bytes))
            .map_err(|_| io::Error::other("file too large for AVI 1.0"))?;
        self.out.write_all(b"00dc")?;
        self.out.write_all(&u32le(size))?;
        self.out.write_all(jpeg)?;
        let pad = jpeg.len() % 2;
        if pad == 1 {
            self.out.write_all(&[0])?;
        }
        self.index.push(IndexEntry { offset, size });
        self.movi_bytes += 8 + u64::from(size) + pad as u64;
        self.max_frame = self.max_frame.max(size);
        Ok(())
    }

    /// Write the index and patch the sizes. Returns the underlying writer.
    pub fn finish(mut self) -> io::Result<W> {
        let movi_end = HEADER_LEN + self.movi_bytes;
        self.out.write_all(b"idx1")?;
        self.out.write_all(&u32le((self.index.len() * 16) as u32))?;
        for e in &self.index {
            self.out.write_all(b"00dc")?;
            self.out.write_all(&u32le(AVIIF_KEYFRAME))?;
            self.out.write_all(&u32le(e.offset))?;
            self.out.write_all(&u32le(e.size))?;
        }
        let file_end = movi_end + 8 + self.index.len() as u64 * 16;
        let riff = u32::try_from(file_end - 8)
            .map_err(|_| io::Error::other("file too large for AVI 1.0"))?;
        let patch = |out: &mut W, at: u64, value: u32| -> io::Result<()> {
            out.seek(SeekFrom::Start(at))?;
            out.write_all(&u32le(value))
        };
        patch(&mut self.out, RIFF_SIZE_AT, riff)?;
        patch(&mut self.out, TOTAL_FRAMES_AT, self.index.len() as u32)?;
        patch(&mut self.out, STREAM_LENGTH_AT, self.index.len() as u32)?;
        patch(&mut self.out, MOVI_SIZE_AT, (4 + self.movi_bytes) as u32)?;
        // Suggested buffer sizes: the largest frame.
        patch(&mut self.out, 60, self.max_frame)?;
        patch(&mut self.out, 144, self.max_frame)?;
        self.out.seek(SeekFrom::Start(file_end))?;
        self.out.flush()?;
        Ok(self.out)
    }
}

/// What the tests (and callers that want to check a file) can read back.
#[cfg(test)]
pub(crate) mod read {
    pub struct Parsed {
        pub width: u32,
        pub height: u32,
        pub fps: u32,
        pub total_frames: u32,
        pub frames: Vec<Vec<u8>>,
        pub index: Vec<(u32, u32)>,
    }

    fn u32_at(b: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
    }

    pub fn parse(bytes: &[u8]) -> Parsed {
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(u32_at(bytes, 4) as usize, bytes.len() - 8, "RIFF size");
        assert_eq!(&bytes[8..12], b"AVI ");
        assert_eq!(&bytes[12..16], b"LIST");
        assert_eq!(&bytes[20..24], b"hdrl");
        assert_eq!(&bytes[24..28], b"avih");
        let total_frames = u32_at(bytes, 48);
        let width = u32_at(bytes, 64);
        let height = u32_at(bytes, 68);
        assert_eq!(&bytes[100..104], b"strh");
        assert_eq!(&bytes[108..112], b"vids");
        assert_eq!(&bytes[112..116], b"MJPG");
        let fps = u32_at(bytes, 132);
        assert_eq!(u32_at(bytes, 140), total_frames, "stream length");
        assert_eq!(&bytes[164..168], b"strf");
        assert_eq!(&bytes[212..216], b"LIST");
        assert_eq!(&bytes[220..224], b"movi");
        let movi_size = u32_at(bytes, 216) as usize;
        let movi_end = 216 + 4 + movi_size;
        let mut frames = Vec::new();
        let mut at = 224;
        while at < movi_end {
            assert_eq!(&bytes[at..at + 4], b"00dc", "chunk at {at}");
            let size = u32_at(bytes, at + 4) as usize;
            frames.push(bytes[at + 8..at + 8 + size].to_vec());
            at += 8 + size + size % 2;
        }
        assert_eq!(at, movi_end, "chunks fill the movie list");
        assert_eq!(&bytes[movi_end..movi_end + 4], b"idx1");
        let idx_size = u32_at(bytes, movi_end + 4) as usize;
        assert_eq!(movi_end + 8 + idx_size, bytes.len(), "index ends the file");
        let index = (0..idx_size / 16)
            .map(|i| {
                let e = movi_end + 8 + i * 16;
                assert_eq!(&bytes[e..e + 4], b"00dc");
                (u32_at(bytes, e + 8), u32_at(bytes, e + 12))
            })
            .collect();
        Parsed {
            width,
            height,
            fps,
            total_frames,
            frames,
            index,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn write(frames: &[&[u8]], fps: u32) -> Vec<u8> {
        let mut w = AviWriter::new(Cursor::new(Vec::new()), 64, 48, fps).unwrap();
        for f in frames {
            w.write_frame(f).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    #[test]
    fn header_is_exactly_the_documented_length() {
        let w = AviWriter::new(Cursor::new(Vec::new()), 64, 48, 30).unwrap();
        assert_eq!(w.bytes_written(), HEADER_LEN);
        assert_eq!(w.frames(), 0);
        assert_eq!((w.width(), w.height()), (64, 48));
    }

    #[test]
    fn empty_file_is_still_valid() {
        let parsed = read::parse(&write(&[], 30));
        assert_eq!((parsed.width, parsed.height, parsed.fps), (64, 48, 30));
        assert_eq!(parsed.total_frames, 0);
        assert!(parsed.frames.is_empty());
    }

    #[test]
    fn frames_round_trip_with_padding_and_index() {
        let a: &[u8] = &[1, 2, 3]; // odd length: padded
        let b: &[u8] = &[4, 5, 6, 7];
        let parsed = read::parse(&write(&[a, b, a], 25));
        assert_eq!(parsed.total_frames, 3);
        assert_eq!(parsed.fps, 25);
        assert_eq!(parsed.frames, vec![a.to_vec(), b.to_vec(), a.to_vec()]);
        // Offsets count from the `movi` tag: first chunk at 4, then 8+3+1 further, and so on.
        assert_eq!(parsed.index, vec![(4, 3), (16, 4), (28, 3)]);
    }

    #[test]
    fn bytes_written_tracks_the_chunks() {
        let mut w = AviWriter::new(Cursor::new(Vec::new()), 8, 8, 30).unwrap();
        w.write_frame(&[0; 5]).unwrap(); // 8 header + 5 + 1 pad
        assert_eq!(w.bytes_written(), HEADER_LEN + 14);
        w.write_frame(&[0; 6]).unwrap();
        assert_eq!(w.bytes_written(), HEADER_LEN + 14 + 14);
    }

    #[test]
    fn rate_is_at_least_one() {
        let parsed = read::parse(&write(&[], 0));
        assert_eq!(parsed.fps, 1);
    }
}
