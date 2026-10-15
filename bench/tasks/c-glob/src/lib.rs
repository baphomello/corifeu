//! Classify a path against an ordered list of file classes.

/// A named class and the glob patterns that select files into it.
pub struct FileClass {
    pub name: String,
    pub include: Vec<String>,
}

/// Return the name of the first class whose patterns match `path`.
pub fn classify<'a>(classes: &'a [FileClass], path: &str) -> Option<&'a str> {
    classes.iter().find_map(|c| {
        if c.include.iter().any(|p| glob_match(p, path)) {
            Some(c.name.as_str())
        } else {
            None
        }
    })
}

/// Split a path into its non-empty components.
fn split_segs(path: &str) -> Vec<&str> {
    path.split('/')
        .filter(|seg| !seg.is_empty())
        .collect()
}

/// Match one pattern segment against one path component.
/// `*` matches any run of characters, `?` matches exactly one.
fn match_seg(pattern: &str, component: &str) -> bool {
    match_seg_impl(pattern.as_bytes(), component.as_bytes())
}

/// Byte-level backtracking matcher for a single pattern segment.
fn match_seg_impl(pattern: &[u8], component: &[u8]) -> bool {
    /// Recursive backtracking: try to match `pat[p..]` against `comp[c..]`.
    fn rec(pat: &[u8], comp: &[u8], p: usize, c: usize) -> bool {
        if p == pat.len() {
            return c == comp.len();
        }
        match pat[p] {
            b'*' => {
                // `*` matches any run: try consuming zero, one, two, ... bytes.
                let mut star = c;
                while star <= comp.len() {
                    if rec(pat, comp, p + 1, star) {
                        return true;
                    }
                    star += 1;
                }
                false
            }
            b'?' => c < comp.len() && rec(pat, comp, p + 1, c + 1),
            other => c < comp.len() && pat[p] == comp[c] && rec(pat, comp, p + 1, c + 1),
        }
    }
    rec(pattern, component, 0, 0)
}

/// Match a whole glob pattern against a whole path.
/// `**` matches any number of path components, including none.
fn glob_match(pattern: &str, path: &str) -> bool {
    let pats = split_segs(pattern);
    let path_segs = split_segs(path);

    // Try each pattern component against the path components, allowing
    // `**` to absorb zero or more remaining components.
    let mut pi = 0;
    let mut consumed = 0;

    while pi < pats.len() {
        match pats[pi] {
            "**" => {
                // `**` can match any number of remaining path components.
                for take in 0..=(path_segs.len() - consumed) {
                    if glob_match_suffix(&pats[pi + 1..], &path_segs[consumed + take..]) {
                        return true;
                    }
                }
                return false;
            }
            other => {
                if consumed < path_segs.len() && match_seg(other, &path_segs[consumed]) {
                    consumed += 1;
                    pi += 1;
                } else {
                    return false;
                }
            }
        }
    }

    consumed == path_segs.len()
}

fn glob_match_suffix(pattern: &[&str], path: &[&str]) -> bool {
    let mut pi = 0;
    let mut si = 0;

    while pi < pattern.len() && si < path.len() {
        match pattern[pi] {
            "**" => {
                // `**` matches any number of remaining components; try the
                // rest of the pattern against every split point.
                for take in 0..=(path.len() - si) {
                    if glob_match_suffix(&pattern[pi + 1..], &path[si + take..]) {
                        return true;
                    }
                }
                return false;
            }
            other => {
                if match_seg(other, &path[si]) {
                    si += 1;
                    pi += 1;
                } else {
                    return false;
                }
            }
        }
    }

    pi == pattern.len() && si == path.len()
}

#[cfg(test)]
mod slice1_segs {
    use super::*;

    #[test] fn splits_on_slash() {
        assert_eq!(split_segs("src/bank/deposit.ts"), vec!["src", "bank", "deposit.ts"]);
    }
    #[test] fn single_component() {
        assert_eq!(split_segs("Cargo.toml"), vec!["Cargo.toml"]);
    }
    #[test] fn drops_empty_components() {
        assert_eq!(split_segs("a//b"), vec!["a", "b"]);
        assert_eq!(split_segs("/a/b/"), vec!["a", "b"]);
    }
    #[test] fn empty_path_has_no_components() {
        assert!(split_segs("").is_empty());
    }
}

#[cfg(test)]
mod slice2_seg {
    use super::*;

