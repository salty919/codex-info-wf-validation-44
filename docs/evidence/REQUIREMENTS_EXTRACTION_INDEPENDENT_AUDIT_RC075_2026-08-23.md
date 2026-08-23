# RC-075 fresh independent audit (2026-08-23)

## Scope and verdict

- Scope: `RC-075 / WIN-J-010, WIN-J-011` bounded daemon work, fingerprint, backfill, and cursor/retry rules.
- Evaluator: fresh read-only evaluator, separate from the authoring agent.
- Verdict: `INCONCLUSIVE / HOLD`.
- Product runtime, same-release artifact lineage, and freeze capture are not asserted by this static record.

## Static projection under audit

- Integer daemon interval is `5..3600`, default `60`; each event allows at most one scan and one SQLite transaction.
- Transaction, record, file, and aggregate bounds are `1024 rows/1 MiB`, `4 MiB`, `256 MiB`, and `2 GiB`.
- Canonical fingerprint uses regular non-symlink JSONL under the sessions root, sorted by relative path, with device/inode, size, mtime_ns, last complete LF offset, and last complete row SHA-256.
- Unchanged fingerprint yields scan/write/retry zero; append, rotate/replace, and truncate use the existing cursor rules; one outage epoch consumes one backfill latch; scan/restart retry is at most one and same-callback retry is zero.
- Rejected or failed work retains checkpoint/cursor, DB, backup, gap ledger, source log, and last-good root; no interpolation, copied sample, old-period reattribution, or synthetic gap success.

## Raw deterministic gate results

```text
git diff --check: exit 0
bash -n scripts/windows_requirements_extraction_check.sh: exit 0
bash scripts/windows_requirements_extraction_check.sh: exit 0 (MACHINE_GATE_PASS)
bash scripts/requirements_intake_guard.sh: exit 1 (requirements extraction incomplete; implementation/evaluation/release blocked)
```

## Required evidence not yet available

Real daemon bounded counters, fingerprint/cursor/backfill/restart traces, same-release artifact lineage, and freeze capture remain required for product closure.

The fresh evaluator found the static projection internally consistent and confirmed the deterministic machine gate, but the required runtime, release-lineage, and freeze evidence is absent; current-release PASS/CLOSED promotion is prohibited.

## Audited bytes at provisional record creation

```text
script     38ab3b1335fe35abc9c8e1130354aa8bb7e19f92f42041dd8087afef392b91ac
conflicts  f1d6db5707b5d840c455ca7c28e6a4a37193b3a2788f7ed63c3e7c108c2eb6b3
contracts  44ee5056861e6b2c548cf9da23ed1cb354a28b0b0ae4d9001b549c1915663e0e
data_rows  ab7cf49f29366611fcf1fe1e005ad299e6305b0fdc8b0986cff337aafa71668f
policy     22dd15febc29548197bf76f59ee769b6ee62e2b557aa22f5a4f4406f1517ba0c
cross-scan 9fd1e2d32342fbe69475b8bd7a4a75df5fa1fa6734f18c38fe9b2199a03ca761
tracker    99b5709a2eb32d91235ae738436306de61be7cad194b8dc8de9dd9cb3b160e93
```
