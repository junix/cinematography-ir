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

/// Monotone cubic (Fritsch–Carlson) interpolation of authored keyframe
/// channels, ported from lemo-opuscar `core/lib.js:18-42` (`monotone`),
/// where it backs `track()` for per-channel keyframe curves.
///
/// `keys` are `(x, y)` pairs with strictly increasing `x`. Outside the key
/// range the first/last value is held. Unlike plain cubic interpolation the
/// curve never overshoots a key: tangents at sign changes of the secant
/// slope are zeroed and endpoint tangents are limited to three times the
/// secant, so authored extremes stay extremes. An empty key set evaluates to
/// `t` unchanged; a single key is constant.
pub fn monotone_cubic(keys: &[(f32, f32)], t: f32) -> f32 {
    let n = keys.len();
    if n == 0 {
        return t;
    }
    if n == 1 {
        return keys[0].1;
    }
    let xs: Vec<f32> = keys.iter().map(|key| key.0).collect();
    let ys: Vec<f32> = keys.iter().map(|key| key.1).collect();
    // Secant slopes and initial tangents (one-sided at the ends).
    let mut d = vec![0.0f32; n - 1];
    for i in 0..n - 1 {
        d[i] = (ys[i + 1] - ys[i]) / (xs[i + 1] - xs[i]);
    }
    let mut m = vec![0.0f32; n];
    m[0] = d[0];
    m[n - 1] = d[n - 2];
    for i in 1..n - 1 {
        m[i] = if d[i - 1] * d[i] <= 0.0 {
            0.0
        } else {
            (d[i - 1] + d[i]) / 2.0
        };
    }
    // Fritsch–Carlson limiter: a tangent steeper than 3x its secant would
    // overshoot the segment, so scale the pair back to the safe circle.
    for i in 0..n - 1 {
        if d[i] == 0.0 {
            m[i] = 0.0;
            m[i + 1] = 0.0;
            continue;
        }
        let a = m[i] / d[i];
        let b = m[i + 1] / d[i];
        let s = a * a + b * b;
        if s > 9.0 {
            let k = 3.0 / s.sqrt();
            m[i] = k * a * d[i];
            m[i + 1] = k * b * d[i];
        }
    }
    if t <= xs[0] {
        return ys[0];
    }
    if t >= xs[n - 1] {
        return ys[n - 1];
    }
    let mut i = 0;
    while t > xs[i + 1] {
        i += 1;
    }
    let h = xs[i + 1] - xs[i];
    let u = (t - xs[i]) / h;
    let u2 = u * u;
    let u3 = u2 * u;
    (2.0 * u3 - 3.0 * u2 + 1.0) * ys[i]
        + (u3 - 2.0 * u2 + u) * h * m[i]
        + (-2.0 * u3 + 3.0 * u2) * ys[i + 1]
        + (u3 - u2) * h * m[i + 1]
}

#[cfg(test)]
#[path = "easing_tests.rs"]
mod tests;
