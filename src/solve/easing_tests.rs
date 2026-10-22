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

// `monotone_cubic` is the Fritsch–Carlson port of lemo-opuscar
// core/lib.js:18-42; these tests pin the properties that port preserved.

#[test]
fn monotone_cubic_hits_keys_and_holds_outside_the_range() {
    let keys = [(0.0, 0.0), (8.0, 0.0), (40.0, 4.0)];
    for (x, y) in keys {
        assert!((monotone_cubic(&keys, x) - y).abs() < 1e-6);
    }
    assert_eq!(monotone_cubic(&keys, -5.0), 0.0);
    assert_eq!(monotone_cubic(&keys, 100.0), 4.0);
    // A single key is constant; no keys leaves the input untouched.
    assert_eq!(monotone_cubic(&[(4.0, 2.5)], 0.0), 2.5);
    assert_eq!(monotone_cubic(&[(4.0, 2.5)], 9.0), 2.5);
    assert_eq!(monotone_cubic(&[], 0.3), 0.3);
}

#[test]
fn monotone_cubic_never_overshoots_the_authored_extremes() {
    // A V shape (up then down): linear or plain cubic interpolation would
    // push past the key at x=2; Fritsch–Carlson zeroes the tangent there.
    let keys = [(0.0, 0.0), (2.0, 1.0), (4.0, 0.0), (6.0, 1.0)];
    for step in 0..=600 {
        let x = step as f32 / 100.0;
        let y = monotone_cubic(&keys, x);
        assert!((0.0..=1.0).contains(&y), "overshoot {y} at {x}");
    }
    assert_eq!(monotone_cubic(&keys, 2.0), 1.0);
    // The tangent at the peak is zero, so the curve leaves it flat.
    let eps = 1e-3;
    assert!((monotone_cubic(&keys, 2.0 + eps) - 1.0).abs() < 1e-4);
    // Two keys with equal secant slope everywhere degrade to the line.
    let line = [(0.0, 0.0), (10.0, 5.0)];
    for step in 0..=100 {
        let x = step as f32 / 10.0;
        assert!((monotone_cubic(&line, x) - x / 2.0).abs() < 1e-5);
    }
}

#[test]
fn monotone_cubic_is_monotone_between_keys_of_one_sign() {
    // Strictly increasing keys: sampled values must never decrease.
    let keys = [(0.0, 0.0), (10.0, 1.0), (30.0, 9.0), (40.0, 9.5)];
    let mut previous = f32::NEG_INFINITY;
    for step in 0..=400 {
        let x = step as f32 / 10.0;
        let y = monotone_cubic(&keys, x);
        assert!(
            y >= previous - 1e-5,
            "not monotone at {x}: {y} < {previous}"
        );
        previous = y;
    }
}

#[test]
fn monotone_cubic_limits_steep_endpoint_tangents() {
    // Secants disagree wildly (d0 = 10, d1 = 1): the one-sided tangents
    // (10, 5.5, 1) are 5.5x the secant on the second segment, so the 3x
    // limiter must kick in there. With it every segment stays inside the
    // band spanned by its own keys.
    let keys = [(0.0, 0.0), (1.0, 10.0), (11.0, 20.0)];
    for step in 0..=1100 {
        let x = step as f32 / 100.0;
        let y = monotone_cubic(&keys, x);
        let expected = if x <= 1.0 { (0.0, 10.0) } else { (10.0, 20.0) };
        assert!(
            y >= expected.0 - 1e-4 && y <= expected.1 + 1e-4,
            "segment overshoot {y} at {x}"
        );
    }
}
