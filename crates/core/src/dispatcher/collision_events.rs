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
    #[allow(dead_code)]
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

    #[test]
    fn every_nonempty_source_collision_receiver_has_a_production_family_route() {
        let metadata: Metadata =
            serde_json::from_str(include_str!("../../../../assets/collision-events.json")).unwrap();
        for event in metadata.events {
            if event.action_count == 0 {
                continue;
            }
            let receivers: Vec<_> = catalog::object_names()
                .filter(|receiver| {
                    catalog::is_a(receiver, &event.owner) && has_event(receiver, &event.target)
                })
                .collect();
            assert!(
                !receivers.is_empty(),
                "source Collision receiver {} -> {} has no catalogued receiver",
                event.owner,
                event.target,
            );
            for receiver in receivers.iter().copied() {
                let routed = receiver == "oPlayer1"
                    || receiver == "oGoldDoor"
                    || crate::enemies::handles(receiver)
                    || crate::items::handles(receiver)
                    || crate::traps::handles(receiver)
                    || crate::world_events::handles(receiver);
                if routed {
                    continue;
                }
                // Abstract source receivers such as oEnemy supply inherited
                // callbacks through concrete descendants, never their own
                // instance adapter.
                assert!(
                    receivers.iter().any(|child| {
                        *child != receiver
                            && catalog::is_a(child, receiver)
                            && (crate::enemies::handles(child)
                                || crate::items::handles(child)
                                || crate::traps::handles(child)
                                || crate::world_events::handles(child))
                    }),
                    "source Collision receiver {} -> {} has no production family adapter",
                    receiver,
                    event.target,
                );
            }
        }
    }

    #[test]
    fn every_gameplay_object_with_inherited_source_step_or_alarm_has_a_production_route() {
        use std::path::Path;

        #[derive(Deserialize)]
        struct Coverage {
            objects: Vec<Object>,
        }
        #[derive(Deserialize)]
        struct Object {
            object: String,
            category: String,
            source_path: String,
            source_events: Vec<String>,
        }

        fn strip_gml_comments(code: &str) -> String {
            let chars: Vec<_> = code.chars().collect();
            let mut output = String::with_capacity(code.len());
            let mut index = 0;
            let mut quoted = false;
            while index < chars.len() {
                if chars[index] == '"' {
                    quoted = !quoted;
                    output.push(chars[index]);
                    index += 1;
                } else if !quoted && chars[index] == '/' && chars.get(index + 1) == Some(&'/') {
                    index += 2;
                    while index < chars.len() && chars[index] != '\n' {
                        index += 1;
                    }
                } else if !quoted && chars[index] == '/' && chars.get(index + 1) == Some(&'*') {
                    index += 2;
                    while index + 1 < chars.len()
                        && !(chars[index] == '*' && chars[index + 1] == '/')
                    {
                        index += 1;
                    }
                    index = (index + 2).min(chars.len());
                } else {
                    output.push(chars[index]);
                    index += 1;
                }
            }
            output
        }

        fn source_event_has_code(root: &Path, object: &Object, event: &str) -> bool {
            let source_stem = object.source_path.trim_end_matches(".xml");
            let path = root.join(format!("{source_stem}.events/{event}"));
            let Ok(xml) = std::fs::read_to_string(path) else {
                return false;
            };
            let mut remaining = xml.as_str();
            while let Some(start) = remaining.find("<argument kind=\"STRING\"") {
                let Some(open_end) = remaining[start..].find('>') else {
                    return false;
                };
                let content_start = start + open_end + 1;
                let Some(end) = remaining[content_start..].find("</argument>") else {
                    remaining = &remaining[content_start..];
                    continue;
                };
                if !strip_gml_comments(&remaining[content_start..content_start + end])
                    .trim()
                    .is_empty()
                {
                    return true;
                }
                remaining = &remaining[content_start + end + "</argument>".len()..];
            }
            false
        }

        let coverage: Coverage =
            serde_json::from_str(include_str!("../../../../assets/coverage.json")).unwrap();
        let source_root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../references/classic-1.1");
        let mut missing = Vec::new();
        for object in coverage
            .objects
            .iter()
            .filter(|object| object.category == "gameplay")
        {
            let has_source_step = coverage.objects.iter().any(|owner| {
                catalog::is_a(&object.object, &owner.object)
                    && owner.source_events.iter().any(|event| event == "Step.xml")
                    && source_event_has_code(&source_root, owner, "Step.xml")
            });
            if has_source_step {
                let routed = object.object == "oPlayer1"
                    || crate::enemies::handles(&object.object)
                    || crate::items::handles(&object.object)
                    || crate::traps::handles(&object.object)
                    || crate::world_events::handles(&object.object)
                    || matches!(
                        object.object.as_str(),
                        "oFinalBoss" | "oLevel" | "oSunRoom" | "oTunnelMan" | "oVolcanoFlame"
                    );
                if !routed {
                    missing.push(object.object.as_str());
                }
            }

            let has_source_alarm = coverage.objects.iter().any(|owner| {
                catalog::is_a(&object.object, &owner.object)
                    && owner.source_events.iter().any(|event| {
                        event.starts_with("Alarm ")
                            && source_event_has_code(&source_root, owner, event)
                    })
            });
            if has_source_alarm {
                let alarm_routed = object.object == "oPlayer1"
                    || crate::enemies::handles(&object.object)
                    || crate::items::handles(&object.object)
                    || crate::traps::handles(&object.object)
                    || crate::world_events::handles(&object.object)
                    || matches!(
                        object.object.as_str(),
                        "oMoonRoom" | "oStarsRoom" | "oSunRoom" | "oVolcanoFlame"
                    );
                if !alarm_routed {
                    missing.push(object.object.as_str());
                }
            }
        }
        assert!(
            missing.is_empty(),
            "source Step receivers missing a production event family: {}",
            missing.join(", ")
        );
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
