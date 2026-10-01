use std::collections::BTreeMap;

use visioncortex::BinaryImage;
use vtracer::ir::{Layer, Paint, RegionMask, Segmentation};
use vtracer::{CancelToken, Color, ColorImage, PointI32};

use crate::models::{AppError, ErrorCode};

const MAX_ALPHA_MASK_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Copy)]
struct AlphaBounds {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}

impl AlphaBounds {
    fn new(x: usize, y: usize) -> Self {
        Self {
            left: x,
            top: y,
            right: x,
            bottom: y,
        }
    }

    fn include(&mut self, x: usize, y: usize) {
        self.left = self.left.min(x);
        self.top = self.top.min(y);
        self.right = self.right.max(x);
        self.bottom = self.bottom.max(y);
    }

    fn width(self) -> usize {
        self.right - self.left + 1
    }

    fn height(self) -> usize {
        self.bottom - self.top + 1
    }
}

pub fn split_by_source_alpha(
    segmentation: Segmentation,
    source: &ColorImage,
    cancel: &CancelToken,
) -> Result<Segmentation, AppError> {
    split_with_budget(segmentation, source, cancel, MAX_ALPHA_MASK_BYTES)
}

fn split_with_budget(
    segmentation: Segmentation,
    source: &ColorImage,
    cancel: &CancelToken,
    budget_bytes: usize,
) -> Result<Segmentation, AppError> {
    let mut result = Segmentation::new(segmentation.width, segmentation.height);
    let mut planned_mask_bytes = 0usize;

    for layer in segmentation.layers {
        check_cancel(cancel)?;
        let base_color = layer.paint.color();
        let mut bounds: BTreeMap<u8, AlphaBounds> = BTreeMap::new();

        for local_y in 0..layer.mask.image.height {
            if local_y % 32 == 0 {
                check_cancel(cancel)?;
            }
            for local_x in 0..layer.mask.image.width {
                if !layer.mask.image.get_pixel(local_x, local_y) {
                    continue;
                }

                let (global_x, global_y) = global_position(&layer, local_x, local_y, source)?;
                let alpha = source.get_pixel(global_x, global_y).a;
                if alpha == 0 {
                    continue;
                }

                bounds
                    .entry(alpha)
                    .and_modify(|bbox| bbox.include(local_x, local_y))
                    .or_insert_with(|| AlphaBounds::new(local_x, local_y));
            }
        }

        let layer_mask_bytes: usize = bounds
            .values()
            .map(|bbox| bbox.width().saturating_mul(bbox.height()).div_ceil(8))
            .sum();

        planned_mask_bytes = planned_mask_bytes
            .checked_add(layer_mask_bytes)
            .ok_or_else(alpha_budget_error)?;

        if planned_mask_bytes > budget_bytes {
            return Err(alpha_budget_error());
        }

        let mut masks: BTreeMap<u8, (AlphaBounds, BinaryImage)> = bounds
            .into_iter()
            .map(|(alpha, bbox)| {
                (
                    alpha,
                    (bbox, BinaryImage::new_w_h(bbox.width(), bbox.height())),
                )
            })
            .collect();

        for local_y in 0..layer.mask.image.height {
            if local_y % 32 == 0 {
                check_cancel(cancel)?;
            }
            for local_x in 0..layer.mask.image.width {
                if !layer.mask.image.get_pixel(local_x, local_y) {
                    continue;
                }

                let (global_x, global_y) = global_position(&layer, local_x, local_y, source)?;
                let alpha = source.get_pixel(global_x, global_y).a;
                if alpha == 0 {
                    continue;
                }

                let (bbox, mask) = masks.get_mut(&alpha).ok_or_else(|| {
                    AppError::new(
                        ErrorCode::TraceFailed,
                        "Mask transparansi berubah saat tracing.",
                    )
                })?;
                mask.set_pixel(local_x - bbox.left, local_y - bbox.top, true);
            }
        }

        for (alpha, (bbox, mask)) in masks {
            result.layers.push(Layer {
                paint: Paint::Solid(Color::new_rgba(
                    base_color.r,
                    base_color.g,
                    base_color.b,
                    alpha,
                )),
                mask: RegionMask::new(
                    mask,
                    PointI32 {
                        x: layer.mask.offset.x + bbox.left as i32,
                        y: layer.mask.offset.y + bbox.top as i32,
                    },
                ),
            });
        }
    }

    Ok(result)
}

fn global_position(
    layer: &Layer,
    local_x: usize,
    local_y: usize,
    source: &ColorImage,
) -> Result<(usize, usize), AppError> {
    let global_x = layer.mask.offset.x + local_x as i32;
    let global_y = layer.mask.offset.y + local_y as i32;

    if global_x < 0
        || global_y < 0
        || global_x as usize >= source.width
        || global_y as usize >= source.height
    {
        return Err(AppError::new(
            ErrorCode::TraceFailed,
            "Mask tracing keluar dari batas gambar.",
        ));
    }

    Ok((global_x as usize, global_y as usize))
}

fn check_cancel(cancel: &CancelToken) -> Result<(), AppError> {
    if cancel.is_cancelled() {
        Err(AppError::new(ErrorCode::Cancelled, "Preview dibatalkan."))
    } else {
        Ok(())
    }
}

fn alpha_budget_error() -> AppError {
    AppError::new(
        ErrorCode::TraceFailed,
        "Kompleksitas transparansi melebihi batas aman preview.",
    )
}

#[cfg(test)]
mod tests {
    use visioncortex::BinaryImage;
    use vtracer::ir::{Layer, Paint, RegionMask, Segmentation};
    use vtracer::{CancelToken, Color, ColorImage, PointI32};

    use super::split_with_budget;

    fn fixture() -> (Segmentation, ColorImage) {
        let width = 8u32;
        let height = 8u32;
        let width_usize = width as usize;
        let height_usize = height as usize;
        let source = ColorImage {
            pixels: (0..width_usize * height_usize)
                .flat_map(|index| {
                    let alpha = if index % 2 == 0 { 128 } else { 255 };
                    [40, 120, 220, alpha]
                })
                .collect(),
            width: width_usize,
            height: height_usize,
        };

        let mut mask = BinaryImage::new_w_h(width_usize, height_usize);
        for y in 0..height_usize {
            for x in 0..width_usize {
                mask.set_pixel(x, y, true);
            }
        }

        let mut segmentation = Segmentation::new(width, height);
        segmentation.layers.push(Layer {
            paint: Paint::Solid(Color::new_rgba(40, 120, 220, 255)),
            mask: RegionMask::new(mask, PointI32 { x: 0, y: 0 }),
        });

        (segmentation, source)
    }

    #[test]
    fn alpha_split_preserves_distinct_levels() -> Result<(), String> {
        let (segmentation, source) = fixture();
        let split = split_with_budget(segmentation, &source, &CancelToken::new(), 1024)
            .map_err(|error| error.message)?;

        let mut alphas: Vec<u8> = split
            .layers
            .iter()
            .map(|layer| layer.paint.color().a)
            .collect();
        alphas.sort_unstable();
        assert_eq!(alphas, vec![128, 255]);
        Ok(())
    }

    #[test]
    fn alpha_split_enforces_mask_budget_before_allocation() {
        let (segmentation, source) = fixture();
        let result = split_with_budget(segmentation, &source, &CancelToken::new(), 1);
        assert!(result.is_err());
    }
}
