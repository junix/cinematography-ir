//! Scene projection (ADR-1115 D4 stage 2): world → plan canvas (top-down)
//! and world → frame canvas (through the camera). Both honour the document's
//! `CoordinateSystem`; nothing here assumes Y-up or −Z-forward.

use crate::math::{horizontal_fov_deg, identity_basis, Basis, Vec3};
use crate::model::CoordinateSystem;
use crate::view::canvas::{Rect, Vec2};

/// Top-down projection: page-right = world right, page-up = world forward,
/// so an identity camera sits at the bottom looking up the page.
#[derive(Debug, Clone)]
pub struct PlanProjection {
    basis: Basis,
    /// Canvas units per metre.
    pub scale: f32,
    /// Canvas position of the world origin.
    pub origin: Vec2,
    pub width: f32,
    pub height: f32,
}

impl PlanProjection {
    /// Fits `points` into a `width × height` canvas with `margin` (fraction).
    /// The extent never drops below `min_extent_m` so tiny scenes stay legible.
    pub fn fit(
        cs: &CoordinateSystem,
        points: &[Vec3],
        width: f32,
        height: f32,
        margin: f32,
        min_extent_m: f32,
    ) -> PlanProjection {
        let basis = identity_basis(cs);
        let mut min = Vec2::new(f32::INFINITY, f32::INFINITY);
        let mut max = Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
        for p in points {
            let page = Vec2::new(p.dot(basis.right), -p.dot(basis.forward));
            min = Vec2::new(min.x.min(page.x), min.y.min(page.y));
            max = Vec2::new(max.x.max(page.x), max.y.max(page.y));
        }
        if !min.x.is_finite() {
            min = Vec2::new(-1.0, -1.0);
            max = Vec2::new(1.0, 1.0);
        }
        let center = Vec2::new((min.x + max.x) / 2.0, (min.y + max.y) / 2.0);
        let extent_x = (max.x - min.x).max(min_extent_m);
        let extent_y = (max.y - min.y).max(min_extent_m);
        let usable_w = width * (1.0 - 2.0 * margin);
        let usable_h = height * (1.0 - 2.0 * margin);
        let scale = (usable_w / extent_x).min(usable_h / extent_y);
        let origin = Vec2::new(
            width / 2.0 - center.x * scale,
            height / 2.0 - center.y * scale,
        );
        PlanProjection {
            basis,
            scale,
            origin,
            width,
            height,
        }
    }

    pub fn to_canvas(&self, world: Vec3) -> Vec2 {
        Vec2::new(
            self.origin.x + world.dot(self.basis.right) * self.scale,
            self.origin.y - world.dot(self.basis.forward) * self.scale,
        )
    }

    /// Page angle (degrees, y-down) of a world direction; `None` when the
    /// direction has no horizontal component.
    pub fn direction_deg(&self, world_dir: Vec3) -> Option<f32> {
        let page = Vec2::new(
            world_dir.dot(self.basis.right),
            -world_dir.dot(self.basis.forward),
        );
        (page.length() > 1e-5).then(|| page.angle_deg())
    }

    pub fn length(&self, metres: f32) -> f32 {
        metres * self.scale
    }

    /// World-space horizontal grid lines (every `step_m`) as canvas segments.
    pub fn grid_lines(&self, step_m: f32) -> Vec<(Vec2, Vec2)> {
        let mut lines = Vec::new();
        if self.scale <= 0.0 || step_m <= 0.0 {
            return lines;
        }
        let step = self.length(step_m);
        if step < 6.0 {
            return lines;
        }
        let first_x = self.origin.x.rem_euclid(step);
        let mut x = first_x;
        while x <= self.width {
            lines.push((Vec2::new(x, 0.0), Vec2::new(x, self.height)));
            x += step;
        }
        let first_y = self.origin.y.rem_euclid(step);
        let mut y = first_y;
        while y <= self.height {
            lines.push((Vec2::new(0.0, y), Vec2::new(self.width, y)));
            y += step;
        }
        lines
    }
}

