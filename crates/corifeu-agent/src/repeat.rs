//! When a repeated action is a warning, and when it ends the run (ADR-0050).

use crate::Action;

/// What to do about an action the doer is about to take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repeat {
    /// Nothing like it since the last change. Run it.
    New,
    /// Already run, with nothing changed since, so the result would be
    /// identical. Deliver `warning` as the result of this call instead of
    /// running it, and let the doer decide what to do with the step.
    Warned {
        strike: u32,
        of: u32,
        warning: String,
    },
    /// The strikes are spent. The run stops.
    Spent { strikes: u32 },
}

/// Counts consecutive repeats.
///
/// The measured default is six. Three killed the winning Task B round at step
/// 26, because it hit four dead ends in a row at steps 28–31 and again at
/// 50–53 before finding the fix — a run that was not looping, only searching.
#[derive(Debug, Clone)]
pub struct Detector {
    strike_limit: u32,
    strikes: u32,
    /// Actions taken since the last artefact change. Cleared when one happens,
    /// because the same command against changed code is a different action
    /// (ADR-0050 §4, ADR-0018).
    since_change: Vec<Action>,
}

impl Default for Detector {
    fn default() -> Self {
        Self::with_strikes(6)
    }
}

impl Detector {
    pub fn with_strikes(strike_limit: u32) -> Self {
        Self {
            strike_limit,
            strikes: 0,
            since_change: Vec::new(),
        }
    }

    pub fn consider(&mut self, action: &Action) -> Repeat {
        if !self.since_change.contains(action) {
            self.strikes = 0;
            self.since_change.push(action.clone());
            return Repeat::New;
        }

        self.strikes += 1;
        if self.strikes >= self.strike_limit {
            return Repeat::Spent {
                strikes: self.strikes,
            };
        }
        Repeat::Warned {
            strike: self.strikes,
            of: self.strike_limit,
            warning: warning(action),
        }
    }

    /// An artefact changed, so every earlier action could now give a different
    /// answer.
    pub fn artefact_changed(&mut self) {
        self.since_change.clear();
        self.strikes = 0;
    }

    pub fn strikes(&self) -> u32 {
        self.strikes
    }
}

/// ADR-0050 §3: the warning points at the oracle, never at more reading.
///
/// The first version of this text told a stuck doer to inspect part of the file
/// it had not read yet. It spent five steps and 291 output tokens doing exactly
/// that and nothing else. Under ADR-0047 the artefact is already in front of it,
/// so reading is never the answer; only a check or a write can produce
/// information that does not already exist.
fn warning(action: &Action) -> String {
    format!(
        "REPEAT: you already ran `{} {}` and nothing has changed since, so the result would \
         be identical and this call was not run. Reading will not help either — the file you \
         are editing is included in full, fresh from disk, on every step. Two things can \
         produce information you do not already have: run the check to get a fresh error, or \
         write a change to the file.",
        action.tool, action.argument
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(tool: &str, argument: &str) -> Action {
        Action::new(tool, argument)
    }

    #[test]
    fn a_first_action_runs() {
        assert_eq!(
            Detector::default().consider(&act("run", "cargo test")),
            Repeat::New
        );
    }

    #[test]
    fn a_repeat_warns_rather_than_stopping() {
        let mut d = Detector::default();
        d.consider(&act("run", "cargo test"));
        let second = d.consider(&act("run", "cargo test"));

        assert!(matches!(
            second,
            Repeat::Warned {
                strike: 1,
                of: 6,
                ..
            }
        ));
    }

    #[test]
    fn the_warning_sends_the_doer_to_the_oracle_not_to_the_file() {
        let mut d = Detector::default();
        d.consider(&act("read_file", "src/lib.rs"));
        let Repeat::Warned { warning, .. } = d.consider(&act("read_file", "src/lib.rs")) else {
            panic!("the second identical read must warn");
        };

        assert!(warning.contains("run the check"));
        assert!(warning.contains("write a change"));
        assert!(
            !warning.contains("read the rest") && !warning.contains("have not read"),
            "the first version of this text produced a five-step read loop: {warning}"
        );
    }

    #[test]
    fn a_different_action_resets_the_strikes() {
        let mut d = Detector::default();
        for _ in 0..3 {
            d.consider(&act("run", "cargo test"));
        }
        assert_eq!(d.strikes(), 2);

        d.consider(&act("run", "cargo build"));
        assert_eq!(d.strikes(), 0, "searching is not looping");
    }

    #[test]
    fn four_dead_ends_in_a_row_survive_the_measured_default() {
        // Task B's winning round hit four consecutive repeats twice and went
        // on to reach 21 of 21. A limit of three would have ended it at the
        // first of them.
        let mut d = Detector::default();
        let mut verdicts = Vec::new();
        for _ in 0..5 {
            verdicts.push(d.consider(&act("run", "cargo test")));
        }
        assert!(
            verdicts.iter().all(|v| !matches!(v, Repeat::Spent { .. })),
            "five consecutive repeats must still be survivable"
        );
    }

    #[test]
    fn the_sixth_consecutive_repeat_ends_the_run() {
        let mut d = Detector::default();
        let mut last = Repeat::New;
        for _ in 0..7 {
            last = d.consider(&act("run", "cargo test"));
        }
        assert_eq!(last, Repeat::Spent { strikes: 6 });
    }

    #[test]
    fn changing_the_artefact_makes_the_same_command_new_again() {
        let mut d = Detector::default();
        d.consider(&act("run", "cargo test"));
        d.consider(&act("run", "cargo test"));

        d.artefact_changed();

        assert_eq!(
            d.consider(&act("run", "cargo test")),
            Repeat::New,
            "the same command against changed code is a different action"
        );
        assert_eq!(d.strikes(), 0);
    }
}
