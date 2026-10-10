//! Classic's scripted sprite assignment and automatic animation arithmetic.
//!
//! Sprite assignment does not restart a valid image index. Automatic image
//! advancement belongs to the engine's endframe pass, after Step and collision
//! events, rather than to an actor's AI or movement controller.

/// Assign `sprite_index`, preserving an index valid in the newly selected
/// sprite. This also applies when a script assigns the current sprite again.
/// Explicit source `image_index` writes remain separate operations.
pub fn assign(sprite: &mut String, image_index: &mut f64, next: &str) {
    let frames = crate::catalog::sprite_frame_count(next) as f64;
    if frames <= image_index.floor() {
        *image_index = 0.0;
    }
    if sprite != next {
        sprite.clear();
        sprite.push_str(next);
    }
}

/// Advance an activated instance once at endframe. Returns whether the caller
/// should dispatch Animation End. The canonical runner subtracts the frame
/// count once before the callback; it does not reduce the index modulo the
/// count, even when a script selected an unusually large image speed.
pub fn advance(image_index: &mut f64, image_speed: f64, frames: usize) -> bool {
    *image_index += image_speed;
    if image_index.floor() >= frames as f64 {
        *image_index -= frames as f64;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignment_preserves_a_valid_fraction_and_resets_an_invalid_index() {
        let mut sprite = "sRunLeft".to_owned();
        let mut index = 2.75;
        assign(&mut sprite, &mut index, "sLookRunL");
        assert_eq!(index, 2.75);
        assign(&mut sprite, &mut index, "sJumpLeft");
        assert_eq!(index, 0.0);
        assert_eq!(sprite, "sJumpLeft");
    }

    #[test]
    fn same_sprite_assignment_still_applies_the_source_range_check() {
        let mut sprite = "sRunLeft".to_owned();
        let mut index = 6.0;
        assign(&mut sprite, &mut index, "sRunLeft");
        assert_eq!(index, 0.0);
        index = -0.25;
        assign(&mut sprite, &mut index, "sRunLeft");
        assert_eq!(index, -0.25);
    }

    #[test]
    fn endframe_wraps_once_before_reporting_animation_end() {
        let mut index = 5.75;
        assert!(advance(&mut index, 0.5, 6));
        assert_eq!(index, 0.25);
        assert!(advance(&mut index, 13.0, 6));
        assert_eq!(index, 7.25);
    }

    #[test]
    fn sprite_less_instances_still_advance_in_the_canonical_animation_pass() {
        let mut index = 0.0;
        assert!(advance(&mut index, 1.0, 0));
        assert_eq!(index, 1.0);
    }
}
