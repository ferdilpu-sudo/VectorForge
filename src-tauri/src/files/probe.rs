use std::fs::{self, File};
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use image::ImageDecoder;
use image::codecs::bmp::BmpDecoder;
use image::codecs::jpeg::JpegDecoder;
use image::codecs::png::PngDecoder;
use image::codecs::webp::WebPDecoder;
use image::metadata::Orientation;

use crate::models::{AppError, ErrorCode, SourceFormat};

const MAX_PIXELS: u64 = 30_000_000;
const SAMPLE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct SourceProbe {
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub format: SourceFormat,
    pub has_alpha: bool,
    pub orientation: Orientation,
    pub fingerprint: String,
}

pub fn probe_source(path: &Path) -> Result<SourceProbe, AppError> {
    match catch_unwind(AssertUnwindSafe(|| probe_source_inner(path))) {
        Ok(result) => result,
        Err(_) => Err(AppError::new(
            ErrorCode::ImageCorrupt,
            "Gambar rusak atau gagal dibaca.",
        )),
    }
}

fn probe_source_inner(path: &Path) -> Result<SourceProbe, AppError> {
    let expected = format_from_extension(path)?;
    let detected = detect_format(path)?;
    if expected != detected {
        return Err(AppError::new(
            ErrorCode::ImageCorrupt,
            "Ekstensi file tidak sesuai dengan header gambar.",
        ));
    }

    let metadata = fs::metadata(path).map_err(file_io_error)?;
    if !metadata.is_file() {
        return Err(AppError::new(
            ErrorCode::FileNotFound,
            "Sumber bukan file biasa.",
        ));
    }

    let (raw_width, raw_height, has_alpha, orientation) = match detected {
        SourceFormat::Png => {
            let reader = BufReader::new(File::open(path).map_err(file_io_error)?);
            let decoder = PngDecoder::new(reader).map_err(image_error)?;
            if decoder.is_apng().map_err(image_error)? {
                return Err(animated_error());
            }
            decoder_metadata(decoder)?
        }
        SourceFormat::Jpeg => {
            let reader = BufReader::new(File::open(path).map_err(file_io_error)?);
            let decoder = JpegDecoder::new(reader).map_err(image_error)?;
            decoder_metadata(decoder)?
        }
        SourceFormat::Webp => {
            let reader = BufReader::new(File::open(path).map_err(file_io_error)?);
            let decoder = WebPDecoder::new(reader).map_err(image_error)?;
            if decoder.has_animation() {
                return Err(animated_error());
            }
            decoder_metadata(decoder)?
        }
        SourceFormat::Bmp => {
            let reader = BufReader::new(File::open(path).map_err(file_io_error)?);
            let decoder = BmpDecoder::new(reader).map_err(image_error)?;
            decoder_metadata(decoder)?
        }
    };

    let (width, height) = oriented_dimensions(raw_width, raw_height, orientation);
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels > MAX_PIXELS {
        return Err(AppError::new(
            ErrorCode::ImageTooLarge,
            "Gambar melebihi batas 30 MP.",
        ));
    }

    let fingerprint = sampled_fingerprint(path, metadata.len())?;
    Ok(SourceProbe {
        width,
        height,
        bytes: metadata.len(),
        format: detected,
        has_alpha,
        orientation,
        fingerprint,
    })
}

fn decoder_metadata<D: ImageDecoder>(
    mut decoder: D,
) -> Result<(u32, u32, bool, Orientation), AppError> {
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 {
        return Err(AppError::new(
            ErrorCode::ImageCorrupt,
            "Dimensi gambar tidak valid.",
        ));
    }
    let has_alpha = decoder.color_type().has_alpha();
    let orientation = decoder.orientation().map_err(image_error)?;
    Ok((width, height, has_alpha, orientation))
}

fn detect_format(path: &Path) -> Result<SourceFormat, AppError> {
    let mut file = File::open(path).map_err(file_io_error)?;
    let mut header = [0u8; 32];
    let read = file.read(&mut header).map_err(file_io_error)?;
    if read == 0 {
        return Err(AppError::new(
            ErrorCode::ImageCorrupt,
            "File gambar kosong.",
        ));
    }

    let format = image::guess_format(&header[..read]).map_err(|error| {
        AppError::with_details(
            ErrorCode::ImageCorrupt,
            "Header gambar tidak valid.",
            error.to_string(),
        )
    })?;

    match format {
        image::ImageFormat::Png => Ok(SourceFormat::Png),
        image::ImageFormat::Jpeg => Ok(SourceFormat::Jpeg),
        image::ImageFormat::WebP => Ok(SourceFormat::Webp),
        image::ImageFormat::Bmp => Ok(SourceFormat::Bmp),
        _ => Err(AppError::new(
            ErrorCode::UnsupportedFormat,
            "Format gambar tidak didukung.",
        )),
    }
}

