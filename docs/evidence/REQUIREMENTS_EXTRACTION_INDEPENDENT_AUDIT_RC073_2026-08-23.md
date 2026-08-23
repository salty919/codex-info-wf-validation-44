# RC-073 fresh independent audit (2026-08-23)

## Scope and verdict

- Scope: `RC-073 / WIN-J-015` migration-switch three-path contract.
- Evaluator: fresh read-only evaluator, separate from the authoring agent.
- Verdict: `INCONCLUSIVE / HOLD`.
- Static projection and machine structure are consistent; runtime journal/recovery, same-release artifact lineage, and freeze capture are unavailable. `CLOSED`/PASS is prohibited.

## Static projection checked

- Old-schema startup rejects read/write/publish and does not create an empty replacement.
- Explicit `UsageStore::migrate_verified` closes writer/API/UI admission and validates every row/type/value, unique key, quick_check, schema, count, fingerprint, and period/partition boundary.
- `migration-switch-v1` is owner-only 0600 UTF-8 JSON <=64 KiB with exact phases and interrupts.
- Current missing/double/empty and candidate/lock/rename/fsync/reload/pair failures choose one rollback or roll-forward from journal/path/inode/hash; recovery precedes writer/publish.
- OLD DB, backups, checkpoint, old memory/root remain retained; same-operation re-entry and foreign/second operation mutation are zero.

## Raw gate results

```text
git diff --check: exit 0
bash -n scripts/windows_requirements_extraction_check.sh: exit 0
bash scripts/windows_requirements_extraction_check.sh: exit 0
windows-requirements-extraction-check: MACHINE_GATE_PASS (overall extraction remains HOLD until independent semantic audits and conflict closure pass)
requirements_intake_guard: exit 1 (extraction incomplete; implementation/evaluation/release blocked)
```

## Missing closure evidence

Required but unavailable: same-release runtime path/inode/hash/phase, old-schema/candidate/failure traces, five crash/lock/rename interrupts, rollback/roll-forward replay, current count, publication/DataGeneration, re-entry/foreign mutation, release lineage, and freeze capture. RC-073 remains `FIXED_PENDING_FRESH_AUDIT / DATA-PROTECTION`.

## Audited bytes

```text
script     06494168f4ca8732b20e7517f1c7bd4376bdbdd23b87d682afb5d29f2d6b13f6
conflicts  e7c15e7de6bc858a8e13ab618e4c52a33078679a1dfc7b4c56840cff8f08a9a9
contracts  d67b7215e5975c23a12e69db2d7d1d5449c606d510c2cc6260deeff682efa87f
cross-scan a45fb81d776a1b9c9281ef61807badf469b3ebb9a7f78768524b0438c72ffeb0
tracker    028e86290c938235563de2fcaecfb337c1784a7f1694b52c3378bfca71894d2f
```
