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
#[path = "easing_tests.rs"]
mod tests;

