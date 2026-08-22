# Live state runtime evidence (current release)

- Release SHA-256: `816de5ddd00d8d12cc42b20fdac974b6a79d7da162e99d2c9d26a8824bb8726e`
- Launch: `CODEX_INFO_DEBUG=1 ./run.sh`
- Observed process: `target/release/codex_info` (fresh PID, non-deleted executable)

## Raw bounded diagnostics

```text
thread active paths=4
thread active owner roots=1
thread root snapshots=1
thread descendant snapshots=0
thread descendants skipped inactive=62
thread snapshot rows=1
state thread result rows=1
local collect succeeded rows=2 samples=1036
```

The current cycle published one root row and skipped 62 historical child rows whose rollout paths were not held by a current Codex process. Local usage completed independently with two model rows. This is runtime evidence for the active-path gate, not a substitute for the missing stop/restart and multi-server traces or current visual acceptance images.

## Gate status

This evidence is supporting evidence only. The independent V12 audit remains `INCONCLUSIVE/HOLD` until current-SHA X/Windows captures, process stop→restart trace, and multi-server/collector admission trace are attached.
