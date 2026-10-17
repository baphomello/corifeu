# 0069 — A local runtime has an owner, and it is Corifeu

- **Status:** Accepted
- **Date:** 2026-10-17
- **Refines:** [ADR-0008](0008-local-runtimes.md)
- **Evidence:** defect 29, defect 66, [troubleshooting](../troubleshooting.md#a-local-model-appears-to-use-its-memory-twice)

## Context
[ADR-0008](0008-local-runtimes.md) says which local runtimes Corifeu speaks to. It says nothing
about who starts them, and the answer until now was: a person, by hand, in a terminal Corifeu
never sees. A provider is an address, and an address is all Corifeu has.

That is cheap and it has a cost nobody had counted. Two measurements, both on the project
owner's machine:

| | |
|---|---:|
| model weights, Q4_K_XL 9B | ~5.5 GB |
| VRAM in use, `-ngl 99` | **7.69 GB** of 8 |
| **host RAM, private, one server** | **8.39 GB** |

The weights are entirely on the GPU and the host still holds its own copy. That is defect 29,
measured in three rounds and parked because nothing in the delivery numbers moved.

What moved was the user's machine. **Two servers were running at once** — 7.1 GB and 6.2 GB in
the task manager, on a 32 GB machine, with the desktop at 97% memory. Nothing had gone wrong in
either server. Nothing had gone wrong in Corifeu either, because Corifeu was not involved:
there is no code anywhere that would notice a second server, refuse to be it, or stop one that
is no longer needed.

Both halves of this had already been written down. Defect 29 carries the memory measurements,
`troubleshooting.md` explains the double copy, and a note told the operator not to leave the
server running. **Three pieces of documentation, zero mechanisms, and the same failure twice.**
That is the whole argument for this ADR: the problem was not that we did not know.

`--no-host`, the one flag whose name promised a cure, was measured and changes nothing: 8,388 MB
against 8,386 MB. The host copy is not a buffer that can be turned off from the command line.

## Decision
**A local provider may declare how to start itself, and Corifeu then owns it.**

```yaml
providers:
  local:
    kind: llama-cpp
    base_url: "http://127.0.0.1:9931"
    manage:
      command: ["llama-server", "-m", "model.gguf", "-ngl", "99"]
      sleep_idle_s: 300      # default
      stop_when_done: true   # default
      ready_timeout_s: 180   # default
      stop: null             # default: kill the process corifeu started
```

1. **Probe before starting.** A `base_url` that answers has an owner. Corifeu uses it and
   **never stops it**: guessing whose server it is, is how a tool kills the one a person was
   using. The run says which of the two happened.
2. **A lock makes the probe a decision.** Two `corifeu` processes can both find nothing and both
   start a server, so the endpoint carries a lock file holding the pid of the Corifeu that owns
   it. A lock whose owner is gone is a crash, not an owner, and is taken.
3. **Stop what it started, and only that.** `stop_when_done` is on by default.
4. **Idle sleep is on by default.** `sleep_idle_s` adds `--sleep-idle-seconds` where the runtime
   is llama.cpp and the command does not already name it, so the runtime releases its memory
   when nobody is using it and forgetting costs nothing. A command that names the flag itself
   keeps what it says — a command that reaches the runtime across a boundary carries the whole
   argument list inside one string, and appending to the launcher's argv would configure the
   launcher.
5. **Stopping may take a command.** Killing the child is right whenever the runtime is the
   child. It is not right across a boundary: Corifeu in WSL starting a server on the Windows
   host gets a launcher that exits at once, and the server it spawned is nobody's child. `stop`
   names what to run instead.
6. **The config refuses what cannot work**, before anything starts: two providers managing one
   address, `manage` with no `base_url` to probe, `manage` on a CLI agent, an empty command.
7. **`corifeu doctor` says who owns each runtime** — the program, whether it sleeps, how it
   stops — and **warns on a runtime with no owner**, which is the state every project is in
   today.

## Alternatives considered
- **Leave it to the operator and document it harder.** What we had. Three documents and the
  failure repeated; the second time it cost the user their machine for the morning. A note
  cannot refuse to start a second process.
- **Kill any other server found at the address.** Removes the duplicate with certainty and is
  unacceptable: the server may be serving something else, or somebody else. Corifeu stops what
  Corifeu started.
- **A single global daemon that owns every model.** The right answer at a larger size, and far
  more than this needs. Revisit when a second model has to be resident at once.
- **A port scan for stray servers.** Finds servers no project declared, and cannot tell a
  colleague's work from a leak. The declared address is the only one Corifeu has a claim on.
- **Fix the host copy instead.** Preferable if it were available. `--no-host` was the candidate
  and was measured flat; the remaining explanations are inside the loader and are not reachable
  from configuration. Not having the problem twice is worth more than halving it once.

## Consequences
- **A project that adopts `manage` cannot produce two servers for one endpoint**, which is the
  failure this came from.
- **A forgotten server costs nothing after five idle minutes.** The operator's discipline stops
  being load-bearing.
- Corifeu now spawns processes in a third place, beside [ADR-0037](0037-sandboxed-command-execution.md)'s
  sandbox and [ADR-0007](0007-cli-providers-first.md)'s agent CLIs. This one is not sandboxed and
  must not be: the command comes from the project's own configuration, which is the same trust
  level as `execution.allowed_commands`, and the runtime needs the GPU.
- **`manage` is opt-in and every existing project keeps working.** The doctor's warning is what
  moves them, not a breaking change.
- Defect 29 is unchanged and stays open. This decision bounds its cost at one copy instead of
  removing it.
