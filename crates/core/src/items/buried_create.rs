//! RNG-free inherited Create bodies for terrain's Gold Nugget, three Big gems and nineteen
//! Underground-set items. The caller still owns post-Create sale overrides.
use crate::Entity;

pub(super) fn initialize(entity: &mut Entity, has_spectacles: bool) {
    if !matches!(
        entity.object.as_str(),
        "oGoldNugget" | "oLampItem" | "oLampRedItem"
    ) {
        initialize_source_jungle(entity, has_spectacles);
    }
}

pub(super) fn initialize_source_jungle(entity: &mut Entity, has_spectacles: bool) {
    let gem = match entity.object.as_str() {
        "oGoldNugget" => Some(("Gold Nugget", 500.0, true)),
        "oSapphireBig" => Some(("Big Sapphire", 1200.0, false)),
        "oEmeraldBig" => Some(("Big Emerald", 800.0, false)),
        "oRubyBig" => Some(("Big Ruby", 1600.0, false)),
        _ => None,
    };
    let item = match entity.object.as_str() {
        "oLampItem" | "oLampRedItem" => ("Lamp", 0.0, "", ""),
        "oJetpack" => ("Jetpack", 20000.0, "", "JETPACK FOR $"),
        "oCapePickup" => ("Cape", 12000.0, "", "A CAPE FOR $"),
        "oShotgun" => ("Shotgun", 15000.0, "", "A SHOTGUN FOR $"),
        "oMattock" => ("Mattock", 8000.0, "", "A MATTOCK FOR $"),
        "oTeleporter" => ("Teleporter", 10000.0, "", "A TELEPORTER FOR $"),
        "oGloves" => ("Gloves", 8000.0, "CLIMBING GLOVES", "CLIMBING GLOVES FOR $"),
        "oSpectacles" => ("Spectacles", 8000.0, "", "SPECTACLES FOR $"),
        "oWebCannon" => ("Web Cannon", 2000.0, "", "A WEB CANNON FOR $"),
        "oPistol" => ("Pistol", 5000.0, "", "A PISTOL FOR $"),
        "oMitt" => ("Mitt", 4000.0, "PITCHER'S MITT", "PITCHER'S MITT FOR $"),
        "oPaste" => ("Paste", 3000.0, "BOMB PASTE", "BOMB PASTE FOR $"),
        "oSpringShoes" => (
            "Spring Shoes",
            5000.0,
            "SPRINGY SHOES",
            "SPRINGY SHOES FOR $",
        ),
        "oSpikeShoes" => ("Spike Shoes", 4000.0, "", "SPIKE SHOES FOR $"),
        "oMachete" => ("Machete", 7000.0, "", "A MACHETE FOR $"),
        "oBombBox" => (
            "Bomb Box",
            10000.0,
            "A BOX OF 12 BOMBS",
            "A BOX OF 12 BOMBS FOR $",
        ),
        "oBow" => ("Bow", 1000.0, "BOW AND ARROWS", "BOW AND ARROWS FOR $"),
        "oCompass" => ("Compass", 3000.0, "", "A COMPASS FOR $"),
        "oParaPickup" => ("Parachute", 2000.0, "", "A PARACHUTE FOR $"),
        "oRopePile" => ("Rope Pile", 2500.0, "", "EXTRA ROPE FOR $"),
        _ => {
            if gem.is_none() {
                return;
            }
            ("", 0.0, "", "")
        }
    };
    if entity.value("source_pickup_create_initialized") != 0.0 {
        return;
    }
    entity
        .data
        .insert("source_pickup_create_initialized".into(), 1.0);
    // oDrawnSprite -> oTreasure/oItem, then the selected object's own Create.
    entity.data.insert("blinkToggle".into(), 0.0);
    entity.held = false;
    for (name, value) in [
        ("LEFT", 18.0),
        ("RIGHT", 19.0),
        ("myGrav", 0.6),
        ("trigger", 0.0),
    ] {
        entity.data.insert(name.into(), value);
    }
    // Every selected child calls makeActive: this initializes motion fields,
    // rather than activating a GameMaker instance or its engine registration.
    entity.vel = Default::default();
    for name in ["xVel", "yVel", "xAcc", "yAcc"] {
        entity.data.insert(name.into(), 0.0);
    }
    if let Some((kind, value, can_collect)) = gem {
        entity.text.insert("type".into(), kind.into());
        entity.state = 1;
        for (name, value) in [
            ("value", value),
            ("canCollect", f64::from(can_collect)),
            ("yOff", 4.0),
            ("STATIC", 0.0),
            ("ACTIVE", 1.0),
        ] {
            entity.data.insert(name.into(), value);
        }
        // Gold Nugget has no own Alarm write; Big gems delay collection.
        if !can_collect {
            entity.alarms[0] = 20;
        }
    } else {
        for name in [
            "armed",
            "trigger",
            "safe",
            "value",
            "colBot",
            "bloodless",
            "forSale",
            "stolen",
            "inDiceHouse",
            "cimg",
            "stuck",
            "sticky",
            "enemyID",
            "stickyXDiff",
            "stickyYDiff",
        ] {
            entity.data.insert(name.into(), 0.0);
        }
        for (name, value) in [
            ("active", 1.0),
            ("new", 1.0),
            ("DUCKING", 12.0),
            ("canPickUp", 1.0),
            ("bounceFactor", 0.5),
            ("frictionFactor", 0.3),
            ("breakPieces", 1.0),
            ("depth", if has_spectacles { 51.0 } else { 101.0 }),
            (
                "heavy",
                f64::from(matches!(entity.object.as_str(), "oJetpack" | "oBombBox")),
            ),
        ] {
            entity.data.insert(name.into(), value);
        }
        let (kind, cost, description, purchase) = item;
        entity.text.insert("type".into(), kind.into());
        entity.text.insert("shopDesc".into(), description.into());
        if matches!(entity.object.as_str(), "oLampItem" | "oLampRedItem") {
            entity.data.insert("trigger".into(), 1.0);
            entity.data.insert("heavy".into(), 1.0);
            entity.data.insert("value".into(), 1000.0);
        } else {
            entity
                .text
                .insert("buyMessage".into(), format!("{purchase}{cost:.0}."));
        }
        entity.data.insert("cost".into(), cost);
        if entity.object == "oBow" {
            entity.set_image_speed(0.0);
        }
    }
}
