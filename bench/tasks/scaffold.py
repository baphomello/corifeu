"""Cut a reference crate down to what a round is meant to start from.

The reference crate in `c-glob/` is complete: every function implemented, every
test present, 26 cases green. A round starts from a hole in it, and which hole
is the whole experiment:

    --mode slices   every function a todo!(), every test kept.
                    One doer, four slices, red to green. Measures cost.

    --mode pair     slice 1's implementation AND its tests both removed.
                    QA writes the test, the coder makes it pass (ADR-0053).
                    `c-glob/held-out/slice1.rs` is copied beside the crate as
                    `held-out.rs`, so what the round produces can be judged
                    against twelve cases it never saw.

Nothing here talks to a model. Run it, read the counts it prints, and the round
starts from a state you can describe exactly.
"""

import argparse
import os
import shutil

HERE = os.path.dirname(os.path.abspath(__file__))

# Each slice: the function's signature line, the stub that replaces its body,
# and the test module that covers it.
SLICES = [
    ("fn split_segs(path: &str) -> Vec<&str> {", "slice 1", "slice1_segs"),
    ("fn match_seg(pattern: &str, component: &str) -> bool {", "slice 2", "slice2_seg"),
    ("fn glob_match(pattern: &str, path: &str) -> bool {", "slice 3", "slice3_glob"),
    (
        "pub fn classify<'a>(classes: &'a [FileClass], path: &str) -> Option<&'a str> {",
        "slice 4",
        "slice4_classify",
    ),
]


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


def cut_tests(src, module):
    """Remove `#[cfg(test)] mod <module> { ... }`, and return it with the rest."""
    start = src.rindex("#[cfg(test)]", 0, src.index(f"mod {module}"))
    end = closing_brace(src, start)
    return src[:start] + src[end + 2 :], src[start : end + 1]


def build(mode):
    src = open(f"{HERE}/c-glob/src/lib.rs", encoding="utf-8").read()

    if mode == "slices":
        for signature, label, _ in SLICES:
            src = stub_body(src, signature, label)
        return src

    # The pair gets one hole and no tests at all: it has to produce both halves,
    # and a test left behind for another slice would do part of its job.
    for _, _, module in SLICES:
        src, _ = cut_tests(src, module)
    signature, label, _ = SLICES[0]
    src = stub_body(src, signature, label)
    return src + "\n// QA writes `mod slice1_segs` here.\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dest", help="directory to build the crate in; replaced if it exists")
    parser.add_argument("--mode", choices=["slices", "pair"], default="slices")
    args = parser.parse_args()

    src = build(args.mode)

    shutil.rmtree(args.dest, ignore_errors=True)
    os.makedirs(f"{args.dest}/src", exist_ok=True)
    shutil.copy(f"{HERE}/c-glob/Cargo.toml", f"{args.dest}/Cargo.toml")
    with open(f"{args.dest}/src/lib.rs", "w", encoding="utf-8") as out:
        out.write(src)

    held_out = 0
    if args.mode == "pair":
        # Beside the crate, not inside it: a file under src/ would be compiled,
        # and the point is that the round never sees these cases.
        cases = open(f"{HERE}/c-glob/held-out/slice1.rs", encoding="utf-8").read()
        with open(f"{args.dest}/held-out.rs", "w", encoding="utf-8") as out:
            out.write(cases)
        held_out = cases.count("#[test]")

    print(
        f"{args.dest}: {src.count('todo!')} stubs, {src.count('#[test]')} tests"
        + (f", {held_out} held out" if held_out else "")
    )


main()