fn format_from_extension(path: &Path) -> Result<SourceFormat, AppError> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);

    match extension.as_deref() {
        Some("png") => Ok(SourceFormat::Png),
        Some("jpg" | "jpeg") => Ok(SourceFormat::Jpeg),
        Some("webp") => Ok(SourceFormat::Webp),
        Some("bmp") => Ok(SourceFormat::Bmp),
        _ => Err(AppError::new(
            ErrorCode::UnsupportedFormat,
            "Format gambar tidak didukung.",
        )),
    }
}

fn oriented_dimensions(width: u32, height: u32, orientation: Orientation) -> (u32, u32) {
    match orientation {
        Orientation::Rotate90
        | Orientation::Rotate270
        | Orientation::Rotate90FlipH
        | Orientation::Rotate270FlipH => (height, width),
        _ => (width, height),
    }
}

fn sampled_fingerprint(path: &Path, bytes: u64) -> Result<String, AppError> {
    let mut file = File::open(path).map_err(file_io_error)?;
    let mut hash = Fnv64::new();
    hash.update(&bytes.to_le_bytes());

    for start in sample_offsets(bytes) {
        file.seek(SeekFrom::Start(start)).map_err(file_io_error)?;
        let remaining = bytes.saturating_sub(start).min(SAMPLE_BYTES as u64) as usize;
        let mut buffer = vec![0u8; remaining];
        file.read_exact(&mut buffer).map_err(file_io_error)?;
        hash.update(&start.to_le_bytes());
        hash.update(&buffer);
    }

    Ok(format!("{:016x}", hash.finish()))
}

fn sample_offsets(bytes: u64) -> Vec<u64> {
    let chunk = SAMPLE_BYTES as u64;
    let middle = bytes
        .saturating_div(2)
        .saturating_sub(chunk.saturating_div(2));
    let tail = bytes.saturating_sub(chunk);

    let mut offsets = vec![0, middle, tail];
    offsets.sort_unstable();
    offsets.dedup();
    offsets
}

struct Fnv64(u64);

impl Fnv64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x00000100000001B3;

    fn new() -> Self {
        Self(Self::OFFSET)
    }

    fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    fn finish(self) -> u64 {
        self.0
    }
}

fn file_io_error(error: std::io::Error) -> AppError {
    match error.kind() {
        std::io::ErrorKind::NotFound => {
            AppError::new(ErrorCode::FileNotFound, "File sumber tidak ditemukan.")
        }
        std::io::ErrorKind::PermissionDenied => {
            AppError::new(ErrorCode::AccessDenied, "Akses file sumber ditolak.")
        }
        _ => AppError::with_details(
            ErrorCode::ImageCorrupt,
            "Gambar gagal dibaca.",
            error.to_string(),
        ),
    }
}

fn image_error(error: image::ImageError) -> AppError {
    AppError::with_details(
        ErrorCode::ImageCorrupt,
        "Gambar rusak atau gagal dibaca.",
        error.to_string(),
    )
}

fn animated_error() -> AppError {
    AppError::new(
        ErrorCode::UnsupportedFormat,
        "Gambar animasi atau multi-frame tidak didukung.",
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use image::metadata::Orientation;

    use super::{SourceFormat, format_from_extension, oriented_dimensions, sample_offsets};

    #[test]
    fn extensions_match_schema_formats() -> Result<(), String> {
        let cases = [
            ("a.png", SourceFormat::Png),
            ("b.jpg", SourceFormat::Jpeg),
            ("c.JPEG", SourceFormat::Jpeg),
            ("d.webp", SourceFormat::Webp),
            ("e.bmp", SourceFormat::Bmp),
        ];

        for (path, expected) in cases {
            let actual = format_from_extension(Path::new(path)).map_err(|error| error.message)?;
            if actual != expected {
                return Err(format!("unexpected format for {path}: {actual:?}"));
            }
        }

        assert!(format_from_extension(Path::new("bad.gif")).is_err());
        Ok(())
    }

    #[test]
    fn orientation_swaps_display_dimensions_when_required() {
        assert_eq!(
            oriented_dimensions(400, 300, Orientation::Rotate90),
            (300, 400)
        );
        assert_eq!(
            oriented_dimensions(400, 300, Orientation::Rotate270FlipH),
            (300, 400)
        );
        assert_eq!(
            oriented_dimensions(400, 300, Orientation::FlipHorizontal),
            (400, 300)
        );
    }

    #[test]
    fn sample_offsets_are_unique_for_small_files() {
        assert_eq!(sample_offsets(12), vec![0]);
        assert_eq!(sample_offsets(200_000).len(), 3);
    }
}
