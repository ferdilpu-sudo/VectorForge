//! Conservative, opt-in contour-level quality guard (QV-008).
//!
//! Compares fitted geometry before and after spline simplification/straightening.
//! On excessive drift or area change, retains the original fitted contour.
//! Runs before composition, preserving shared cutout boundaries. This is NOT
//! a raster-versus-SVG image fidelity metric and cannot detect bad segmentation.

use vtracer::PointF64;
use vtracer::fitter::FittedGeom;
use vtracer::simplify::{CurvePass, SimplifyCurves};

use super::geometry::PreserveStraightEdges;

const MAX_CURVES: usize = 96;
const SAMPLES_PER_CURVE: usize = 4;

#[derive(Debug, Clone, Copy)]
pub(crate) struct GuardedContourPass {
    pub(crate) simplify_tolerance: f64,
    pub(crate) corner_threshold_radians: f64,
    pub(crate) line_aware: bool,
    pub(crate) max_deviation: f64,
    pub(crate) max_mean_deviation: f64,
}

impl CurvePass for GuardedContourPass {
    fn open(&self, geom: FittedGeom) -> FittedGeom {
        self.run(geom, false)
    }

    fn ring(&self, geom: FittedGeom) -> FittedGeom {
        self.run(geom, true)
    }
}

impl GuardedContourPass {
    fn apply(&self, original: FittedGeom, ring: bool, with_simplify: bool) -> FittedGeom {
        let mut candidate = original;
        if with_simplify && self.simplify_tolerance > 0.0 {
            let simplify = SimplifyCurves {
                tolerance: self.simplify_tolerance,
                corner_threshold: self.corner_threshold_radians,
            };
            candidate = if ring {
                simplify.ring(candidate)
            } else {
                simplify.open(candidate)
            };
        }
        if self.line_aware {
            let preserve = PreserveStraightEdges {
                tolerance: 0.6,
                min_length: 8.0,
            };
            candidate = if ring {
                preserve.ring(candidate)
            } else {
                preserve.open(candidate)
            };
        }
        candidate
    }

    fn run(&self, original: FittedGeom, ring: bool) -> FittedGeom {
        let FittedGeom::Beziers(ref curves) = original else {
            return original;
        };
        // Avoid quadratic cost and overconfident sampling of huge contours.
        // Returning the original is always the conservative outcome.
        if curves.is_empty() || curves.len() > MAX_CURVES {
            return original;
        }
        let candidate = self.apply(original.clone(), ring, true);
        if accepted(
            &original,
            &candidate,
            ring,
            self.max_deviation,
            self.max_mean_deviation,
        ) {
            return candidate;
        }
        // If simplification failed, salvage the safer straight-edge-only pass.
        if self.line_aware && self.simplify_tolerance > 0.0 {
            let line_only = self.apply(original.clone(), ring, false);
            if accepted(
                &original,
                &line_only,
                ring,
                self.max_deviation,
                self.max_mean_deviation,
            ) {
                return line_only;
            }
        }
        original
    }
}

fn sample(curves: &[[PointF64; 4]]) -> Vec<PointF64> {
    let mut result = Vec::with_capacity(curves.len() * SAMPLES_PER_CURVE + 1);
    if let Some(first) = curves.first() {
        result.push(first[0]);
    }
    for curve in curves {
        for n in 1..=SAMPLES_PER_CURVE {
            let t = n as f64 / SAMPLES_PER_CURVE as f64;
            let u = 1.0 - t;
            let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            result.push(PointF64 {
                x: weights.iter().zip(curve).map(|(w, p)| *w * p.x).sum(),
                y: weights.iter().zip(curve).map(|(w, p)| *w * p.y).sum(),
            });
        }
    }
    result
}

fn distance_to_segment(p: PointF64, a: PointF64, b: PointF64) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let squared = dx * dx + dy * dy;
    if squared <= 1e-20 {
        return ((p.x - a.x).powi(2) + (p.y - a.y).powi(2)).sqrt();
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / squared).clamp(0.0, 1.0);
    let nearest = PointF64 {
        x: a.x + t * dx,
        y: a.y + t * dy,
    };
    ((p.x - nearest.x).powi(2) + (p.y - nearest.y).powi(2)).sqrt()
}

fn directed_error(a: &[PointF64], b: &[PointF64], max: f64) -> Option<f64> {
    let mut sum = 0.0;
    for &point in a {
        let nearest = b
            .windows(2)
            .map(|w| distance_to_segment(point, w[0], w[1]))
            .fold(f64::INFINITY, f64::min);
        if !nearest.is_finite() || nearest > max {
            return None;
        }
        sum += nearest;
    }
    Some(sum / a.len() as f64)
}

fn signed_area(points: &[PointF64]) -> f64 {
    points
        .windows(2)
        .map(|w| w[0].x * w[1].y - w[1].x * w[0].y)
        .sum::<f64>()
        / 2.0
}

