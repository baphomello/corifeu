# 0022 — Retention by compaction

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
The event log grows without bound. The proposed default was "keep the last 10 full runs,
then compact", with the explicit caveat that 10 was a guess and should be justified by an
estimate of real run sizes.

## Estimated size of one run

**These are estimates from assumed parameters, not measurements — Corifeu does not exist
yet to measure.** Assumptions: a medium feature of 10 tasks; a doer loop averaging 20 steps
per task; 4 bytes per token.

| Component | Per task | Reasoning |
|---|---|---|
| Assembled view for the doer | ~20 KB | 5k tokens, typical contracted view |
| Doer turns (20 steps, delta-stored) | ~80 KB | ~2 KB assistant + ~2 KB tool result per step |
| Checker view + findings | ~15 KB | diff plus structured findings |
| Messages and task metadata | ~5 KB | typed, small |
| **Total per task** | **~120 KB** | |

A 10-task run is therefore **~1.2 MB of event text**, plus artefact content.

Artefact content is where a naive estimate goes wrong in both directions. Naively, 10 tasks
× 3 file versions × 8 KB ≈ 240 KB. But **content addressing deduplicates aggressively**: an
agent re-reading the same unchanged file twenty times stores it once. Net artefact cost is
closer to **100–300 KB** for a typical run.

| Run size | Estimate |
|---|---|
| Small (3 tasks, converges quickly) | ~0.4 MB |
| **Typical (10 tasks)** | **~1.5 MB** |
| Large (20 tasks, several correction loops) | ~5 MB |
| Pathological (budget exhaustion, long loops) | ~30 MB |

Ten typical runs is roughly **15 MB** — negligible. Ten pathological runs is **300 MB**,
which is not. **The variance, not the mean, is the problem**, and a count-based limit alone
controls the mean while ignoring the tail.

## Decision
Retention is governed by **two limits, whichever binds first**:

| Setting | Default | Meaning |
|---|---|---|
| `retention.full_runs` | **10** | Runs kept in full detail |
| `retention.max_bytes` | **500 MB** | Ceiling across all run data |
| `retention.always_keep` | **3** | Never compacted, regardless of size |

Compaction preserves, permanently: the run summary, final artefacts, all metering
aggregates, and all escalations and their resolutions. It discards intermediate agent
turns, superseded artefact versions, and tool-result payloads.

The recommendation is therefore to **keep 10 as the count default and add the byte ceiling**
that the original proposal lacked. Ten is defensible for typical runs — roughly 15 MB — and
it is the byte ceiling that protects against the tail.

Compaction runs at the end of a run, not on a timer, so it never competes with an active
run.

## Alternatives considered
- **Count only (the original proposal).** Unbounded in the tail; ten pathological runs are
  300 MB.
- **Size only.** Unpredictable history depth; a user cannot say how many runs they can look
  back at.
- **Time-based** ("30 days"). Mismatched to usage — an intense week and a quiet month differ
  by orders of magnitude.
- **Never compact, let the user prune.** Users do not prune, and discover the policy when
  the disk fills.

## Consequences
- Compaction must be **irreversible and obvious**. A user looking for a compacted run sees
  that it was compacted and when, not an empty result.
- Metering aggregates survive compaction permanently, so long-term savings reporting
  ([ADR-0031](0031-token-savings-metering.md)) is unaffected by retention.
- **These numbers need replacing with measurements.** Dogfooding should report real run
  sizes, and this ADR should be superseded once it can cite a run rather than an estimate.
- `corifeu doctor` reports current usage against both limits.
