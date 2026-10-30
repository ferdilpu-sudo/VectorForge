//! Optional, conservative straight-edge repair for fitted spline geometry.
//!
//! Unlike post-export SVG rewriting, this pass runs on fitted contours before
//! compositing. In cutout mode VTracer runs it once for each shared boundary,
//! so neighbouring regions reuse the exact same corrected geometry.
//!
//! It changes control points only: anchors, ring seams and junction endpoints
//! remain untouched. It is not a full primitive reconstruction algorithm.

use vtracer::PointF64;
use vtracer::fitter::FittedGeom;
use vtracer::simplify::CurvePass;

#[derive(Debug, Clone, Copy)]
pub(crate) struct PreserveStraightEdges {
    /// Maximum perpendicular displacement of each handle from its chord (px).
    pub(crate) tolerance: f64,
    /// Short edges remain unchanged to avoid flattening tiny organic details.
    pub(crate) min_length: f64,
}

impl CurvePass for PreserveStraightEdges {
    fn open(&self, geom: FittedGeom) -> FittedGeom {
        self.repair(geom)
    }

    fn ring(&self, geom: FittedGeom) -> FittedGeom {
        self.repair(geom)
    }
}

impl PreserveStraightEdges {
    fn repair(&self, geom: FittedGeom) -> FittedGeom {
        match geom {
            FittedGeom::Beziers(mut curves) => {
                for curve in &mut curves {
                    if is_nearly_straight(curve, self.tolerance, self.min_length) {
                        let start = curve[0];
                        let end = curve[3];
                        curve[1] = interpolate(start, end, 1.0 / 3.0);
                        curve[2] = interpolate(start, end, 2.0 / 3.0);
                    }
                }
                FittedGeom::Beziers(curves)
            }
            other => other,
        }
    }
}

fn interpolate(a: PointF64, b: PointF64, t: f64) -> PointF64 {
    PointF64 {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
    }
}

fn is_nearly_straight(curve: &[PointF64; 4], tolerance: f64, min_length: f64) -> bool {
    if !(tolerance.is_finite() && tolerance > 0.0) {
        return false;
    }
    if !curve.iter().all(|p| p.x.is_finite() && p.y.is_finite()) {
        return false;
    }
    let start = curve[0];
    let end = curve[3];
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_sq = dx * dx + dy * dy;
    if !length_sq.is_finite() || length_sq < min_length * min_length {
        return false;
    }
    let length = length_sq.sqrt();
    let mut previous_projection = 0.0;
    for handle in [curve[1], curve[2]] {
        let vx = handle.x - start.x;
        let vy = handle.y - start.y;
        let projection = (vx * dx + vy * dy) / length_sq;
        let distance = (vx * dy - vy * dx).abs() / length;
        // Guard against overshooting handles or loops near a straight chord.
        if !projection.is_finite()
            || projection < previous_projection
            || projection > 1.0
            || distance > tolerance
        {
            return false;
        }
        previous_projection = projection;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{PreserveStraightEdges, interpolate};
    use vtracer::PointF64;
    use vtracer::fitter::FittedGeom;
    use vtracer::simplify::CurvePass;

    fn p(x: f64, y: f64) -> PointF64 {
        PointF64 { x, y }
    }

    fn repair(curves: Vec<[PointF64; 4]>, closed: bool) -> Vec<[PointF64; 4]> {
        let pass = PreserveStraightEdges {
            tolerance: 0.6,
            min_length: 8.0,
        };
        let result = if closed {
            pass.ring(FittedGeom::Beziers(curves))
        } else {
            pass.open(FittedGeom::Beziers(curves))
        };
        match result {
            FittedGeom::Beziers(curves) => curves,
            _ => unreachable!("unexpected geometry kind"),
        }
    }

    #[test]
    fn fixes_almost_horizontal_edge_without_moving_endpoints() {
        let original = [p(5.0, 12.0), p(35.0, 12.4), p(66.0, 11.6), p(100.0, 12.0)];
        let out = repair(vec![original], false);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0][0], original[0]);
        assert_eq!(out[0][3], original[3]);
        assert_eq!(out[0][1], interpolate(original[0], original[3], 1.0 / 3.0));
        assert_eq!(out[0][2], interpolate(original[0], original[3], 2.0 / 3.0));
    }

    #[test]
    fn leaves_organic_curve_and_short_edges_unchanged() {
        let curved = [p(0.0, 0.0), p(10.0, 14.0), p(30.0, 14.0), p(40.0, 0.0)];
        let short = [p(0.0, 0.0), p(1.0, 0.1), p(2.0, -0.1), p(3.0, 0.0)];
        let out = repair(vec![curved, short], false);
        assert_eq!(out, vec![curved, short]);
    }

    #[test]
    fn keeps_corner_anchors_and_closed_ring_seam() {
        let top = [p(0.0, 0.0), p(10.0, 0.2), p(20.0, -0.2), p(30.0, 0.0)];
        let side = [p(30.0, 0.0), p(30.1, 10.0), p(29.9, 20.0), p(30.0, 30.0)];
        let bottom = [p(30.0, 30.0), p(20.0, 30.1), p(10.0, 29.9), p(0.0, 30.0)];
        let left = [p(0.0, 30.0), p(-0.1, 20.0), p(0.1, 10.0), p(0.0, 0.0)];
        let ring = vec![top, side, bottom, left];
        let out = repair(ring.clone(), true);
        assert_eq!(out.len(), 4);
        for (original, repaired) in ring.iter().zip(out.iter()) {
            assert_eq!(original[0], repaired[0]);
            assert_eq!(original[3], repaired[3]);
        }
        assert_eq!(out.first().unwrap()[0], out.last().unwrap()[3]);
    }

    #[test]
    fn rejects_handle_overshoot_and_nonfinite_coordinates() {
        let overshoot = [p(0.0, 0.0), p(50.0, 0.1), p(30.0, -0.1), p(40.0, 0.0)];
        let nonfinite = [p(0.0, 0.0), p(f64::NAN, 0.0), p(30.0, 0.0), p(40.0, 0.0)];
        assert_eq!(repair(vec![overshoot, nonfinite], false), vec![overshoot, nonfinite]);
    }

    #[test]
    fn passes_polygon_geometry_through_unchanged() {
        let line = vec![p(0.0, 0.0), p(20.0, 0.0), p(20.0, 10.0)];
        let pass = PreserveStraightEdges {
            tolerance: 0.6,
            min_length: 8.0,
        };
        match pass.open(FittedGeom::Polyline(line.clone())) {
            FittedGeom::Polyline(out) => assert_eq!(out, line),
            _ => unreachable!("polygon geometry was converted"),
        }
    }
}
