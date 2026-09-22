use super::*;
use crate::model::Unit;

fn right_handed() -> CoordinateSystem {
    CoordinateSystem::default()
}

fn left_handed_unity() -> CoordinateSystem {
    CoordinateSystem {
        units: Unit::Meters,
        handedness: Handedness::Left,
        up_axis: AxisName::Y,
        forward_axis: SignedAxis::PositiveZ,
    }
}

fn approx(a: Vec3, b: Vec3) -> bool {
    (a - b).length() < 1e-4
}

#[test]
fn identity_basis_matches_opengl_and_unity() {
    let gl = identity_basis(&right_handed());
    assert!(approx(gl.right, Vec3::new(1.0, 0.0, 0.0)));
    assert!(approx(gl.forward, Vec3::new(0.0, 0.0, -1.0)));
    let unity = identity_basis(&left_handed_unity());
    assert!(approx(unity.right, Vec3::new(1.0, 0.0, 0.0)));
    assert!(approx(unity.forward, Vec3::new(0.0, 0.0, 1.0)));
}

#[test]
fn positive_yaw_turns_left_in_both_handednesses() {
    for cs in [right_handed(), left_handed_unity()] {
        let id = identity_basis(&cs);
        let turned = oriented_basis(
            &cs,
            EulerDeg {
                pitch: 0.0,
                yaw: 90.0,
                roll: 0.0,
            },
        );
        assert!(approx(turned.forward, -id.right), "{cs:?}");
    }
}

#[test]
fn positive_pitch_tilts_up_in_both_handednesses() {
    for cs in [right_handed(), left_handed_unity()] {
        let id = identity_basis(&cs);
        let tilted = oriented_basis(
            &cs,
            EulerDeg {
                pitch: 90.0,
                yaw: 0.0,
                roll: 0.0,
            },
        );
        assert!(approx(tilted.forward, id.up), "{cs:?}");
    }
}

#[test]
fn look_direction_round_trips_through_euler() {
    for cs in [right_handed(), left_handed_unity()] {
        for (dir, expect_yaw, expect_pitch) in [
            (Vec3::new(-1.0, 0.0, 0.0), 90.0, 0.0),
            (Vec3::new(0.0, 1.0, 0.0), 0.0, 90.0),
        ] {
            let (yaw, pitch) = look_direction_to_yaw_pitch(&cs, dir).unwrap();
            assert!((yaw - expect_yaw).abs() < 1e-3, "{cs:?} yaw {yaw}");
            assert!((pitch - expect_pitch).abs() < 1e-3, "{cs:?} pitch {pitch}");
            let basis = oriented_basis(
                &cs,
                EulerDeg {
                    pitch,
                    yaw,
                    roll: 0.0,
                },
            );
            assert!(
                approx(basis.forward, dir),
                "{cs:?} forward {:?}",
                basis.forward
            );
        }
        let diagonal = Vec3::new(0.3, 0.5, -0.8).normalized().unwrap();
        let (yaw, pitch) = look_direction_to_yaw_pitch(&cs, diagonal).unwrap();
        let basis = oriented_basis(
            &cs,
            EulerDeg {
                pitch,
                yaw,
                roll: 0.0,
            },
        );
        assert!(approx(basis.forward, diagonal), "{cs:?}");
    }
}

#[test]
fn spherical_round_trip() {
    let cs = right_handed();
    let offset = Vec3::new(1.2, 0.7, -2.5);
    let (r, az, el) = to_spherical(&cs, offset).unwrap();
    assert!(approx(from_spherical(&cs, r, az, el), offset));
}

#[test]
fn side_of_axis_is_positive_on_the_right() {
    let cs = right_handed();
    let from = Vec3::new(-1.5, 0.0, 0.0);
    let to = Vec3::new(1.5, 0.0, 0.0);
    // Facing +X in a Y-up right-handed system, +Z is to the right.
    assert!(side_of_axis(&cs, from, to, Vec3::new(0.0, 1.4, 5.0)).unwrap() > 0.0);
    assert!(side_of_axis(&cs, from, to, Vec3::new(0.0, 1.4, -5.0)).unwrap() < 0.0);
    assert!(side_of_axis(&cs, from, from, Vec3::ZERO).is_none());
}

