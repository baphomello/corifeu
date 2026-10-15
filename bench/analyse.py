"""Read one cell's rows and print the numbers round five registered questions for.

    bench/analyse.py bench/rounds/round5-<stamp>/1-c-glob-rank-default
    bench/analyse.py <cell> <cell>          # side by side

Every number in docs/measurement/round5-*.md comes from here. The first cell's
came from a shell pipeline typed once, which nobody can repeat and nobody can
check; there are seven cells and twelve questions, and a transcription mistake
in either would be invisible.

Reads results.tsv for the per-slice rows and steps.tsv for the trace. Says what
it cannot answer rather than printing a zero.
"""

import collections
import csv
import sys
from pathlib import Path

REACHED = ("GREEN", "DELIVERED_EARLY")


def read(cell):
    cell = Path(cell)
    rows = list(csv.reader(open(cell / "results.tsv"), delimiter="\t"))
    slices = [r for r in rows if r and r[0].isdigit() and len(r) > 9]
    totals = [r for r in rows if r and r[0] == "TOTAL"]
    finals = [r for r in rows if r and r[0] == "FINAL"]
    trace = Path(cell / "steps.tsv")
    steps = list(csv.reader(open(trace), delimiter="\t"))[1:] if trace.exists() else []
    return {
        "name": cell.name,
        "slices": slices,
        "totals": totals,
        "finals": finals,
        "again": [r for r in rows if r and r[0] == "AGAIN"],
        "final": {(r[1], r[2]): r[3] for r in rows if r and r[0] == "FINAL"},
        "steps": steps,
    }


def outcome(row):
    """The outcome word, without a provider error's explanation after it."""
    return row[3].split()[0]


def pct(n, of):
    return f"{100 * n / of:.0f}%" if of else "-"


def delivery(c):
    print("  the crate at the end")
    for what, n in collections.Counter(r[3] for r in c["finals"]).most_common():
        print(f"    {n:>3}  {what}")
    closed = sum(1 for r in c["slices"] if outcome(r) == "GREEN")
    early = sum(1 for r in c["slices"] if outcome(r) == "DELIVERED_EARLY")
    over = sum(int(f.split("=")[1]) for r in c["totals"] for f in r if f.startswith("overtaken="))
    print(f"    slices: {closed} closed, {early} delivered early, {over} overtaken")
    print("  per slice")
    by = collections.defaultdict(collections.Counter)
    for r in c["slices"]:
        by[r[2]][outcome(r)] += 1
    for module in sorted(by):
        counts = ", ".join(f"{k.lower()} {v}" for k, v in by[module].most_common())
        print(f"    {module:<16} {counts}")


def q6(c):
    """ADR-0063: the end-of-plan re-probe, and what it found."""
    rows = c["again"]
    if not rows:
        print("    no AGAIN rows — every slice closed under its own oracle")
        return
    verdicts = collections.Counter(r[4] for r in rows)
    green = verdicts.get("GREEN", 0)
    print(f"    {len(rows)} slices did not close; the re-probe found "
          f"{green} green and {verdicts.get('RED', 0)} red ({pct(green, len(rows))} late, "
          f"not missing)")
    # A red re-probe is work that is actually absent. Whether those runs still
    # delivered is the question ADR-0063 turns on.
    delivered = {k for k, v in c["final"].items() if v.startswith("test result: ok")}
    red_runs = {(r[1], r[2]) for r in rows if r[4] == "RED"}
    both = sorted(red_runs & delivered)
    print(f"    runs holding a red re-probe: {len(red_runs)}, of which delivered: {len(both)}")
    if not both:
        print("    every delivered run had each slice closed, early, or green on re-probe")


def q8(c):
    if not c["steps"]:
        print("    no steps.tsv — this cell ran before the trace existed")
        return
    flags = collections.Counter(s[7] for s in c["steps"])
    total = sum(flags.values())
    work = flags["-"] + flags.get("rollback", 0)
    for k, n in flags.most_common():
        print(f"    {k:<10} {n:>4}  {pct(n, total)}")
    print(f"    productive {work} of {total} ({pct(work, total)})")


