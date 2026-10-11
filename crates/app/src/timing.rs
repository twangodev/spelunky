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

/// Selects presentation frames from the browser's animation-frame callbacks.
/// Deadlines retain their phase so a cap between two display rates can alternate
/// short and long frame intervals instead of falling to the lower divisor.
#[cfg(any(test, target_arch = "wasm32"))]
#[derive(Debug, Default)]
pub struct PresentationCadence {
    cap: Option<u32>,
    next_frame: Option<f64>,
    last_callback: Option<f64>,
}

#[cfg(any(test, target_arch = "wasm32"))]
impl PresentationCadence {
    pub fn ready(&mut self, now: f64, cap: Option<u32>) -> bool {
        if !now.is_finite() {
            return false;
        }
        // Zero is rejected by save validation; treat it as uncapped defensively.
        let cap = cap.filter(|cap| *cap > 0);
        if self.cap != cap || self.last_callback.is_some_and(|last| now < last) {
            self.next_frame = None;
        }
        self.cap = cap;
        self.last_callback = Some(now);
        let Some(cap) = cap else {
            self.next_frame = None;
            return true;
        };
        let interval = 1.0 / f64::from(cap);
        let Some(deadline) = self.next_frame else {
            self.next_frame = Some(now + interval);
            return true;
        };
        if now + ROUNDING_TOLERANCE < deadline {
            return false;
        }
        // A delayed callback presents once, skipping missed presentation slots.
        // FixedClock alone decides how many simulation ticks that time requires.
        let elapsed_slots = ((now - deadline + ROUNDING_TOLERANCE) / interval)
            .floor()
            .max(0.0);
        self.next_frame = Some(deadline + (elapsed_slots + 1.0) * interval);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_caps_select_raf_slots_without_changing_elapsed_simulation_time() {
        for refresh in [60, 120, 144, 240] {
            for cap in [None, Some(30), Some(60), Some(120), Some(144), Some(240)] {
                let mut cadence = PresentationCadence::default();
                let mut clock = FixedClock::new();
                let mut rendered = 0;
                let mut ticks = 0;
                let mut last_frame = 0.0;
                for callback in 0..=refresh * 10 {
                    let now = f64::from(callback) / f64::from(refresh);
                    if cadence.ready(now, cap) {
                        rendered += 1;
                        ticks += clock.advance(now - last_frame).ticks;
                        last_frame = now;
                    }
                }
                assert_eq!(
                    rendered,
                    cap.unwrap_or(refresh).min(refresh) * 10 + 1,
                    "{refresh}Hz RAF, cap {cap:?}"
                );
                assert_eq!(ticks, TICK_RATE * 10, "{refresh}Hz RAF, cap {cap:?}");
                assert_eq!(last_frame, 10.0);
            }
        }
    }

    #[test]
    fn browser_cap_changes_and_suspension_do_not_create_presentation_bursts() {
        let mut cadence = PresentationCadence::default();
        assert!(cadence.ready(1.0, Some(60))); // Startup is immediately presentable.
        assert!(!cadence.ready(1.001, Some(60))); // An expose/resize keeps the phase.
        assert!(cadence.ready(1.002, Some(144))); // Cap changes take effect immediately.
        assert!(!cadence.ready(1.003, Some(144)));
        assert!(cadence.ready(1.004, None));
        assert!(cadence.ready(1.005, None)); // Uncapped does not invent a 60Hz gate.
        assert!(cadence.ready(1.006, Some(240)));
        assert!(!cadence.ready(1.007, Some(240)));
        assert!(cadence.ready(90.0, Some(240))); // One render after a suspended tab.
        assert!(!cadence.ready(90.0, Some(240))); // Never replay missed render slots.
        assert!(!cadence.ready(f64::NAN, Some(240)));
        assert!(!cadence.ready(f64::INFINITY, Some(240)));
        assert!(cadence.ready(0.0, Some(240))); // A replaced monotonic epoch resets.
    }

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