/// Pinhole projection through the solved camera into a `width × height`
/// frame canvas with the given sensor aspect.
#[derive(Debug, Clone)]
pub struct FrameProjection {
    pub position: Vec3,
    pub basis: Basis,
    pub focal_length_mm: f32,
    pub sensor_width_mm: f32,
    pub aspect: f32,
    pub width: f32,
    pub height: f32,
}

/// Minimum depth (metres) a point must have in front of the lens.
const NEAR_M: f32 = 0.05;

impl FrameProjection {
    pub fn hfov_deg(&self) -> f32 {
        horizontal_fov_deg(self.focal_length_mm, self.sensor_width_mm)
    }

    /// Canvas position and depth of a world point; `None` when behind the lens.
    pub fn project(&self, world: Vec3) -> Option<(Vec2, f32)> {
        let d = world - self.position;
        let depth = d.dot(self.basis.forward);
        if depth < NEAR_M {
            return None;
        }
        let x = d.dot(self.basis.right) / depth;
        let y = d.dot(self.basis.up) / depth;
        let half_w = self.sensor_width_mm / (2.0 * self.focal_length_mm);
        let half_h = half_w / self.aspect;
        let ndc_x = x / half_w;
        let ndc_y = y / half_h;
        Some((
            Vec2::new(
                (ndc_x + 1.0) / 2.0 * self.width,
                (1.0 - ndc_y) / 2.0 * self.height,
            ),
            depth,
        ))
    }

