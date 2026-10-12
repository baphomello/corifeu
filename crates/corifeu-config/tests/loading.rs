//! End-to-end loading: precedence, validation, and the errors a user sees.

use corifeu_config::{Layer, Severity, Source, layer_from_str, load};
use std::path::PathBuf;

const MINIMAL: &str = r#"
version: 1
roles:
  lead:
    category: lead
    provider: claude
    model: opus
    required: true
  coder:
    category: doers
    provider: local
    model: qwen2.5-coder:7b
    required: true
    writes: [source]
oracles:
  test: { command: ["npm", "test"] }
"#;

fn project(text: &str) -> Layer {
    layer_from_str(text, Source::Project(PathBuf::from("project.yaml"))).expect("parses")
}

fn global(text: &str) -> Layer {
    layer_from_str(text, Source::Global(PathBuf::from("global.yaml"))).expect("parses")
}

fn env(name: &str, value: &str) -> Layer {
    corifeu_config::env::from_pairs(vec![(name.to_string(), value.to_string())])
        .pop()
        .expect("one layer")
}

#[test]
fn minimal_configuration_loads() {
    let loaded = load(vec![project(MINIMAL)]).expect("valid");
    assert_eq!(loaded.config.version, 1);
    assert_eq!(loaded.config.roles.len(), 2);
    assert!(loaded.warnings.is_empty());
}

#[test]
fn defaults_apply_where_nothing_is_set() {
    let loaded = load(vec![project(MINIMAL)]).expect("valid");
    // ADR-0018: three rounds per pair. ADR-0022: ten full runs.
    assert_eq!(loaded.config.limits.rounds_per_pair, 3);
    assert_eq!(loaded.config.retention.full_runs, 10);
    // ADR-0025: every git gate defaults to its most conservative option, so
    // clicking through the wizard without reading cannot surprise anyone.
    assert_eq!(
        loaded.config.git.push.when,
        corifeu_config::model::GateWhen::Never
    );
}

#[test]
fn env_beats_project_which_beats_global() {
    let loaded = load(vec![
        global("version: 1\nautonomy: autonomous\nlimits: { rounds_per_pair: 1 }\n"),
        project(MINIMAL),
        project("version: 1\nautonomy: semi-autonomous\n"),
        env("CORIFEU_AUTONOMY", "supervised"),
    ])
    .expect("valid");

    assert_eq!(
        loaded.config.autonomy,
        corifeu_config::model::Autonomy::Supervised
    );
    // The global layer still supplies what nothing stronger set.
    assert_eq!(loaded.config.limits.rounds_per_pair, 1);
}

#[test]
fn explain_names_the_layer_that_set_each_value() {
    let loaded = load(vec![
        global("version: 1\nlimits: { rounds_per_pair: 1 }\n"),
        project(MINIMAL),
        env("CORIFEU_AUTONOMY", "supervised"),
    ])
    .expect("valid");

    let entries = loaded.entries().expect("serialises");
    let find = |path: &str| {
        entries
            .iter()
            .find(|e| e.path == path)
            .unwrap_or_else(|| panic!("{path} missing"))
    };

    assert_eq!(find("autonomy").source.label(), "env CORIFEU_AUTONOMY");
    assert!(
        find("limits.rounds_per_pair")
            .source
            .label()
            .starts_with("global")
    );
    assert!(find("version").source.label().starts_with("project"));
    // A value nobody set is attributed to the default, which is usually the
    // question being asked.
    assert_eq!(find("retention.full_runs").source, Source::Default);
}

#[test]
fn syntax_error_reports_line_and_column() {
    let err = layer_from_str("a: 1\n\tb: 2\n", Source::Project(PathBuf::from("p.yaml")))
        .expect_err("tab is invalid");
    let text = err.to_string();
    assert!(text.contains("line"), "no line number in: {text}");
    assert!(text.contains("tab"), "cause not named in: {text}");
}

#[test]
fn shape_error_names_the_field_and_its_source() {
    let err = load(vec![project(
        "version: 1\nlimits: { rounds_per_pair: lots }\nroles: {}\n",
    )])
    .expect_err("not a number");
    let text = err.to_string();
    assert!(
        text.contains("limits.rounds_per_pair"),
        "no path in: {text}"
    );
    assert!(text.contains("project.yaml"), "no source in: {text}");
}

#[test]
fn unknown_field_is_rejected() {
    let err = load(vec![project(
        "version: 1\nroles: {}\nautonmy: supervised\n",
    )])
    .expect_err("typo");
    assert!(err.to_string().contains("autonmy"), "{err}");
}

#[test]
fn exactly_one_lead_is_required() {
    let two = MINIMAL.replace(
        "  coder:\n    category: doers",
        "  second:\n    category: lead\n    provider: claude\n    model: opus\n  coder:\n    category: doers",
    );
    let err = load(vec![project(&two)]).expect_err("two leads");
    assert!(err.to_string().contains("exactly one"), "{err}");

    let none = MINIMAL.replace("category: lead", "category: planners");
    let err = load(vec![project(&none)]).expect_err("no lead");
    assert!(err.to_string().contains("`lead`"), "{err}");
}

#[test]
fn a_writing_doer_is_required() {
    let err = load(vec![project(
        &MINIMAL.replace("    writes: [source]\n", ""),
    )])
    .expect_err("no writer");
    assert!(err.to_string().contains("writes"), "{err}");
}

#[test]
fn the_lead_role_may_not_write() {
    let cfg = MINIMAL.replace(
        "    model: opus\n    required: true",
        "    model: opus\n    required: true\n    writes: [docs]",
    );
    let err = load(vec![project(&cfg)]).expect_err("lead writes");
    assert!(err.to_string().contains("writes nothing"), "{err}");
}

#[test]
fn dangling_references_are_rejected() {
    let cfg = format!("{MINIMAL}channels:\n  - [coder, ghost]\n");
    let err = load(vec![project(&cfg)]).expect_err("unknown role");
    assert!(err.to_string().contains("ghost"), "{err}");
}

#[test]
fn local_only_refuses_a_remote_provider() {
    let cfg = format!(
        "{MINIMAL}privacy: {{ allow_remote_providers: false }}\n\
         providers:\n  claude: {{ kind: cli }}\n  local: {{ kind: ollama }}\n"
    );
    let err = load(vec![project(&cfg)]).expect_err("remote lead");
    assert!(err.to_string().contains("not local"), "{err}");
}

#[test]
fn self_testing_warns_but_still_loads() {
    let cfg = MINIMAL.replace("writes: [source]", "writes: [source, test]");
    let loaded = load(vec![project(&cfg)]).expect("permitted");
    let warning = loaded
        .warnings
        .iter()
        .find(|p| p.severity == Severity::Warning)
        .expect("warned");
    assert!(
        warning.message.contains("marking its own homework"),
        "{warning}"
    );
}

#[test]
fn sibling_test_files_classify_as_tests() {
    // ADR-0035: the case a directory prefix cannot express.
    let loaded = load(vec![project(MINIMAL)]).expect("valid");
    let classes = &loaded.config.file_classes;
    assert_eq!(
        classes[0].name, "test",
        "test must be declared before source"
    );
    assert_eq!(classes[1].name, "source");
}
