//! A model that emits its call as text instead of through the tool-call field.

use corifeu_provider::call_in_text;

#[test]
fn bare_json_is_a_call() {
    let call = call_in_text(r#"{"name": "run", "arguments": {"argv": ["cargo", "test"]}}"#)
        .expect("a bare object is the measured Ollama shape");
    assert_eq!(call.name, "run");
    assert_eq!(call.arguments["argv"][0], "cargo");
}

#[test]
fn any_fence_tag_is_dropped_not_just_json() {
    // Measured: qwen2.5-coder fenced JSON under ```xml. Matching a list of
    // known tags would be a list with no end.
    for tag in ["json", "xml", "", "JSON", "tool_call"] {
        let fenced =
            format!("```{tag}\n{{\"name\": \"run\", \"arguments\": {{\"argv\": [\"ls\"]}}}}\n```");
        assert!(call_in_text(&fenced).is_some(), "tag {tag:?}");
    }
}

#[test]
fn prose_around_an_object_is_not_a_call() {
    assert!(call_in_text(r#"I could send {"name": "run"} but I would rather ask."#).is_none());
}

#[test]
fn an_object_without_arguments_is_not_a_call() {
    assert!(call_in_text(r#"{"name": "run"}"#).is_none());
    assert!(call_in_text(r#"{"name": "run", "arguments": "ls"}"#).is_none());
}

#[test]
fn ordinary_prose_is_left_alone() {
    assert!(call_in_text("I'll start by exploring the project structure.").is_none());
}
