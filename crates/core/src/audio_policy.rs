//! Platform-independent policy for streamed music playback.

use std::collections::HashSet;

/// Targets for the old and incoming tracks once the incoming stream is live.
pub fn waiting_crossfade_targets(
    waiting: bool,
    playing: bool,
    is_current: bool,
) -> Option<(f32, f32)> {
    (waiting && playing && is_current).then_some((0.0, 1.0))
}

/// A silent, zero-target track stays alive while its stream is still pending.
pub fn keep_music_track(waiting: bool, finished: bool, target: f32, gain: f32) -> bool {
    !finished && (waiting || target > 0.0 || gain > 0.0)
}

/// Cancel a pending stream when a newer request no longer names that resource.
pub fn should_cancel_waiting_music_track(
    waiting: bool,
    track_name: &str,
    requested: Option<&str>,
) -> bool {
    waiting && requested != Some(track_name)
}

/// Suppress RAF retries after browser playback rejection until an unlock event.
pub fn pending_music_start(
    requested: Option<(&'static str, bool, bool)>,
    current: Option<&'static str>,
    failed: &HashSet<&'static str>,
) -> Option<(&'static str, bool, bool)> {
    requested.filter(|(name, _, _)| current != Some(*name) && !failed.contains(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejected_fade_in_preserves_the_current_track() {
        let mut outgoing_target = 1.0;
        let mut incoming_target = 0.0;

        let targets = waiting_crossfade_targets(true, false, true);
        assert_eq!(targets, None, "a rejected stream must not start a fade");
        if let Some((outgoing, incoming)) = targets {
            outgoing_target = outgoing;
            incoming_target = incoming;
        }
        assert_eq!((outgoing_target, incoming_target), (1.0, 0.0));
    }

    #[test]
    fn successful_fade_in_starts_crossfade_after_playback() {
        let mut outgoing_target = 1.0;
        let mut incoming_target = 0.0;

        let targets = waiting_crossfade_targets(true, true, true);
        assert_eq!(targets, Some((0.0, 1.0)));
        if let Some((outgoing, incoming)) = targets {
            outgoing_target = outgoing;
            incoming_target = incoming;
        }
        assert_eq!((outgoing_target, incoming_target), (0.0, 1.0));

        assert_eq!(
            waiting_crossfade_targets(true, true, false),
            None,
            "a superseded request must not take over the crossfade"
        );
    }

    #[test]
    fn pending_zero_gain_music_track_is_kept_until_playback_or_rejection() {
        assert!(keep_music_track(true, false, 0.0, 0.0));
        assert!(
            !keep_music_track(true, true, 0.0, 0.0),
            "a rejected pending stream should be removed"
        );
    }

    #[test]
    fn newer_music_request_cancels_only_superseded_waiting_tracks() {
        assert!(should_cancel_waiting_music_track(
            true,
            "cave",
            Some("temple")
        ));
        assert!(should_cancel_waiting_music_track(true, "cave", None));
        assert!(!should_cancel_waiting_music_track(
            true,
            "cave",
            Some("cave")
        ));
        assert!(!should_cancel_waiting_music_track(
            false,
            "cave",
            Some("temple")
        ));
    }

    #[test]
    fn rejected_streamed_music_waits_for_an_unlock_retry() {
        let request = Some(("cave", true, false));
        let mut failed = HashSet::new();
        assert_eq!(pending_music_start(request, None, &failed), request);

        failed.insert("cave");
        assert_eq!(pending_music_start(request, None, &failed), None);

        // Audio::unlock clears this gate from a user-gesture handler.
        failed.clear();
        assert_eq!(pending_music_start(request, None, &failed), request);
    }
}
