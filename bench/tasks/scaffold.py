"""Cut a reference project down to what a round is meant to start from.

A task is a directory holding `task.json`, the complete project it describes,
and one held-out suite per slice. The project is green: every function
implemented, every test present. A round starts from a hole in it, and which
hole is the whole experiment:

    --mode slices       every function a stub, every test kept.
                        One doer, four slices, red to green. Measures cost.

    --mode pair --slice N
                        slice N's implementation and every test removed, along
                        with the private helpers that exist only to serve that
                        slice. QA writes the test, the coder makes it pass
                        (ADR-0053). `held-out/slice<N>.rs` is copied beside the
                        project so what the round produces can be judged
                        against cases it was never shown.

`task.json` says which language the project is in. Rust blocks end at a brace
and Python blocks end when the indentation comes back, so the cutting differs;
nothing else does.

Nothing here talks to a model. Run it, read the counts it prints, and the round
starts from a state you can describe exactly.
"""

import argparse
import json
import os
import re
import shutil

HERE = os.path.dirname(os.path.abspath(__file__))


class Rust:
    """Blocks delimited by braces."""

    stub = "todo!"
    test = "#[test]"

    @staticmethod
    def closing_brace(src, at):
        """Index of the `}` that closes the `{` at or after `at`."""
        depth, i = 0, src.index("{", at)
        while True:
            depth += (src[i] == "{") - (src[i] == "}")
            if depth == 0:
                return i
            i += 1

    @classmethod
    def stub_body(cls, src, signature, label):
        at = src.index(signature)
        brace = at + len(signature) - 1
        return (
            src[: brace + 1]
            + f'\n    todo!("{label}")\n'
            + src[cls.closing_brace(src, brace) :]
        )

    @classmethod
    def delete_fn(cls, src, signature):
        """Remove a function and the comment that introduces it.

        Taking the comment too is not tidiness: a `///` with no item after it
        does not compile.
        """
        at = src.index(signature)
        return src[: comment_above(src, at, "//")] + src[cls.closing_brace(src, at) + 2 :]

    @classmethod
    def cut_tests(cls, src, module):
        start = comment_above(
            src, src.rindex("#[cfg(test)]", 0, src.index(f"mod {module}")), "//"
        )
        return src[:start] + src[cls.closing_brace(src, start) + 2 :]


class TypeScript:
    """Blocks delimited by braces, like Rust, and tests grouped by `describe`.

    A `describe("<module>", ...)` block is what `vitest run -t <module>`
    selects, which is how a slice's oracle picks out its own cases — the same
    shape as `cargo test --lib <module>` and `pytest -k <module>`.
    """

    stub = "Error("
    test = "test("
    comment = "//"

    @classmethod
    def stub_body(cls, src, signature, label):
        at = src.index(signature)
        brace = at + len(signature) - 1
        return (
            src[: brace + 1]
            + f'\n  throw new Error("{label}");\n'
            + src[Rust.closing_brace(src, brace) :]
        )

    @classmethod
    def delete_fn(cls, src, signature):
        at = src.index(signature)
        return (
            src[: comment_above(src, at, "//")]
            + src[past_line_end(src, Rust.closing_brace(src, at)) :]
        )

    @classmethod
    def cut_tests(cls, src, module):
        at = src.index(f'describe("{module}"')
        start = comment_above(src, at, "//")
        # The block ends `});`, not `}`, so the line has to be finished rather
        # than the brace counted and two characters assumed.
        return tidy(src[:start] + src[past_line_end(src, Rust.closing_brace(src, at)) :])


class Python:
    """Blocks delimited by indentation.

    A test is a top-level `def test_<module>_...`, not a class, so that `pytest
    -k <module>` selects a slice's cases the way `cargo test --lib <module>`
    does. That keeps one oracle shape for both languages, which is the only
    reason the comparison in round five's Q3 means anything.
    """

    stub = "NotImplementedError"
    test = "def test_"

    @staticmethod
    def head(lines, signature):
        wanted = signature.strip()
        for i, line in enumerate(lines):
            if line.strip() == wanted:
                return i
        raise ValueError(f"no line reading {wanted!r}")

    @staticmethod
    def last_line(lines, head):
        """Index of the block's last non-blank line."""
        indent = len(lines[head]) - len(lines[head].lstrip())
        last = head
        for i in range(head + 1, len(lines)):
            if not lines[i].strip():
                continue
            if len(lines[i]) - len(lines[i].lstrip()) <= indent:
                break
            last = i
        return last

    @classmethod
    def stub_body(cls, src, signature, label):
        lines = src.split("\n")
        head = cls.head(lines, signature)
        body = [f'    raise NotImplementedError("{label}")']
        return "\n".join(lines[: head + 1] + body + lines[cls.last_line(lines, head) + 1 :])

    @classmethod
    def delete_fn(cls, src, signature):
        lines = src.split("\n")
        head = cls.head(lines, signature)
        while head and lines[head - 1].lstrip().startswith("#"):
            head -= 1
        return tidy("\n".join(lines[:head] + lines[cls.last_line(lines, head) + 1 :]))

    @classmethod
    def cut_tests(cls, src, module):
        """Every top-level `def test_<module>_...`, taken one at a time.

        One at a time because deleting a block shifts every index after it, and
        a slice's cases are not contiguous in a file written to read well.
        """
        while True:
            lines = src.split("\n")
            found = next(
                (i for i, l in enumerate(lines) if l.startswith(f"def test_{module}_")),
                None,
            )
            if found is None:
                return tidy(src)
            start = found
            while start and lines[start - 1].lstrip().startswith("#"):
                start -= 1
            src = "\n".join(lines[:start] + lines[cls.last_line(lines, found) + 1 :])


