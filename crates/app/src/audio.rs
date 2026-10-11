//! Embedded Classic audio with nonfatal native and browser backends.
const CROSSFADE_SECONDS: f64 = 0.35;
const MAX_EFFECTS: usize = 48;

static ASSETS: &[(&str, &[u8])] = &[
    ("alert", include_bytes!("../../../assets/audio/alert.wav")),
    ("alien", include_bytes!("../../../assets/audio/alien.wav")),
    (
        "arrowtrap",
        include_bytes!("../../../assets/audio/arrowtrap.wav"),
    ),
    ("bat", include_bytes!("../../../assets/audio/bat.wav")),
    (
        "bigjump",
        include_bytes!("../../../assets/audio/bigjump.wav"),
    ),
    ("blink1", include_bytes!("../../../assets/audio/blink1.wav")),
    ("blink2", include_bytes!("../../../assets/audio/blink2.wav")),
    ("boing", include_bytes!("../../../assets/audio/boing.wav")),
    (
        "bombready",
        include_bytes!("../../../assets/audio/bombready.wav"),
    ),
    ("boss", include_bytes!("../../../assets/audio/boss.ogg")),
    (
        "bowpull",
        include_bytes!("../../../assets/audio/bowpull.wav"),
    ),
    ("break", include_bytes!("../../../assets/audio/break.wav")),
    ("cave", include_bytes!("../../../assets/audio/cave.ogg")),
    (
        "cavemandie",
        include_bytes!("../../../assets/audio/cavemandie.wav"),
    ),
    (
        "chestopen",
        include_bytes!("../../../assets/audio/chestopen.wav"),
    ),
    ("click", include_bytes!("../../../assets/audio/click.wav")),
    ("climb1", include_bytes!("../../../assets/audio/climb1.wav")),
    ("climb2", include_bytes!("../../../assets/audio/climb2.wav")),
    ("coin", include_bytes!("../../../assets/audio/coin.wav")),
    (
        "credits",
        include_bytes!("../../../assets/audio/credits.ogg"),
    ),
    ("crunch", include_bytes!("../../../assets/audio/crunch.wav")),
    ("damsel", include_bytes!("../../../assets/audio/damsel.wav")),
    ("die", include_bytes!("../../../assets/audio/die.wav")),
    (
        "explosion",
        include_bytes!("../../../assets/audio/explosion.wav"),
    ),
    ("flame", include_bytes!("../../../assets/audio/flame.wav")),
    ("frog", include_bytes!("../../../assets/audio/frog.wav")),
    ("gem", include_bytes!("../../../assets/audio/gem.wav")),
    ("ghost", include_bytes!("../../../assets/audio/ghost.wav")),
    (
        "giantspider",
        include_bytes!("../../../assets/audio/giantspider.wav"),
    ),
    (
        "gspiderjump",
        include_bytes!("../../../assets/audio/gspiderjump.wav"),
    ),
    ("hit", include_bytes!("../../../assets/audio/hit.wav")),
    ("hurt", include_bytes!("../../../assets/audio/hurt.wav")),
    ("ice", include_bytes!("../../../assets/audio/ice.ogg")),
    ("ignite", include_bytes!("../../../assets/audio/ignite.wav")),
    (
        "jetpack",
        include_bytes!("../../../assets/audio/jetpack.wav"),
    ),
    ("jump", include_bytes!("../../../assets/audio/jump.wav")),
    ("kiss", include_bytes!("../../../assets/audio/kiss.wav")),
    ("land", include_bytes!("../../../assets/audio/land.wav")),
    ("laser", include_bytes!("../../../assets/audio/laser.wav")),
    (
        "lasercharge",
        include_bytes!("../../../assets/audio/lasercharge.wav"),
    ),
    (
        "letsexplore",
        include_bytes!("../../../assets/audio/letsexplore.wav"),
    ),
    ("lush", include_bytes!("../../../assets/audio/lush.ogg")),
    (
        "mattockbreak",
        include_bytes!("../../../assets/audio/mattockbreak.wav"),
    ),
    ("monkey", include_bytes!("../../../assets/audio/monkey.wav")),
    ("pause", include_bytes!("../../../assets/audio/pause.wav")),
    ("pfall", include_bytes!("../../../assets/audio/pfall.wav")),
    ("pickup", include_bytes!("../../../assets/audio/pickup.wav")),
    (
        "psychic",
        include_bytes!("../../../assets/audio/psychic.wav"),
    ),
    ("push", include_bytes!("../../../assets/audio/push.wav")),
    (
        "shotgun",
        include_bytes!("../../../assets/audio/shotgun.wav"),
    ),
    ("slam", include_bytes!("../../../assets/audio/slam.wav")),
    (
        "smallexplode",
        include_bytes!("../../../assets/audio/smallexplode.wav"),
    ),
    (
        "spiderjump",
        include_bytes!("../../../assets/audio/spiderjump.wav"),
    ),
    ("splash", include_bytes!("../../../assets/audio/splash.wav")),
    ("steps", include_bytes!("../../../assets/audio/steps.wav")),
    (
        "teleport",
        include_bytes!("../../../assets/audio/teleport.wav"),
    ),
    ("temple", include_bytes!("../../../assets/audio/temple.ogg")),
    ("tfall", include_bytes!("../../../assets/audio/tfall.wav")),
    ("throw", include_bytes!("../../../assets/audio/throw.wav")),
    ("thud", include_bytes!("../../../assets/audio/thud.wav")),
    ("thump", include_bytes!("../../../assets/audio/thump.wav")),
    ("title", include_bytes!("../../../assets/audio/title.ogg")),
    ("trap", include_bytes!("../../../assets/audio/trap.wav")),
    (
        "victory",
        include_bytes!("../../../assets/audio/victory.ogg"),
    ),
    ("whip", include_bytes!("../../../assets/audio/whip.wav")),
    (
        "yetiyell",
        include_bytes!("../../../assets/audio/yetiyell.wav"),
    ),
    ("zombie", include_bytes!("../../../assets/audio/zombie.wav")),
];

