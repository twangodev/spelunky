//! Authored editor instances, independent of the file's one-character grid.
use crate::{Area, CollisionPrecision, Entity, Rng, Vec2, World, catalog};
mod create;
mod dark_fall;
mod effects;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PreviewContext {
    pub city_of_gold: bool,
    pub treasure_vision: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewSourceError {
    MissingGameClock,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreviewInstance {
    pub id: u64,
    pub tile: char,
    pub object: &'static str,
    pub sprite: &'static str,
    pub pos: Vec2,
    pub depth: i32,
    pub text: String,
    alive: bool,
    body: Option<PreviewBody>,
}
impl PreviewInstance {
    fn entity(&self) -> Entity {
        let mut entity = Entity::new(self.id, self.object, self.pos);
        entity.set_sprite(self.sprite);
        entity.data.insert("depth".into(), self.depth as f64);
        if let Some(body) = &self.body {
            entity.vel = body.vel;
            entity.state = body.state;
            entity.data = body.data.clone();
            entity.animation = body.animation;
        }
        entity
    }
    pub fn mask(&self) -> catalog::CollisionMask {
        catalog::instance_mask(
            self.object,
            self.sprite,
            self.pos,
            self.body.as_ref().map_or(0.0, |body| body.animation),
            Vec2::new(1.0, 1.0),
            0.0,
        )
    }
    pub fn animation(&self) -> f64 {
        self.body.as_ref().map_or(0.0, |body| body.animation)
    }
    pub fn is_door(&self) -> bool {
        matches!(self.object, "oExit" | "oEntrance")
    }
    pub fn is_sign(&self) -> bool {
        self.object == "oMsgSign"
    }
    pub fn cell(&self) -> (usize, usize) {
        (
            ((self.pos.x - 16.0) / 16.0) as usize,
            ((self.pos.y - 16.0) / 16.0) as usize,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
struct PreviewBody {
    vel: Vec2,
    state: i32,
    animation: f64,
    data: std::collections::BTreeMap<String, f64>,
}
impl PreviewBody {
    fn from_entity(entity: &Entity) -> Self {
        Self {
            vel: entity.vel,
            state: entity.state,
            animation: entity.animation,
            data: entity.data.clone(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct PreviewRoom {
    instances: Vec<PreviewInstance>,
    global_order: Vec<u64>,
    family_order: Vec<u64>,
    next_id: u64,
    destroyed_room_instance: bool,
    pub context: PreviewContext,
    pub rng: Rng,
    tick: u64,
    pub source_error: Option<PreviewSourceError>,
    pub check_water: bool,
    pub sounds: Vec<&'static str>,
    drawn_instances: Vec<PreviewInstance>,
}
impl PartialEq for PreviewRoom {
    fn eq(&self, other: &Self) -> bool {
        self.instances == other.instances
            && self.global_order == other.global_order
            && self.family_order == other.family_order
            && self.next_id == other.next_id
            && self.destroyed_room_instance == other.destroyed_room_instance
            && self.context == other.context
            && self.rng.algorithm() == other.rng.algorithm()
            && self.rng.state_bits() == other.rng.state_bits()
            && self.tick == other.tick
            && self.source_error == other.source_error
            && self.check_water == other.check_water
            && self.sounds == other.sounds
            && self.drawn_instances == other.drawn_instances
    }
}
impl PreviewRoom {
    pub fn new(context: PreviewContext) -> Self {
        Self::with_rng(context, Rng::new_canonical(0))
    }
    pub fn with_rng(context: PreviewContext, rng: Rng) -> Self {
        Self {
            instances: Vec::new(),
            global_order: Vec::new(),
            family_order: Vec::new(),
            next_id: 1,
            destroyed_room_instance: false,
            context,
            rng,
            tick: 0,
            source_error: None,
            check_water: false,
            sounds: Vec::new(),
            drawn_instances: Vec::new(),
        }
    }
    pub fn instance(&self, id: u64) -> Option<&PreviewInstance> {
        self.instances
            .iter()
            .find(|instance| instance.id == id && instance.alive)
    }
    pub fn instance_mut(&mut self, id: u64) -> Option<&mut PreviewInstance> {
        self.instances
            .iter_mut()
            .find(|instance| instance.id == id && instance.alive)
    }
    pub fn draw_instances(&self) -> impl Iterator<Item = &PreviewInstance> {
        self.drawn_instances.iter()
    }
    pub fn at_point(&self, point: Vec2, family: &str) -> Option<u64> {
        self.family_order.iter().copied().find(|&id| {
            self.instance(id).is_some_and(|instance| {
                catalog::is_a(instance.object, family)
                    && instance.mask().contains(point, CollisionPrecision::Precise)
            })
        })
    }
    fn at_rectangle(&self, pos: Vec2) -> Option<u64> {
        self.family_order.iter().copied().find(|&id| {
            self.instance(id).is_some_and(|instance| {
                catalog::is_a(instance.object, "oDrawnSprite")
                    && instance
                        .mask()
                        .intersects_rectangle_bbox(pos, Vec2::new(pos.x + 15.0, pos.y + 15.0))
            })
        })
    }
    fn destroy(&mut self, id: u64) {
        let Some(receiver) = self.instance(id).cloned() else {
            return;
        };
        if effects::inherits_solid_destroy(receiver.object) {
            self.check_water = true;
        }
        if let Some(spike) = effects::inherited_spike_target(self, &receiver) {
            self.destroy(spike);
        }
        if receiver.object == "oBlock" {
            let mut world = World::new(42, 34, Area::Mines, 1);
            crate::items::explosives::destroy_generated_block_literal(
                &mut world,
                &mut self.rng,
                &receiver.entity(),
                self.context.city_of_gold,
            );
            for mut entity in world.entities {
                if matches!(entity.object.as_str(), "oGoldChunk" | "oGoldNugget") {
                    crate::items::init_entity(&mut entity);
                }
                self.append_body(entity);
            }
        }
        if let Some(instance) = self.instance_mut(id) {
            instance.alive = false;
            self.destroyed_room_instance = true;
        }
    }
    fn append_body(&mut self, entity: Entity) {
        let object = catalog::object(&entity.object).expect("bundled source debris");
        let sprite = match entity.sprite.as_str() {
            "sRubbleLush" => "sRubbleLush",
            "sRubbleLushSmall" => "sRubbleLushSmall",
            "sGoldChunk" => "sGoldChunk",
            "sGoldNugget" => "sGoldNugget",
            "sSmokePuff" => "sSmokePuff",
            "sRubbleDark" => "sRubbleDark",
            _ => unreachable!("source debris sprite"),
        };
        let id = self.next_id;
        self.next_id += 1;
        self.instances.push(PreviewInstance {
            id,
            tile: '0',
            object: object.name.as_str(),
            sprite,
            pos: entity.pos,
            depth: object.depth,
            text: String::new(),
            alive: true,
            body: Some(PreviewBody::from_entity(&entity)),
        });
        self.global_order.push(id);
        self.family_order.push(id);
    }
    /// Editor Step resource812 runs after gold461/462 and rubble525/526.
    /// A fragment born by painting therefore first moves on the next tick.
    pub fn step_effects(&mut self, camera: Vec2) {
        effects::step(self, camera, self.context.treasure_vision);
        self.tick += 1;
    }
    pub fn room_instance_destroyed(&mut self) {
        self.destroyed_room_instance = true;
    }
    /// Source cleanup rebuilds descendants from the current global list before
    /// Draw's stable depth sort. The arrays are deliberately not one birth list.
    pub fn finish_source_frame(&mut self) {
        dark_fall::outside_room(self);
        dark_fall::collisions(self);
        if self.destroyed_room_instance {
            let live: std::collections::BTreeSet<_> = self
                .instances
                .iter()
                .filter(|instance| instance.alive)
                .map(|instance| instance.id)
                .collect();
            self.instances.retain(|instance| instance.alive);
            self.global_order.retain(|id| live.contains(id));
            self.family_order.clone_from(&self.global_order);
            self.destroyed_room_instance = false;
        }
        let depths: std::collections::BTreeMap<_, _> = self
            .instances
            .iter()
            .map(|instance| (instance.id, instance.depth))
            .collect();
        self.global_order
            .sort_by_key(|id| std::cmp::Reverse(depths[id]));
        self.drawn_instances = self
            .global_order
            .iter()
            .filter_map(|&id| self.instance(id).cloned())
            .collect();
        dark_fall::advance_animation(self);
    }
    pub fn create_tile(&mut self, tile: char, x: usize, y: usize) -> Option<u64> {
        let pos = Vec2::new(16.0 + x as f64 * 16.0, 16.0 + y as f64 * 16.0);
        if self.at_rectangle(pos).is_some() {
            let hit = self
                .at_point(Vec2::new(pos.x + 8.0, pos.y + 12.0), "oDrawnSprite")
                .or_else(|| self.at_point(Vec2::new(pos.x + 8.0, pos.y + 4.0), "oDrawnSprite"));
            if let Some(id) = hit {
                let instance = self.instance(id).unwrap();
                let protected = matches!(instance.tile, 'S' | '{' | 'Y' | 'E' | 'T');
                if !protected || instance.pos == pos {
                    self.destroy(id);
                }
            }
        }
        let (object, mut sprite, _) = definition(tile)?;
        create::consume_rng(object, &mut self.rng);
        if tile == 'B' && self.context.city_of_gold {
            sprite = "sGoldBlock";
        }
        if tile == ')'
            && self.instances.iter().any(|instance| {
                instance.alive
                    && catalog::is_a(instance.object, "oTree")
                    && instance.mask().contains(
                        Vec2::new(pos.x - 16.0, pos.y),
                        CollisionPrecision::BoundingBox,
                    )
            })
        {
            sprite = "sLeavesRight";
        }
        let id = self.next_id;
        self.next_id += 1;
        let depth = classic_depth(tile, x);
        self.instances.push(PreviewInstance {
            id,
            tile,
            object,
            sprite,
            pos,
            depth,
            text: String::new(),
            alive: true,
            body: None,
        });
        self.global_order.push(id);
        self.family_order.push(id);
        Some(id)
    }
    pub fn erase(&mut self, x: usize, y: usize) {
        let pos = Vec2::new(16.0 + x as f64 * 16.0, 16.0 + y as f64 * 16.0);
        if let Some(id) = self.at_rectangle(pos) {
            let instance = self.instance(id).unwrap();
            if !matches!(instance.tile, 'S' | 'T') || instance.pos == pos {
                self.destroy(id);
            }
        }
    }
}

pub fn cursor_sprite(tile: char) -> Option<&'static str> {
    definition(tile).map(|(_, _, sprite)| sprite)
}
pub fn preview_sprite(tile: char) -> Option<&'static str> {
    definition(tile).map(|(_, sprite, _)| sprite)
}
fn definition(tile: char) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match tile {
        '@' => ("oEntrance", "sEntrance", "sEntrance"),
        'X' => ("oExit", "sExit", "sExit"),
        'I' => ("oMsgSign", "sMsgSign", "sMsgSign"),
        '1' => ("oBrick", "sBrick", "sBrick"),
        '2' => ("oLush", "sLush", "sLush"),
        'w' => ("oWaterTile", "sWater", "sWater"),
        '3' => ("oDark", "sDark", "sDark"),
        'i' => ("oIceTile", "sIce", "sIce"),
        'd' => ("oDarkFall", "sDarkFall", "sDarkFall"),
        '4' => ("oTemple", "sTemple", "sTemple"),
        'l' => ("oLavaTile", "sLava", "sLava"),
        'L' => ("oLadderOrange", "sLadder", "sLadder"),
        'P' => ("oLadderTop", "sLadderTop", "sLadderTop"),
        'v' => ("oVine", "sVine", "sVine"),
        't' => ("oVineTop", "sVineTop", "sVineTop"),
        '|' => ("oTreeTile", "sTreeTrunk", "sTreeTrunk"),
        'x' => ("oTreeTile", "sTreeTop", "sTreeTop"),
        ')' => ("oLeavesTile", "sLeaves", "sLeaves"),
        'q' => ("oTreeBranchTile", "sTreeBranchLeft", "sTreeBranchLeft"),
        'B' => ("oBlock", "sBlock", "sBlock"),
        '&' => ("oWebTile", "sWebTile", "sWeb"),
        'r' => ("oRockTile", "sRockTile", "sRockTile"),
        'j' => ("oJarTile", "sJarTile", "sJarTile"),
        'k' => ("oBonesTile", "sFakeBonesLeft", "sFakeBonesLeft"),
        'b' => ("oBatTile", "sBatHang", "sBatLeft"),
        'n' => ("oSnakeTile", "sSnakeLeft", "sSnakeLeft"),
        's' => ("oSpiderTile", "sSpiderHang", "sSpider"),
        'S' => ("oGiantSpiderTile", "sGiantSpiderHang", "sGiantSpiderHang"),
        'K' => ("oSkeletonTile", "sSkeletonLeft", "sSkeletonLeft"),
        'h' => ("oCavemanTile", "sCavemanLeft", "sCavemanLeft"),
        '!' => ("oShopkeeperTile", "sShopLeftIco", "sShopLeftIco"),
        'f' => ("oFrogTile", "sFrogLeft", "sFrogLeft"),
        'F' => ("oFireFrogTile", "sFireFrogLeft", "sFireFrogLeft"),
        'z' => ("oZombieTile", "sZombieLeft", "sZombieLeft"),
        'A' => ("oVampireTile", "sVampireLeft", "sVampireLeft"),
        'p' => ("oPiranhaTile", "sPiranhaTile", "sPiranhaTile"),
        '{' => ("oMegaMouthTile", "sMegaMouthTile", "sMegaMouthTile"),
        'M' => ("oManTrapTile", "sManTrapLeft", "sManTrapLeft"),
        'm' => ("oMonkeyTile", "sMonkeyLeft", "sMonkeyLeft"),
        'y' => ("oYetiTile", "sYetiLeft", "sYetiLeft"),
        'Y' => ("oYetiKingTile", "sYetiKingLeft", "sYetiKingLeft"),
        'a' => ("oAlienTile", "sAlien", "sAlien"),
        'U' => ("oUFOTile", "sUFO", "sUFO"),
        'E' => ("oAlienBossTile", "sAlienBossTile", "sAlienBoss"),
        'H' => ("oHawkmanTile", "sHawkLeft", "sHawkLeft"),
        'T' => ("oTombLordTile", "sTombLordLeft", "sTombLordLeft"),
        '^' => ("oSpikes", "sSpikes", "sSpikes"),
        '<' => ("oArrowTrapLeft", "sArrowTrapLeft", "sArrowTrapLeft"),
        '>' => ("oArrowTrapRight", "sArrowTrapRight", "sArrowTrapRight"),
        ']' => ("oSpearTrapTileTop", "sSpearTrap1", "sSpearTrap1"),
        '[' => ("oSpearTrapTileBot", "sSpearTrap2", "sSpearTrap2"),
        '_' => ("oSpringTrapTile", "sSpringTrap", "sSpringTrap"),
        '+' => ("oSmashTrapTile", "sSmashTrap", "sSmashTrap"),
        '$' => ("oGoldBarTile", "sGoldBarTile", "sGoldBarTile"),
        '*' => ("oGoldBarsTile", "sGoldBarsTile", "sGoldBarsTile"),
        '#' => ("oGoldIdolTile", "sGoldIdolTile", "sGoldIdolTile"),
        'O' => (
            "oCrystalSkullTile",
            "sCrystalSkullTile",
            "sCrystalSkullTile",
        ),
        '5' => ("oEmeraldBigTile", "sEmeraldBigTile", "sEmeraldBigTile"),
        '6' => ("oSapphireBigTile", "sSapphireBigTile", "sSapphireBigTile"),
        '7' => ("oRubyBigTile", "sRubyBigTile", "sRubyBigTile"),
        '8' => ("oDiamondTile", "sDiamondTile", "sDiamondTile"),
        'c' => ("oChestTile", "sChestTile", "sChestTile"),
        'C' => ("oCrateTile", "sCrateTile", "sCrateTile"),
        'D' => ("oDamselTile", "sDamselLeftIco", "sDamselLeftIco"),
        '.' => ("oBombBagTile", "sBombBagTile", "sBombBagTile"),
        ':' => ("oBombBoxTile", "sBombBoxTile", "sBombBoxTile"),
        'u' => ("oBombPasteTile", "sBombPasteTile", "sBombPasteTile"),
        'R' => ("oRopePileTile", "sRopePileTile", "sRopePileTile"),
        '`' => ("oParachuteTile", "sParachuteTile", "sParachuteTile"),
        'o' => ("oCompassTile", "sCompassTile", "sCompassTile"),
        '/' => ("oMacheteTile", "sMacheteTile", "sMacheteTile"),
        '~' => ("oSpringShoesTile", "sSpringShoesTile", "sSpringShoesTile"),
        'V' => ("oSpikeShoesTile", "sSpikeShoesTile", "sSpikeShoesTile"),
        '}' => ("oBowTile", "sBowTile", "sBowTile"),
        '-' => ("oPistolTile", "sPistolTile", "sPistolTile"),
        '=' => ("oShotgunTile", "sShotgunTile", "sShotgunTile"),
        'W' => ("oWebCannonTile", "sWebCannonTile", "sWebCannonTile"),
        '%' => ("oSpectaclesTile", "sSpectaclesTile", "sSpectaclesTile"),
        'G' => ("oGlovesTile", "sGlovesTile", "sGlovesTile"),
        'g' => ("oMittTile", "sMittTile", "sMittTile"),
        '?' => ("oTeleporterTile", "sTeleporterTile", "sTeleporterTile"),
        '(' => ("oMattockTile", "sMattockTile", "sMattockTile"),
        '\\' => ("oCapeTile", "sCapeTile", "sCapeTile"),
        'J' => ("oJetpackTile", "sJetpackTile", "sJetpackTile"),
        _ => return None,
    })
}

/// `scrCreateTile` sets every preview to depth 100, then gives entrances,
/// exits, and signs a column-relative foreground depth. The editor room is 672
/// pixels wide (`rLevelEditor.xml`), and preview column zero starts at x=16.
fn classic_depth(tile: char, column: usize) -> i32 {
    match tile {
        '@' | 'X' | 'I' => 59 + column as i32,
        _ => 100,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_previews_follow_classic_editor_depths_by_column() {
        let mut room = PreviewRoom::new(PreviewContext::default());
        let boss = room.create_tile('{', 10, 10).unwrap();
        let water = room.create_tile('w', 11, 10).unwrap();
        let sign = room.create_tile('I', 12, 10).unwrap();
        let entrance = room.create_tile('@', 13, 10).unwrap();

        let boss_mask = room.instance(boss).unwrap().mask();
        for id in [water, sign, entrance] {
            let pos = room.instance(id).unwrap().pos;
            assert!(
                boss_mask.intersects_rectangle_bbox(pos, Vec2::new(pos.x + 15.0, pos.y + 15.0),)
            );
        }

        room.finish_source_frame();
        let draw_order: Vec<_> = room
            .draw_instances()
            .map(|instance| (instance.object, instance.depth))
            .collect();
        assert_eq!(
            draw_order,
            [
                ("oMegaMouthTile", 100),
                ("oWaterTile", 100),
                ("oEntrance", 72),
                ("oMsgSign", 71),
            ],
            "doors and signs sort by column after scrCreateTile assigns depth 100"
        );
    }

    #[test]
    fn door_and_sign_depth_uses_editor_room_column_formula() {
        assert_eq!(classic_depth('@', 0), 59);
        assert_eq!(classic_depth('X', 39), 98);
        assert_eq!(classic_depth('I', 12), 71);
        for tile in ['1', 'w', '{', 'S', 'T'] {
            assert_eq!(classic_depth(tile, 12), 100, "preview depth for {tile:?}");
        }
    }

    #[test]
    fn source_block_destroy_runs_literal_birth_rng_then_rubble_float_motion() {
        let mut room = PreviewRoom::with_rng(PreviewContext::default(), Rng::new_canonical(872));
        room.create_tile('B', 4, 4);
        room.finish_source_frame();
        let mut expected = room.rng.clone();
        let mut positions = Vec::new();
        for _ in 0..3 {
            positions.push(Vec2::new(
                88.0 + expected.source_rand(0, 8).unwrap() as f64
                    - expected.source_rand(0, 8).unwrap() as f64,
                88.0 + expected.source_rand(0, 8).unwrap() as f64
                    - expected.source_rand(0, 8).unwrap() as f64,
            ));
        }
        room.erase(4, 4);
        room.finish_source_frame();
        assert_eq!(room.rng.state_bits(), expected.state_bits());
        let debris: Vec<_> = room.draw_instances().collect();
        assert_eq!(
            debris.iter().map(|i| i.sprite).collect::<Vec<_>>(),
            ["sRubbleLush", "sRubbleLushSmall", "sRubbleLushSmall"]
        );
        assert_eq!(debris.iter().map(|i| i.pos).collect::<Vec<_>>(), positions);
        assert!(debris.iter().all(|i| i.depth == 1));
        room.step_effects(Vec2::default());
        room.finish_source_frame();
        assert_eq!(
            room.draw_instances().map(|i| i.pos).collect::<Vec<_>>(),
            positions,
            "birthframe has no Step, firstStep zero velocity"
        );
        room.step_effects(Vec2::default());
        room.finish_source_frame();
        for (actual, initial) in room.draw_instances().zip(&positions) {
            assert_eq!(actual.pos, Vec2::new(initial.x, initial.y + 0.6));
        }
    }
    #[test]
    fn source_gold_reports_missing_clock_only_after_integer_launch_step() {
        let mut room = PreviewRoom::with_rng(
            PreviewContext {
                city_of_gold: true,
                ..Default::default()
            },
            Rng::new_canonical(927),
        );
        room.create_tile('B', 4, 4);
        room.erase(4, 4);
        room.finish_source_frame();
        assert_eq!(room.draw_instances().count(), 4);
        assert!(room.draw_instances().all(|i| i.depth == 100));
        let before: Vec<_> = room
            .instances
            .iter()
            .map(|i| (i.id, i.pos, i.body.as_ref().unwrap().vel))
            .collect();
        room.step_effects(Vec2::default());
        room.finish_source_frame();
        assert_eq!(room.source_error, None);
        for (id, pos, vel) in before {
            assert_eq!(
                room.instance(id).unwrap().pos,
                Vec2::new(pos.x + vel.x, pos.y + vel.y)
            );
        }
        let after: Vec<_> = room.instances.iter().map(|i| (i.id, i.pos)).collect();
        room.step_effects(Vec2::default());
        room.finish_source_frame();
        assert_eq!(
            room.source_error,
            Some(PreviewSourceError::MissingGameClock)
        );
        for (id, pos) in after {
            let instance = room.instance(id).unwrap();
            assert_eq!(instance.pos, pos, "no fabricated fractional clock movement");
            let data = &instance.body.as_ref().unwrap().data;
            assert_eq!(data.get("mtXPrev"), Some(&pos.x));
            assert_eq!(data.get("mtYPrev"), Some(&pos.y));
            assert_eq!(data.get("xVelInteger"), Some(&0.0));
            assert_eq!(data.get("yVelInteger"), Some(&0.0));
        }
    }
    #[test]
    fn source_gold_both_side_contacts_do_not_apply_one_sided_x_correction() {
        let mut witnessed = 0;
        for seed in 0..16 {
            let mut room = PreviewRoom::with_rng(
                PreviewContext {
                    city_of_gold: true,
                    ..Default::default()
                },
                Rng::new_canonical(seed),
            );
            room.create_tile('B', 4, 4);
            room.erase(4, 4);
            // Ordinary paint places a real brick over some of the emitted gold.
            room.create_tile('1', 4, 4);
            room.finish_source_frame();
            let before: std::collections::BTreeMap<_, _> = room
                .instances
                .iter()
                .filter(|i| i.body.is_some())
                .map(|i| (i.id, i.pos.x))
                .collect();
            room.step_effects(Vec2::default());
            room.finish_source_frame();
            for i in room.instances.iter().filter(|i| i.body.is_some()) {
                let body = i.body.as_ref().unwrap();
                if body.data.get("colLeft") == Some(&1.0) && body.data.get("colRight") == Some(&1.0)
                {
                    witnessed += 1;
                    assert_eq!(
                        i.pos.x, before[&i.id],
                        "both contacts block motion and source nested correction does nothing"
                    );
                }
            }
        }
        assert!(
            witnessed > 0,
            "actual source spawned gold must contact both sides"
        );
    }
    #[test]
    fn source_precise_lower_miss_falls_back_to_upper_opaque_darkfall_pixel() {
        let mut room = PreviewRoom::new(PreviewContext::default());
        let id = room.create_tile('d', 4, 4).unwrap();
        let dark = room.instance(id).unwrap();
        assert!(
            !dark
                .mask()
                .contains(Vec2::new(88.0, 92.0), CollisionPrecision::Precise)
        );
        assert!(
            dark.mask()
                .contains(Vec2::new(88.0, 84.0), CollisionPrecision::Precise)
        );
        room.create_tile('1', 4, 4);
        room.finish_source_frame();
        assert!(
            room.instance(id).is_none(),
            "upper fallback removes real prior preview"
        );
        assert_eq!(
            room.draw_instances().map(|i| i.object).collect::<Vec<_>>(),
            ["oBrick"]
        );
    }
    #[test]
    fn source_protected_overlap_darkfall_breaks_and_effects_finish_their_lifecycle() {
        let mut room = PreviewRoom::with_rng(PreviewContext::default(), Rng::new_canonical(937));
        room.create_tile('S', 1, 12);
        room.finish_source_frame();
        let tree = room.create_tile('|', 2, 12).unwrap();
        room.finish_source_frame();
        assert!(room.instance(tree).is_some());
        let mut expected = room.rng.clone();
        for _ in 0..3 {
            for range in [13, 13, 3, 3, 4] {
                expected.range(range);
            }
        }
        let dark = room.create_tile('d', 2, 12).unwrap();
        room.finish_source_frame();
        assert!(
            room.instance(tree).is_some(),
            "first protected Spider prevented replacing Tree"
        );
        assert!(
            room.instance(dark).is_none(),
            "actual precise automatic collision breaks DarkFall"
        );
        assert_eq!(room.sounds, ["sndBreak"]);
        assert!(room.check_water);
        assert_eq!(room.rng.state_bits(), expected.state_bits());
        let smoke = room
            .instances
            .iter()
            .find(|i| i.object == "oSmokePuff")
            .unwrap()
            .id;
        let smoke_pos = room.instance(smoke).unwrap().pos;
        assert_eq!(
            room.instances
                .iter()
                .filter(|i| i.object == "oRubbleDark")
                .count(),
            3
        );
        // Collision births have no Step this frame, but do receive post-Draw animation.
        assert_eq!(
            room.instance(smoke)
                .unwrap()
                .body
                .as_ref()
                .unwrap()
                .animation,
            0.4
        );
        room.step_effects(Vec2::new(16.0, 192.0));
        room.finish_source_frame();
        assert_eq!(room.instance(smoke).unwrap().pos.y, smoke_pos.y - 0.1);
        let mut saw_final_draw = false;
        for _ in 0..100 {
            room.step_effects(Vec2::new(16.0, 192.0));
            room.finish_source_frame();
            if room.instance(smoke).is_none() && room.draw_instances().any(|i| i.id == smoke) {
                saw_final_draw = true;
            }
        }
        assert!(
            saw_final_draw,
            "AnimationEnd destroys after source final Draw"
        );
        assert!(
            !room
                .instances
                .iter()
                .any(|i| i.alive && i.object == "oRubbleDark"),
            "naturally falling fragments eventually leave room and action203 destroys them"
        );
        assert!(
            room.instance(tree).is_some(),
            "effects never erase saved/static tree"
        );
    }
}
