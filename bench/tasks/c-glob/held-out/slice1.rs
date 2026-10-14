// Cases slice 1 is never shown. ADR-0053 turns on whether a pair where QA also
// wrote the implementation produces worse code than one where it did not, and
// that cannot be read off the tests QA wrote — those it passes by construction.
// These are run against the finished crate instead.
//
// Every case here passes against the reference implementation in
// `../src/lib.rs`. A case the reference fails would measure the suite, not the
// work.
#[cfg(test)]
mod held_out {
    use super::*;

    #[test]
    fn splits_on_slash() {
        assert_eq!(
            split_segs("src/bank/deposit.ts"),
            vec!["src", "bank", "deposit.ts"]
        );
    }

    #[test]
    fn a_path_with_no_separator_is_one_component() {
        assert_eq!(split_segs("Cargo.toml"), vec!["Cargo.toml"]);
    }

    #[test]
    fn repeated_separators_collapse() {
        assert_eq!(split_segs("a//b"), vec!["a", "b"]);
        assert_eq!(split_segs("a///////b"), vec!["a", "b"]);
    }

    #[test]
    fn leading_and_trailing_separators_are_dropped() {
        assert_eq!(split_segs("/a/b/"), vec!["a", "b"]);
        assert_eq!(split_segs("//a//"), vec!["a"]);
    }

    #[test]
    fn an_empty_path_has_no_components() {
        assert!(split_segs("").is_empty());
    }

    /// Nothing but separators is still nothing. The usual way to get this wrong
    /// is to special-case the empty string and forget this one.
    #[test]
    fn a_path_of_separators_alone_has_no_components() {
        assert!(split_segs("/").is_empty());
        assert!(split_segs("///").is_empty());
    }

    /// Splitting is not normalising. `.` and `..` are ordinary components.
    #[test]
    fn dot_components_are_kept_as_written() {
        assert_eq!(split_segs("./a/../b"), vec![".", "a", "..", "b"]);
    }

    #[test]
    fn a_backslash_is_not_a_separator() {
        assert_eq!(split_segs("a\\b/c"), vec!["a\\b", "c"]);
    }

    #[test]
    fn components_keep_their_spaces_and_dots() {
        assert_eq!(split_segs("my docs/a.b.c"), vec!["my docs", "a.b.c"]);
    }

    #[test]
    fn components_are_not_ascii_only() {
        assert_eq!(split_segs("café/ñandú"), vec!["café", "ñandú"]);
    }

    /// A component may be a single character, including one that looks like a
    /// wildcard elsewhere in this crate.
    #[test]
    fn single_character_components_survive() {
        assert_eq!(split_segs("a/*/?"), vec!["a", "*", "?"]);
    }

    #[test]
    fn a_long_path_keeps_every_component_in_order() {
        let path = (0..100)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("/");
        let segs = split_segs(&path);
        assert_eq!(segs.len(), 100);
        assert_eq!(segs[0], "0");
        assert_eq!(segs[99], "99");
    }
}