fn asset(name: &str) -> Option<(&'static str, &'static [u8])> {
    let name = name.strip_prefix("global.").unwrap_or(name);
    let name = name
        .strip_prefix("snd")
        .or_else(|| name.strip_prefix("mus"))
        .unwrap_or(name);
    let name = name
        .strip_suffix(".wav")
        .or_else(|| name.strip_suffix(".ogg"))
        .unwrap_or(name);
    ASSETS
        .iter()
        .copied()
        .find(|(stem, _)| stem.eq_ignore_ascii_case(name))
}

fn volume(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
fn pan(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

pub use backend::Audio;

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::*;
    use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source, SpatialSink};
    use std::{collections::HashMap, io::Cursor, time::Instant};

    struct Track {
        name: &'static str,
        sink: Sink,
        gain: f32,
        target: f32,
    }
    struct Effect {
        name: &'static str,
        sink: SpatialSink,
    }
    pub struct Audio {
        stream: Option<OutputStream>,
        effects: Vec<Effect>,
        tracks: Vec<Track>,
        current: Option<&'static str>,
        music_volume: f32,
        effects_volume: f32,
        sound_pans: HashMap<&'static str, f32>,
        music_frequencies: HashMap<&'static str, u32>,
        paused: bool,
        last_update: Instant,
    }
    impl Default for Audio {
        fn default() -> Self {
            Self::new()
        }
    }
    impl Audio {
        pub fn new() -> Self {
            let stream = OutputStreamBuilder::open_default_stream()
                .ok()
                .map(|mut stream| {
                    stream.log_on_drop(false);
                    stream
                });
            Self {
                stream,
                effects: Vec::new(),
                tracks: Vec::new(),
                current: None,
                music_volume: 0.65,
                effects_volume: 0.8,
                sound_pans: HashMap::new(),
                music_frequencies: HashMap::new(),
                paused: false,
                last_update: Instant::now(),
            }
        }
        pub fn unlock(&mut self) {}
        pub fn play(&mut self, name: &str, position: f32) {
            let Some(mixer) = self.stream.as_ref().map(|stream| stream.mixer().clone()) else {
                return;
            };
            self.play_with_mixer(&mixer, name, position, false);
        }
        /// Source `SS_IsSoundPlaying` gate, based on actual mixer completion.
        /// Normal `play` remains free to overlap the same sound.
        pub fn play_if_idle(&mut self, name: &str, position: f32) {
            let Some(mixer) = self.stream.as_ref().map(|stream| stream.mixer().clone()) else {
                return;
            };
            self.play_with_mixer(&mixer, name, position, true);
        }
        fn play_with_mixer(
            &mut self,
            mixer: &rodio::mixer::Mixer,
            name: &str,
            position: f32,
            only_if_idle: bool,
        ) {
            if self.paused {
                return;
            }
            self.effects.retain(|effect| !effect.sink.empty());
            let Some((name, bytes)) = asset(name) else {
                return;
            };
            if only_if_idle && self.effects.iter().any(|effect| effect.name == name) {
                return;
            }
            let Ok(source) = Decoder::try_from(Cursor::new(bytes)) else {
                return;
            };
            if self.effects.len() >= MAX_EFFECTS {
                self.effects.remove(0).sink.stop();
            }
            let sink = SpatialSink::connect_new(
                mixer,
                [
                    self.sound_pans
                        .get(name)
                        .copied()
                        .unwrap_or_else(|| pan(position)),
                    0.0,
                    0.0,
                ],
                [-1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
            );
            sink.set_volume(self.effects_volume);
            sink.append(source);
            self.effects.push(Effect { name, sink });
        }
        /// Cancel matching effect voices only; music is a separate resource lane.
        pub fn stop_sound(&mut self, name: &str) {
            let Some((name, _)) = asset(name) else {
                return;
            };
            self.effects.retain(|effect| {
                if effect.name == name {
                    effect.sink.stop();
                    false
                } else {
                    true
                }
            });
        }
        /// Persist a named pan for subsequent starts. Existing voices retain
        /// their start pan; the source DLL's active-voice behavior is unverified.
        pub fn set_sound_pan(&mut self, name: &str, position: f32) {
            if let Some((name, _)) = asset(name) {
                self.sound_pans.insert(name, pan(position));
            }
        }
        /// Apply a positive source frequency only to matching music tracks,
        /// including the next start. Zero is outside this bounded contract.
        pub fn set_music_frequency(&mut self, name: &str, hz: u32) {
            let Some((name, _)) = asset(name) else {
                return;
            };
            if hz == 0 {
                return;
            }
            self.music_frequencies.insert(name, hz);
            for track in &self.tracks {
                if track.name == name {
                    track.sink.set_speed(hz as f32 / 44_100.0);
                }
            }
        }
        /// Empty name fades out. Repeating a selection never restarts a completed one-shot.
        pub fn music(&mut self, name: &str, looping: bool) {
            if name.is_empty() {
                self.current = None;
                for track in &mut self.tracks {
                    track.target = 0.0;
                }
                return;
            }
            let Some(mixer) = self.stream.as_ref().map(|stream| stream.mixer().clone()) else {
                return;
            };
            self.music_with_mixer(&mixer, name, looping);
        }
        fn music_with_mixer(&mut self, mixer: &rodio::mixer::Mixer, name: &str, looping: bool) {
            let Some((name, bytes)) = asset(name) else {
                return;
            };
            if self.current == Some(name) {
                return;
            }
            if looping {
                // Vorbis can expose an initial empty span. Source::repeat_infinite
                // buffers that span as EOF; the decoder loop reads through it
                // and rewinds the seekable embedded clip after real EOF.
                let Ok(source) = Decoder::builder()
                    .with_data(Cursor::new(bytes))
                    .with_byte_len(bytes.len() as u64)
                    .with_seekable(true)
                    .build_looped()
                else {
                    return;
                };
                self.start_music_source(mixer, name, source);
            } else {
                let Ok(source) = Decoder::try_from(Cursor::new(bytes)) else {
                    return;
                };
                self.start_music_source(mixer, name, source);
            }
        }
        fn start_music_source<S>(
            &mut self,
            mixer: &rodio::mixer::Mixer,
            name: &'static str,
            source: S,
        ) where
            S: Source + Send + 'static,
        {
            for track in &mut self.tracks {
                track.target = 0.0;
            }
            // Rapid transitions should not accumulate an unbounded number of decoders.
            while self.tracks.len() >= 3 {
                self.tracks.remove(0).sink.stop();
            }
            let sink = Sink::connect_new(mixer);
            sink.set_speed(
                self.music_frequencies.get(name).copied().unwrap_or(44_100) as f32 / 44_100.0,
            );
            sink.set_volume(0.0);
            sink.append(source);
            if self.paused {
                sink.pause();
            }
            self.tracks.push(Track {
                name,
                sink,
                gain: 0.0,
                target: 1.0,
            });
            self.current = Some(name);
        }
        /// Source `stopAllMusic`: stop every track immediately, including any
        /// outgoing crossfade. Effects, pause state, and volume are unaffected.
        pub fn stop_music(&mut self) {
            self.current = None;
            for track in self.tracks.drain(..) {
                track.sink.stop();
            }
        }
        pub fn set_volumes(&mut self, music: f32, effects: f32) {
            self.music_volume = volume(music);
            self.effects_volume = volume(effects);
            for track in &self.tracks {
                track.sink.set_volume(track.gain * self.music_volume);
            }
            for effect in &self.effects {
                effect.sink.set_volume(self.effects_volume);
            }
        }
        pub fn pause(&mut self, paused: bool) {
            self.paused = paused;
            self.last_update = Instant::now();
            for track in &self.tracks {
                if paused {
                    track.sink.pause();
                } else {
                    track.sink.play();
                }
            }
            for effect in &self.effects {
                if paused {
                    effect.sink.pause();
                } else {
                    effect.sink.play();
                }
            }
        }
        pub fn update(&mut self) {
            let now = Instant::now();
            let delta = now.duration_since(self.last_update).as_secs_f64();
            self.last_update = now;
            self.effects.retain(|effect| !effect.sink.empty());
            if self.paused {
                return;
            }
            let step = (delta / CROSSFADE_SECONDS) as f32;
            for track in &mut self.tracks {
                track.gain = if track.target > track.gain {
                    (track.gain + step).min(track.target)
                } else {
                    (track.gain - step).max(track.target)
                };
                track.sink.set_volume(track.gain * self.music_volume);
            }
            self.tracks.retain(|track| {
                let keep = track.target > 0.0 || track.gain > 0.0;
                if !keep {
                    track.sink.stop();
                }
                keep
            });
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn explicit_stop_ends_all_mixer_tracks_without_fading_or_stopping_effects() {
            for paused in [false, true] {
                let (mixer, mut output) = rodio::mixer::mixer(1, 44_100);
                let tracks = [1.0, 0.0]
                    .into_iter()
                    .map(|target| {
                        let sink = Sink::connect_new(&mixer);
                        sink.append(
                            rodio::buffer::SamplesBuffer::new(1, 44_100, vec![0.25; 1024])
                                .repeat_infinite(),
                        );
                        Track {
                            name: "title",
                            sink,
                            gain: 1.0,
                            target,
                        }
                    })
                    .collect();
                let (effects_mixer, mut effects_output) = rodio::mixer::mixer(2, 44_100);
                let effect = SpatialSink::connect_new(
                    &effects_mixer,
                    [0.0, 0.0, 0.0],
                    [-1.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                );
                effect.append(
                    rodio::buffer::SamplesBuffer::new(2, 44_100, vec![0.1; 1024]).repeat_infinite(),
                );
                let mut audio = Audio {
                    stream: None, // No host audio device is required for this test.
                    effects: vec![Effect {
                        name: "flame",
                        sink: effect,
                    }],
                    tracks,
                    current: Some("title"),
                    music_volume: 0.4,
                    effects_volume: 0.7,
                    sound_pans: HashMap::new(),
                    music_frequencies: HashMap::new(),
                    paused: false,
                    last_update: Instant::now(),
                };
                assert!(output.by_ref().take(512).any(|sample| sample > 0.0));
                audio.music("", true);
                assert_eq!(audio.tracks.len(), 2);
                assert!(output.by_ref().take(512).any(|sample| sample > 0.0));
                audio.pause(paused);
                audio.stop_music();
                assert!(audio.tracks.is_empty());
                assert_eq!(audio.current, None);
                assert_eq!(audio.paused, paused);
                assert_eq!((audio.music_volume, audio.effects_volume), (0.4, 0.7));
                assert_eq!(audio.effects.len(), 1);
                audio.stop_music(); // Idempotent even after an already silent boundary.
                audio.pause(false);
                audio.update();
                // Rodio applies stop controls on its next audio processing
                // block. Both infinite sources must end, with no 350 ms fade.
                assert!(output.by_ref().take(2048).count() < 2048);
                assert!(
                    effects_output
                        .by_ref()
                        .take(4096)
                        .any(|sample| sample > 0.0)
                );
            }
        }
        #[test]
        fn natural_victory_plays_once_while_loop_requests_continue_and_stop_can_rearm() {
            let mut game = spelunky_core::Game::shell(42, spelunky_core::RulesPreset::Classic);
            spelunky_core::ending::begin_custom(&mut game);
            for _ in 0..50 {
                game.step(&spelunky_core::InputFrame::default());
            }
            let ending = game.ending.as_ref().unwrap();
            assert_eq!(ending.music, "musVictory");
            assert!(!ending.music_looping);
            let (_, bytes) = asset(&ending.music).unwrap();
            let decoded = Decoder::try_from(Cursor::new(bytes)).unwrap();
            let channels = usize::from(decoded.channels());
            let sample_rate = decoded.sample_rate() as usize;
            // Drain beyond the actual decoded clip length, adapting to the
            // headless mixer's stereo 44100Hz format. No wall clock sleep.
            let frames = decoded.count() / channels;
            let budget =
                usize::try_from(frames as u64 * 88_200 / sample_rate as u64 + 16_384).unwrap();
            for looping in [ending.music_looping, true] {
                let (mixer, mut output) = rodio::mixer::mixer(2, 44_100);
                let mut audio = Audio {
                    stream: None,
                    effects: Vec::new(),
                    tracks: Vec::new(),
                    current: None,
                    music_volume: 0.65,
                    effects_volume: 0.8,
                    sound_pans: HashMap::new(),
                    music_frequencies: HashMap::new(),
                    paused: false,
                    last_update: Instant::now(),
                };
                audio.music_with_mixer(&mixer, &ending.music, looping);
                // Bypass only wall-clock crossfade progression in this
                // headless fixture; the actual decoder, queue and mixer run.
                audio.tracks[0].gain = 1.0;
                audio.tracks[0].sink.set_volume(audio.music_volume);
                let (consumed, audible) = output
                    .by_ref()
                    .take(budget)
                    .fold((0, false), |(count, audible), sample| {
                        (count + 1, audible || sample.abs() > 1e-8)
                    });
                assert_eq!(consumed, budget);
                assert!(audible, "the decoded clip must reach the mixer audibly");
                if looping {
                    assert!(!audio.tracks[0].sink.empty());
                    let tail_audible = (0..4096).fold(false, |audible, _| {
                        let sample = output.next().unwrap_or(0.0);
                        audible || sample.abs() > 1e-8
                    });
                    assert!(tail_audible, "looped PCM must continue after the first EOF");
                } else {
                    assert!(audio.tracks[0].sink.empty(), "one-shot music must end");
                    // A live Sink keeps its empty mixer queue alive with
                    // silence; clip completion does not imply mixer EOF.
                    assert!(output.by_ref().take(4096).all(|sample| sample == 0.0));
                }
                // Host polling must not replay a completed one-shot.
                audio.update();
                audio.music_with_mixer(&mixer, &ending.music, looping);
                assert_eq!(audio.tracks.len(), 1);
                if !looping {
                    assert!(audio.tracks[0].sink.empty());
                    assert!(output.by_ref().take(4096).all(|sample| sample == 0.0));
                }
                // Explicit cancellation clears the latch, so the same
                // one-shot can start on a later source request.
                audio.stop_music();
                assert!(output.by_ref().take(2048).count() < 2048);
                audio.music_with_mixer(&mixer, &ending.music, false);
                assert_eq!(audio.tracks.len(), 1);
                audio.tracks[0].gain = 1.0;
                audio.tracks[0].sink.set_volume(audio.music_volume);
                // Match CPAL's fixed sample pump: a pending stereo source
                // can return None for one channel-alignment sample.
                let audible = (0..budget).fold(false, |audible, _| {
                    let sample = output.next().unwrap_or(0.0);
                    audible || sample.abs() > 1e-8
                });
                assert!(audible, "an explicit stop must rearm audible playback");
                assert!(audio.tracks[0].sink.empty());
                audio.stop_music();
            }
        }

        #[test]
        fn idle_gate_uses_clip_completion_and_aliases_while_normal_play_overlaps() {
            let (mixer, mut output) = rodio::mixer::mixer(2, 44_100);
            let mut audio = Audio {
                stream: None,
                effects: Vec::new(),
                tracks: Vec::new(),
                current: None,
                music_volume: 0.65,
                effects_volume: 0.8,
                sound_pans: HashMap::new(),
                music_frequencies: HashMap::new(),
                paused: false,
                last_update: Instant::now(),
            };
            audio.play_with_mixer(&mixer, "sndFlame", 0.0, true);
            assert_eq!(audio.effects.len(), 1);
            for alias in ["flame", "FLAME.wav", "global.sndFlame", "sndFlame"] {
                audio.play_with_mixer(&mixer, alias, 0.0, true);
                assert_eq!(audio.effects.len(), 1, "duplicate {alias}");
            }
            audio.play_with_mixer(&mixer, "global.sndFlame", 0.0, false);
            assert_eq!(audio.effects.len(), 2, "normal playback must still overlap");
            assert!(output.by_ref().take(4096).any(|sample| sample.abs() > 1e-8));
            // Consume decoded audio until the mixer reports actual completion;
            // no simulation-tick duration or fixed retrigger interval is used.
            for _ in 0..88_200 {
                output.next();
                if audio.effects.iter().all(|effect| effect.sink.empty()) {
                    break;
                }
            }
            assert!(audio.effects.iter().all(|effect| effect.sink.empty()));
            audio.play_with_mixer(&mixer, "flame.wav", 0.0, true);
            assert_eq!(audio.effects.len(), 1);
            assert_eq!(audio.effects[0].name, "flame");
            assert!(output.by_ref().take(4096).any(|sample| sample.abs() > 1e-8));
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use super::*;
    use std::{
        cell::RefCell,
        collections::{HashMap, HashSet, VecDeque},
        rc::Rc,
    };
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::{JsFuture, spawn_local};
    use web_sys::{
        AudioBuffer, AudioBufferSourceNode, AudioContext, AudioScheduledSourceNode, GainNode,
        StereoPannerNode,
    };

    #[derive(Default)]
    struct Cache {
        buffers: HashMap<&'static str, AudioBuffer>,
        pending: HashSet<&'static str>,
        failed: HashSet<&'static str>,
    }
    struct Effect {
        name: &'static str,
        source: AudioBufferSourceNode,
        gain: GainNode,
        panner: StereoPannerNode,
        end: f64,
    }
    struct Track {
        name: &'static str,
        source: AudioBufferSourceNode,
        gain_node: GainNode,
        gain: f32,
        target: f32,
    }
    pub struct Audio {
        context: Option<AudioContext>,
        cache: Rc<RefCell<Cache>>,
        queued: VecDeque<(&'static str, f32)>,
        effects: Vec<Effect>,
        tracks: Vec<Track>,
        requested_music: Option<(&'static str, bool)>,
        current_music: Option<&'static str>,
        music_volume: f32,
        effects_volume: f32,
        sound_pans: HashMap<&'static str, f32>,
        music_frequencies: HashMap<&'static str, u32>,
        unlocked: bool,
        paused: bool,
        last_update: f64,
    }
    impl Default for Audio {
        fn default() -> Self {
            Self::new()
        }
    }
    impl Audio {
        pub fn new() -> Self {
            Self {
                context: AudioContext::new().ok(),
                cache: Rc::default(),
                queued: VecDeque::new(),
                effects: Vec::new(),
                tracks: Vec::new(),
                requested_music: None,
                current_music: None,
                music_volume: 0.65,
                effects_volume: 0.8,
                sound_pans: HashMap::new(),
                music_frequencies: HashMap::new(),
                unlocked: false,
                paused: false,
                last_update: 0.0,
            }
        }
        /// Call synchronously inside a keyboard, pointer, or controller gesture.
        pub fn unlock(&mut self) {
            if self.context.is_none() {
                self.context = AudioContext::new().ok();
            }
            if let Some(context) = &self.context {
                if !self.paused {
                    if let Ok(promise) = context.resume() {
                        spawn_local(async move {
                            let _ = JsFuture::from(promise).await;
                        });
                    }
                    self.unlocked = true;
                }
            }
        }
        fn decode(&self, name: &'static str) {
            let Some(context) = &self.context else {
                return;
            };
            let mut cache = self.cache.borrow_mut();
            if cache.buffers.contains_key(name)
                || cache.pending.contains(name)
                || cache.failed.contains(name)
            {
                return;
            }
            let Some((_, bytes)) = asset(name) else {
                return;
            };
            let array = js_sys::Uint8Array::from(bytes);
            let Ok(promise) = context.decode_audio_data(&array.buffer()) else {
                cache.failed.insert(name);
                return;
            };
            cache.pending.insert(name);
            let cache = self.cache.clone();
            spawn_local(async move {
                let result = JsFuture::from(promise)
                    .await
                    .and_then(|value| value.dyn_into::<AudioBuffer>());
                let mut cache = cache.borrow_mut();
                cache.pending.remove(name);
                match result {
                    Ok(buffer) => {
                        cache.buffers.insert(name, buffer);
                    }
                    Err(_) => {
                        cache.failed.insert(name);
                    }
                }
            });
        }
        pub fn play(&mut self, name: &str, position: f32) {
            if self.paused {
                return;
            }
            let Some((name, _)) = asset(name) else {
                return;
            };
            self.decode(name);
            if self.queued.len() >= MAX_EFFECTS {
                self.queued.pop_front();
            }
            self.queued.push_back((name, pan(position)));
        }
        /// Gate on the audio clock and pending playback, so asynchronous decode
        /// cannot turn repeated per-tick requests into a burst of duplicates.
        pub fn play_if_idle(&mut self, name: &str, position: f32) {
            let Some((name, _)) = asset(name) else {
                return;
            };
            let now = self
                .context
                .as_ref()
                .map_or(0.0, AudioContext::current_time);
            if self.queued.iter().any(|(queued, _)| *queued == name)
                || self
                    .effects
                    .iter()
                    .any(|effect| effect.name == name && effect.end > now)
            {
                return;
            }
            self.play(name, position);
        }
        /// Clearing queued playback prevents a pending decode from resurrecting
        /// the effect. Decode completion only populates the reusable buffer cache.
        pub fn stop_sound(&mut self, name: &str) {
            let Some((name, _)) = asset(name) else {
                return;
            };
            self.queued.retain(|(queued, _)| *queued != name);
            self.effects.retain(|effect| {
                if effect.name == name {
                    effect.gain.gain().set_value(0.0);
                    let _ = AudioScheduledSourceNode::stop(effect.source.unchecked_ref());
                    let _ = effect.source.disconnect();
                    let _ = effect.panner.disconnect();
                    let _ = effect.gain.disconnect();
                    false
                } else {
                    true
                }
            });
        }
        /// Persist a named pan for later and queued starts. Existing voices
        /// retain their start pan pending source DLL evidence.
        pub fn set_sound_pan(&mut self, name: &str, position: f32) {
            if let Some((name, _)) = asset(name) {
                self.sound_pans.insert(name, pan(position));
            }
        }
        /// Persist positive source frequency through deferred music decoding.
        pub fn set_music_frequency(&mut self, name: &str, hz: u32) {
            let Some((name, _)) = asset(name) else {
                return;
            };
            if hz == 0 {
                return;
            }
            self.music_frequencies.insert(name, hz);
            for track in &self.tracks {
                if track.name == name {
                    track.source.playback_rate().set_value(hz as f32 / 44_100.0);
                }
            }
        }
        pub fn music(&mut self, name: &str, looping: bool) {
            if name.is_empty() {
                self.requested_music = None;
                self.current_music = None;
                for track in &mut self.tracks {
                    track.target = 0.0;
                }
            } else if let Some((name, _)) = asset(name) {
                self.requested_music = Some((name, looping));
                self.decode(name);
            }
        }
        /// Source `stopAllMusic`, without waiting for a presentation-driven
        /// fade. Clearing the request prevents a pending decode from restarting
        /// music after this boundary.
        pub fn stop_music(&mut self) {
            self.requested_music = None;
            self.current_music = None;
            for track in self.tracks.drain(..) {
                track.gain_node.gain().set_value(0.0);
                let _ = AudioScheduledSourceNode::stop(track.source.unchecked_ref());
                let _ = track.source.disconnect();
                let _ = track.gain_node.disconnect();
            }
        }
        pub fn set_volumes(&mut self, music: f32, effects: f32) {
            self.music_volume = volume(music);
            self.effects_volume = volume(effects);
            for track in &self.tracks {
                track
                    .gain_node
                    .gain()
                    .set_value(track.gain * self.music_volume);
            }
            for effect in &self.effects {
                effect.gain.gain().set_value(self.effects_volume);
            }
        }
        pub fn pause(&mut self, paused: bool) {
            if self.paused == paused {
                return;
            }
            self.paused = paused;
            if let Some(context) = &self.context {
                self.last_update = context.current_time();
                let result = if paused {
                    context.suspend()
                } else if self.unlocked {
                    context.resume()
                } else {
                    return;
                };
                if let Ok(promise) = result {
                    spawn_local(async move {
                        let _ = JsFuture::from(promise).await;
                    });
                }
            }
        }
        fn start_effect(&mut self, name: &'static str, position: f32) {
            let Some(context) = &self.context else {
                return;
            };
            let Some(buffer) = self.cache.borrow().buffers.get(name).cloned() else {
                return;
            };
            let (Ok(source), Ok(gain), Ok(panner)) = (
                context.create_buffer_source(),
                context.create_gain(),
                context.create_stereo_panner(),
            ) else {
                return;
            };
            source.set_buffer(Some(&buffer));
            gain.gain().set_value(self.effects_volume);
            panner
                .pan()
                .set_value(self.sound_pans.get(name).copied().unwrap_or(position));
            if source.connect_with_audio_node(&panner).is_err()
                || panner.connect_with_audio_node(&gain).is_err()
                || gain
                    .connect_with_audio_node(&context.destination())
                    .is_err()
                || source.start().is_err()
            {
                return;
            }
            if self.effects.len() >= MAX_EFFECTS {
                let old = self.effects.remove(0);
                let _ = AudioScheduledSourceNode::stop(old.source.unchecked_ref());
                let _ = old.gain.disconnect();
                let _ = old.panner.disconnect();
            }
            self.effects.push(Effect {
                name,
                source,
                gain,
                panner,
                end: context.current_time() + buffer.duration(),
            });
        }
        fn start_music(&mut self, name: &'static str, looping: bool) {
            let Some(context) = &self.context else {
                return;
            };
            let Some(buffer) = self.cache.borrow().buffers.get(name).cloned() else {
                return;
            };
            let (Ok(source), Ok(gain_node)) =
                (context.create_buffer_source(), context.create_gain())
            else {
                return;
            };
            source.set_buffer(Some(&buffer));
            source.set_loop(looping);
            source.playback_rate().set_value(
                self.music_frequencies.get(name).copied().unwrap_or(44_100) as f32 / 44_100.0,
            );
            gain_node.gain().set_value(0.0);
            if source.connect_with_audio_node(&gain_node).is_err()
                || gain_node
                    .connect_with_audio_node(&context.destination())
                    .is_err()
                || source.start().is_err()
            {
                return;
            }
            for track in &mut self.tracks {
                track.target = 0.0;
            }
            while self.tracks.len() >= 3 {
                let old = self.tracks.remove(0);
                let _ = AudioScheduledSourceNode::stop(old.source.unchecked_ref());
                let _ = old.gain_node.disconnect();
            }
            self.tracks.push(Track {
                name,
                source,
                gain_node,
                gain: 0.0,
                target: 1.0,
            });
            self.current_music = Some(name);
        }
        pub fn update(&mut self) {
            let Some(context) = &self.context else {
                return;
            };
            let now = context.current_time();
            let delta = (now - self.last_update).max(0.0);
            self.last_update = now;
            if self.paused || !self.unlocked {
                return;
            }
            self.effects.retain(|effect| {
                if effect.end > now {
                    true
                } else {
                    let _ = effect.source.disconnect();
                    let _ = effect.panner.disconnect();
                    let _ = effect.gain.disconnect();
                    false
                }
            });
            if let Some((name, looping)) = self.requested_music {
                if self.current_music != Some(name) {
                    self.decode(name);
                    self.start_music(name, looping);
                }
            }
            let count = self.queued.len();
            for _ in 0..count {
                if let Some((name, position)) = self.queued.pop_front() {
                    if self.cache.borrow().buffers.contains_key(name) {
                        self.start_effect(name, position);
                    } else if !self.cache.borrow().failed.contains(name) {
                        self.decode(name);
                        self.queued.push_back((name, position));
                    }
                }
            }
            let step = (delta / CROSSFADE_SECONDS) as f32;
            for track in &mut self.tracks {
                track.gain = if track.target > track.gain {
                    (track.gain + step).min(track.target)
                } else {
                    (track.gain - step).max(track.target)
                };
                track
                    .gain_node
                    .gain()
                    .set_value(track.gain * self.music_volume);
            }
            self.tracks.retain(|track| {
                let keep = track.target > 0.0 || track.gain > 0.0;
                if !keep {
                    let _ = AudioScheduledSourceNode::stop(track.source.unchecked_ref());
                    let _ = track.gain_node.disconnect();
                }
                keep
            });
        }
    }
    impl Drop for Audio {
        fn drop(&mut self) {
            for track in &self.tracks {
                let _ = AudioScheduledSourceNode::stop(track.source.unchecked_ref());
            }
            for effect in &self.effects {
                let _ = AudioScheduledSourceNode::stop(effect.source.unchecked_ref());
            }
            if let Some(context) = &self.context {
                let _ = context.close();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_sound_and_music_aliases_resolve_to_embedded_assets() {
        let source =
            include_str!("../../../references/classic-1.1/Objects/oMusic.events/Create.xml");
        let mut count = 0;
        for line in source
            .lines()
            .filter(|line| line.contains("= SS_LoadSound("))
        {
            let alias = line.split('=').next().unwrap().trim();
            let filename = line
                .split('"')
                .nth(1)
                .unwrap()
                .split('/')
                .next_back()
                .unwrap();
            let (stem, bytes) = asset(alias).unwrap_or_else(|| panic!("missing {alias}"));
            assert_eq!(stem, filename.split('.').next().unwrap());
            assert!(!bytes.is_empty());
            count += 1;
        }
        assert!(count >= 60);
        assert!(asset("unknown").is_none());
    }
    #[test]
    fn sanitizes_public_volume_and_pan_values() {
        assert_eq!(volume(f32::NAN), 0.0);
        assert_eq!(volume(9.0), 1.0);
        assert_eq!(pan(f32::INFINITY), 0.0);
        assert_eq!(pan(-9.0), -1.0);
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_decoder_accepts_every_original_asset() {
        for (name, bytes) in ASSETS {
            let mut source = rodio::Decoder::try_from(std::io::Cursor::new(*bytes))
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            assert!(source.next().is_some(), "empty {name}");
        }
    }
}