def q9(c):
    if not c["steps"]:
        print("    no steps.tsv")
        return
    n = sum(1 for s in c["steps"] if "rollback" in s[7])
    print(f"    {n} rollback step(s) in {len(c['steps'])} steps")


def q10(c):
    if not c["steps"]:
        print("    no steps.tsv")
        return
    first, last = {}, {}
    for s in c["steps"]:
        key, step = (s[0], s[1], s[2]), int(s[4])
        last[key] = max(last.get(key, 0), step)
        if "repeat" in s[7]:
            first[key] = min(first.get(key, 1 << 30), step)
    ends = {(r[0], r[1], r[2]): outcome(r) for r in c["slices"]}
    warned = collections.Counter(ends.get(k, "?") for k in first)
    quiet = collections.Counter(v for k, v in ends.items() if k not in first)
    green = sum(n for o, n in warned.items() if o in REACHED)
    print(f"    saw a repeat   : {sum(warned.values()):>3}, reached {green}  {dict(warned)}")
    print(f"    never repeated : {sum(quiet.values()):>3}, reached "
          f"{sum(n for o, n in quiet.items() if o in REACHED)}  {dict(quiet)}")
    after = sum(last[k] - first[k] for k in first)
    spent = sum(last.values())
    print(f"    steps after the first repeat: {after} of {spent} ({pct(after, spent)})")


def q11_q12(c):
    per = collections.defaultdict(lambda: collections.Counter())
    for r in c["slices"]:
        steps = int(r[4])
        if not steps:
            continue
        acc = per[outcome(r)]
        acc["n"] += 1
        acc["steps"] += steps
        acc["prompt"] += int(r[5])
        acc["seconds"] += round(float(r[8]))
    print(f"    {'outcome':<17}{'n':>3}{'steps/slice':>13}{'sec/step':>10}{'prompt/step':>13}")
    for o in sorted(per):
        a = per[o]
        print(f"    {o:<17}{a['n']:>3}{a['steps'] / a['n']:>13.1f}"
              f"{a['seconds'] / a['steps']:>10.1f}{a['prompt'] / a['steps']:>13.0f}")


def q7(c):
    paired = collections.defaultdict(dict)
    for r in c["slices"]:
        paired[(r[0], r[2])][r[1]] = outcome(r)
    both = neither = one = 0
    for seen in paired.values():
        if len(seen) < 2:
            continue
        good = sum(1 for o in seen.values() if o in REACHED)
        both += good == 2
        one += good == 1
        neither += good == 0
    print(f"    both reached {both}, neither {neither}, exactly one {one}")
    # The same model against the same slice, which is what says whether the line
    # above is a difference between models or a difference between draws.
    within = collections.defaultdict(collections.Counter)
    for r in c["slices"]:
        within[(r[1], r[2])]["n"] += 1
        within[(r[1], r[2])]["ok"] += outcome(r) in REACHED
    print("    the same model, the same slice:")
    for (model, module) in sorted(within):
        a = within[(model, module)]
        print(f"      {model[:20]:<21}{module:<16} {a['ok']}/{a['n']}")


def report(cell):
    c = read(cell)
    print(f"\n{'=' * 72}\n{c['name']}  —  {len(c['totals'])} iterations, "
          f"{len(c['slices'])} slice runs, {len(c['steps'])} steps\n{'=' * 72}")
    for title, fn in (
        ("delivery", delivery),
        ("Q6  did asking again change the verdict", q6),
        ("Q8  what a step produced", q8),
        ("Q9  did ADR-0051's rollback fire", q9),
        ("Q10 does warning a looping model work", q10),
        ("Q11/Q12 slow, or just long", q11_q12),
        ("Q7  hard, or hard for one model", q7),
    ):
        print(f"\n{title}")
        fn(c)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    for one in sys.argv[1:]:
        report(one)