#[test]
fn angle_helpers() {
    assert!((wrap_deg(190.0) + 170.0).abs() < 1e-5);
    assert!((lerp_angle_deg(350.0, 10.0, 0.5) - 360.0).abs() < 1e-4);
    assert!((horizontal_fov_deg(18.0, 36.0) - 90.0).abs() < 1e-4);
}

#[test]
fn hashes_are_stable() {
    assert_eq!(stable_hash("alice"), stable_hash("alice"));
    assert_ne!(stable_hash("alice"), stable_hash("bob"));
    let mut a = 7;
    let mut b = 7;
    assert_eq!(splitmix64(&mut a), splitmix64(&mut b));
}

#[test]
fn zero_directions_are_rejected_not_wrapped() {
    for cs in [right_handed(), left_handed_unity()] {
        assert!(
            look_direction_to_yaw_pitch(&cs, Vec3::ZERO).is_none(),
            "{cs:?}: a zero direction has no yaw/pitch"
        );
        assert!(
            to_spherical(&cs, Vec3::ZERO).is_none(),
            "{cs:?}: a zero offset has no spherical decomposition"
        );
    }
}

#[test]
fn looking_straight_up_keeps_yaw_at_zero() {
    for cs in [right_handed(), left_handed_unity()] {
        let id = identity_basis(&cs);
        let (yaw, pitch) = look_direction_to_yaw_pitch(&cs, id.up).unwrap();
        assert!(
            yaw.abs() < 1e-4,
            "{cs:?}: no horizontal component, so yaw stays 0: {yaw}"
        );
        assert!((pitch - 90.0).abs() < 1e-3, "{cs:?} pitch {pitch}");
    }
}

#[test]
fn wrap_deg_stays_in_the_minus_180_to_180_range() {
    // Documented range is (-180, 180]: the +180 boundary maps to itself
    // from both sides, and full turns collapse to zero.
    for (value, expected) in [
        (0.0, 0.0),
        (180.0, 180.0),
        (-180.0, 180.0),
        (540.0, 180.0),
        (-540.0, 180.0),
        (360.0, 0.0),
        (-360.0, 0.0),
        (190.0, -170.0),
        (-190.0, 170.0),
    ] {
        assert!(
            (wrap_deg(value) - expected).abs() < 1e-5,
            "wrap_deg({value}) != {expected}"
        );
    }
}

#[test]
fn lerp_angle_deg_reaches_both_endpoints_through_the_wrap() {
    assert_eq!(lerp_angle_deg(350.0, 10.0, 0.0), 350.0);
    // The output continues through the wrap without renormalising
    // (the midpoint is 360, not 0), so t=1 lands on 370 ≡ 10 (mod 360).
    let end = lerp_angle_deg(350.0, 10.0, 1.0);
    assert!((end - 370.0).abs() < 1e-4, "{end}");
    assert!((wrap_deg(end) - 10.0).abs() < 1e-4);
}

#[test]
fn normalized_rejects_zero_and_sub_threshold_vectors() {
    assert!(Vec3::ZERO.normalized().is_none());
    assert!(
        Vec3::new(1e-8, 0.0, 0.0).normalized().is_none(),
        "length 1e-8 is numerically zero"
    );
    let unit = Vec3::new(3.0, 4.0, 0.0).normalized().unwrap();
    assert!((unit.x - 0.6).abs() < 1e-6 && (unit.y - 0.8).abs() < 1e-6);
    assert!((unit.length() - 1.0).abs() < 1e-6);
}

#[test]
fn unit_float_maps_the_top_bits_into_the_half_open_unit_interval() {
    assert_eq!(unit_float(0), 0.0);
    // Only the top 24 bits count: bit 40 is the first step.
    assert_eq!(unit_float(1u64 << 40), 1.0 / (1u64 << 24) as f32);
    let largest = unit_float(u64::MAX);
    assert!(
        largest < 1.0 && (largest - 1.0).abs() < 1e-6,
        "the maximum input stays strictly below 1.0: {largest}"
    );
}

#[test]
fn horizontal_direction_quarter_turns_left_in_both_handednesses() {
    for cs in [right_handed(), left_handed_unity()] {
        let id = identity_basis(&cs);
        assert!(approx(horizontal_direction(&cs, 0.0), id.forward), "{cs:?}");
        assert!(
            approx(horizontal_direction(&cs, 90.0), -id.right),
            "{cs:?}: +90° azimuth turns left, matching +yaw"
        );
    }
}
