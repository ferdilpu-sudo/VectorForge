use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, RgbaImage};

use crate::models::{AppError, ErrorCode};

pub fn decode_preview(
    path: &Path,
    orientation: Orientation,
    max_side: u32,
) -> Result<RgbaImage, AppError> {
    decode_catching_panic(path, |image| {
        normalize_preview_image(image, orientation, max_side)
    })
}

pub fn decode_full(path: &Path, orientation: Orientation) -> Result<RgbaImage, AppError> {
    decode_catching_panic(path, |mut image| {
        image.apply_orientation(orientation);
        image.into_rgba8()
    })
}

fn decode_catching_panic(
    path: &Path,
    normalize: impl FnOnce(DynamicImage) -> RgbaImage,
) -> Result<RgbaImage, AppError> {
    match catch_unwind(AssertUnwindSafe(|| {
        let image = image::open(path).map_err(image_error)?;
        Ok::<RgbaImage, AppError>(normalize(image))
    })) {
        Ok(result) => result,
        Err(_) => Err(AppError::new(
            ErrorCode::ImageCorrupt,
            "Decoder gambar berhenti secara tidak terduga.",
        )),
    }
}

fn normalize_preview_image(
    mut image: DynamicImage,
    orientation: Orientation,
    max_side: u32,
) -> RgbaImage {
    image.apply_orientation(orientation);

    if image.width().max(image.height()) > max_side {
        image = image.resize(max_side, max_side, FilterType::Triangle);
    }

    image.into_rgba8()
}

fn image_error(error: image::ImageError) -> AppError {
    AppError::with_details(
        ErrorCode::ImageCorrupt,
        "Gambar gagal didekode.",
        error.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use image::metadata::Orientation;
    use image::{DynamicImage, RgbaImage};

    use super::{decode_full, normalize_preview_image};

    #[test]
    fn preview_does_not_upscale_small_sources() {
        let image = DynamicImage::ImageRgba8(RgbaImage::new(320, 200));
        let result = normalize_preview_image(image, Orientation::NoTransforms, 1024);
        assert_eq!(result.dimensions(), (320, 200));
    }

    #[test]
    fn preview_resizes_to_requested_bound() {
        let image = DynamicImage::ImageRgba8(RgbaImage::new(1600, 800));
        let result = normalize_preview_image(image, Orientation::NoTransforms, 800);
        assert_eq!(result.dimensions(), (800, 400));
    }

    #[test]
    fn full_decode_applies_orientation_without_resize() -> Result<(), String> {
        let path = std::env::temp_dir().join(format!(
            "vectorforge-full-decode-{}.png",
            uuid::Uuid::new_v4()
        ));
        RgbaImage::new(800, 400)
            .save(&path)
            .map_err(|error| error.to_string())?;

        let result = decode_full(&path, Orientation::Rotate90)
            .map_err(|error| error.message);
        let _ = std::fs::remove_file(&path);
        let result = result?;
        assert_eq!(result.dimensions(), (400, 800));
        Ok(())
    }

    #[test]
    fn orientation_is_applied_before_resize() {
        let image = DynamicImage::ImageRgba8(RgbaImage::new(800, 400));
        let result = normalize_preview_image(image, Orientation::Rotate90, 600);
        assert_eq!(result.dimensions(), (300, 600));
    }
}
