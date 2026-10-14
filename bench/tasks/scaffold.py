"""Cut a reference crate down to what a round is meant to start from.

A task is a directory holding `task.json`, the complete crate it describes, and
one held-out suite per slice. The crate is green: every function implemented,
every test present. A round starts from a hole in it, and which hole is the
whole experiment:

    --mode slices       every function a todo!(), every test kept.
                        One doer, four slices, red to green. Measures cost.

    --mode pair --slice N
                        slice N's implementation and every test removed, along
                        with the private helpers that exist only to serve that
                        slice. QA writes the test, the coder makes it pass
                        (ADR-0053). `held-out/slice<N>.rs` is copied beside the
                        crate so what the round produces can be judged against
                        cases it was never shown.

Nothing here talks to a model. Run it, read the counts it prints, and the round
starts from a state you can describe exactly.
"""

import argparse
import json
import os
import shutil

HERE = os.path.dirname(os.path.abspath(__file__))


def closing_brace(src, at):
    """Index of the `}` that closes the `{` at or after `at`."""
    depth, i = 0, src.index("{", at)
    while True:
        depth += (src[i] == "{") - (src[i] == "}")
        if depth == 0:
            return i
        i += 1


def comment_above(src, at):
    """Index where the comment block introducing the item at `at` begins."""
    start = at
    for line in reversed(src[:at].rstrip("\n").split("\n")):
        if not line.lstrip().startswith("//"):
            break
        start -= len(line) + 1
    return start


def stub_body(src, signature, label):
    at = src.index(signature)
    brace = at + len(signature) - 1
    return src[: brace + 1] + f'\n    todo!("{label}")\n' + src[closing_brace(src, brace) :]


def delete_fn(src, signature):
    """Remove a function and the comment that introduces it.

    Taking the comment too is not tidiness: a `///` with no item after it does
    not compile.
    """
    at = src.index(signature)
    return src[: comment_above(src, at)] + src[closing_brace(src, at) + 2 :]


def cut_tests(src, module):
    start = comment_above(src, src.rindex("#[cfg(test)]", 0, src.index(f"mod {module}")))
    return src[:start] + src[closing_brace(src, start) + 2 :]


def build(task, mode, which):
    src = open(f"{task['dir']}/{task['artefact']}", encoding="utf-8").read()

    if mode == "slices":
        for n, slice_ in enumerate(task["slices"], start=1):
            src = stub_body(src, slice_["signature"], f"slice {n}")
        return src

    # The pair gets one hole and no tests at all: it has to produce both halves,
    # and a test left behind for another slice would do part of its job. The
    # helpers go with it — leaving `glob_match_suffix` in place while
    # `glob_match` is a todo!() hands the coder the recursion it was asked to
    # write, and the round would be measuring reading rather than work.
    chosen = task["slices"][which - 1]
    for slice_ in task["slices"]:
        src = cut_tests(src, slice_["module"])
    for helper in chosen["helpers"]:
        src = delete_fn(src, helper)
    src = stub_body(src, chosen["signature"], f"slice {which}")
    return src + f"\n// QA writes `mod {chosen['module']}` here.\n"


def read_task(name):
    directory = f"{HERE}/{name}"
    task = json.load(open(f"{directory}/task.json", encoding="utf-8"))
    task["dir"] = directory
    return task


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dest", help="directory to build the crate in; replaced if it exists")
    parser.add_argument("--task", default="c-glob", help="a directory beside this script")
    parser.add_argument("--mode", choices=["slices", "pair"], default="slices")
    parser.add_argument(
        "--slice",
        type=int,
        default=1,
        dest="which",
        help="which slice the pair works; ignored by --mode slices",
    )
    args = parser.parse_args()

    task = read_task(args.task)
    if not 1 <= args.which <= len(task["slices"]):
        parser.error(f"{task['name']} has {len(task['slices'])} slices")

    held_out_file = f"{task['dir']}/held-out/slice{args.which}.rs"
    if args.mode == "pair" and not os.path.exists(held_out_file):
        # A pair round with nothing to judge it by answers half a question, and
        # the half it answers is the one already answered.
        parser.error(
            f"{task['name']} slice {args.which} has no held-out suite at {held_out_file}"
        )

    src = build(task, args.mode, args.which)

    shutil.rmtree(args.dest, ignore_errors=True)
    os.makedirs(f"{args.dest}/{os.path.dirname(task['artefact'])}", exist_ok=True)
    shutil.copy(f"{task['dir']}/Cargo.toml", f"{args.dest}/Cargo.toml")
    shutil.copy(f"{task['dir']}/task.json", f"{args.dest}/task.json")
    with open(f"{args.dest}/{task['artefact']}", "w", encoding="utf-8") as out:
        out.write(src)

    held_out = 0
    if args.mode == "pair":
        # Beside the crate, not inside it: a file under src/ would be compiled,
        # and the point is that the round never sees these cases.
        cases = open(held_out_file, encoding="utf-8").read()
        with open(f"{args.dest}/held-out.rs", "w", encoding="utf-8") as out:
            out.write(cases)
        held_out = cases.count("#[test]")

    print(
        f"{args.dest}: {task['name']}, {src.count('todo!')} stubs, "
        f"{src.count('#[test]')} tests" + (f", {held_out} held out" if held_out else "")
    )


main()
