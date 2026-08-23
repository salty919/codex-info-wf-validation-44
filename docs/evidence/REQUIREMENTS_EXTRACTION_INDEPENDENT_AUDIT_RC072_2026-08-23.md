# RC-072 fresh independent audit (2026-08-23)

## Scope and verdict

- Scope: `RC-072 / WIN-J-014, WIN-J-015, WIN-L-016` explicit customer restore operation.
- Evaluator: fresh read-only evaluator, separate from the authoring agent.
- Verdict: `INCONCLUSIVE / HOLD`.
- Product runtime, same-release artifact lineage, and freeze capture are not asserted by this static record.

## Static projection under audit

- The customer command is the existing `codex-info-server-setup restore --generation 1`.
- All writer/API/UI services are stopped before restore; the current DB is retained rather than deleted.
- The selected complete verified generation is checked for SHA-256, `quick_check`, schema, row count, deterministic fingerprint, and reset-period boundary.
- Staging is on the same filesystem, followed by flush and atomic replacement, reload, REST status/details pair verification, and UI reload verification.
- Candidate, validation, lock, staging, replace, reload, and pair failures retain the current DB, every verified backup, old memory/root, and history; switch/publication remains zero.
- Restore is separate from `migrate --dry-run` and `migrate --apply`; automatic restore, implicit schema conversion, current/backup deletion, empty DB replacement, and duplicate re-entry effects are forbidden.

## Raw deterministic gate results

```text
git diff --check: exit 0
bash -n scripts/windows_requirements_extraction_check.sh: exit 0
bash scripts/windows_requirements_extraction_check.sh: exit 0 (MACHINE_GATE_PASS)
bash scripts/requirements_intake_guard.sh: exit 1 (requirements extraction incomplete; implementation/evaluation/release blocked)
```

## Required evidence not yet available

Stop/order/process trace, path/inode/hash trace, candidate validation trace, flush/fsync/atomic-replace trace, reload and REST/UI pair trace, failure retention trace, same-operation re-entry trace, same-release artifact lineage, and freeze capture remain required for product closure.

The fresh evaluator confirmed that the static projection is internally consistent and that the deterministic machine gate passes, but the missing runtime, release-lineage, and freeze evidence prevents current-release PASS/CLOSED promotion.

## Audited bytes at provisional record creation

```text
script     6db64afa7eed8f8836a0880a664bb969521cf5eb476eafdf7bbe796fd3d44adc
conflicts  ddff557b9756b1a5017bb464e4ba7a6397f68e5400030d2e0a5402d2175f1ff7
contracts  e0724d23b568fbc76e14d9de0ef2aea30d39fc467b0907632e8ddce18ce6815f
data_rows  ab7cf49f29366611fcf1fe1e005ad299e6305b0fdc8b0986cff337aafa71668f
runbook    ceacf05373bd52afb2d4466679db6769781ab83050af5b471c15dec05d92fea2
policy     22dd15febc29548197bf76f59ee769b6ee62e2b557aa22f5a4f4406f1517ba0c
cross-scan eec66f70561aeeb925adebd8c02a667e88884a8caeca8a5b11a01212c1350bdf
tracker    aee34d1ae2600b52c6c2f99e244d3336670fd18551df4c5667dc249765cb31bc
```
