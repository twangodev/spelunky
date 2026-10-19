//! Platform-independent level editing, with reversible tile and metadata edits.
use crate::{
    level::{LEVEL_HEIGHT, LEVEL_WIDTH, Level},
    model::World,
};

pub mod preview;
use crate::Vec2;
use preview::{PreviewContext, PreviewRoom};

/// Original palette groups. Eraser is `0` and is a port convenience.
pub const BLOCK_PALETTE: &str = "@XI1LPB&rjk2wvt|x)q3di4l";
pub const ENEMY_PALETTE: &str = "bnsSKh!fFzAMmp{aUEyYHT";
pub const TRAP_PALETTE: &str = "^<>][_+";
pub const ITEM_PALETTE: &str = "$*#O5678cCD.:uR`o/~V}-=W%Gg?(\\J";

/// Completed shortcut stages control which source editor palette entries are
/// available. Later stages imply their earlier prerequisites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorPaletteProgress {
    pub shortcut_5: bool,
    pub shortcut_9: bool,
    pub shortcut_13: bool,
}

impl EditorPaletteProgress {
    pub const fn all_unlocked() -> Self {
        Self {
            shortcut_5: true,
            shortcut_9: true,
            shortcut_13: true,
        }
    }

    pub const fn from_shortcuts(shortcut_5: bool, shortcut_9: bool, shortcut_13: bool) -> Self {
        Self {
            shortcut_5: shortcut_5 || shortcut_9 || shortcut_13,
            shortcut_9: shortcut_9 || shortcut_13,
            shortcut_13,
        }
    }

    fn palette(self) -> Vec<char> {
        let (blocks, enemies, traps) = if self.shortcut_13 {
            (23, 21, 6)
        } else if self.shortcut_9 {
            (21, 19, 5)
        } else if self.shortcut_5 {
            (18, 14, 4)
        } else {
            (10, 6, 2)
        };
        let mut palette = vec!['0'];
        palette.extend(BLOCK_PALETTE.chars().take(blocks + 1));
        palette.extend(ENEMY_PALETTE.chars().take(enemies + 1));
        palette.extend(TRAP_PALETTE.chars().take(traps + 1));
        palette.extend(ITEM_PALETTE.chars());
        palette
    }
}

#[derive(Debug, Clone)]
pub struct EditorState {
    pub level: Level,
    pub selected_tile: char,
    pub palette: Vec<char>,
    pub previews: PreviewRoom,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

#[derive(Debug, Clone)]
struct Snapshot {
    level: Level,
    previews: PreviewRoom,
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new(Level::default())
    }
}

impl EditorState {
    pub fn new(level: Level) -> Self {
        Self::with_context(level, PreviewContext::default())
    }

    pub fn with_context(level: Level, context: PreviewContext) -> Self {
        Self::with_source_rng(level, context, crate::Rng::new_canonical(0))
    }
    pub fn with_source_rng(level: Level, context: PreviewContext, rng: crate::Rng) -> Self {
        let previews = build_previews(&level, context, rng);
        let palette_progress = EditorPaletteProgress::all_unlocked();
        Self {
            level,
            selected_tile: '@',
            palette: palette_progress.palette(),
            previews,
            undo: vec![],
            redo: vec![],
        }
    }

    pub fn set_palette_progress(&mut self, progress: EditorPaletteProgress) {
        self.palette = progress.palette();
        if !self.palette.contains(&self.selected_tile) {
            self.selected_tile = self
                .palette
                .iter()
                .copied()
                .find(|&tile| tile != '0')
                .unwrap_or('0');
        }
    }

    pub fn select(&mut self, tile: char) -> Result<(), String> {
        if !self.palette.contains(&tile) {
            return Err("Tile is not in the Classic palette".into());
        }
        self.selected_tile = tile;
        Ok(())
    }

    pub fn set_tile(&mut self, x: usize, y: usize, tile: char) -> Result<(), String> {
        self.edit_tile(x, y, tile, false)
    }

