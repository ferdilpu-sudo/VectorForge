//! Bounded raster feedback: compare the actual SVG render to the source raster.
//!
//! This optional check does not change segmentation or color quantization.
//! It compares two fitting candidates from the SAME segmentation at a bounded
//! resolution. Source samples and rendered pixels are premultiplied RGBA so
//! partial-alpha mismatch contributes to the error. It does not guarantee
//! professional artistic quality or pixel-identical output.

use resvg::{tiny_skia, usvg};
use vtracer::{ColorImage, VectorDoc};

use crate::models::{AppError, ErrorCode};

use super::svg::write_alpha_svg;

pub(crate) const MAX_FEEDBACK_PIXELS: usize = 8_000_000;
const MAX_EVAL_SIDE: usize = 384;
const MAX_EVAL_SVG_BYTES: usize = 8 * 1024 * 1024;

/// Permits insignificant antialiasing differences while rejecting clear loss.
const ACCEPT_ABSOLUTE_DELTA: f64 = 0.35;
const ACCEPT_RELATIVE_DELTA: f64 = 0.01;

pub(crate) fn prefer_candidate(
    source: &ColorImage,
    candidate: &VectorDoc,
    baseline: &VectorDoc,
) -> Result<bool, AppError> {
    if source.width != candidate.width as usize
        || source.height != candidate.height as usize
        || candidate.width != baseline.width
        || candidate.height != baseline.height
    {
        return Err(evaluation_error("Dimensi kandidat vektor tidak konsisten."));
    }
    let max_side = source.width.max(source.height);
    if max_side == 0 || !valid_rgba_buffer(source) {
        return Err(evaluation_error("Buffer gambar sumber tidak valid."));
    }
    let side = max_side.min(MAX_EVAL_SIDE);
    let width = (source.width * side / max_side).max(1) as u32;
    let height = (source.height * side / max_side).max(1) as u32;
    let reference = reference_rgba(source, width as usize, height as usize);
    let baseline_score = compare_svg(source, baseline, width, height, &reference)?;
    let candidate_score = compare_svg(source, candidate, width, height, &reference)?;
    Ok(candidate_score
        <= baseline_score + ACCEPT_ABSOLUTE_DELTA + baseline_score * ACCEPT_RELATIVE_DELTA)
}

fn evaluation_error(details: &str) -> AppError {
    AppError::new(ErrorCode::TraceFailed, details)
}

fn compare_svg(
    source: &ColorImage,
    doc: &VectorDoc,
    width: u32,
    height: u32,
    reference: &[[u8; 4]],
) -> Result<f64, AppError> {
    let svg = write_alpha_svg(doc).svg;
    if svg.len() > MAX_EVAL_SVG_BYTES {
        return Err(evaluation_error(
            "SVG terlalu kompleks untuk evaluasi raster (8 MiB).",
        ));
    }
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).map_err(|error| {
        AppError::with_details(
            ErrorCode::TraceFailed,
            "SVG tidak dapat dirender untuk evaluasi kualitas.",
            error.to_string(),
        )
    })?;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| evaluation_error("Gagal mengalokasikan buffer evaluasi visual."))?;
    let transform = tiny_skia::Transform::from_scale(
        width as f32 / source.width as f32,
        height as f32 / source.height as f32,
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    pixel_error(reference, pixmap.pixels(), width as usize)
}

fn valid_rgba_buffer(source: &ColorImage) -> bool {
    source
        .width
        .checked_mul(source.height)
        .and_then(|pixels| pixels.checked_mul(4))
        == Some(source.pixels.len())
        && source.width > 0
        && source.height > 0
}

/// Score one real vector document for *diagnostic QA only*; does not
/// independently pick a tracing configuration or mutate a vector document.
#[cfg(test)]
pub(crate) fn render_score(source: &ColorImage, doc: &VectorDoc) -> Result<f64, AppError> {
    if source.width != doc.width as usize || source.height != doc.height as usize {
        return Err(evaluation_error(
            "Dimensi sumber dan dokumen SVG tidak sama.",
        ));
    }
    if !valid_rgba_buffer(source)
        || source.width.saturating_mul(source.height) > MAX_FEEDBACK_PIXELS
    {
        return Err(evaluation_error(
            "Buffer sumber tidak valid atau melewati batas QA 8 MP.",
        ));
    }
    let max_side = source.width.max(source.height);
    let side = max_side.min(MAX_EVAL_SIDE);
    let width = (source.width * side / max_side).max(1) as u32;
    let height = (source.height * side / max_side).max(1) as u32;
    let reference = reference_rgba(source, width as usize, height as usize);
    compare_svg(source, doc, width, height, &reference)
}

fn source_pixel(source: &ColorImage, x: usize, y: usize) -> [u8; 4] {
    let index = (y * source.width + x) * 4;
    let a = u32::from(source.pixels[index + 3]);
    let premul =
        |channel: usize| ((u32::from(source.pixels[index + channel]) * a + 127) / 255) as u8;
    [premul(0), premul(1), premul(2), a as u8]
}

