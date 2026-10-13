//! Which class a file belongs to (ADR-0035).
//!
//! Ownership is by class, not by directory, because the case that matters is a
//! test sitting beside the source it tests. `src/bank/deposit_test.ts` and
//! `src/bank/deposit.ts` share a directory and must not share an owner, so a
//! path prefix cannot express the rule and an ordered glob can.
//!
//! First match wins. `test` is declared before `source` in the defaults, which
//! is the whole reason the order is part of the configuration rather than an
//! implementation detail.

/// One class and the globs that claim a path for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    pub name: String,
    include: Vec<String>,
    exclude: Vec<String>,
}

impl Class {
    /// Brace alternatives are expanded once, here, so matching stays a hot
    /// path with no parsing in it.
    pub fn new(
        name: impl Into<String>,
        include: impl IntoIterator<Item = impl Into<String>>,
        exclude: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            include: expand_all(include),
            exclude: expand_all(exclude),
        }
    }

    pub fn claims(&self, path: &str) -> bool {
        self.include.iter().any(|p| matches(p, path))
            && !self.exclude.iter().any(|p| matches(p, path))
    }
}

fn expand_all(patterns: impl IntoIterator<Item = impl Into<String>>) -> Vec<String> {
    patterns
        .into_iter()
        .flat_map(|p| expand(&p.into()))
        .collect()
}

/// Ordered classes. The first whose globs claim a path owns it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Classes(Vec<Class>);

impl Classes {
    pub fn new(classes: impl IntoIterator<Item = Class>) -> Self {
        Self(classes.into_iter().collect())
    }

    /// The class owning `path`, or `None` where no class claims it. An
    /// unclassified file is not a free-for-all: it belongs to nobody, and a
    /// role that owns no class for it cannot write it.
    pub fn classify(&self, path: &str) -> Option<&str> {
        let path = normalise(path);
        self.0
            .iter()
            .find(|class| class.claims(&path))
            .map(|class| class.name.as_str())
    }

    pub fn names(&self) -> Vec<&str> {
        self.0.iter().map(|c| c.name.as_str()).collect()
    }
}

/// Separators are normalised and a leading `./` dropped, so a Windows path and
/// a relative one classify the same as the path a glob is written against.
fn normalise(path: &str) -> String {
    let path = path.replace('\\', "/");
    path.strip_prefix("./").unwrap_or(&path).to_string()
}

/// Match a path against one glob.
///
/// `*` and `?` stop at a separator; `**` crosses them. That distinction is the
/// reason `*.toml` claims `Cargo.toml` and leaves `crates/x/Cargo.toml` alone.
///
/// `pattern` is taken literally, braces included. Alternatives are expanded
/// once by [`Class::new`], so nothing on this path parses them.
pub fn matches(pattern: &str, path: &str) -> bool {
    let normalised = normalise(path);
    let pattern: Vec<&str> = pattern.split('/').collect();
    let path: Vec<&str> = normalised.split('/').collect();
    segments(&pattern, &path)
}

fn segments(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => (0..=path.len()).any(|skip| segments(rest, &path[skip..])),
        Some((head, rest)) => match path.split_first() {
            Some((first, tail)) => segment(head, first) && segments(rest, tail),
            None => false,
        },
    }
}

