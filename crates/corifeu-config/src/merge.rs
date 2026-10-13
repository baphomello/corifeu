//! Deep merge across layers, recording which layer set each value.
//!
//! Maps merge key by key. **Lists are replaced wholesale** — merging them by
//! position or identity produces results nobody predicts, so a project config
//! that sets `channels` replaces the global one rather than appending to it
//! (ADR-0029).

use crate::source::{Layer, Source};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Dotted path to a leaf, e.g. `roles.coder.model`.
pub type Path = String;

/// The merged tree plus the origin of every leaf that something actually set.
#[derive(Debug, Clone)]
pub struct Merged {
    pub value: Value,
    pub origins: BTreeMap<Path, Source>,
}

impl Merged {
    /// The layer that set `path`, or `None` when the model's default applies.
    pub fn origin(&self, path: &str) -> Option<&Source> {
        self.origins.get(path)
    }
}

/// Merge layers into one tree. Input order does not matter; layers are sorted
/// by precedence, then applied weakest first so stronger ones overwrite.
pub fn merge(mut layers: Vec<Layer>) -> Merged {
    layers.sort_by_key(|l| std::cmp::Reverse(l.source.precedence()));

    let mut value = Value::Object(Map::new());
    let mut origins = BTreeMap::new();

    for layer in layers {
        apply(&mut value, &layer.value, "", &layer.source, &mut origins);
    }

    Merged { value, origins }
}

fn apply(
    target: &mut Value,
    incoming: &Value,
    prefix: &str,
    source: &Source,
    origins: &mut BTreeMap<Path, Source>,
) {
    let Value::Object(incoming_map) = incoming else {
        // A non-map at the root is meaningless; ignore rather than panic. The
        // typed deserialization step reports it properly.
        return;
    };

    if !target.is_object() {
        *target = Value::Object(Map::new());
    }
    let target_map = target.as_object_mut().expect("just ensured object");

    for (key, incoming_child) in incoming_map {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };

        match (target_map.get_mut(key), incoming_child) {
            // Two maps: recurse, so one layer can override a single nested key
            // without restating its siblings.
            (Some(existing @ Value::Object(_)), Value::Object(_)) => {
                apply(existing, incoming_child, &path, source, origins);
            }
            // Anything else replaces, lists included.
            _ => {
                target_map.insert(key.clone(), incoming_child.clone());
                record_leaves(incoming_child, &path, source, origins);
            }
        }
    }
}

/// Record the origin of a replaced subtree. A scalar records itself; a map
/// records each leaf beneath it, so `--explain` can attribute a single nested
/// value rather than only the branch it arrived in.
fn record_leaves(value: &Value, path: &str, source: &Source, origins: &mut BTreeMap<Path, Source>) {
    match value {
        Value::Object(map) if !map.is_empty() => {
            for (key, child) in map {
                record_leaves(child, &format!("{path}.{key}"), source, origins);
            }
        }
        _ => {
            origins.insert(path.to_string(), source.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn yaml(s: &str) -> Value {
        serde_saphyr::from_str(s).expect("valid yaml")
    }

    fn global(s: &str) -> Layer {
        Layer::new(Source::Global(PathBuf::from("global.yaml")), yaml(s))
    }

    fn project(s: &str) -> Layer {
        Layer::new(Source::Project(PathBuf::from("project.yaml")), yaml(s))
    }

    #[test]
    fn stronger_layer_wins() {
        let m = merge(vec![
            global("autonomy: autonomous"),
            project("autonomy: supervised"),
        ]);
        assert_eq!(m.value["autonomy"], "supervised");
        assert!(matches!(m.origin("autonomy"), Some(Source::Project(_))));
    }

    #[test]
    fn layer_order_does_not_matter() {
        let a = merge(vec![
            global("autonomy: autonomous"),
            project("autonomy: supervised"),
        ]);
        let b = merge(vec![
            project("autonomy: supervised"),
            global("autonomy: autonomous"),
        ]);
        assert_eq!(a.value, b.value);
    }

    #[test]
    fn nested_override_keeps_siblings() {
        let m = merge(vec![
            global("roles:\n  coder:\n    provider: local\n    model: small\n"),
            project("roles:\n  coder:\n    model: big\n"),
        ]);
        assert_eq!(m.value["roles"]["coder"]["model"], "big");
        assert_eq!(m.value["roles"]["coder"]["provider"], "local");
        assert!(matches!(
            m.origin("roles.coder.model"),
            Some(Source::Project(_))
        ));
        assert!(matches!(
            m.origin("roles.coder.provider"),
            Some(Source::Global(_))
        ));
    }

    #[test]
    fn lists_replace_rather_than_merge() {
        let m = merge(vec![
            global("channels: [[a, b], [c, d]]"),
            project("channels: [[e, f]]"),
        ]);
        assert_eq!(m.value["channels"].as_array().expect("array").len(), 1);
    }

    #[test]
    fn unset_values_have_no_origin() {
        let m = merge(vec![project("version: 1")]);
        assert!(m.origin("autonomy").is_none());
    }
}
