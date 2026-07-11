//! "Nice" axis ticks — a min/max generalization of the web `niceTicks`
//! (`packages/charts/src/palette.ts`): steps are 1/2/5×10ⁿ, chosen as the
//! smallest that covers the range in at most `target` steps.

/// Tick positions covering `min..=max` on a 1/2/5×10ⁿ step, aiming for
/// roughly `target` intervals. The first tick is at or below `min`, the last
/// at or above `max` (the web algorithm's zero-based behavior, extended to
/// arbitrary — including negative — ranges).
pub fn nice_ticks(min: f64, max: f64, target: usize) -> Vec<f64> {
    if !min.is_finite() || !max.is_finite() || max <= min {
        return vec![min, min + 1.0];
    }
    let raw = (max - min) / target.max(1) as f64;
    let mag = 10f64.powf(raw.log10().floor());
    let step = [1.0, 2.0, 5.0, 10.0]
        .iter()
        .map(|s| s * mag)
        .find(|s| *s >= raw)
        .unwrap_or(10.0 * mag);

    let mut ticks = Vec::new();
    let mut i = (min / step).floor() as i64;
    loop {
        let v = i as f64 * step;
        if v > max + step * 0.001 {
            break;
        }
        ticks.push(v);
        i += 1;
    }
    if ticks.last().is_none_or(|last| *last < max) {
        ticks.push(ticks.last().copied().unwrap_or(min) + step);
    }
    ticks
}

#[cfg(test)]
mod tests {
    use super::nice_ticks;

    fn assert_ticks(got: Vec<f64>, want: &[f64]) {
        assert_eq!(got.len(), want.len(), "got {got:?}, want {want:?}");
        for (g, w) in got.iter().zip(want) {
            assert!((g - w).abs() < 1e-9, "got {got:?}, want {want:?}");
        }
    }

    #[test]
    fn zero_to_100() {
        assert_ticks(
            nice_ticks(0.0, 100.0, 5),
            &[0.0, 20.0, 40.0, 60.0, 80.0, 100.0],
        );
        // target 4 → step 50 (first of 1/2/5×10ⁿ ≥ 25), web parity.
        assert_ticks(nice_ticks(0.0, 100.0, 4), &[0.0, 50.0, 100.0]);
    }

    #[test]
    fn zero_to_7_is_sensible() {
        // raw 1.75 → step 2; last tick covers the max.
        assert_ticks(nice_ticks(0.0, 7.0, 4), &[0.0, 2.0, 4.0, 6.0, 8.0]);
    }

    #[test]
    fn small_fractional_range() {
        // raw 0.18 → step 0.2 (first nice step ≥ raw).
        assert_ticks(nice_ticks(0.0, 0.9, 5), &[0.0, 0.2, 0.4, 0.6, 0.8, 1.0]);
        // Coarser target folds to a 0.5 step, still covering the max.
        assert_ticks(nice_ticks(0.0, 0.9, 4), &[0.0, 0.5, 1.0]);
    }

    #[test]
    fn negative_range() {
        assert_ticks(nice_ticks(-50.0, 100.0, 4), &[-50.0, 0.0, 50.0, 100.0]);
        assert_ticks(nice_ticks(-8.0, -1.0, 4), &[-8.0, -6.0, -4.0, -2.0, 0.0]);
    }

    #[test]
    fn degenerate_ranges() {
        assert_ticks(nice_ticks(0.0, 0.0, 4), &[0.0, 1.0]);
        assert_ticks(nice_ticks(5.0, 5.0, 4), &[5.0, 6.0]);
        // Inverted / non-finite input falls back without panicking.
        assert_eq!(nice_ticks(3.0, 1.0, 4).len(), 2);
        assert_eq!(nice_ticks(0.0, f64::INFINITY, 4).len(), 2);
    }
}
