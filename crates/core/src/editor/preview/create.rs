//! The Create RNG calls that run in rLevelEditor, before scrCreateTile
//! restores the authored editor sprite. Other palette objects consume no RNG.
use crate::Rng;

/// Preserve the original stream even though the editor hides block variants.
/// isLevel() excludes the optional gem/item calls in this room; cityOfGold
/// changes Temple art after its unconditional roll and does not skip the roll.
pub(super) fn consume_rng(object: &str, rng: &mut Rng) {
    match object {
        "oBrick" => {
            let _ = rng.range(10); // rand(1, 10)
            let _ = rng.range(100); // rand(1, 100)
        }
        "oLush" | "oDark" | "oTemple" => {
            let _ = rng.range(100); // rand(1, 100)
        }
        _ => {}
    }
}
