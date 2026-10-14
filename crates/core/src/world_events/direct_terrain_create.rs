//! Closed runtime Create bodies not owned by the room-stamp constructor.
//! Historical generation and replay births retain their existing handlers.
use crate::{Game, GenerationRevision, Vec2};

pub(super) fn create(game: &mut Game, id: u64) -> bool {
    if game.generation_revision != GenerationRevision::SourceClassicV2 {
        return false;
    }
    let city = game.flags.contains("cityOfGold")
        || game.flags.contains("city_of_gold")
        || game.world.name == "City of Gold";
    let cemetery = game.flags.contains("cemetary") || game.flags.contains("cemetery");
    let Some(entity) = game.world.entity_mut(id) else {
        return false;
    };
    if !matches!(
        entity.object.as_str(),
        "oMoai"
            | "oMoai2"
            | "oMoai3"
            | "oMoaiInside"
            | "oIceBlock"
            | "oIce"
            | "oTreeBranch"
            | "oFrozenCaveman"
    ) {
        return false;
    }
    if entity.value("runtime_terrain_create_initialized") != 0.0 {
        return true;
    }
    // Every selected object inherits DrawnSprite; Branch/Frozen have no Solid
    // parent. The marker precedes Ice's synchronous nested instance_create.
    entity.data.insert("blinkToggle".into(), 0.0);
    entity.text.insert("type".into(), String::new());
    entity
        .data
        .insert("runtime_terrain_create_initialized".into(), 1.0);
    if !matches!(entity.object.as_str(), "oTreeBranch" | "oFrozenCaveman") {
        entity.data.insert("invincible".into(), 0.0);
        entity.data.insert("shopWall".into(), 0.0);
        entity.data.insert("cleanDeath".into(), 0.0);
        entity.text.insert("type".into(), "None".into());
        entity.text.insert("treasure".into(), String::new());
    }
    let ice_origin = match entity.object.as_str() {
        "oMoai" | "oMoai2" | "oMoai3" | "oMoaiInside" => {
            entity.data.insert("invincible".into(), 1.0);
            None
        }
        "oIceBlock" => {
            // Solid -> MoveableSolid -> IceBlock, including the script bounds.
            entity.vel = Vec2::default();
            for (key, value) in [
                ("push_gravity", 0.6),
                ("collision_left", 0.0),
                ("collision_top", 0.0),
                ("collision_right", 16.0),
                ("collision_bottom", 16.0),
            ] {
                entity.data.insert(key.into(), value);
            }
            if city {
                entity.set_sprite("sGoldBlock");
            }
            None
        }
        "oTreeBranch" => {
            if cemetery {
                entity.set_sprite("sTreeBranchDeadR");
            }
            None
        }
        "oIce" => Some(entity.pos),
        _ => None,
    };
    // No entity/index borrow survives a nested callback. The Ice parent is
    // already live with inherited defaults when FrozenCaveman Create runs.
    if let Some(origin) = ice_origin {
        if game.rng.range(80) == 0 {
            game.spawn("oFrozenCaveman", origin);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, RulesPreset, World};

    #[test]
    fn completed_ice_create_is_not_replayed_and_preserves_caller_fields() {
        let mut game = Game::shell(13, RulesPreset::Classic);
        game.world = World::new(20, 16, Area::Ice, 9);
        let id = game.spawn("oIce", Vec2::new(80.0, 80.0));
        let before = game.rng.clone();
        let count = game.world.entities.len();
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("shopWall".into(), 1.0);
        super::super::create_instance(&mut game, id);
        assert_eq!(game.world.entity(id).unwrap().value("shopWall"), 1.0);
        assert_eq!(game.world.entities.len(), count);
        assert_eq!(game.rng.clone().next_u64(), before.clone().next_u64());
    }
}
