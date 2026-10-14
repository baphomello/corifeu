"""Cut a reference crate down to what a round is meant to start from.

The reference crate in `c-glob/` is complete: every function implemented, every
test present, 26 cases green. A round starts from a hole in it, and which hole
is the whole experiment:

    --mode slices   every function a todo!(), every test kept.
                    One doer, four slices, red to green. Measures cost.

    --mode pair     one slice's implementation AND every test removed. QA
                    writes the test, the coder makes it pass (ADR-0053). Which
                    slice is `--slice`, and slice size is the variable Q4 is
                    about: slice 1 is four lines, slice 3 is recursive.
                    `c-glob/held-out/slice<n>.rs` is copied beside the crate as
                    `held-out.rs`, so what the round produces can be judged
                    against cases it was never shown.

Nothing here talks to a model. Run it, read the counts it prints, and the round
starts from a state you can describe exactly.
"""

import argparse
import os
import shutil

HERE = os.path.dirname(os.path.abspath(__file__))

# Each slice: the function's signature line, the test module that covers it, and
# the private helpers that exist only to serve it.
#
# The helpers have to go when the slice they serve is stubbed. Leaving
# `glob_match_suffix` in place while `glob_match` is a todo!() hands the coder
# the recursion it was asked to write, and the round would be measuring reading
# rather than work.
SLICES = {
    1: (
        "fn split_segs(path: &str) -> Vec<&str> {",
        "slice1_segs",
        [],
    ),
    2: (
        "fn match_seg(pattern: &str, component: &str) -> bool {",
        "slice2_seg",
        ["fn match_seg_impl(pattern: &[u8], component: &[u8]) -> bool {"],
    ),
    3: (
        "fn glob_match(pattern: &str, path: &str) -> bool {",
        "slice3_glob",
        ["fn glob_match_suffix(pattern: &[&str], path: &[&str]) -> bool {"],
    ),
    4: (
        "pub fn classify<'a>(classes: &'a [FileClass], path: &str) -> Option<&'a str> {",
        "slice4_classify",
        [],
    ),
}


def closing_brace(src, at):
    """Index of the `}` that closes the `{` at or after `at`."""
    depth, i = 0, src.index("{", at)
    while True:
        depth += (src[i] == "{") - (src[i] == "}")
        if depth == 0:
            return i
        i += 1


def stub_body(src, signature, label):
    at = src.index(signature)
    brace = at + len(signature) - 1
    end = closing_brace(src, brace)
    return src[: brace + 1] + f'\n    todo!("{label}")\n' + src[end:]


def delete_fn(src, signature):
    """Remove a function and the doc comment that introduces it.

    Leaving the comment behind is not cosmetic: a `///` with no item after it
    does not compile.
    """
    at = src.index(signature)
    start = at
    for line in reversed(src[:at].rstrip("\n").split("\n")):
        stripped = line.lstrip()
        if not (stripped.startswith("///") or stripped.startswith("//")):
            break
        start -= len(line) + 1
    return src[:start] + src[closing_brace(src, at) + 2 :]


def cut_tests(src, module):
    """Remove `#[cfg(test)] mod <module> { ... }`, and return it with the rest."""
    start = src.rindex("#[cfg(test)]", 0, src.index(f"mod {module}"))
    end = closing_brace(src, start)
    return src[:start] + src[end + 2 :], src[start : end + 1]


def build(mode, which):
    src = open(f"{HERE}/c-glob/src/lib.rs", encoding="utf-8").read()

    if mode == "slices":
        for number, (signature, _, _) in sorted(SLICES.items()):
            src = stub_body(src, signature, f"slice {number}")
        return src

    # The pair gets one hole and no tests at all: it has to produce both halves,
    # and a test left behind for another slice would do part of its job.
    signature, module, helpers = SLICES[which]
    for _, (_, other, _) in sorted(SLICES.items()):
        src, _ = cut_tests(src, other)
    for helper in helpers:
        src = delete_fn(src, helper)
    src = stub_body(src, signature, f"slice {which}")
    return src + f"\n// QA writes `mod {module}` here.\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dest", help="directory to build the crate in; replaced if it exists")
    parser.add_argument("--mode", choices=["slices", "pair"], default="slices")
    parser.add_argument(
        "--slice",
        type=int,
        choices=sorted(SLICES),
        default=1,
        dest="which",
        help="which slice the pair works; ignored by --mode slices",
    )
    args = parser.parse_args()

    src = build(args.mode, args.which)

    shutil.rmtree(args.dest, ignore_errors=True)
    os.makedirs(f"{args.dest}/src", exist_ok=True)
    shutil.copy(f"{HERE}/c-glob/Cargo.toml", f"{args.dest}/Cargo.toml")
    with open(f"{args.dest}/src/lib.rs", "w", encoding="utf-8") as out:
        out.write(src)

    held_out = 0
    if args.mode == "pair":
        # Beside the crate, not inside it: a file under src/ would be compiled,
        # and the point is that the round never sees these cases.
        cases = open(f"{HERE}/c-glob/held-out/slice{args.which}.rs", encoding="utf-8").read()
        with open(f"{args.dest}/held-out.rs", "w", encoding="utf-8") as out:
            out.write(cases)
        held_out = cases.count("#[test]")

    print(
        f"{args.dest}: {src.count('todo!')} stubs, {src.count('#[test]')} tests"
        + (f", {held_out} held out" if held_out else "")
    )


main()
