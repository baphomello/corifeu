//! A plan is rejected before any of it runs, or it is not a plan.

use corifeu_exec::Allowlist;
use corifeu_planner::{Brief, EMIT_PLAN, Error, Plan, PlannedSlice, Planner, Rejected};
use corifeu_provider::{Budget, Message, Provider, Stop, ToolCall, Turn};
use serde_json::{Value, json};
use std::cell::RefCell;

fn cargo_test() -> Allowlist {
    Allowlist::new([vec!["cargo".into(), "test".into()]])
}

fn slice(task: &str, artefact: &str, oracle: &[&str]) -> PlannedSlice {
    PlannedSlice {
        task: task.into(),
        artefact: artefact.into(),
        oracle: oracle.iter().map(|s| (*s).to_string()).collect(),
    }
}

fn good() -> Plan {
    Plan {
        slices: vec![
            slice(
                "Split a path into components.",
                "src/lib.rs",
                &["cargo", "test", "--lib", "slice1_segs"],
            ),
            slice(
                "Match one pattern segment.",
                "src/lib.rs",
                &["cargo", "test", "--lib", "slice2_seg"],
            ),
        ],
    }
}

struct Replay(RefCell<Vec<Turn>>);

impl Provider for Replay {
    fn runtime(&self) -> &'static str {
        "replay"
    }
    fn chat(
        &self,
        _: &str,
        _: &[Message],
        _: &[Value],
        _: &Budget,
    ) -> Result<Turn, corifeu_provider::Error> {
        Ok(self.0.borrow_mut().remove(0))
    }
}

fn turn(tool_calls: Vec<ToolCall>, content: &str) -> Turn {
    Turn {
        content: content.into(),
        thinking: String::new(),
        tool_calls,
        stop: Stop::Done,
        prompt_tokens: 400,
        output_tokens: 120,
        speed: None,
    }
}

fn planner(provider: &Replay) -> Planner<'_> {
    Planner {
        provider,
        model: "scripted".into(),
        budget: Budget::judge(),
    }
}

fn brief() -> Brief {
    Brief {
        goal: "Classify a path against ordered glob rules.".into(),
        context: "Rust library crate, cargo test.".into(),
    }
}

// ------------------------------------------------------------------ the check

#[test]
fn a_plan_whose_slices_each_name_a_permitted_oracle_is_executable() {
    assert_eq!(good().check(&cargo_test()), Ok(()));
}

#[test]
fn an_empty_plan_is_rejected() {
    assert_eq!(
        Plan { slices: vec![] }.check(&cargo_test()),
        Err(Rejected::Empty)
    );
}

#[test]
fn a_slice_with_no_oracle_could_never_end() {
    let mut plan = good();
    plan.slices[1].oracle.clear();
    assert!(matches!(
        plan.check(&cargo_test()),
        Err(Rejected::NoOracle { at: 1, .. })
    ));
}

#[test]
fn a_slice_with_no_artefact_has_nothing_to_pin() {
    let mut plan = good();
    plan.slices[0].artefact = "  ".into();
    assert!(matches!(
        plan.check(&cargo_test()),
        Err(Rejected::NoArtefact { at: 0, .. })
    ));
}

/// Two slices on one command cannot be told apart: neither goes from failing to
/// passing on its own, so finishing either looks like finishing both.
#[test]
fn two_slices_may_not_share_an_oracle() {
    let mut plan = good();
    plan.slices[1].oracle = plan.slices[0].oracle.clone();

    let rejected = plan.check(&cargo_test()).unwrap_err();
    assert!(matches!(
        rejected,
        Rejected::SharedOracle {
            first: 0,
            second: 1,
            ..
        }
    ));
    assert!(
        rejected
            .to_string()
            .contains("neither can go from failing to passing")
    );
}

/// ADR-0037. Catching this at plan time costs one rejection; catching it at
/// slice time costs a doer round for every slice that carries it.
#[test]
fn an_oracle_the_sandbox_would_refuse_is_rejected_before_anything_runs() {
    let mut plan = good();
    plan.slices[0].oracle = vec!["make".into(), "check".into()];

    let rejected = plan.check(&cargo_test()).unwrap_err();
    assert!(matches!(
        rejected,
        Rejected::OracleNotPermitted { at: 0, .. }
    ));
    assert!(rejected.to_string().contains("could never run"));
}

// ----------------------------------------------------------------- the asking

fn emit(slices: Value) -> ToolCall {
    ToolCall {
        name: EMIT_PLAN.into(),
        arguments: json!({ "slices": slices }),
    }
}

#[test]
fn the_whole_list_comes_back_from_one_call() {
    let provider = Replay(RefCell::new(vec![turn(
        vec![emit(json!([
            {"task": "Split a path into components.", "artefact": "src/lib.rs",
             "oracle": ["cargo", "test", "--lib", "slice1_segs"]},
            {"task": "Match one pattern segment.", "artefact": "src/lib.rs",
             "oracle": ["cargo", "test", "--lib", "slice2_seg"]}
        ]))],
        "",
    )]));

    let plan = planner(&provider).plan(&brief(), &cargo_test()).unwrap();
    assert_eq!(plan, good());
}

#[test]
fn a_planner_that_talks_instead_of_planning_is_an_error_that_quotes_it() {
    let provider = Replay(RefCell::new(vec![turn(
        vec![],
        "I would start by considering the overall architecture of the problem.",
    )]));

    let failed = planner(&provider)
        .plan(&brief(), &cargo_test())
        .unwrap_err();
    assert!(matches!(failed, Error::NoPlan { .. }));
    assert!(failed.to_string().contains("overall architecture"));
}

#[test]
fn a_call_missing_the_slices_array_does_not_become_an_empty_plan() {
    let provider = Replay(RefCell::new(vec![turn(
        vec![ToolCall {
            name: EMIT_PLAN.into(),
            arguments: json!({"plan": []}),
        }],
        "",
    )]));
    assert!(matches!(
        planner(&provider).plan(&brief(), &cargo_test()),
        Err(Error::Shape(_))
    ));
}

#[test]
fn a_slice_missing_its_oracle_array_is_a_shape_error_not_a_silent_drop() {
    let provider = Replay(RefCell::new(vec![turn(
        vec![emit(
            json!([{"task": "Do something.", "artefact": "src/lib.rs"}]),
        )],
        "",
    )]));
    assert!(matches!(
        planner(&provider).plan(&brief(), &cargo_test()),
        Err(Error::Shape(_))
    ));
}

#[test]
fn a_plan_that_parses_but_cannot_run_is_rejected_by_the_same_call() {
    let provider = Replay(RefCell::new(vec![turn(
        vec![emit(json!([
            {"task": "One.", "artefact": "src/lib.rs", "oracle": ["cargo", "test", "--lib", "a"]},
            {"task": "Two.", "artefact": "src/lib.rs", "oracle": ["cargo", "test", "--lib", "a"]}
        ]))],
        "",
    )]));

    assert!(matches!(
        planner(&provider).plan(&brief(), &cargo_test()),
        Err(Error::Rejected(Rejected::SharedOracle { .. }))
    ));
}