    fn edit_tile(&mut self, x: usize, y: usize, tile: char, fresh: bool) -> Result<(), String> {
        let mut next = self.level.clone();
        next.set_tile(x, y, tile)?;
        // Keep door/sign records attached to their cells when row-major order changes.
        let previous = self.level.get_tile(x, y).unwrap_or('0');
        for door in [true, false] {
            if matches_record(previous, door) != matches_record(tile, door) {
                remap_records(&self.level, &mut next, door);
            }
        }
        if fresh {
            // scrCreateTile creates a new door/sign, whose Create clears its text.
            if let Some(index) = record_index(&next, x, y, true) {
                materialize_records(&mut next, true);
                next.exit_targets[index].clear();
            } else if let Some(index) = record_index(&next, x, y, false) {
                materialize_records(&mut next, false);
                next.signs[index].clear();
            }
        }
        let mut previews = self.previews.clone();
        if let Some(id) = previews.create_tile(tile, x, y) {
            if !fresh {
                if let Some(index) = record_index(&next, x, y, true) {
                    previews.instance_mut(id).unwrap().text = next.exit_target(index).to_owned();
                } else if let Some(index) = record_index(&next, x, y, false) {
                    previews.instance_mut(id).unwrap().text = next.sign_message(index).to_owned();
                }
            }
        }
        self.commit(next, previews);
        Ok(())
    }

    pub fn paint(&mut self, x: usize, y: usize) -> Result<(), String> {
        self.edit_tile(x, y, self.selected_tile, true)
    }
    pub fn erase(&mut self, x: usize, y: usize) -> Result<(), String> {
        let mut next = self.level.clone();
        next.set_tile(x, y, '0')?;
        for door in [true, false] {
            remap_records(&self.level, &mut next, door);
        }
        let mut previews = self.previews.clone();
        previews.erase(x, y);
        self.commit(next, previews);
        Ok(())
    }

    pub fn set_exit_target(&mut self, x: usize, y: usize, target: String) -> Result<(), String> {
        if target.contains(['\r', '\n']) {
            return Err("Door target must be a single line".into());
        }
        let index =
            record_index(&self.level, x, y, true).ok_or("Cell is not an entrance or exit")?;
        if self.level.exit_target(index) == target {
            return Ok(());
        }
        let mut next = self.level.clone();
        materialize_records(&mut next, true);
        next.exit_targets[index] = target.clone();
        let mut previews = self.previews.clone();
        if let Some(id) = door_at(&previews, x, y) {
            previews.instance_mut(id).unwrap().text = target;
        }
        self.commit(next, previews);
        Ok(())
    }

    pub fn set_sign(&mut self, x: usize, y: usize, message: String) -> Result<(), String> {
        if message.contains(['\r', '\n']) {
            return Err("Sign message must be a single line".into());
        }
        let index = record_index(&self.level, x, y, false).ok_or("Cell is not a sign")?;
        if self.level.sign_message(index) == message {
            return Ok(());
        }
        let mut next = self.level.clone();
        materialize_records(&mut next, false);
        next.signs[index] = message.clone();
        let mut previews = self.previews.clone();
        if let Some(id) = previews.at_point(anchor(x, y), "oMsgSign") {
            previews.instance_mut(id).unwrap().text = message;
        }
        self.commit(next, previews);
        Ok(())
    }

