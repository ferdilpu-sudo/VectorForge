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
    if max_side == 0 || source.pixels.len() != source.width * source.height * 4 {
        return Err(evaluation_error("Buffer gambar sumber tidak valid."));
    }
    let side = max_side.min(MAX_EVAL_SIDE);
    let width = (source.width * side / max_side).max(1) as u32;
    let height = (source.height * side / max_side).max(1) as u32;
    let reference = reference_rgba(source, width as usize, height as usize);
    let baseline_score = compare_svg(source, baseline, width, height, &reference)?;
    let candidate_score = compare_svg(source, candidate, width, height, &reference)?;
    Ok(candidate_score <= baseline_score + ACCEPT_ABSOLUTE_DELTA
        + baseline_score * ACCEPT_RELATIVE_DELTA)
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
        return Err(evaluation_error("SVG terlalu kompleks untuk evaluasi raster (8 MiB)."));
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
    Ok(pixel_error(reference, pixmap.pixels(), width as usize))
}

fn source_pixel(source: &ColorImage, x: usize, y: usize) -> [u8; 4] {
    let index = (y * source.width + x) * 4;
    let a = u32::from(source.pixels[index + 3]);
    let premul = |channel: usize| {
        ((u32::from(source.pixels[index + channel]) * a + 127) / 255) as u8
    };
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
) -> f64 {
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
    if reference.is_empty() {
        return f64::INFINITY;
    }
    let average = all / reference.len() as f64;
    if edges == 0 {
        average
    } else {
        (average + edge / edges as f64) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use resvg::tiny_skia;
    use vtracer::ir::{MultiPath, Paint, PathCmd, Shape, SubPath};
    use vtracer::{Color, ColorImage, PointF64, VectorDoc};

    use super::{MAX_EVAL_SIDE, pixel_error, prefer_candidate, reference_rgba};

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
        assert_eq!(pixel_error(&pixels, &blank, MAX_EVAL_SIDE), 0.0);
        let black = vec![
            tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, 255).unwrap();
            MAX_EVAL_SIDE
        ];
        assert!(pixel_error(&pixels, &black, MAX_EVAL_SIDE) > 50.0);
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
        assert!(pixel_error(&source, &reversed, 2) > pixel_error(&source, &matching, 2));
    }
}
