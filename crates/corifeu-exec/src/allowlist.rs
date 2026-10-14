//! Which commands a role may run (ADR-0037, stage one).

use crate::Command;

/// Argument-vector prefixes a role is permitted to run.
///
/// Matching is per element, not per character, which is what makes an argv
/// allowlist worth having: `["cargo", "test"]` permits `cargo test --lib x`
/// and refuses `cargo publish`, with no string prefix able to blur the two.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Allowlist {
    prefixes: Vec<Vec<String>>,
}

impl Allowlist {
    /// An empty allowlist permits nothing.
    ///
    /// ADR-0037 declares the project's build, test, lint and format commands at
    /// install time and sends everything else for approval, so "nothing is
    /// pre-approved" is the correct starting state. A default that permitted
    /// everything would be the one configuration where forgetting to configure
    /// is indistinguishable from configuring it open.
    pub fn new(prefixes: impl IntoIterator<Item = Vec<String>>) -> Self {
        Self {
            prefixes: prefixes.into_iter().filter(|p| !p.is_empty()).collect(),
        }
    }

    pub fn permits(&self, command: &Command) -> bool {
        let argv = command.argv();
        self.prefixes
            .iter()
            .any(|prefix| argv.len() >= prefix.len() && argv[..prefix.len()] == prefix[..])
    }

    /// Rendered for the agent when a command is refused. A denial it cannot
    /// tell from a bug is a denial it will retry (ADR-0052).
    pub fn entries(&self) -> Vec<String> {
        self.prefixes.iter().map(|p| p.join(" ")).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.prefixes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> Allowlist {
        Allowlist::new([
            vec!["cargo".into(), "test".into()],
            vec!["cargo".into(), "build".into()],
        ])
    }

    #[test]
    fn a_prefix_permits_further_arguments() {
        assert!(list().permits(&Command::new("cargo").arg("test").arg("--lib")));
    }

    #[test]
    fn the_prefix_itself_is_permitted() {
        assert!(list().permits(&Command::new("cargo").arg("test")));
    }

    #[test]
    fn a_sibling_subcommand_is_refused() {
        assert!(!list().permits(&Command::new("cargo").arg("publish")));
    }

    #[test]
    fn matching_is_per_element_not_per_character() {
        // A string allowlist on "cargo test" would admit this. An argv one
        // cannot: "testament" is a different element from "test".
        assert!(!list().permits(&Command::new("cargo").arg("testament")));
    }

    #[test]
    fn an_empty_allowlist_permits_nothing() {
        assert!(!Allowlist::default().permits(&Command::new("cargo").arg("test")));
    }

    #[test]
    fn an_empty_prefix_is_dropped_rather_than_permitting_everything() {
        let careless = Allowlist::new([vec![]]);
        assert!(careless.is_empty());
        assert!(!careless.permits(&Command::new("rm").arg("-rf").arg("/")));
    }
}