    /// Metadata changes preserve live previews; explicit grid replacement rebuilds
    /// the authored room. Painting and erasing use their distinct source queries.
    pub fn replace_level(&mut self, next: Level) {
        let previews = if next.tiles == self.level.tiles {
            self.previews.clone()
        } else {
            build_previews(&next, self.previews.context, self.previews.rng.clone())
        };
        self.commit(next, previews);
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            level: self.level.clone(),
            previews: self.previews.clone(),
        }
    }
    fn commit(&mut self, next: Level, previews: PreviewRoom) {
        if next == self.level && previews == self.previews {
            return;
        }
        self.undo.push(self.snapshot());
        if self.undo.len() > 256 {
            self.undo.remove(0);
        }
        self.level = next;
        self.previews = previews;
        self.redo.clear();
    }
    pub fn set_preview_text(&mut self, id: u64, text: String) -> Result<(), String> {
        if text.contains(['\r', '\n']) {
            return Err("Text must be a single line".into());
        }
        let instance = self
            .previews
            .instance(id)
            .ok_or("Preview no longer exists")?;
        if !instance.is_door() && !instance.is_sign() {
            return Err("Preview is not a sign or door".into());
        }
        let (x, y) = instance.cell();
        let door = instance.is_door();
        let mut next = self.level.clone();
        if let Some(index) = record_index(&next, x, y, door) {
            materialize_records(&mut next, door);
            if door {
                next.exit_targets[index] = text.clone();
            } else {
                next.signs[index] = text.clone();
            }
        }
        let mut previews = self.previews.clone();
        previews.instance_mut(id).unwrap().text = text;
        self.commit(next, previews);
        Ok(())
    }
    /// scrTestLevel/save gather text only from surviving preview instances,
    /// independently of levelArray. Classic does not write unmatched records.
    pub fn document(&self) -> Level {
        let mut next = self.level.clone();
        for door in [true, false] {
            let mut records = Vec::new();
            for y in 0..LEVEL_HEIGHT {
                for x in 0..LEVEL_WIDTH {
                    if !matches_record(self.level.get_tile(x, y).unwrap_or('0'), door) {
                        continue;
                    }
                    let id = if door {
                        door_at(&self.previews, x, y)
                    } else {
                        self.previews.at_point(anchor(x, y), "oMsgSign")
                    };
                    if let Some(instance) = id.and_then(|id| self.previews.instance(id)) {
                        records.push(instance.text.clone());
                    }
                }
            }
            if door {
                next.exit_targets = records;
            } else {
                next.signs = records;
            }
        }
        next
    }
    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.snapshot());
        self.level = previous.level;
        self.previews = previous.previews;
        true
    }
    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(self.snapshot());
        self.level = next.level;
        self.previews = next.previews;
        true
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn test_play(&self) -> World {
        self.document().to_world()
    }
}

fn anchor(x: usize, y: usize) -> Vec2 {
    Vec2::new(16.0 + x as f64 * 16.0, 16.0 + y as f64 * 16.0)
}
fn door_at(previews: &PreviewRoom, x: usize, y: usize) -> Option<u64> {
    previews
        .at_point(anchor(x, y), "oExit")
        .or_else(|| previews.at_point(anchor(x, y), "oEntrance"))
}
fn build_previews(level: &Level, context: PreviewContext, rng: crate::Rng) -> PreviewRoom {
    let mut previews = PreviewRoom::with_rng(context, rng);
    let mut door_index = 0;
    let mut sign_index = 0;
    // The source creates every cell, including zero: a later empty cell may
    // remove an earlier overlapping preview while its saved anchor remains.
    for y in 0..LEVEL_HEIGHT {
        for x in 0..LEVEL_WIDTH {
            let tile = level.get_tile(x, y).unwrap_or('0');
            previews.create_tile(tile, x, y);
            let (family, records, index) = match tile {
                '@' => ("oEntrance", &level.exit_targets, &mut door_index),
                'X' => ("oExit", &level.exit_targets, &mut door_index),
                'I' => ("oMsgSign", &level.signs, &mut sign_index),
                _ => continue,
            };
            if !records.is_empty() {
                if let Some(id) = previews.at_point(anchor(x, y), family) {
                    // Retain the port's final-record fallback for malformed
                    // short lists; valid original files never need this adapter.
                    previews.instance_mut(id).unwrap().text = records
                        .get(*index)
                        .or_else(|| records.last())
                        .cloned()
                        .unwrap();
                    *index += 1;
                }
            }
        }
    }
    previews.finish_source_frame();
    previews
}

