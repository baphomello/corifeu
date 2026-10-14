//! The three things a doer can do, and how they are described to a model.

use serde_json::{Value, json};

pub const RUN: &str = "run";
pub const WRITE_FILE: &str = "write_file";
pub const READ_FILE: &str = "read_file";

/// The schemas sent to the model.
///
/// `run` takes an **array**, and that is not cosmetic. Measured on
/// `ornith1.5:9b`, same prompt, one schema change:
///
/// ```text
/// command: { type: "string" }                ->  "pwd && ls -la"
/// argv:    { type: "array", items: string }  ->  ["ls", "-la"]
/// ```
///
/// ADR-0060 forbids a shell. Declaring the parameter as a vector is what makes
/// the model stop asking for one, rather than having every turn refused.
pub fn schemas() -> Vec<Value> {
    vec![
        json!({"type": "function", "function": {
            "name": RUN,
            "description": "Run a command as an argument vector. There is no shell: no pipes, \
                            no redirection, no globs, and no `&&`. Each argument is a separate \
                            element.",
            "parameters": {"type": "object", "properties": {
                "argv": {"type": "array", "items": {"type": "string"},
                         "description": "Program first, then one element per argument."}},
                "required": ["argv"]}}}),
        json!({"type": "function", "function": {
            "name": WRITE_FILE,
            "description": "Replace a file's entire contents. Send the whole file, not a patch.",
            "parameters": {"type": "object", "properties": {
                "path": {"type": "string"},
                "content": {"type": "string"}},
                "required": ["path", "content"]}}}),
        json!({"type": "function", "function": {
            "name": READ_FILE,
            "description": "Read a file other than the one you are editing. The file you are \
                            editing is already included in full on every step.",
            "parameters": {"type": "object", "properties": {"path": {"type": "string"}},
                "required": ["path"]}}}),
    ]
}

/// What makes two calls the same call, for loop detection (ADR-0050).
///
/// For a write it is the path alone: two writes of different content to one
/// file are different actions and must not collapse into a repeat.
pub fn identifying_argument(name: &str, arguments: &Value) -> String {
    match name {
        RUN => arguments
            .get("argv")
            .and_then(Value::as_array)
            .map(|argv| {
                argv.iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default(),
        _ => arguments
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    }
}

pub fn argv(arguments: &Value) -> Option<Vec<String>> {
    let argv: Vec<String> = arguments
        .get("argv")?
        .as_array()?
        .iter()
        .filter_map(|a| a.as_str().map(str::to_string))
        .collect();
    (!argv.is_empty()).then_some(argv)
}

pub fn string(arguments: &Value, key: &str) -> Option<String> {
    arguments.get(key)?.as_str().map(str::to_string)
}
