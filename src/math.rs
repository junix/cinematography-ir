//! Small, dependency-free linear algebra shared by the solver, views, and
//! adapters. Everything here honours the document's `CoordinateSystem`; no
//! module may assume a backend convention.

use std::ops::{Add, Mul, Neg, Sub};

pub use crate::model::Vec3;

use crate::model::{AxisName, CoordinateSystem, EulerDeg, Handedness, SignedAxis};

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, rhs: f32) -> Vec3 {
        Vec3 {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3 {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }

    pub fn dot(self, rhs: Vec3) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    pub fn cross(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }

    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// Unit vector, or `None` when the input is (numerically) zero.
    pub fn normalized(self) -> Option<Vec3> {
        let len = self.length();
        (len > 1e-6).then(|| self * (1.0 / len))
    }

    pub fn lerp(self, rhs: Vec3, t: f32) -> Vec3 {
        self + (rhs - self) * t
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

/// Column-major 3×3 matrix; `cols[i]` is the image of basis vector `i`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat3 {
    pub cols: [Vec3; 3],
}

impl Mat3 {
    pub const IDENTITY: Mat3 = Mat3 {
        cols: [
            Vec3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            Vec3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            Vec3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        ],
    };

    pub fn apply(&self, v: Vec3) -> Vec3 {
        self.cols[0] * v.x + self.cols[1] * v.y + self.cols[2] * v.z
    }

    pub fn mul(&self, rhs: &Mat3) -> Mat3 {
        Mat3 {
            cols: [
                self.apply(rhs.cols[0]),
                self.apply(rhs.cols[1]),
                self.apply(rhs.cols[2]),
            ],
        }
    }

    /// Rodrigues rotation about a unit axis by `radians` (right-hand rule).
    pub fn rotation(axis: Vec3, radians: f32) -> Mat3 {
        let (s, c) = radians.sin_cos();
        let rotate = |v: Vec3| v * c + axis.cross(v) * s + axis * (axis.dot(v) * (1.0 - c));
        Mat3 {
            cols: [
                rotate(Mat3::IDENTITY.cols[0]),
                rotate(Mat3::IDENTITY.cols[1]),
                rotate(Mat3::IDENTITY.cols[2]),
            ],
        }
    }
}

/// An orthonormal camera/subject frame in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Basis {
    pub right: Vec3,
    pub up: Vec3,
    pub forward: Vec3,
}

pub fn signed_axis_vec(axis: SignedAxis) -> Vec3 {
    match axis {
        SignedAxis::PositiveX => Vec3::new(1.0, 0.0, 0.0),
        SignedAxis::NegativeX => Vec3::new(-1.0, 0.0, 0.0),
        SignedAxis::PositiveY => Vec3::new(0.0, 1.0, 0.0),
        SignedAxis::NegativeY => Vec3::new(0.0, -1.0, 0.0),
        SignedAxis::PositiveZ => Vec3::new(0.0, 0.0, 1.0),
        SignedAxis::NegativeZ => Vec3::new(0.0, 0.0, -1.0),
    }
}

pub fn axis_name_vec(axis: AxisName) -> Vec3 {
    match axis {
        AxisName::X => Vec3::new(1.0, 0.0, 0.0),
        AxisName::Y => Vec3::new(0.0, 1.0, 0.0),
        AxisName::Z => Vec3::new(0.0, 0.0, 1.0),
    }
}

/// The frame of an object with identity rotation in this coordinate system.
pub fn identity_basis(cs: &CoordinateSystem) -> Basis {
    let up = axis_name_vec(cs.up_axis);
    let forward = signed_axis_vec(cs.forward_axis);
    let right = match cs.handedness {
        Handedness::Right => forward.cross(up),
        Handedness::Left => up.cross(forward),
    };
    Basis { right, up, forward }
}

/// Sign applied to rotation angles so that positive rotations follow the
/// physical right-hand rule in both handednesses. The numeric Rodrigues
/// formula is already the right-hand rule in right-handed coordinates; in a
/// left-handed frame the same numbers describe a left-hand-rule rotation, so
/// the angle is negated to keep `+yaw = turn left`, `+pitch = tilt up`.
pub fn rotation_sign(cs: &CoordinateSystem) -> f32 {
    match cs.handedness {
        Handedness::Right => 1.0,
        Handedness::Left => -1.0,
    }
}

/// World rotation for `euler`: yaw about up, then pitch about right, then
/// roll about forward (intrinsic), honouring handedness.
pub fn rotation_matrix(cs: &CoordinateSystem, euler: EulerDeg) -> Mat3 {
    let id = identity_basis(cs);
    let sign = rotation_sign(cs);
    let yaw = Mat3::rotation(id.up, sign * euler.yaw.to_radians());
    let pitch = Mat3::rotation(id.right, sign * euler.pitch.to_radians());
    let roll = Mat3::rotation(id.forward, sign * euler.roll.to_radians());
    yaw.mul(&pitch).mul(&roll)
}

/// The rotated frame of an object.
pub fn oriented_basis(cs: &CoordinateSystem, euler: EulerDeg) -> Basis {
    let id = identity_basis(cs);
    let m = rotation_matrix(cs, euler);
    Basis {
        right: m.apply(id.right),
        up: m.apply(id.up),
        forward: m.apply(id.forward),
    }
}

/// Yaw (about up) and pitch (elevation) that make `forward` point along
/// `direction`. Returns `None` for a zero direction. Roll is not inferred.
pub fn look_direction_to_yaw_pitch(cs: &CoordinateSystem, direction: Vec3) -> Option<(f32, f32)> {
    let id = identity_basis(cs);
    let d = direction.normalized()?;
    let sign = rotation_sign(cs);
    let elevation = d.dot(id.up).clamp(-1.0, 1.0);
    let horizontal = d - id.up * elevation;
    let yaw = match horizontal.normalized() {
        Some(h) => {
            let sin = id.forward.cross(h).dot(id.up);
            let cos = id.forward.dot(h);
            sign * sin.atan2(cos).to_degrees()
        }
        None => 0.0,
    };
    // Elevation is handedness-free: +pitch always tilts up.
    let pitch = elevation.asin().to_degrees();
    Some((yaw, pitch))
}

/// Horizontal direction obtained by rotating `forward` about up by `azimuth_deg`.
pub fn horizontal_direction(cs: &CoordinateSystem, azimuth_deg: f32) -> Vec3 {
    let id = identity_basis(cs);
    let sign = rotation_sign(cs);
    Mat3::rotation(id.up, sign * azimuth_deg.to_radians()).apply(id.forward)
}

/// Spherical decomposition of an offset (camera − target): radius, azimuth in
/// degrees measured from `forward` about `up`, and elevation in degrees.
pub fn to_spherical(cs: &CoordinateSystem, offset: Vec3) -> Option<(f32, f32, f32)> {
    let radius = offset.length();
    if radius <= 1e-6 {
        return None;
    }
    let (azimuth, elevation) = look_direction_to_yaw_pitch(cs, offset)?;
    Some((radius, azimuth, elevation))
}

/// Inverse of [`to_spherical`].
pub fn from_spherical(
    cs: &CoordinateSystem,
    radius: f32,
    azimuth_deg: f32,
    elevation_deg: f32,
) -> Vec3 {
    let id = identity_basis(cs);
    let h = horizontal_direction(cs, azimuth_deg);
    let (s, c) = elevation_deg.to_radians().sin_cos();
    (h * c + id.up * s) * radius
}

/// Which side of the vertical plane through `from → to` a point lies on:
/// `> 0` on the right of the direction of travel, `< 0` on the left,
/// `≈ 0` on the axis. Returns `None` when the axis is degenerate.
pub fn side_of_axis(cs: &CoordinateSystem, from: Vec3, to: Vec3, point: Vec3) -> Option<f32> {
    let id = identity_basis(cs);
    let along = to - from;
    let along_h = along - id.up * along.dot(id.up);
    let along_h = along_h.normalized()?;
    let right = match cs.handedness {
        Handedness::Right => along_h.cross(id.up),
        Handedness::Left => id.up.cross(along_h),
    };
    Some((point - from).dot(right))
}

/// Shortest-arc interpolation between two angles in degrees.
pub fn lerp_angle_deg(a: f32, b: f32, t: f32) -> f32 {
    let delta = wrap_deg(b - a);
    a + delta * t
}

/// Wraps an angle into `(-180, 180]`.
pub fn wrap_deg(value: f32) -> f32 {
    let mut v = value.rem_euclid(360.0);
    if v > 180.0 {
        v -= 360.0;
    }
    v
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Horizontal field of view in degrees for a rectilinear lens.
pub fn horizontal_fov_deg(focal_length_mm: f32, sensor_width_mm: f32) -> f32 {
    2.0 * (sensor_width_mm / (2.0 * focal_length_mm))
        .atan()
        .to_degrees()
}

/// Deterministic 64-bit hash (FNV-1a) for seeds and palette assignment.
pub fn stable_hash(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// SplitMix64 step: deterministic pseudo-random stream from a seed.
pub fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Uniform float in `[0, 1)` from a SplitMix64 output.
pub fn unit_float(bits: u64) -> f32 {
    ((bits >> 40) as f32) / ((1u64 << 24) as f32)
}

#[cfg(test)]
mod tests {
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
}