fn accepted(
    original: &FittedGeom,
    candidate: &FittedGeom,
    closed: bool,
    max: f64,
    mean: f64,
) -> bool {
    let (FittedGeom::Beziers(source), FittedGeom::Beziers(result)) = (original, candidate) else {
        return false;
    };
    if source.is_empty()
        || result.is_empty()
        || source.len() > MAX_CURVES
        || result.len() > MAX_CURVES
    {
        return false;
    }
    // In particular, never move mosaic junctions or ring seam anchors.
    if source[0][0] != result[0][0] || source.last().map(|c| c[3]) != result.last().map(|c| c[3]) {
        return false;
    }
    let a = sample(source);
    let b = sample(result);
    if !a
        .iter()
        .chain(b.iter())
        .all(|p| p.x.is_finite() && p.y.is_finite())
    {
        return false;
    }
    if closed {
        let original_area = signed_area(&a);
        let candidate_area = signed_area(&b);
        if !original_area.is_finite()
            || !candidate_area.is_finite()
            || (original_area - candidate_area).abs() > (original_area.abs() * 0.015).max(0.5)
            || original_area * candidate_area < 0.0
        {
            return false;
        }
    }
    let Some(forward_mean) = directed_error(&a, &b, max) else {
        return false;
    };
    let Some(backward_mean) = directed_error(&b, &a, max) else {
        return false;
    };
    forward_mean <= mean && backward_mean <= mean
}

#[cfg(test)]
mod tests {
    use super::{GuardedContourPass, accepted};
    use vtracer::PointF64;
    use vtracer::fitter::FittedGeom;
    use vtracer::simplify::CurvePass;

    fn p(x: f64, y: f64) -> PointF64 {
        PointF64 { x, y }
    }

    fn line(y: f64) -> FittedGeom {
        FittedGeom::Beziers(vec![[p(0.0, y), p(10.0, y), p(20.0, y), p(30.0, y)]])
    }

    fn guard() -> GuardedContourPass {
        GuardedContourPass {
            simplify_tolerance: 2.5,
            corner_threshold_radians: std::f64::consts::PI / 3.0,
            line_aware: true,
            max_deviation: 0.6,
            max_mean_deviation: 0.2,
        }
    }

    #[test]
    fn rejects_shifted_or_malformed_contours() {
        let source = line(0.0);
        let moved = line(2.0);
        assert!(!accepted(&source, &moved, false, 0.6, 0.2));

        let FittedGeom::Beziers(mut broken) = line(0.0) else {
            unreachable!()
        };
        broken[0][1].y = f64::NAN;
        assert!(!accepted(
            &source,
            &FittedGeom::Beziers(broken),
            false,
            0.6,
            0.2
        ));
    }

    #[test]
    fn accepts_identical_and_small_control_point_repair() {
        let source = line(0.0);
        assert!(accepted(&source, &source, false, 0.6, 0.2));
        let FittedGeom::Beziers(mut slight) = line(0.0) else {
            unreachable!()
        };
        slight[0][1].y = 0.1;
        assert!(accepted(
            &source,
            &FittedGeom::Beziers(slight),
            false,
            0.6,
            0.2
        ));
    }

    #[test]
    fn rejects_area_change_on_closed_ring() {
        let ring = vec![
            [p(0.0, 0.0), p(10.0, 0.0), p(20.0, 0.0), p(30.0, 0.0)],
            [p(30.0, 0.0), p(30.0, 10.0), p(30.0, 20.0), p(30.0, 30.0)],
            [p(30.0, 30.0), p(20.0, 30.0), p(10.0, 30.0), p(0.0, 30.0)],
            [p(0.0, 30.0), p(0.0, 20.0), p(0.0, 10.0), p(0.0, 0.0)],
        ];
        let mut shifted = ring.clone();
        shifted[1][1].x = 33.0;
        shifted[1][2].x = 33.0;
        assert!(!accepted(
            &FittedGeom::Beziers(ring),
            &FittedGeom::Beziers(shifted),
            true,
            5.0,
            5.0,
        ));
    }

    #[test]
    fn unsafe_geometry_falls_back_to_original_when_guard_is_strict() {
        let original = vec![[p(0.0, 0.0), p(10.0, 0.4), p(20.0, -0.4), p(30.0, 0.0)]];
        let mut strict = guard();
        strict.simplify_tolerance = 0.0;
        strict.max_deviation = 0.0;
        strict.max_mean_deviation = 0.0;
        let FittedGeom::Beziers(result) = strict.open(FittedGeom::Beziers(original.clone())) else {
            unreachable!("curve kind changed");
        };
        assert_eq!(result, original, "unsafe straightening must roll back");
    }

    #[test]
    fn leaves_polygons_and_oversized_contours_unchanged() {
        let polygon = FittedGeom::Polyline(vec![p(0.0, 0.0), p(30.0, 0.0)]);
        match guard().open(polygon) {
            FittedGeom::Polyline(points) => assert_eq!(points.len(), 2),
            _ => unreachable!("polygon converted"),
        }
        let oversized =
            FittedGeom::Beziers(vec![
                [p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0), p(3.0, 0.0)];
                97
            ]);
        match guard().open(oversized) {
            FittedGeom::Beziers(curves) => assert_eq!(curves.len(), 97),
            _ => unreachable!("curve chain converted"),
        }
    }
}
