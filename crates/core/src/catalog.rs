//! Original GameMaker object defaults and sprite collision rectangles.
//!
//! Positions use the original instance origin. Sprite bounds are inclusive in
//! the source; [`bounds`] converts them to the core's half-open rectangles.

use crate::{Rect, Vec2};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize)]
pub struct ObjectDef {
    pub name: String,
    pub id: i32,
    pub sprite: String,
    pub parent: String,
    /// Original GameMaker property; this is not the gameplay collision group.
    pub solid: bool,
    pub visible: bool,
    pub depth: i32,
    pub persistent: bool,
    pub mask: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpriteDef {
    pub id: i32,
    pub origin: [i32; 2],
    /// Inclusive source coordinates: left, top, right, bottom.
    pub bounds: [i32; 4],
}

#[derive(Debug, Deserialize)]
struct Catalog {
    objects: BTreeMap<String, ObjectDef>,
    sprites: BTreeMap<String, SpriteDef>,
}

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../../../assets/catalog.json"))
            .expect("bundled Classic catalog must be valid")
    })
}

pub fn object(name: &str) -> Option<&'static ObjectDef> {
    catalog().objects.get(name)
}

/// True for the named object itself and for all descendants of `parent`.
pub fn is_a(name: &str, parent: &str) -> bool {
    let mut current = name;
    // The bound also keeps malformed metadata from hanging a game loop.
    for _ in 0..catalog().objects.len() {
        let Some(def) = object(current) else {
            return false;
        };
        if def.name == parent {
            return true;
        }
        current = &def.parent;
    }
    false
}

/// Classic tests collisions against the oSolid family, including moving blocks.
pub fn is_solid(name: &str) -> bool {
    is_a(name, "oSolid")
}

/// Default object sprite (an empty sprite is not inherited in GameMaker).
pub fn sprite(name: &str) -> Option<&'static str> {
    let def = object(name)?;
    (!def.sprite.is_empty()).then_some(def.sprite.as_str())
}

pub fn sprite_def(name: &str) -> Option<&'static SpriteDef> {
    catalog().sprites.get(name)
}

/// The default mask sprite takes precedence over the visible sprite.
pub fn bounds(name: &str, pos: Vec2) -> Rect {
    let Some(def) = object(name) else {
        return Rect::new(pos.x - 6.0, pos.y - 7.0, 12.0, 14.0);
    };
    let sprite_name = if def.mask.is_empty() {
        &def.sprite
    } else {
        &def.mask
    };
    let Some(sprite) = sprite_def(sprite_name) else {
        return Rect::new(pos.x, pos.y, 0.0, 0.0);
    };
    let [left, top, right, bottom] = sprite.bounds;
    Rect::new(
        pos.x + f64::from(left - sprite.origin[0]),
        pos.y + f64::from(top - sprite.origin[1]),
        f64::from((right - left + 1).max(0)),
        f64::from((bottom - top + 1).max(0)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn gameplay_solidity_follows_source_hierarchy() {
        assert!(is_solid("oHardBlock"));
        assert!(!object("oHardBlock").unwrap().solid);
        assert!(is_solid("oBlock"));
        assert!(is_solid("oPushBlock"));
        assert!(is_a("oPushBlock", "oMoveableSolid"));
        assert!(is_a("oBlock", "oBlock"));
        assert!(!is_solid("oGold"));
        assert!(object("oGold").unwrap().solid);
        assert!(!is_a("missing", "missing"));
    }

    #[test]
    fn original_positions_use_sprite_origins_and_inclusive_bounds() {
        assert_eq!(sprite("oBlock"), Some("sBlock"));
        assert_eq!(sprite("oHardBlock"), Some("sBrick"));
        assert_eq!(
            bounds("oBlock", Vec2::new(32.0, 48.0)),
            Rect::new(32.0, 48.0, 16.0, 16.0)
        );
        let sprite = sprite_def("sBlock").unwrap();
        assert_eq!(sprite.origin, [0, 0]);
        assert_eq!(sprite.bounds, [0, 0, 15, 15]);
        assert_eq!(
            bounds("oDamsel", Vec2::new(32.0, 48.0)),
            Rect::new(26.0, 40.0, 12.0, 16.0)
        );
        assert_eq!(
            bounds("oSnake", Vec2::new(32.0, 48.0)),
            Rect::new(33.0, 52.0, 14.0, 12.0)
        );
    }

    #[test]
    fn catalog_has_valid_references_and_acyclic_inheritance() {
        assert_eq!(catalog().objects.len(), 453);
        for def in catalog().objects.values() {
            for sprite in [&def.sprite, &def.mask] {
                assert!(
                    sprite.is_empty() || sprite_def(sprite).is_some(),
                    "{}: {sprite}",
                    def.name
                );
            }
            let mut seen = BTreeSet::new();
            let mut current = def.name.as_str();
            while !current.is_empty() {
                assert!(seen.insert(current), "cycle in {}", def.name);
                current = &object(current).expect("parent must exist").parent;
            }
        }
    }
}