/// `*` and `?` within one path component.
///
/// Iterative with a single backtrack point, rather than recursive: the
/// measurement's own glob task failed on a matcher that recursed without a
/// bound and hung, which is a defect this shape cannot have.
fn segment(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0, 0);
    let (mut star, mut resume) = (None, 0);

    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            resume = ti;
            pi += 1;
        } else if let Some(at) = star {
            pi = at + 1;
            resume += 1;
            ti = resume;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

/// Expand `*.{json,toml}` into `*.json` and `*.toml`, including when nested.
///
/// `config.md` documents this form, so it is supported rather than quietly
/// treated as a literal brace.
fn expand(pattern: &str) -> Vec<String> {
    let Some(open) = pattern.find('{') else {
        return vec![pattern.to_string()];
    };
    let Some(close) = close_of(pattern, open) else {
        return vec![pattern.to_string()];
    };

    let (head, tail) = (&pattern[..open], &pattern[close + 1..]);
    alternatives(&pattern[open + 1..close])
        .iter()
        .flat_map(|alt| expand(&format!("{head}{alt}{tail}")))
        .collect()
}

fn close_of(pattern: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (at, c) in pattern.char_indices().skip(open) {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

fn alternatives(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (at, c) in body.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(body[start..at].to_string());
                start = at + 1;
            }
            _ => {}
        }
    }
    out.push(body[start..].to_string());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The defaults, in the order that makes them work.
    fn defaults() -> Classes {
        Classes::new([
            Class::new(
                "test",
                ["**/*_test.*", "**/*.test.*", "tests/**"],
                [] as [&str; 0],
            ),
            Class::new("source", ["src/**", "lib/**"], [] as [&str; 0]),
            Class::new(
                "config",
                ["*.{json,toml,yaml,yml}", ".github/**"],
                [] as [&str; 0],
            ),
            Class::new("docs", ["**/*.md", "docs/**"], [] as [&str; 0]),
        ])
    }

    #[test]
    fn double_star_crosses_separators() {
        assert!(matches("src/**", "src/a.rs"));
        assert!(matches("src/**", "src/a/b/c.rs"));
        assert!(!matches("src/**", "lib/a.rs"));
    }

    #[test]
    fn a_single_star_stops_at_a_separator() {
        assert!(matches("*.toml", "Cargo.toml"));
        assert!(
            !matches("*.toml", "crates/x/Cargo.toml"),
            "this is the distinction a directory prefix cannot express"
        );
    }

    #[test]
    fn a_star_does_not_swallow_part_of_the_suffix() {
        // The monolithic Task C asserted the opposite and never found out,
        // because its implementation hung before any assertion ran.
        assert!(matches("**/*.cfg", "a/b.cfg"));
        assert!(!matches("**/*.cfg", "a/b.ccfg"));
    }

    #[test]
    fn a_question_mark_is_exactly_one_character() {
        // Also inverted in the monolithic attempt's own test suite.
        assert!(matches("x?.rs", "xa.rs"));
        assert!(!matches("x?.rs", "x_a.rs"));
        assert!(!matches("x?.rs", "x.rs"));
    }

    #[test]
    fn a_leading_double_star_matches_at_the_root() {
        assert!(matches("**/*.md", "README.md"));
        assert!(matches("**/*.md", "docs/a/b.md"));
        assert!(matches("**/*_test.*", "a_test.rs"));
        assert!(matches("**/*_test.*", "src/bank/deposit_test.ts"));
    }

    #[test]
    fn a_star_backtracks_rather_than_taking_the_first_match() {
        assert!(matches("*_test.*", "a_test_test.ts"));
    }

    #[test]
    fn first_match_wins_so_a_test_beside_its_source_is_a_test() {
        let classes = defaults();
        assert_eq!(classes.classify("src/bank/deposit_test.ts"), Some("test"));
        assert_eq!(classes.classify("src/bank/deposit.ts"), Some("source"));
    }

    #[test]
    fn braces_expand_into_alternatives() {
        let classes = defaults();
        for name in ["Cargo.toml", "package.json", "config.yaml", "x.yml"] {
            assert_eq!(classes.classify(name), Some("config"), "{name}");
        }
    }

    #[test]
    fn nested_braces_expand_too() {
        // Through a Class: expansion happens once at construction, so
        // `matches` itself never sees a brace.
        let styles = Class::new("styles", ["a.{c{s,ss},js}"], [] as [&str; 0]);
        for yes in ["a.cs", "a.css", "a.js"] {
            assert!(styles.claims(yes), "{yes}");
        }
        assert!(!styles.claims("a.ts"));
    }

    #[test]
    fn an_exclude_sends_the_path_on_to_the_next_class() {
        let classes = Classes::new([
            Class::new("source", ["src/**"], ["src/generated/**"]),
            Class::new("generated", ["**"], [] as [&str; 0]),
        ]);
        assert_eq!(classes.classify("src/a.rs"), Some("source"));
        assert_eq!(classes.classify("src/generated/a.rs"), Some("generated"));
    }

    #[test]
    fn an_unclaimed_path_belongs_to_nobody() {
        assert_eq!(defaults().classify("Makefile"), None);
    }

    #[test]
    fn windows_separators_classify_the_same() {
        assert_eq!(defaults().classify(r"src\bank\deposit.ts"), Some("source"));
        assert_eq!(defaults().classify("./src/bank/deposit.ts"), Some("source"));
    }
}