LANGUAGES = {"rust": Rust, "python": Python, "typescript": TypeScript}


def comment_above(src, at, prefix):
    """Index where the comment block introducing the item at `at` begins."""
    start = at
    for line in reversed(src[:at].rstrip("\n").split("\n")):
        if not line.lstrip().startswith(prefix):
            break
        start -= len(line) + 1
    return start


def past_line_end(src, at):
    """Index just after the newline that ends the line containing `at`."""
    end = src.find("\n", at)
    return len(src) if end == -1 else end + 1


def tidy(src):
    """Collapse the run of blank lines a deletion leaves behind."""
    return re.sub(r"\n{4,}", "\n\n\n", src).rstrip("\n") + "\n"


def build(task, lang, mode, which):
    src = open(f"{task['dir']}/{task['artefact']}", encoding="utf-8").read()

    if mode == "slices":
        for n, slice_ in enumerate(task["slices"], start=1):
            src = lang.stub_body(src, slice_["signature"], f"slice {n}")
        return src

    # The pair gets one hole and no tests at all: it has to produce both halves,
    # and a test left behind for another slice would do part of its job. The
    # helpers go with it — leaving `glob_match_suffix` in place while
    # `glob_match` is a stub hands the coder the recursion it was asked to
    # write, and the round would be measuring reading rather than work.
    chosen = task["slices"][which - 1]
    for slice_ in task["slices"]:
        src = lang.cut_tests(src, slice_["module"])
    for helper in chosen["helpers"]:
        src = lang.delete_fn(src, helper)
    src = lang.stub_body(src, chosen["signature"], f"slice {which}")
    return src + f"\n# QA writes the cases for {chosen['module']} here.\n"


def read_task(name):
    directory = f"{HERE}/{name}"
    task = json.load(open(f"{directory}/task.json", encoding="utf-8"))
    task["dir"] = directory
    return task


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dest", help="directory to build the project in; replaced if it exists")
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
    if task["language"] not in LANGUAGES:
        parser.error(f"{task['name']}: no cutter for language {task['language']!r}")
    lang = LANGUAGES[task["language"]]

    held_out_file = f"{task['dir']}/held-out/slice{args.which}.rs"
    if args.mode == "pair" and not os.path.exists(held_out_file):
        # A pair round with nothing to judge it by answers half a question, and
        # the half it answers is the one already answered.
        parser.error(
            f"{task['name']} slice {args.which} has no held-out suite at {held_out_file}"
        )

    src = build(task, lang, args.mode, args.which)

    shutil.rmtree(args.dest, ignore_errors=True)
    os.makedirs(f"{args.dest}/{os.path.dirname(task['artefact'])}", exist_ok=True)
    for name in task["copy"] + ["task.json"]:
        shutil.copy(f"{task['dir']}/{name}", f"{args.dest}/{name}")
    # Symlinked rather than copied: a node_modules is tens of thousands of files
    # and every iteration gets its own workspace. The round never writes to it.
    for name in task.get("link", []):
        source = os.path.abspath(f"{task['dir']}/{name}")
        if not os.path.exists(source):
            parser_error = f"{task['name']} declares link {name!r} but {source} does not exist"
            raise SystemExit(f"scaffold.py: error: {parser_error}")
        os.symlink(source, f"{args.dest}/{name}")
    with open(f"{args.dest}/{task['artefact']}", "w", encoding="utf-8") as out:
        out.write(src)

    held_out = 0
    if args.mode == "pair":
        # Beside the project, not inside it: a file the test runner collects
        # would be seen, and the point is that the round never sees these cases.
        cases = open(held_out_file, encoding="utf-8").read()
        with open(f"{args.dest}/held-out.rs", "w", encoding="utf-8") as out:
            out.write(cases)
        held_out = cases.count(lang.test)

    print(
        f"{args.dest}: {task['name']}, {src.count(lang.stub)} stubs, "
        f"{src.count(lang.test)} tests" + (f", {held_out} held out" if held_out else "")
    )


main()
