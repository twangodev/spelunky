//! Nonempty Collision event declarations exported from the preserved Classic XML.
//!
//! The runner resolves the most specific target first, walking the receiving
//! object's ancestry for each target. Selecting an inherited event does not
//! implicitly run additional ancestors; `event_inherited()` belongs to its body.

use crate::catalog;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
struct Metadata {
    events: Vec<Declaration>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(not(test), allow(dead_code))]
struct Declaration {
    owner: String,
    owner_id: i32,
    target: String,
    target_id: i32,
    source: String,
    action_count: usize,
}

fn declarations() -> &'static BTreeMap<&'static str, BTreeSet<&'static str>> {
    static EVENTS: OnceLock<BTreeMap<&'static str, BTreeSet<&'static str>>> = OnceLock::new();
    EVENTS.get_or_init(|| {
        let metadata: Metadata =
            serde_json::from_str(include_str!("../../../../assets/collision-events.json"))
                .expect("bundled source Collision declarations must be valid");
        let mut events: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        for event in metadata.events {
            // The runner's local event lookup ignores events with zero actions.
            if event.action_count == 0 {
                continue;
            }
            let owner = catalog::object(&event.owner).expect("known Collision event owner");
            let target = catalog::object(&event.target).expect("known Collision event target");
            assert_eq!(owner.id, event.owner_id, "source resource ID must match");
            assert_eq!(target.id, event.target_id, "source resource ID must match");
            events
                .entry(owner.name.as_str())
                .or_default()
                .insert(target.name.as_str());
        }
        events
    })
}

fn resolve<'a>(
    receiver: &'a str,
    target: &'a str,
    parent: impl Fn(&'a str) -> Option<&'a str>,
    has_local: impl Fn(&'a str, &'a str) -> bool,
) -> Option<(&'a str, &'a str)> {
    let mut target = Some(target);
    while let Some(other) = target {
        let mut receiver = Some(receiver);
        while let Some(this) = receiver {
            if has_local(this, other) {
                return Some((this, other));
            }
            receiver = parent(this);
        }
        target = parent(other);
    }
    None
}

fn selected(self_object: &str, other_object: &str) -> Option<(&'static str, &'static str)> {
    let receiver = catalog::object(self_object)?.name.as_str();
    let target = catalog::object(other_object)?.name.as_str();
    let events = declarations();
    resolve(
        receiver,
        target,
        |name| {
            let parent = &catalog::object(name)?.parent;
            (!parent.is_empty()).then_some(parent.as_str())
        },
        |receiver, target| {
            events
                .get(receiver)
                .is_some_and(|events| events.contains(target))
        },
    )
}

/// Whether this receiving object has an applicable automatic Collision event.
pub(crate) fn has_event(self_object: &str, other_object: &str) -> bool {
    selected(self_object, other_object).is_some()
}

/// Resource defining the selected event, including inherited receivers.
pub(crate) fn owner(self_object: &str, other_object: &str) -> Option<&'static str> {
    selected(self_object, other_object).map(|event| event.0)
}

/// Target resource named by the selected event, before target inheritance.
#[cfg(test)]
pub(crate) fn event_target(self_object: &str, other_object: &str) -> Option<&'static str> {
    selected(self_object, other_object).map(|event| event.1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn preserved_collision_metadata_matches_every_source_event() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../references/classic-1.1");
        let metadata: Metadata =
            serde_json::from_str(include_str!("../../../../assets/collision-events.json")).unwrap();
        let mut pending = vec![root.join("Objects")];
        let mut sources = BTreeSet::new();
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    pending.push(path);
                } else if path.extension().is_some_and(|extension| extension == "xml") {
                    let xml = std::fs::read_to_string(&path).unwrap();
                    if xml.contains("<event category=\"COLLISION\"") {
                        sources.insert(path.strip_prefix(&root).unwrap().to_path_buf());
                    }
                }
            }
        }
        assert_eq!(sources.len(), 123);
        assert_eq!(sources.len(), metadata.events.len());
        for event in metadata.events {
            assert!(sources.remove(&PathBuf::from(&event.source)));
            let xml = std::fs::read_to_string(root.join(&event.source)).unwrap();
            let owner_path = Path::new(&event.source).parent().unwrap();
            let owner = owner_path.file_name().unwrap().to_str().unwrap();
            assert_eq!(owner.strip_suffix(".events"), Some(event.owner.as_str()));
            assert!(xml.contains(&format!(
                "<event category=\"COLLISION\" with=\"{}\">",
                event.target
            )));
            assert_eq!(xml.matches("<action id=").count(), event.action_count);
            assert_eq!(catalog::object(&event.owner).unwrap().id, event.owner_id);
            assert_eq!(catalog::object(&event.target).unwrap().id, event.target_id);
        }
        assert!(sources.is_empty());
    }

    #[test]
    fn inherited_receivers_and_specific_targets_match_source() {
        assert_eq!(owner("oSnake", "oPlayer1"), Some("oEnemy"));
        assert_eq!(event_target("oSnake", "oPlayer1"), Some("oCharacter"));
        assert_eq!(owner("oBullet", "oDamsel"), Some("oBullet"));
        assert_eq!(event_target("oBullet", "oDamsel"), Some("oDamsel"));
        assert_eq!(event_target("oBullet", "oPlayer1"), Some("oCharacter"));
        assert_eq!(event_target("oWebBall", "oRock"), Some("oItem"));
        assert!(has_event("oMagmaMan", "oMagmaMan"));
        assert!(!has_event("oRock", "oSnake"));
        assert!(!has_event("unknown", "oPlayer1"));
        assert!(!has_event("oBullet", "unknown"));
    }

    #[test]
    fn target_specificity_precedes_receiver_specificity() {
        let parent = |name| match name {
            "receiver_child" => Some("receiver_parent"),
            "target_child" => Some("target_parent"),
            _ => None,
        };
        let has_local = |receiver, target| {
            matches!(
                (receiver, target),
                ("receiver_child", "target_parent") | ("receiver_parent", "target_child")
            )
        };
        assert_eq!(
            resolve("receiver_child", "target_child", parent, has_local),
            Some(("receiver_parent", "target_child"))
        );
    }

    #[test]
    fn empty_local_event_does_not_suppress_nonempty_inheritance() {
        let local_actions = BTreeMap::from([
            (("receiver_child", "target"), 0usize),
            (("receiver_parent", "target"), 1usize),
        ]);
        assert_eq!(
            resolve(
                "receiver_child",
                "target",
                |name| (name == "receiver_child").then_some("receiver_parent"),
                |receiver, target| local_actions
                    .get(&(receiver, target))
                    .is_some_and(|count| *count > 0),
            ),
            Some(("receiver_parent", "target"))
        );
    }
}
