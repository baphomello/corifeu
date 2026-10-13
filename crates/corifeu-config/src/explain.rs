//! Rendering for `corifeu config show --explain`.
//!
//! Prints every effective value with the layer that set it. Without this,
//! four-level precedence is a debugging problem of our own making, which is
//! why it is a phase 1 deliverable rather than a later nicety (ADR-0029).

use crate::merge::Merged;
use crate::source::Source;
use serde_json::Value;

/// One effective value and where it came from.
pub struct Entry {
    pub path: String,
    pub value: String,
    pub source: Source,
}

/// Flatten the effective configuration to leaves, in path order.
///
/// `effective` is the fully-defaulted config serialised back to a tree, so
/// values nobody set still appear — attributed to [`Source::Default`], which
/// is the question `--explain` exists to answer.
pub fn entries(effective: &Value, merged: &Merged) -> Vec<Entry> {
    let mut out = Vec::new();
    walk(effective, "", merged, &mut out);
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn walk(value: &Value, prefix: &str, merged: &Merged, out: &mut Vec<Entry>) {
    match value {
        Value::Object(map) if !map.is_empty() => {
            for (key, child) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                walk(child, &path, merged, out);
            }
        }
        _ => {
            if prefix.is_empty() {
                return;
            }
            out.push(Entry {
                path: prefix.to_string(),
                value: render(value),
                source: merged.origin(prefix).cloned().unwrap_or(Source::Default),
            });
        }
    }
}

fn render(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "~".to_string(),
        other => other.to_string(),
    }
}

/// Human-readable table. Defaults are included: a value that is *not* set is
/// usually what the reader is trying to find out.
pub fn render_table(entries: &[Entry]) -> String {
    let width = entries.iter().map(|e| e.path.len()).max().unwrap_or(0);
    let vwidth = entries
        .iter()
        .map(|e| e.value.len().min(40))
        .max()
        .unwrap_or(0);

    let mut s = String::new();
    for entry in entries {
        let value = if entry.value.len() > 40 {
            format!("{}…", &entry.value[..39])
        } else {
            entry.value.clone()
        };
        s.push_str(&format!(
            "{:<width$}  {:<vwidth$}  {}\n",
            entry.path,
            value,
            entry.source.label(),
            width = width,
            vwidth = vwidth
        ));
    }
    s
}
