#!/usr/bin/env bash
# install.sh <target-repo> — install the orchestra workflow into a repo.
set -euo pipefail
SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DST="${1:?usage: ./install.sh <target-repo>}"
[ -d "$DST/.git" ] || { echo "not a git repo: $DST"; exit 1; }
rm -rf "$DST/.orchestra"
cp -r "$SRC/.orchestra" "$DST/.orchestra"
rm -rf "$DST/.orchestra/codex-home"
bash "$DST/.orchestra/setup-codex-home.sh"
mkdir -p "$DST/.claude/commands"
cp "$SRC/.claude/commands/cycle.md" "$DST/.claude/commands/cycle.md"
grep -qF "Orchestra — you are the orchestrator" "$DST/CLAUDE.md" 2>/dev/null || printf '\n%s\n' "$(cat "$SRC/CLAUDE.orchestra.md")" >> "$DST/CLAUDE.md"
for ignore in '.orchestra/codex-home/' '__pycache__/' '.orchestra/SPEC.md'; do
  grep -qxF "$ignore" "$DST/.gitignore" 2>/dev/null || printf '%s\n' "$ignore" >> "$DST/.gitignore"
done
# merge the enforcement hook + allowlist into .claude/settings.json
python3 - "$SRC/settings.orchestra.json" "$DST/.claude/settings.json" <<'PY'
import json, os, sys
add = json.load(open(sys.argv[1]))
dst = sys.argv[2]
cur = json.load(open(dst)) if os.path.exists(dst) else {}
cur.setdefault("permissions", {}).setdefault("allow", [])
for a in add.get("permissions", {}).get("allow", []):
    if a not in cur["permissions"]["allow"]:
        cur["permissions"]["allow"].append(a)
ch = cur.setdefault("hooks", {}).setdefault("PreToolUse", [])
for h in add.get("hooks", {}).get("PreToolUse", []):
    if h not in ch:
        ch.append(h)
json.dump(cur, open(dst, "w"), indent=2)
print("merged enforcement settings into", dst)
PY
echo "Installed into $DST (guard hook active: src/ & tests/ writes are blocked outside code.py)."