fn reference_rgba(source: &ColorImage, width: usize, height: usize) -> Vec<[u8; 4]> {
    let mut values = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            // Four area samples are fast, bounded and less alias-prone than
            // a single nearest-neighbour pixel for reduced-size evaluation.
            let mut sum = [0u32; 4];
            for sy in [0.25, 0.75] {
                for sx in [0.25, 0.75] {
                    let ix = (((x as f64 + sx) * source.width as f64 / width as f64) as usize)
                        .min(source.width - 1);
                    let iy = (((y as f64 + sy) * source.height as f64 / height as f64) as usize)
                        .min(source.height - 1);
                    let p = source_pixel(source, ix, iy);
                    for channel in 0..4 {
                        sum[channel] += u32::from(p[channel]);
                    }
                }
            }
            values.push(sum.map(|channel| (channel / 4) as u8));
        }
    }
    values
}

fn pixel_error(
    reference: &[[u8; 4]],
    rendered: &[tiny_skia::PremultipliedColorU8],
    width: usize,
) -> Result<f64, AppError> {
    if width == 0
        || reference.is_empty()
        || reference.len() % width != 0
        || reference.len() != rendered.len()
    {
        return Err(evaluation_error(
            "Panjang buffer skor raster tidak konsisten.",
        ));
    }
    let mut all = 0.0;
    let mut edge = 0.0;
    let mut edges = 0usize;
    for (index, (source, target)) in reference.iter().zip(rendered).enumerate() {
        let rgba = [target.red(), target.green(), target.blue(), target.alpha()];
        let error = source
            .iter()
            .zip(rgba)
            .map(|(a, b)| f64::from(a.abs_diff(b)))
            .sum::<f64>()
            / 4.0;
        all += error;

        // Edges matter much more than the often dominant flat background.
        let x = index % width;
        let contrast = if x > 0 {
            source
                .iter()
                .zip(reference[index - 1])
                .map(|(a, b)| a.abs_diff(b))
                .max()
                .unwrap_or(0)
        } else {
            0
        };
        let contrast = if index >= width {
            contrast.max(
                source
                    .iter()
                    .zip(reference[index - width])
                    .map(|(a, b)| a.abs_diff(b))
                    .max()
                    .unwrap_or(0),
            )
        } else {
            contrast
        };
        if contrast >= 20 {
            edge += error;
            edges += 1;
        }
    }
    let average = all / reference.len() as f64;
    let score = if edges == 0 {
        average
    } else {
        (average + edge / edges as f64) / 2.0
    };
    if score.is_finite() {
        Ok(score)
    } else {
        Err(evaluation_error("Hasil skor evaluasi raster tidak finite."))
    }
}

#[cfg(test)]
mod tests {
    use resvg::tiny_skia;
    use vtracer::ir::{MultiPath, Paint, PathCmd, Shape, SubPath};
    use vtracer::{Color, ColorImage, PointF64, VectorDoc};

    use super::{MAX_EVAL_SIDE, pixel_error, prefer_candidate, reference_rgba, render_score};

    #[test]
    fn samples_opaque_and_transparent_without_exposing_hidden_rgb() {
        let source = ColorImage {
            width: 2,
            height: 1,
            pixels: vec![250, 40, 20, 0, 20, 100, 180, 255],
        };
        let reference = reference_rgba(&source, 2, 1);
        assert_eq!(reference[0], [0, 0, 0, 0]);
        assert_eq!(reference[1], [20, 100, 180, 255]);
    }