    /// Screen bounding box of an oriented box standing on `base` with
    /// `dims` (width, height, depth) along the subject's `basis`, scaled by
    /// `scale`. Corners behind the lens are dropped; `None` when none are visible.
    pub fn project_box(
        &self,
        base: Vec3,
        dims: Vec3,
        subject: &Basis,
        scale: Vec3,
    ) -> Option<(Rect, f32)> {
        let half_w = dims.x * scale.x / 2.0;
        let half_d = dims.z * scale.z / 2.0;
        let height = dims.y * scale.y;
        let mut min = Vec2::new(f32::INFINITY, f32::INFINITY);
        let mut max = Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
        let mut nearest = f32::INFINITY;
        let mut visible = 0;
        for sx in [-1.0, 1.0] {
            for sz in [-1.0, 1.0] {
                for sy in [0.0, 1.0] {
                    let corner = base
                        + subject.right * (sx * half_w)
                        + subject.forward * (sz * half_d)
                        + subject.up * (sy * height);
                    if let Some((p, depth)) = self.project(corner) {
                        visible += 1;
                        min = Vec2::new(min.x.min(p.x), min.y.min(p.y));
                        max = Vec2::new(max.x.max(p.x), max.y.max(p.y));
                        nearest = nearest.min(depth);
                    }
                }
            }
        }
        (visible > 0).then(|| (Rect::new(min, max), nearest))
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(Vec2::new(0.0, 0.0), Vec2::new(self.width, self.height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::oriented_basis;
    use crate::model::{AxisName, EulerDeg, Handedness, SignedAxis, Unit};

    #[test]
    fn plan_puts_identity_camera_below_subjects() {
        let cs = CoordinateSystem::default();
        let camera = Vec3::new(0.0, 1.5, 5.0);
        let subject = Vec3::new(0.0, 0.0, 0.0);
        let plan = PlanProjection::fit(&cs, &[camera, subject], 400.0, 400.0, 0.1, 1.0);
        let c = plan.to_canvas(camera);
        let s = plan.to_canvas(subject);
        assert!(
            c.y > s.y,
            "camera (z=+5) must be lower on the page than the subject"
        );
        assert!((c.x - s.x).abs() < 1e-3);
        // Identity camera looks up the page: angle −90°.
        let angle = plan.direction_deg(Vec3::new(0.0, 0.0, -1.0)).unwrap();
        assert!((angle + 90.0).abs() < 1e-3, "{angle}");
        // +X is page-right.
        assert!(plan.direction_deg(Vec3::new(1.0, 0.0, 0.0)).unwrap().abs() < 1e-3);
    }

    #[test]
    fn plan_respects_a_z_up_system() {
        let cs = CoordinateSystem {
            units: Unit::Meters,
            handedness: Handedness::Right,
            up_axis: AxisName::Z,
            forward_axis: SignedAxis::PositiveY,
        };
        let plan = PlanProjection::fit(
            &cs,
            &[Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0)],
            400.0,
            400.0,
            0.1,
            1.0,
        );
        let angle = plan.direction_deg(Vec3::new(0.0, 1.0, 0.0)).unwrap();
        assert!(
            (angle + 90.0).abs() < 1e-3,
            "forward (+Y) points up the page: {angle}"
        );
        // Height (Z) must not influence the plan position.
        assert_eq!(
            plan.to_canvas(Vec3::new(0.0, 0.0, 0.0)),
            plan.to_canvas(Vec3::new(0.0, 0.0, 3.0))
        );
    }

    #[test]
    fn frame_projection_centres_a_point_on_axis() {
        let cs = CoordinateSystem::default();
        let frame = FrameProjection {
            position: Vec3::new(0.0, 1.5, 5.0),
            basis: oriented_basis(&cs, EulerDeg::default()),
            focal_length_mm: 50.0,
            sensor_width_mm: 36.0,
            aspect: 16.0 / 9.0,
            width: 400.0,
            height: 225.0,
        };
        let (p, depth) = frame.project(Vec3::new(0.0, 1.5, 0.0)).unwrap();
        assert!((p.x - 200.0).abs() < 1e-3 && (p.y - 112.5).abs() < 1e-3);
        assert!((depth - 5.0).abs() < 1e-4);
        assert!(
            frame.project(Vec3::new(0.0, 1.5, 6.0)).is_none(),
            "behind the lens"
        );
        // A point to the world-right (+X) lands on the right of the frame.
        let (right, _) = frame.project(Vec3::new(1.0, 1.5, 0.0)).unwrap();
        assert!(right.x > 200.0);
        // A point above the axis lands higher on the canvas (smaller y).
        let (above, _) = frame.project(Vec3::new(0.0, 2.5, 0.0)).unwrap();
        assert!(above.y < 112.5);
        // Horizontal edge of the sensor maps to the canvas edge.
        let half_w = 36.0 / 100.0 * 5.0;
        let (edge, _) = frame.project(Vec3::new(half_w, 1.5, 0.0)).unwrap();
        assert!((edge.x - 400.0).abs() < 1e-2, "{}", edge.x);
    }

    #[test]
    fn project_enforces_the_near_plane() {
        let cs = CoordinateSystem::default();
        let frame = FrameProjection {
            position: Vec3::new(0.0, 1.5, 5.0),
            basis: oriented_basis(&cs, EulerDeg::default()),
            focal_length_mm: 50.0,
            sensor_width_mm: 36.0,
            aspect: 16.0 / 9.0,
            width: 400.0,
            height: 225.0,
        };
        // NEAR_M is 0.05: 6 cm in front of the lens still projects, 4 cm does not.
        let just_in_front = frame.project(Vec3::new(0.0, 1.5, 5.0 - 0.06)).unwrap();
        assert!((just_in_front.1 - 0.06).abs() < 1e-5);
        assert!(
            frame.project(Vec3::new(0.0, 1.5, 5.0 - 0.04)).is_none(),
            "points inside the near plane are behind the lens"
        );
    }

    #[test]
    fn project_box_drops_corners_behind_the_lens() {
        let cs = CoordinateSystem::default();
        let frame = FrameProjection {
            position: Vec3::new(0.0, 1.5, 5.0),
            basis: oriented_basis(&cs, EulerDeg::default()),
            focal_length_mm: 50.0,
            sensor_width_mm: 36.0,
            aspect: 16.0 / 9.0,
            width: 400.0,
            height: 225.0,
        };
        let subject = oriented_basis(&cs, EulerDeg::default());
        let unit = Vec3::new(1.0, 1.0, 1.0);

        // A unit box centred on the optical axis: all eight corners visible,
        // the bounds are symmetric about the frame centre, and the returned
        // depth is the nearest corner (5 m − 0.5 m), not the box centre.
        let (rect, nearest) = frame
            .project_box(Vec3::new(0.0, 1.0, 0.0), unit, &subject, unit)
            .unwrap();
        assert!((rect.min.x + rect.max.x - 400.0).abs() < 1e-2);
        assert!((rect.min.y + rect.max.y - 225.0).abs() < 1e-2);
        assert!((nearest - 4.5).abs() < 1e-4, "{nearest}");

        // Base 0.4 m in front of the camera: the four near corners fall
        // behind the lens and must be dropped; the four survivors all sit at
        // 0.9 m depth, so the bounds stay symmetric.
        let (rect, nearest) = frame
            .project_box(Vec3::new(0.0, 1.0, 4.6), unit, &subject, unit)
            .unwrap();
        assert!((rect.min.x + rect.max.x - 400.0).abs() < 1e-2);
        assert!((rect.min.y + rect.max.y - 225.0).abs() < 1e-2);
        assert!((nearest - 0.9).abs() < 1e-4, "{nearest}");

        // Entirely behind the camera: no visible corner, no bounds.
        assert!(frame
            .project_box(Vec3::new(0.0, 1.0, 6.0), unit, &subject, unit)
            .is_none());
    }

    #[test]
    fn vertical_directions_have_no_page_angle() {
        let cs = CoordinateSystem::default();
        let plan = PlanProjection::fit(
            &cs,
            &[Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0)],
            400.0,
            400.0,
            0.1,
            1.0,
        );
        assert!(
            plan.direction_deg(Vec3::new(0.0, 9.0, 0.0)).is_none(),
            "straight up has no horizontal component"
        );
        assert!(plan.direction_deg(Vec3::new(0.0, -9.0, 0.0)).is_none());
        // A purely horizontal direction keeps its angle.
        assert!(plan.direction_deg(Vec3::new(1.0, 0.0, 0.0)).is_some());
    }

    #[test]
    fn fit_with_no_points_centers_the_origin() {
        let cs = CoordinateSystem::default();
        let plan = PlanProjection::fit(&cs, &[], 400.0, 400.0, 0.1, 1.0);
        assert_eq!(plan.to_canvas(Vec3::ZERO), Vec2::new(200.0, 200.0));
    }

    #[test]
    fn fit_fills_the_tighter_axis_and_respects_the_margin() {
        let cs = CoordinateSystem::default();
        // 10 m wide (x), 4 m deep (z): x is the tighter axis.
        let corners = [
            Vec3::new(-5.0, 0.0, -2.0),
            Vec3::new(5.0, 0.0, -2.0),
            Vec3::new(-5.0, 0.0, 2.0),
            Vec3::new(5.0, 0.0, 2.0),
        ];
        let margin = 0.1;
        let plan = PlanProjection::fit(&cs, &corners, 400.0, 400.0, margin, 1.0);
        // 400 px × 80% usable / 10 m = 32 px per metre on the constrained axis.
        assert!((plan.scale - 32.0).abs() < 1e-3, "{}", plan.scale);
        for corner in &corners {
            let page = plan.to_canvas(*corner);
            assert!(
                (40.0..=360.0).contains(&page.x) && (40.0..=360.0).contains(&page.y),
                "{corner:?} -> {page:?} escapes the 10% margin"
            );
        }
        // The x extremes span the usable width exactly; z spans 4 m × 32 px/m.
        let west = plan.to_canvas(corners[0]);
        let east = plan.to_canvas(corners[1]);
        assert!((east.x - west.x - 320.0).abs() < 1e-2);
        let ys: Vec<f32> = corners.iter().map(|c| plan.to_canvas(*c).y).collect();
        let span = ys.iter().cloned().fold(f32::MIN, f32::max)
            - ys.iter().cloned().fold(f32::MAX, f32::min);
        assert!((span - 4.0 * 32.0).abs() < 1e-2, "{span}");
    }
}
