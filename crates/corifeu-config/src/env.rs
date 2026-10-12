//! Environment-variable overrides.
//!
//! `docs/config.md` originally specified `CORIFEU_` followed by the key path
//! joined with underscores, and called the mapping reversible. It is not:
//! `CORIFEU_LIMITS_ROUNDS_PER_PAIR` could mean `limits.rounds_per_pair` or
//! `limits.rounds.per.pair`, and nothing in the name distinguishes them.
//!
//! **`__` separates path segments; a single `_` is part of a key.** So
//! `CORIFEU_LIMITS__ROUNDS_PER_PAIR` is unambiguously
//! `limits.rounds_per_pair`.

use crate::source::{Layer, Source};
use serde_json::{Map, Value};

pub const PREFIX: &str = "CORIFEU_";

/// Build layers from the process environment.
pub fn layers() -> Vec<Layer> {
    from_pairs(std::env::vars().collect::<Vec<_>>())
}

/// Build layers from explicit pairs. Exposed so tests do not mutate global
/// process state.
///
/// **One layer per variable**, not one for the whole environment. They share a
/// precedence, so merging is unaffected — but `--explain` can then name the
/// exact variable that set a value, which is the only reason anyone runs it.
pub fn from_pairs(pairs: Vec<(String, String)>) -> Vec<Layer> {
    let mut sorted = pairs;
    sorted.sort();

    sorted
        .into_iter()
        .filter_map(|(name, raw)| {
            let path = path_of(&name)?;
            let mut root = Map::new();
            insert(&mut root, &path, scalar(&raw));
            Some(Layer::new(Source::Env(name), Value::Object(root)))
        })
        .collect()
}

/// `CORIFEU_PROVIDERS__LOCAL__BASE_URL` -> `["providers", "local", "base_url"]`
fn path_of(name: &str) -> Option<Vec<String>> {
    let rest = name.strip_prefix(PREFIX)?;
    if rest.is_empty() {
        return None;
    }
    let segments: Vec<String> = rest.split("__").map(|s| s.to_ascii_lowercase()).collect();
    if segments.iter().any(String::is_empty) {
        return None;
    }
    Some(segments)
}

/// Environment values are strings; YAML scalar rules give them the type the
/// model expects, so `3` is a number and `true` is a boolean without a
/// special case per field.
fn scalar(raw: &str) -> Value {
    serde_saphyr::from_str::<Value>(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
}

fn insert(root: &mut Map<String, Value>, path: &[String], value: Value) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };
    let mut cursor = root;
    for segment in parents {
        cursor = cursor
            .entry(segment.clone())
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            // A scalar already sitting where a map is needed means two
            // variables disagree about the shape. The deeper one wins; the
            // typed step reports the result if it is wrong.
            .expect("entry is an object");
    }
    cursor.insert(last.clone(), value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merge::merge;

    fn value_of(pairs: &[(&str, &str)]) -> Value {
        let layers = from_pairs(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        );
        merge(layers).value
    }

    #[test]
    fn single_underscore_stays_in_the_key() {
        let v = value_of(&[("CORIFEU_LIMITS__ROUNDS_PER_PAIR", "5")]);
        assert_eq!(v["limits"]["rounds_per_pair"], 5);
    }

    #[test]
    fn double_underscore_nests() {
        let v = value_of(&[("CORIFEU_PROVIDERS__LOCAL__BASE_URL", "http://h:11434")]);
        assert_eq!(v["providers"]["local"]["base_url"], "http://h:11434");
    }

    #[test]
    fn scalars_get_yaml_types() {
        let v = value_of(&[
            ("CORIFEU_LIMITS__LOOP_DETECTION", "false"),
            ("CORIFEU_LIMITS__ROLE_TIMEOUT_S", "120"),
            ("CORIFEU_AUTONOMY", "supervised"),
        ]);
        assert_eq!(v["limits"]["loop_detection"], false);
        assert_eq!(v["limits"]["role_timeout_s"], 120);
        assert_eq!(v["autonomy"], "supervised");
    }

    #[test]
    fn unprefixed_variables_are_ignored() {
        assert!(from_pairs(vec![("PATH".into(), "/usr/bin".into())]).is_empty());
    }

    #[test]
    fn malformed_names_are_ignored() {
        let pairs = vec![
            ("CORIFEU_".to_string(), "x".to_string()),
            ("CORIFEU_A__".to_string(), "y".to_string()),
            ("CORIFEU___B".to_string(), "z".to_string()),
        ];
        assert!(from_pairs(pairs).is_empty());
    }

    #[test]
    fn each_value_names_its_own_variable() {
        let layers = from_pairs(vec![
            ("CORIFEU_AUTONOMY".into(), "supervised".into()),
            ("CORIFEU_LIMITS__ROUNDS_PER_PAIR".into(), "7".into()),
        ]);
        let merged = merge(layers);
        assert_eq!(
            merged.origin("autonomy").map(Source::label),
            Some("env CORIFEU_AUTONOMY".to_string())
        );
        assert_eq!(
            merged.origin("limits.rounds_per_pair").map(Source::label),
            Some("env CORIFEU_LIMITS__ROUNDS_PER_PAIR".to_string())
        );
    }
}
