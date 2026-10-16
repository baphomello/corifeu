"""Ask one model one question, score the answer, and time it.

Prints one line: `passed tok_s gen_tokens seconds`.

The score comes from running the spec, never from reading the reply. A model
that explains a correct algorithm in prose and writes the wrong code scores what
the code scores, which is the whole point of having an oracle.

Tokens per second is measured from this side of the HTTP call rather than read
out of the server's own timing log, because what a round pays for is wall time
including whatever the server does around generation.
"""

import argparse
import json
import re
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent


def without_thinking(reply: str) -> str:
    """The answer, with the model's reasoning removed.

    A thinking model writes its working into `content` whatever
    `reasoning_format` asks for, and that working is full of fenced drafts. The
    first version of this scored a draft from inside `<think>` instead of the
    answer after it, which measured the drafting rather than the answer.

    An unclosed tag means the reply was cut off mid-thought, so everything from
    the tag onwards is working and there is no answer to find.
    """
    closed = re.sub(r"<think>.*?</think>", "", reply, flags=re.S | re.I)
    return re.sub(r"<think>.*$", "", closed, flags=re.S | re.I)


def code_from(reply: str) -> str:
    """The TypeScript in a reply, however the model chose to wrap it.

    The *last* fenced block rather than the longest: a model that shows an
    attempt and then corrects it means the correction, and a longest-block rule
    would score whichever draft happened to be wordiest.

    A reply with no fence at all is taken whole. Refusing to score it would
    reward the models that ignore the format over the ones that get the answer
    wrong, and those are different failures.
    """
    answer = without_thinking(reply)
    fences = re.findall(r"```(?:ts|typescript|javascript|js)?\s*\n(.*?)```", answer, re.S)
    return fences[-1] if fences else answer


def score(work: Path, code: str) -> int:
    """Cases passed out of ten, by running the spec against the answer."""
    (work / "answer.ts").write_text(code, encoding="utf-8")
    shutil.copy(HERE / "cpf.spec.ts", work / "cpf.spec.ts")
    shutil.copy(HERE / "package.json", work / "package.json")
    modules = work / "node_modules"
    if not modules.exists():
        modules.symlink_to(HERE / "node_modules")

    run = subprocess.run(
        ["npx", "vitest", "run", "cpf.spec.ts"],
        cwd=work,
        capture_output=True,
        text=True,
        timeout=300,
    )
    (work / "vitest.txt").write_text(run.stdout + run.stderr, encoding="utf-8")

    # A file that fails to import reports no case count at all, and counting the
    # rest would read a broken answer as a partial one.
    if "Failed Suites" in run.stdout + run.stderr:
        return 0
    found = re.search(r"Tests\s+(.+?)\s+\(\d+\)", run.stdout + run.stderr)
    if not found:
        return 0
    passed = re.search(r"(\d+) passed", found.group(1))
    return int(passed.group(1)) if passed else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--prompt", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    # A thinking model spends most of its budget before it writes anything, and
    # a truncated answer scores near zero — which measures the budget rather
    # than the model. The reply is kept, so truncation stays visible.
    parser.add_argument("--max-tokens", type=int, default=6144)
    # llama.cpp defaults to 0.80, which is high for code: prose survives a word
    # chosen second-best and code does not compile. The first sweep watched one
    # model score 5 of 10 and then 10 of 10 under identical flags, and that was
    # read as the model's variance when it may have been the sampler's.
    parser.add_argument("--temp", type=float, default=0.2)
    args = parser.parse_args()

    body = json.dumps(
        {
            "model": args.model,
            "messages": [{"role": "user", "content": args.prompt.read_text(encoding="utf-8")}],
            "max_tokens": args.max_tokens,
            "stream": False,
            "reasoning_format": "none",
            "temperature": args.temp,
        }
    ).encode()

    started = time.monotonic()
    try:
        request = urllib.request.Request(
            f"http://{args.host}/v1/chat/completions",
            data=body,
            headers={"content-type": "application/json"},
        )
        with urllib.request.urlopen(request, timeout=900) as response:
            reply = json.load(response)
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as error:
        print(f"ASK_FAILED 0 0 {time.monotonic() - started:.0f}", file=sys.stdout)
        (args.out / "error.txt").write_text(str(error), encoding="utf-8")
        return 0
    seconds = time.monotonic() - started

    content = reply["choices"][0]["message"].get("content") or ""
    generated = int(reply.get("usage", {}).get("completion_tokens", 0))
    (args.out / "reply.txt").write_text(content, encoding="utf-8")

    passed = score(args.out, code_from(content))
    rate = generated / seconds if seconds > 0 else 0
    print(f"{passed} {rate:.1f} {generated} {seconds:.0f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
