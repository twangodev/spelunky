//! Fixed simulation timing, independent of presentation frequency.

use spelunky_core::TICK_RATE;

const MAX_CATCH_UP_TICKS: u32 = 8;
const ROUNDING_TOLERANCE: f64 = 1.0e-9;

#[derive(Debug, Default, Clone, Copy)]
pub struct FrameStep {
    pub ticks: u32,
    pub alpha: f64,
}

#[derive(Debug, Default)]
pub struct FixedClock {
    accumulator: f64,
}

impl FixedClock {
    pub fn new() -> Self {
        Self::default()
    }

    /// `elapsed` is seconds since the previous presentation frame. Excess time
    /// during a stall is discarded instead of causing an unbounded tick burst.
    pub fn advance(&mut self, elapsed: f64) -> FrameStep {
        let step = 1.0 / f64::from(TICK_RATE);
        if elapsed.is_finite() && elapsed > 0.0 {
            self.accumulator += elapsed.min(step * f64::from(MAX_CATCH_UP_TICKS));
        }
        let ticks =
            ((self.accumulator / step + ROUNDING_TOLERANCE).floor() as u32).min(MAX_CATCH_UP_TICKS);
        self.accumulator = (self.accumulator - f64::from(ticks) * step).max(0.0);
        FrameStep {
            ticks,
            alpha: (self.accumulator / step).clamp(0.0, 1.0 - f64::EPSILON),
        }
    }

    /// Call on focus changes, pausing, and browser suspension/resumption.
    pub fn reset(&mut self) {
        self.accumulator = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_rates_produce_identical_simulation_time() {
        for rate in [60, 120, 144, 240] {
            let mut clock = FixedClock::new();
            let mut ticks = 0;
            for _ in 0..rate * 60 {
                let frame = clock.advance(1.0 / f64::from(rate));
                ticks += frame.ticks;
                assert!((0.0..1.0).contains(&frame.alpha));
            }
            assert_eq!(ticks, TICK_RATE * 60, "presentation rate {rate}");
        }
    }

    #[test]
    fn irregular_frames_preserve_elapsed_time() {
        let mut clock = FixedClock::new();
        let mut total = 0;
        for _ in 0..100 {
            for elapsed in [0.001, 0.007, 0.019, 0.003, 0.070] {
                total += clock.advance(elapsed).ticks;
            }
        }
        assert_eq!(total, TICK_RATE * 10);
    }

    #[test]
    fn stalls_and_invalid_time_are_bounded() {
        let mut clock = FixedClock::new();
        assert_eq!(clock.advance(60.0).ticks, MAX_CATCH_UP_TICKS);
        for elapsed in [f64::NAN, f64::INFINITY, -1.0, 0.0] {
            let frame = clock.advance(elapsed);
            assert_eq!(frame.ticks, 0);
            assert!((0.0..1.0).contains(&frame.alpha));
        }
        clock.advance(0.01);
        clock.reset();
        assert_eq!(clock.advance(0.0).alpha, 0.0);
    }
}
