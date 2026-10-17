//! oTempleFake's own Step and the synchronous oTemple Create it invokes.
use crate::{Entity, Game, Vec2};

// scrGenerateItem(x,y,2): switch(rand(0,18)), in literal source order.
const UNDERGROUND: [(&str, f64, f64); 19] = [
    ("oJetpack", 0.0, -2.0),
    ("oCapePickup", 0.0, 0.0),
    ("oShotgun", 0.0, 0.0),
    ("oMattock", 0.0, 0.0),
    ("oTeleporter", 0.0, 3.0),
    ("oGloves", 0.0, -1.0),
    ("oSpectacles", 0.0, 0.0),
    ("oWebCannon", -2.0, 0.0),
    ("oPistol", 0.0, 0.0),
    ("oMitt", 0.0, -1.0),
    ("oPaste", 0.0, 0.0),
    ("oSpringShoes", 0.0, 0.0),
    ("oSpikeShoes", 0.0, 0.0),
    ("oMachete", 0.0, 0.0),
    ("oBombBox", 0.0, -2.0),
    ("oBow", 0.0, 0.0),
    ("oCompass", 0.0, 0.0),
    ("oParaPickup", 0.0, 0.0),
    ("oRopePile", 0.0, 0.0),
];

pub(super) fn create(game: &mut Game, id: u64) {
    let Some(entity) = game.world.entity_mut(id) else {
        return;
    };
    if entity.value("runtime_temple_created") != 0.0 {
        return;
    }
    // oDrawnSprite Create, then oSolid Create, then oTemple Create. These run
    // before the child random draws and any nested synchronous item Create.
    entity.data.insert("blinkToggle".into(), 0.0);
    entity.data.insert("invincible".into(), 0.0);
    entity.data.insert("shopWall".into(), 0.0);
    entity.data.insert("cleanDeath".into(), 0.0);
    entity.text.insert("type".into(), "None".into());
    entity.text.insert("treasure".into(), String::new());
    entity.data.insert("runtime_temple_created".into(), 1.0);
    let origin = entity.pos;
    let roll = game.rng.range(100) + 1;
    game.world
        .entity_mut(id)
        .unwrap()
        .data
        .insert("n".into(), f64::from(roll));
    let city = game.flags.contains("cityOfGold") || game.flags.contains("city_of_gold");
    let sprite = if city {
        Some("sGTemple")
    } else if roll < 20 {
        Some("sTempleGold")
    } else if roll < 30 {
        Some("sTempleGoldBig")
    } else {
        None
    };
    if let Some(sprite) = sprite {
        game.world.entity_mut(id).unwrap().set_sprite(sprite);
        return;
    }
    if !game.is_level()
        || origin.x <= 1.0
        || origin.y <= 1.0
        || origin.x >= game.world.width as f64 * 16.0 - 16.0
        || origin.y >= game.world.height as f64 * 16.0 - 16.0
    {
        return;
    }
    let center = Vec2::new(origin.x + 8.0, origin.y + 8.0);
    for (object, denominator) in [("oSapphireBig", 60), ("oEmeraldBig", 80), ("oRubyBig", 100)] {
        if game.rng.chance(1, denominator) {
            game.spawn(object, center);
            return;
        }
    }
    if game.rng.chance(1, 1200) {
        underground(game, center);
    }
}

fn underground(game: &mut Game, center: Vec2) {
    let (object, dx, dy) = UNDERGROUND[game.rng.range(19) as usize];
    let item = game.spawn(object, Vec2::new(center.x + dx, center.y + dy));
    if let Some(item) = game.world.entity_mut(item) {
        item.data.insert("cost".into(), 0.0);
        item.data.insert("forSale".into(), 0.0);
    }
}

pub(super) fn fake_step(game: &mut Game, fake: &mut Entity) {
    let probe = Vec2::new(fake.pos.x + 8.0, fake.pos.y + 8.0);
    let door = super::point_family(game, probe, "oDoor");
    if !door {
        // instance_create returns only after inherited/nested Create callbacks;
        // the old fake remains alive during those callbacks. It has no parent
        // and no Destroy event, so its retirement does not request checkWater.
        game.spawn("oTemple", fake.pos);
        fake.alive = false;
    }
}

#[cfg(test)]
#[path = "temple_tests.rs"]
mod tests;
