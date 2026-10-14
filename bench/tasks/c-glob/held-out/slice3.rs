// Cases slice 3 is never shown. Slice 1 is four lines; this one is recursive
// and nine cases, which is the point: Q4 asks whether the pair behaves the same
// when writing the implementation costs much more than writing the test.
//
// Every case here follows from the task text — `**` crosses `/`, `*` and `?`
// do not, `*` is any run of characters, `?` is exactly one — and passes against
// the reference implementation in `../src/lib.rs`. Degenerate inputs the task
// text does not settle, such as an empty pattern, are deliberately left out:
// they would measure an accident of the reference rather than the work.
#[cfg(test)]
mod held_out {
    use super::*;

    #[test]
    fn a_literal_pattern_matches_only_itself() {
        assert!(glob_match("src/lib.rs", "src/lib.rs"));
        assert!(!glob_match("src/lib.rs", "src/main.rs"));
    }

    #[test]
    fn question_mark_is_exactly_one_character() {
        assert!(glob_match("?.rs", "a.rs"));
        assert!(!glob_match("?.rs", "ab.rs"));
        assert!(!glob_match("?.rs", ".rs"));
    }

    #[test]
    fn question_mark_does_not_cross_a_slash() {
        assert!(!glob_match("a?c", "a/c"));
    }

    #[test]
    fn star_matches_a_run_of_any_length() {
        assert!(glob_match("a*c", "ac"));
        assert!(glob_match("a*c", "abc"));
        assert!(glob_match("a*c", "abbbbbc"));
    }

    #[test]
    fn several_stars_in_one_segment_backtrack() {
        assert!(glob_match("a*b*c", "axxbyyc"));
        assert!(!glob_match("a*b*c", "axxcyyb"));
    }

    #[test]
    fn star_does_not_cross_a_slash() {
        assert!(glob_match("*.toml", "Cargo.toml"));
        assert!(!glob_match("*.toml", "crates/Cargo.toml"));
    }

    #[test]
    fn a_star_segment_consumes_exactly_one_component() {
        assert!(glob_match("src/*/mod.rs", "src/a/mod.rs"));
        assert!(!glob_match("src/*/mod.rs", "src/a/b/mod.rs"));
        assert!(!glob_match("src/*/mod.rs", "src/mod.rs"));
    }

    #[test]
    fn double_star_crosses_any_number_of_slashes() {
        assert!(glob_match("src/**", "src/a.rs"));
        assert!(glob_match("src/**", "src/a/b/c/d.rs"));
    }

    #[test]
    fn double_star_matches_no_components_at_all() {
        assert!(glob_match("**/*.md", "README.md"));
        assert!(glob_match("**/a.rs", "a.rs"));
    }

    #[test]
    fn double_star_in_the_middle_absorbs_what_is_between() {
        assert!(glob_match("docs/**/*.md", "docs/a/b/c.md"));
        assert!(glob_match("docs/**/*.md", "docs/a.md"));
        assert!(!glob_match("docs/**/*.md", "src/a/b.md"));
    }

    #[test]
    fn a_prefix_that_does_not_match_fails_whatever_follows() {
        assert!(!glob_match("src/**", "docs/a.md"));
        assert!(!glob_match("tests/**", "src/lib.rs"));
    }

    #[test]
    fn matching_is_case_sensitive() {
        assert!(!glob_match("*.TOML", "Cargo.toml"));
        assert!(!glob_match("SRC/**", "src/a.rs"));
    }

    #[test]
    fn a_pattern_longer_than_the_path_does_not_match() {
        assert!(!glob_match("a/b/c", "a/b"));
        assert!(!glob_match("a/*/c", "a/b"));
    }
}
