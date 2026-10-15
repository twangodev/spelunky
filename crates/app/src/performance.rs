//! Opt-in CPU frame diagnostics. Submission time is not GPU execution time.
use serde::Serialize;
use std::collections::VecDeque;

const WINDOW: usize = 240;

#[derive(Clone, Copy, Default, Serialize)]
pub struct RenderSample {
    pub acquire_ms: f64,
    pub prepare_upload_ms: f64,
    pub encode_present_ms: f64,
    pub draw_calls: usize,
    pub upload_bytes: usize,
}

#[derive(Clone, Copy, Default, Serialize)]
pub struct FrameSample {
    pub interval_ms: f64,
    pub update_ms: f64,
    pub scene_ms: f64,
    pub quads: usize,
    pub render: RenderSample,
}

#[derive(Clone, Copy, Default, Serialize)]
pub struct Distribution {
    pub mean: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
}

impl Distribution {
    fn of(values: &mut [f64]) -> Self {
        if values.is_empty() {
            return Self::default();
        }
        values.sort_by(f64::total_cmp);
        let percentile = |p: f64| values[(values.len() as f64 * p).ceil() as usize - 1];
        Self {
            mean: values.iter().sum::<f64>() / values.len() as f64,
            p95: percentile(0.95),
            p99: percentile(0.99),
            max: *values.last().unwrap(),
        }
    }
}

#[derive(Default, Serialize)]
pub struct Report {
    pub samples: usize,
    pub interval_ms: Distribution,
    pub update_ms: Distribution,
    pub scene_ms: Distribution,
    pub acquire_ms: Distribution,
    pub prepare_upload_ms: Distribution,
    pub encode_present_ms: Distribution,
    pub last: FrameSample,
}

#[derive(Default)]
pub struct Profiler {
    pub enabled: bool,
    samples: VecDeque<FrameSample>,
    previous: Option<f64>,
    next_report: f64,
    pub report: Report,
    pub overlay: String,
}

impl Profiler {
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
        self.samples.clear();
        self.previous = None;
        self.next_report = 0.0;
        self.report = Report::default();
        self.overlay.clear();
        #[cfg(target_arch = "wasm32")]
        if let Some(window) = web_sys::window() {
            let _ = js_sys::Reflect::set(
                &window,
                &"__spelunkyPerformance".into(),
                &wasm_bindgen::JsValue::NULL,
            );
        }
    }

    pub fn record(&mut self, started: f64, mut sample: FrameSample) {
        if !self.enabled || !started.is_finite() {
            return;
        }
        let Some(previous) = self.previous.replace(started) else {
            return;
        };
        if started <= previous {
            return;
        }
        sample.interval_ms = (started - previous) * 1000.0;
        if self.samples.len() == WINDOW {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
        if started < self.next_report {
            return;
        }
        self.next_report = started + 0.5;
        let distribution = |select: fn(&FrameSample) -> f64| {
            let mut values = [0.0; WINDOW];
            for (value, sample) in values.iter_mut().zip(&self.samples) {
                *value = select(sample);
            }
            Distribution::of(&mut values[..self.samples.len()])
        };
        self.report = Report {
            samples: self.samples.len(),
            interval_ms: distribution(|s| s.interval_ms),
            update_ms: distribution(|s| s.update_ms),
            scene_ms: distribution(|s| s.scene_ms),
            acquire_ms: distribution(|s| s.render.acquire_ms),
            prepare_upload_ms: distribution(|s| s.render.prepare_upload_ms),
            encode_present_ms: distribution(|s| s.render.encode_present_ms),
            last: sample,
        };
        use std::fmt::Write;
        self.overlay.clear();
        let _ = writeln!(self.overlay, "CPU MS          AVG   P95   P99");
        for (label, d) in [
            ("FRAME", self.report.interval_ms),
            ("UPDATE", self.report.update_ms),
            ("SCENE", self.report.scene_ms),
            ("ACQUIRE", self.report.acquire_ms),
            ("PREP/UPLOAD", self.report.prepare_upload_ms),
            ("ENCODE/PRESENT", self.report.encode_present_ms),
        ] {
            let _ = writeln!(
                self.overlay,
                "{label:<14} {:5.1} {:5.1} {:5.1}",
                d.mean, d.p95, d.p99
            );
        }
        let _ = write!(
            self.overlay,
            "{} QUADS  {} DRAWS  {} KB  F7 CLOSE",
            sample.quads,
            sample.render.draw_calls,
            sample.render.upload_bytes / 1024
        );
        #[cfg(target_arch = "wasm32")]
        if let Some(window) = web_sys::window()
            && let Ok(json) = serde_json::to_string(&self.report)
            && let Ok(value) = js_sys::JSON::parse(&json)
        {
            let _ = js_sys::Reflect::set(&window, &"__spelunkyPerformance".into(), &value);
        }
    }
}

pub struct Stamp {
    #[cfg(not(target_arch = "wasm32"))]
    start: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    start: f64,
}

impl Stamp {
    pub fn start(enabled: bool) -> Option<Self> {
        enabled.then(|| Self {
            #[cfg(not(target_arch = "wasm32"))]
            start: std::time::Instant::now(),
            #[cfg(target_arch = "wasm32")]
            start: browser_now(),
        })
    }

    pub fn elapsed_ms(stamp: &Option<Self>) -> f64 {
        stamp.as_ref().map_or(0.0, |stamp| {
            #[cfg(not(target_arch = "wasm32"))]
            {
                stamp.start.elapsed().as_secs_f64() * 1000.0
            }
            #[cfg(target_arch = "wasm32")]
            {
                (browser_now() - stamp.start).max(0.0)
            }
        })
    }
}

#[cfg(target_arch = "wasm32")]
fn browser_now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map_or(0.0, |p| p.now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_include_spikes_without_averaging_them_away() {
        let mut values = [1.0; 100];
        values[0] = 100.0;
        values[1] = 10.0;
        let d = Distribution::of(&mut values);
        assert_eq!((d.p95, d.p99, d.max), (1.0, 10.0, 100.0));
        assert_eq!(d.mean, 2.08);
        assert_eq!(Distribution::of(&mut []).mean, 0.0);
    }

    #[test]
    fn profiling_is_opt_in_bounded_and_resets_after_toggle() {
        let mut p = Profiler::default();
        p.record(1.0, FrameSample::default());
        assert!(p.samples.is_empty());
        p.toggle();
        for i in 0..500 {
            p.record(i as f64 / 240.0, FrameSample::default());
        }
        assert_eq!(p.samples.len(), WINDOW);
        assert!((p.report.interval_ms.mean - 1000.0 / 240.0).abs() < 1e-9);
        assert!(!p.overlay.is_empty());
        p.toggle();
        assert!(!p.enabled);
        assert!(p.samples.is_empty());
        assert!(p.overlay.is_empty());
        assert!(p.previous.is_none());
    }
}
