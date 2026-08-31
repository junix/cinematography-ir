//! Easing curves mapping normalised time `t ∈ [0, 1]` to progress `u ∈ [0, 1]`.

use crate::model::Easing;

/// Progress for `t` under `easing`. Inputs outside `[0, 1]` are clamped.
pub fn ease(easing: Easing, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match easing {
        Easing::Linear => t,
        Easing::EaseIn => t * t,
        Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        Easing::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
        Easing::SmoothStep => t * t * (3.0 - 2.0 * t),
        // `Hold` keeps the start value until the range ends, then snaps.
        Easing::Hold => {
            if t >= 1.0 {
                1.0
            } else {
                0.0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_fixed() {
        for easing in [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
            Easing::SmoothStep,
            Easing::Hold,
        ] {
            assert_eq!(ease(easing, 0.0), 0.0, "{easing:?}");
            assert_eq!(ease(easing, 1.0), 1.0, "{easing:?}");
        }
    }

    #[test]
    fn curves_are_monotonic() {
        for easing in [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
            Easing::SmoothStep,
        ] {
            let mut previous = 0.0;
            for step in 0..=100 {
                let value = ease(easing, step as f32 / 100.0);
                assert!(value >= previous - 1e-6, "{easing:?} at {step}");
                previous = value;
            }
        }
    }

    #[test]
    fn ease_in_out_is_symmetric() {
        assert!((ease(Easing::EaseInOut, 0.5) - 0.5).abs() < 1e-6);
        assert!((ease(Easing::EaseInOut, 0.25) + ease(Easing::EaseInOut, 0.75) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn inputs_outside_the_unit_interval_clamp() {
        for easing in [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
            Easing::SmoothStep,
            Easing::Hold,
        ] {
            assert_eq!(ease(easing, -1.0), 0.0, "{easing:?}");
            assert_eq!(ease(easing, 2.0), 1.0, "{easing:?}");
        }
    }

    #[test]
    fn hold_keeps_the_start_value_until_the_range_ends() {
        assert_eq!(ease(Easing::Hold, 0.5), 0.0);
        assert_eq!(ease(Easing::Hold, 0.999), 0.0);
        assert_eq!(ease(Easing::Hold, 1.0), 1.0);
    }

    #[test]
    fn analytic_curves_hit_their_exact_midpoints() {
        assert_eq!(ease(Easing::EaseIn, 0.5), 0.25);
        assert_eq!(ease(Easing::EaseOut, 0.5), 0.75);
        assert_eq!(ease(Easing::SmoothStep, 0.5), 0.5);
        assert_eq!(ease(Easing::EaseInOut, 0.25), 0.125);
        assert_eq!(ease(Easing::EaseInOut, 0.75), 0.875);
    }
}
