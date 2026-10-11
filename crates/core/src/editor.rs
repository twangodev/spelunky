//! Platform-independent level editing, with reversible tile and metadata edits.
use crate::{
    level::{LEVEL_HEIGHT, LEVEL_WIDTH, Level},
    model::World,
};

/// Original palette groups. Eraser is `0`; all content is available in this port.
pub const BLOCK_PALETTE: &str = "@XI1LPB&rjk2wvt|x)q3di4l";
pub const ENEMY_PALETTE: &str = "bnsSKh!fFzAMmp{aUEyYHT";
pub const TRAP_PALETTE: &str = "^<>][_+";
pub const ITEM_PALETTE: &str = "$*#O5678cCD.:uR`o/~V}-=W%Gg?(\\J";

#[derive(Debug, Clone)]
pub struct EditorState {
    pub level: Level,
    pub selected_tile: char,
    pub palette: Vec<char>,
    undo: Vec<Level>,
    redo: Vec<Level>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new(Level::default())
    }
}

impl EditorState {
    pub fn new(level: Level) -> Self {
        let palette = format!("0{BLOCK_PALETTE}{ENEMY_PALETTE}{TRAP_PALETTE}{ITEM_PALETTE}")
            .chars()
            .collect();
        Self {
            level,
            selected_tile: '1',
            palette,
            undo: vec![],
            redo: vec![],
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
        self.replace_level(next);
        Ok(())
    }

    pub fn paint(&mut self, x: usize, y: usize) -> Result<(), String> {
        self.edit_tile(x, y, self.selected_tile, true)
    }
    pub fn erase(&mut self, x: usize, y: usize) -> Result<(), String> {
        self.set_tile(x, y, '0')
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
        next.exit_targets[index] = target;
        self.replace_level(next);
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
        next.signs[index] = message;
        self.replace_level(next);
        Ok(())
    }

    /// Use this for metadata edits so they participate in undo/redo.
    pub fn replace_level(&mut self, next: Level) {
        if next == self.level {
            return;
        }
        self.undo.push(self.level.clone());
        // Bound history memory even during long painting sessions.
        if self.undo.len() > 256 {
            self.undo.remove(0);
        }
        self.level = next;
        self.redo.clear();
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(&mut self.level, previous));
        true
    }
    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(&mut self.level, next));
        true
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn test_play(&self) -> World {
        self.level.to_world()
    }
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
