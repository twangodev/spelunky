//! Classic's scripted sprite assignment and automatic animation arithmetic.
//!
//! Sprite assignment does not restart the image index. Automatic image
//! advancement belongs to the engine's endframe pass, after Step and collision
//! events, rather than to an actor's AI or movement controller.

/// Assign `sprite_index` without changing `image_index`.
///
/// Classic preserves the current subimage index when a sprite is assigned,
/// including when the destination has fewer frames. Drawing wraps an
/// out-of-range index to a destination subimage; the index itself remains
/// available to the end-of-frame animation update and source code. Explicit
/// source `image_index` writes remain separate operations.
pub fn assign(sprite: &mut String, _image_index: &mut f64, next: &str) {
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
    fn assignment_preserves_fractional_and_out_of_range_indices() {
        let mut sprite = "sRunLeft".to_owned();
        let mut index = 2.75;
        assign(&mut sprite, &mut index, "sLookRunL");
        assert_eq!(index, 2.75);
        assign(&mut sprite, &mut index, "sJumpLeft");
        assert_eq!(index, 2.75);
        assert_eq!(sprite, "sJumpLeft");

        // The normal player run-to-jump transition changes `sRunLeft` (six
        // frames) to `sJumpLeft` (one frame). `characterSprite()` does not
        // reset image_index there, so a later running pose remains out of
        // range on the jump sprite until drawing/endframe processing wraps it.
        index = 2.75;
        sprite = "sRunLeft".to_owned();
        assign(&mut sprite, &mut index, "sJumpLeft");
        assert_eq!(index, 2.75);

        // `sCrawlLeft` has ten frames and `sRunLeft` has six; the source
        // characterSprite() also changes between these without an explicit
        // image_index write.
        index = 8.5;
        sprite = "sCrawlLeft".to_owned();
        assert_eq!(crate::catalog::sprite_frame_count(&sprite), 10);
        assert_eq!(crate::catalog::sprite_frame_count("sRunLeft"), 6);
        assign(&mut sprite, &mut index, "sRunLeft");
        assert_eq!(sprite, "sRunLeft");
        assert_eq!(index, 8.5);
    }

    #[test]
    fn same_sprite_assignment_leaves_the_image_index_untouched() {
        let mut sprite = "sRunLeft".to_owned();
        let mut index = 6.0;
        assign(&mut sprite, &mut index, "sRunLeft");
        assert_eq!(index, 6.0);
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
