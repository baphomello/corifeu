"""Classify a path against an ordered list of file classes.

Task C, function for function and case for case, in Python. Round five's Q3
asks whether ADR-0062's floor of twelve steps is a property of how a model
works or a property of the Rust compiler telling it what is wrong. The only way
that question has an answer is if the two tasks differ in nothing but the
language, so this file is a port and not a rewrite: same decomposition, same
helper chain, same 4-8-9-5 split of the cases.
"""


class FileClass:
    """A named class and the glob patterns that select files into it."""

    def __init__(self, name: str, include: list[str]) -> None:
        self.name = name
        self.include = include


def classify(classes: list[FileClass], path: str) -> str | None:
    """Return the name of the first class whose patterns match `path`."""
    for c in classes:
        if any(glob_match(p, path) for p in c.include):
            return c.name
    return None


def split_segs(path: str) -> list[str]:
    """Split a path into its non-empty components."""
    return [seg for seg in path.split("/") if seg]


def match_seg(pattern: str, component: str) -> bool:
    """Match one pattern segment against one path component.

    `*` matches any run of characters, `?` matches exactly one.
    """
    return _match_seg_impl(pattern, component)


def _match_seg_impl(pattern: str, component: str) -> bool:
    """Backtracking matcher for a single pattern segment."""

    def rec(p: int, c: int) -> bool:
        if p == len(pattern):
            return c == len(component)
        if pattern[p] == "*":
            # `*` matches any run: try consuming zero, one, two, ... characters.
            return any(rec(p + 1, star) for star in range(c, len(component) + 1))
        if pattern[p] == "?":
            return c < len(component) and rec(p + 1, c + 1)
        return c < len(component) and pattern[p] == component[c] and rec(p + 1, c + 1)

    return rec(0, 0)


def glob_match(pattern: str, path: str) -> bool:
    """Match a whole glob pattern against a whole path.

    `**` matches any number of path components, including none.
    """
    pats = split_segs(pattern)
    path_segs = split_segs(path)

    consumed = 0
    pi = 0
    while pi < len(pats):
        if pats[pi] == "**":
            # `**` can match any number of remaining path components.
            return any(
                _glob_match_suffix(pats[pi + 1 :], path_segs[consumed + take :])
                for take in range(len(path_segs) - consumed + 1)
            )
        if consumed < len(path_segs) and match_seg(pats[pi], path_segs[consumed]):
            consumed += 1
            pi += 1
        else:
            return False

    return consumed == len(path_segs)


def _glob_match_suffix(pattern: list[str], path: list[str]) -> bool:
    pi = 0
    si = 0
    while pi < len(pattern) and si < len(path):
        if pattern[pi] == "**":
            # `**` matches any number of remaining components; try the rest of
            # the pattern against every split point.
            return any(
                _glob_match_suffix(pattern[pi + 1 :], path[si + take :])
                for take in range(len(path) - si + 1)
            )
        if match_seg(pattern[pi], path[si]):
            si += 1
            pi += 1
        else:
            return False

    return pi == len(pattern) and si == len(path)


def test_slice1_segs_splits_on_slash():
    assert split_segs("src/bank/deposit.ts") == ["src", "bank", "deposit.ts"]


def test_slice1_segs_single_component():
    assert split_segs("Cargo.toml") == ["Cargo.toml"]


def test_slice1_segs_drops_empty_components():
    assert split_segs("a//b") == ["a", "b"]
    assert split_segs("/a/b/") == ["a", "b"]


def test_slice1_segs_empty_path_has_no_components():
    assert split_segs("") == []


def test_slice2_seg_literal_must_be_exact():
    assert match_seg("src", "src")
    assert not match_seg("src", "lib")


def test_slice2_seg_case_is_significant():
    assert not match_seg("SRC", "src")


def test_slice2_seg_star_matches_any_run():
    assert match_seg("*.ts", "deposit.ts")
    assert not match_seg("*.ts", "deposit.js")


def test_slice2_seg_star_matches_nothing_too():
    assert match_seg("*.ts", ".ts")
    assert match_seg("*", "")


def test_slice2_seg_bare_star_matches_anything():
    assert match_seg("*", "deposit.ts")


def test_slice2_seg_question_matches_exactly_one():
    assert match_seg("?.ts", "a.ts")
    assert not match_seg("?.ts", "ab.ts")
    assert not match_seg("?.ts", ".ts")


def test_slice2_seg_star_backtracks():
    # A greedy `*` that never gives characters back fails this.
    assert match_seg("*_test.*", "deposit_test.ts")
    assert match_seg("*_test.*", "a_test_test.ts")
    assert not match_seg("*_test.*", "deposit.ts")


def test_slice2_seg_several_wildcards():
    assert match_seg("*-*.spec.?s", "user-auth.spec.ts")
    assert not match_seg("*-*.spec.?s", "userauth.spec.ts")


def test_slice3_glob_literal_path():
    assert glob_match("Cargo.toml", "Cargo.toml")
    assert not glob_match("Cargo.toml", "Cargo.lock")


def test_slice3_glob_star_does_not_cross_a_slash():
    assert glob_match("*.toml", "Cargo.toml")
    assert not glob_match("*.toml", "crates/Cargo.toml")


def test_slice3_glob_double_star_absorbs_components():
    assert glob_match("src/**", "src/bank/deposit.ts")
    assert glob_match("src/**", "src/a.ts")


def test_slice3_glob_double_star_matches_zero_components():
    assert glob_match("**/*_test.*", "deposit_test.ts")
    assert glob_match("**/*.md", "README.md")


def test_slice3_glob_double_star_in_the_middle():
    assert glob_match("**/*_test.*", "src/bank/deposit_test.ts")
    assert glob_match("docs/**/*.md", "docs/a/b/c.md")


def test_slice3_glob_single_star_segment_is_exactly_one():
    assert glob_match("src/*/mod.rs", "src/a/mod.rs")
    assert not glob_match("src/*/mod.rs", "src/a/b/mod.rs")
    assert not glob_match("src/*/mod.rs", "src/mod.rs")


def test_slice3_glob_bare_double_star_matches_everything():
    assert glob_match("**", "a/b/c.ts")
    assert glob_match("**", "x")


def test_slice3_glob_non_matching_returns_false():
    assert not glob_match("src/**", "docs/a.md")
    assert not glob_match("tests/**", "src/lib.rs")


def test_slice3_glob_pattern_longer_than_path():
    assert not glob_match("a/b/c", "a/b")


def test_slice4_classify_first_match_wins():
    classes = [FileClass("test", ["**/*_test.*"]), FileClass("source", ["src/**"])]
    assert classify(classes, "src/bank/deposit_test.ts") == "test"
    assert classify(classes, "src/bank/deposit.ts") == "source"


def test_slice4_classify_order_decides_the_answer():
    classes = [FileClass("source", ["src/**"]), FileClass("test", ["**/*_test.*"])]
    assert classify(classes, "src/bank/deposit_test.ts") == "source"


def test_slice4_classify_no_class_matches():
    assert classify([FileClass("source", ["src/**"])], "README.md") is None


def test_slice4_classify_any_pattern_in_a_class_matches():
    classes = [FileClass("config", ["*.toml", "*.yaml", ".github/**"])]
    assert classify(classes, "Cargo.toml") == "config"
    assert classify(classes, "app.yaml") == "config"
    assert classify(classes, ".github/workflows/ci.yml") == "config"


def test_slice4_classify_empty_class_list_matches_nothing():
    assert classify([], "src/a.ts") is None