    #[test] fn literal_must_be_exact() {
        assert!(match_seg("src", "src"));
        assert!(!match_seg("src", "lib"));
    }
    #[test] fn case_is_significant() {
        assert!(!match_seg("SRC", "src"));
    }
    #[test] fn star_matches_any_run() {
        assert!(match_seg("*.ts", "deposit.ts"));
        assert!(!match_seg("*.ts", "deposit.js"));
    }
    #[test] fn star_matches_nothing_too() {
        assert!(match_seg("*.ts", ".ts"));
        assert!(match_seg("*", ""));
    }
    #[test] fn bare_star_matches_anything() {
        assert!(match_seg("*", "deposit.ts"));
    }
    #[test] fn question_matches_exactly_one() {
        assert!(match_seg("?.ts", "a.ts"));
        assert!(!match_seg("?.ts", "ab.ts"));
        assert!(!match_seg("?.ts", ".ts"));
    }
    #[test] fn star_backtracks() {
        // A greedy `*` that never gives characters back fails this.
        assert!(match_seg("*_test.*", "deposit_test.ts"));
        assert!(match_seg("*_test.*", "a_test_test.ts"));
        assert!(!match_seg("*_test.*", "deposit.ts"));
    }
    #[test] fn several_wildcards() {
        assert!(match_seg("*-*.spec.?s", "user-auth.spec.ts"));
        assert!(!match_seg("*-*.spec.?s", "userauth.spec.ts"));
    }
}

#[cfg(test)]
mod slice3_glob {
    use super::*;

    #[test] fn literal_path() {
        assert!(glob_match("Cargo.toml", "Cargo.toml"));
        assert!(!glob_match("Cargo.toml", "Cargo.lock"));
    }
    #[test] fn star_does_not_cross_a_slash() {
        assert!(glob_match("*.toml", "Cargo.toml"));
        assert!(!glob_match("*.toml", "crates/Cargo.toml"));
    }
    #[test] fn double_star_absorbs_components() {
        assert!(glob_match("src/**", "src/bank/deposit.ts"));
        assert!(glob_match("src/**", "src/a.ts"));
    }
    #[test] fn double_star_matches_zero_components() {
        assert!(glob_match("**/*_test.*", "deposit_test.ts"));
        assert!(glob_match("**/*.md", "README.md"));
    }
    #[test] fn double_star_in_the_middle() {
        assert!(glob_match("**/*_test.*", "src/bank/deposit_test.ts"));
        assert!(glob_match("docs/**/*.md", "docs/a/b/c.md"));
    }
    #[test] fn single_star_segment_is_exactly_one() {
        assert!(glob_match("src/*/mod.rs", "src/a/mod.rs"));
        assert!(!glob_match("src/*/mod.rs", "src/a/b/mod.rs"));
        assert!(!glob_match("src/*/mod.rs", "src/mod.rs"));
    }
    #[test] fn bare_double_star_matches_everything() {
        assert!(glob_match("**", "a/b/c.ts"));
        assert!(glob_match("**", "x"));
    }
    #[test] fn non_matching_returns_false() {
        assert!(!glob_match("src/**", "docs/a.md"));
        assert!(!glob_match("tests/**", "src/lib.rs"));
    }
    #[test] fn pattern_longer_than_path() {
        assert!(!glob_match("a/b/c", "a/b"));
    }
}

#[cfg(test)]
mod slice4_classify {
    use super::*;

    fn class(name: &str, patterns: &[&str]) -> FileClass {
        FileClass {
            name: name.to_string(),
            include: patterns.iter().map(|p| p.to_string()).collect(),
        }
    }

    #[test] fn first_match_wins() {
        let classes = vec![
            class("test", &["**/*_test.*"]),
            class("source", &["src/**"]),
        ];
        assert_eq!(classify(&classes, "src/bank/deposit_test.ts"), Some("test"));
        assert_eq!(classify(&classes, "src/bank/deposit.ts"), Some("source"));
    }
    #[test] fn order_decides_the_answer() {
        let a = vec![class("source", &["src/**"]), class("test", &["**/*_test.*"])];
        assert_eq!(classify(&a, "src/bank/deposit_test.ts"), Some("source"));
    }
    #[test] fn no_class_matches() {
        let classes = vec![class("source", &["src/**"])];
        assert_eq!(classify(&classes, "README.md"), None);
    }
    #[test] fn any_pattern_in_a_class_matches() {
        let classes = vec![class("config", &["*.toml", "*.yaml", ".github/**"])];
        assert_eq!(classify(&classes, "Cargo.toml"), Some("config"));
        assert_eq!(classify(&classes, "app.yaml"), Some("config"));
        assert_eq!(classify(&classes, ".github/workflows/ci.yml"), Some("config"));
    }
    #[test] fn empty_class_list_matches_nothing() {
        assert_eq!(classify(&[], "src/a.ts"), None);
    }
}