    #[test]
    fn perfect_transparency_scores_zero_and_opaque_mismatch_is_penalized() {
        let pixels = vec![[0, 0, 0, 0]; MAX_EVAL_SIDE];
        let blank = vec![tiny_skia::PremultipliedColorU8::TRANSPARENT; MAX_EVAL_SIDE];
        assert_eq!(pixel_error(&pixels, &blank, MAX_EVAL_SIDE).unwrap(), 0.0);
        let black =
            vec![tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, 255).unwrap(); MAX_EVAL_SIDE];
        assert!(pixel_error(&pixels, &black, MAX_EVAL_SIDE).unwrap() > 50.0);
    }

    fn solid_doc(r: u8, g: u8, b: u8) -> VectorDoc {
        let mut doc = VectorDoc::new(64, 64);
        let point = |x, y| PointF64 { x, y };
        doc.shapes.push(Shape {
            paint: Paint::Solid(Color::new(r, g, b)),
            path: MultiPath {
                subpaths: vec![SubPath {
                    commands: vec![
                        PathCmd::MoveTo(point(0.0, 0.0)),
                        PathCmd::LineTo(point(64.0, 0.0)),
                        PathCmd::LineTo(point(64.0, 64.0)),
                        PathCmd::LineTo(point(0.0, 64.0)),
                        PathCmd::Close,
                    ],
                }],
            },
        });
        doc
    }

    #[test]
    fn raster_feedback_prefers_matching_svg_render() -> Result<(), String> {
        let source = ColorImage {
            width: 64,
            height: 64,
            pixels: [0, 0, 0, 255].repeat(64 * 64),
        };
        let black = solid_doc(0, 0, 0);
        let white = solid_doc(255, 255, 255);
        assert!(
            prefer_candidate(&source, &black, &white).map_err(|e| e.message.clone())?,
            "matching black SVG must beat white baseline"
        );
        assert!(
            !prefer_candidate(&source, &white, &black).map_err(|e| e.message.clone())?,
            "mismatched white SVG must lose to black baseline"
        );
        Ok(())
    }

    #[test]
    fn rejects_different_pixel_buffer_lengths_instead_of_partial_zip() {
        let rgba = [[0, 0, 0, 0], [255, 255, 255, 255]];
        let single = [tiny_skia::PremultipliedColorU8::TRANSPARENT];
        assert!(pixel_error(&rgba, &single, 2).is_err());
        assert!(pixel_error(&rgba, &single, 0).is_err());
    }

    #[test]
    fn diagnostic_render_score_rejects_invalid_dimensions() {
        let source = ColorImage {
            width: 2,
            height: 2,
            pixels: vec![0; 16],
        };
        assert!(render_score(&source, &solid_doc(0, 0, 0)).is_err());
        let invalid_buffer = ColorImage {
            pixels: vec![0; 15],
            ..source
        };
        let mut small = VectorDoc::new(2, 2);
        small.shapes.clear();
        assert!(render_score(&invalid_buffer, &small).is_err());
    }

    fn thin_stroke_doc() -> VectorDoc {
        let mut doc = solid_doc(255, 255, 255);
        let point = |x, y| PointF64 { x, y };
        doc.shapes.push(Shape {
            paint: Paint::Solid(Color::new(0, 0, 0)),
            path: MultiPath {
                subpaths: vec![SubPath {
                    commands: vec![
                        PathCmd::MoveTo(point(29.0, 10.0)),
                        PathCmd::LineTo(point(32.0, 10.0)),
                        PathCmd::LineTo(point(32.0, 54.0)),
                        PathCmd::LineTo(point(29.0, 54.0)),
                        PathCmd::Close,
                    ],
                }],
            },
        });
        doc
    }

    #[test]
    fn raster_feedback_rejects_missing_thin_stroke() -> Result<(), String> {
        let mut pixels = Vec::with_capacity(64 * 64 * 4);
        for y in 0..64 {
            for x in 0..64 {
                let color = if (29..32).contains(&x) && (10..54).contains(&y) {
                    0
                } else {
                    255
                };
                pixels.extend_from_slice(&[color, color, color, 255]);
            }
        }
        let source = ColorImage {
            width: 64,
            height: 64,
            pixels,
        };
        let correct = thin_stroke_doc();
        let missing = solid_doc(255, 255, 255);
        assert!(
            prefer_candidate(&source, &correct, &missing).map_err(|error| error.message.clone())?,
            "the candidate with the stroke should be preferred"
        );
        assert!(
            !prefer_candidate(&source, &missing, &correct)
                .map_err(|error| error.message.clone())?,
            "dropping a thin stroke must be detected at native QA scale"
        );
        Ok(())
    }

    #[test]
    fn alpha_disagreement_is_not_hidden_by_matching_rgb() {
        let reference = [[30, 20, 10, 128]];
        let matching = [tiny_skia::PremultipliedColorU8::from_rgba(30, 20, 10, 128)
            .expect("valid premultiplied pixel")];
        let opaque = [tiny_skia::PremultipliedColorU8::from_rgba(30, 20, 10, 255)
            .expect("valid premultiplied pixel")];
        let exact = pixel_error(&reference, &matching, 1).expect("valid buffer");
        let mismatch = pixel_error(&reference, &opaque, 1).expect("valid buffer");
        assert_eq!(exact, 0.0);
        assert!(mismatch > 0.0, "alpha changes must affect the score");
    }

    #[test]
    fn edge_pixels_carry_more_weight_than_uniform_background() {
        let source = vec![[0, 0, 0, 255], [255, 255, 255, 255]];
        let matching = vec![
            tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, 255).unwrap(),
            tiny_skia::PremultipliedColorU8::from_rgba(255, 255, 255, 255).unwrap(),
        ];
        let reversed = vec![
            tiny_skia::PremultipliedColorU8::from_rgba(255, 255, 255, 255).unwrap(),
            tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, 255).unwrap(),
        ];
        assert!(
            pixel_error(&source, &reversed, 2).unwrap()
                > pixel_error(&source, &matching, 2).unwrap()
        );
    }
}
