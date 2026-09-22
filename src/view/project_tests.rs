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