fn matches_record(tile: char, door: bool) -> bool {
    if door {
        matches!(tile, '@' | 'X')
    } else {
        tile == 'I'
    }
}
fn record_index(level: &Level, x: usize, y: usize, door: bool) -> Option<usize> {
    if x >= LEVEL_WIDTH || y >= LEVEL_HEIGHT || !matches_record(level.get_tile(x, y)?, door) {
        return None;
    }
    let index = y * LEVEL_WIDTH + x;
    Some(
        level
            .tiles
            .iter()
            .take(index)
            .filter(|tile| matches_record(**tile, door))
            .count(),
    )
}
fn materialize_records(level: &mut Level, door: bool) {
    let count = level
        .tiles
        .iter()
        .take(LEVEL_WIDTH * LEVEL_HEIGHT)
        .filter(|tile| matches_record(**tile, door))
        .count();
    let stored = if door {
        level.exit_targets.len()
    } else {
        level.signs.len()
    };
    let records = (0..count.max(stored))
        .map(|index| {
            if door {
                level.exit_target(index)
            } else {
                level.sign_message(index)
            }
            .to_owned()
        })
        .collect();
    if door {
        level.exit_targets = records;
    } else {
        level.signs = records;
    }
}
fn remap_records(old: &Level, next: &mut Level, door: bool) {
    let mut records = Vec::new();
    for y in 0..LEVEL_HEIGHT {
        for x in 0..LEVEL_WIDTH {
            if next
                .get_tile(x, y)
                .is_some_and(|tile| matches_record(tile, door))
            {
                let text = record_index(old, x, y, door)
                    .map(|index| {
                        if door {
                            old.exit_target(index)
                        } else {
                            old.sign_message(index)
                        }
                    })
                    .unwrap_or("");
                records.push(text.to_owned());
            }
        }
    }
    // Extra imported records are ignored by Classic's loader, but belong to
    // the custom file and must survive editing existing cells.
    let old_count = old
        .tiles
        .iter()
        .take(LEVEL_WIDTH * LEVEL_HEIGHT)
        .filter(|tile| matches_record(**tile, door))
        .count();
    let old_records = if door { &old.exit_targets } else { &old.signs };
    records.extend(old_records.iter().skip(old_count).cloned());
    if door {
        next.exit_targets = records;
    } else {
        next.signs = records;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_progress_exposes_classic_palette_prefixes_in_source_order() {
        let cases = [
            (false, false, false, 11, 7, 3),
            (true, false, false, 19, 15, 5),
            (true, true, false, 22, 20, 6),
            (false, true, false, 22, 20, 6),
            (true, true, true, 24, 22, 7),
        ];
        for (first, second, third, block_count, enemy_count, trap_count) in cases {
            let progress = EditorPaletteProgress::from_shortcuts(first, second, third);
            let mut editor = EditorState::default();
            editor.set_palette_progress(progress);
            let expected: Vec<_> = std::iter::once('0')
                .chain(BLOCK_PALETTE.chars().take(block_count))
                .chain(ENEMY_PALETTE.chars().take(enemy_count))
                .chain(TRAP_PALETTE.chars().take(trap_count))
                .chain(ITEM_PALETTE.chars())
                .collect();
            assert_eq!(editor.palette, expected);
        }

        let implied = EditorPaletteProgress::from_shortcuts(false, false, true);
        let mut editor = EditorState::default();
        editor.select('J').unwrap();
        editor.set_palette_progress(implied);
        assert_eq!(editor.palette.len(), 85);
        assert_eq!(editor.selected_tile, 'J');
    }

    #[test]
    fn locking_a_selected_tile_moves_selection_to_first_available_source_tile() {
        let mut editor = EditorState::default();
        editor.select('q').unwrap();
        editor.set_palette_progress(EditorPaletteProgress::from_shortcuts(false, false, false));
        assert_eq!(editor.selected_tile, '@');
        assert!(!editor.palette.contains(&'q'));
        assert!(editor.palette.contains(&'$'));
    }

    #[test]
    fn first_paint_creates_the_classic_entrance() {
        let mut editor = EditorState::default();

        assert_eq!(editor.selected_tile, '@');
        editor.paint(0, 0).unwrap();

        assert_eq!(editor.level.get_tile(0, 0), Some('@'));
        let id = editor
            .previews
            .at_point(Vec2::new(16.0, 16.0), "oEntrance")
            .expect("first paint should create an entrance preview");
        let entrance = editor.previews.instance(id).unwrap();
        assert_eq!(entrance.tile, '@');
        assert_eq!(entrance.object, "oEntrance");
    }
}
