# RC-061 fresh independent audit (2026-08-23)

## Scope

- RC-061 / GLOBAL:AUD-001, GLOBAL:AUD-020..021, WIN-E-001..002, WIN-J-010..013, WIN-L-016
- Read-only audit of the customer operations runbook, conflict ledger, and extraction machine gate.
- No product implementation, installed-service, Windows runtime, release, or freeze evidence was assumed.

## Independent result

`INCONCLUSIVE (overall HOLD)`: the runbook contains the requested static operation path, but the runbook itself marks implementation and runtime evidence `PRODUCT_PENDING`. RC-061 remains `FIXED_PENDING_FRESH_AUDIT`, not `CLOSED`.

## Evidence

- `docs/WINDOWS_REQUIREMENTS_OPEN_CONFLICTS_2026-08-23.md` records RC-061 as `FIXED_PENDING_FRESH_AUDIT / SERVER-OPERATIONS`.
- `docs/CUSTOMER_OPERATIONS_RUNBOOK.md:3-8` marks the document `EXTRACTION_CONTRACT / PRODUCT_PENDING` and forbids treating the operation as executable before implementation evidence.
- `:13-36` defines UI `./run.sh`, UI-less `CODEX_INFO_API_LISTEN=127.0.0.1:8787 ./run.sh`, GUI/visible-HWND/external-bind zero, and no Cargo/repository/run.sh requirement for the normal customer route.
- `:44-90` defines setup install, systemd target/start/status/health/stop/restart, and health/status/auth/ready separation.
- `:96-110` defines update/rollback/uninstall and retention of settings/history/backups.
- `:121-132` defines failure retention of the previous server/unit/DB.
- `:162-179` defines restore/migrate and retention of the old DB/backups.

## Reproducible checks

- `git diff --check`: exit 0.
- `bash scripts/windows_requirements_extraction_check.sh`: exit 0, `MACHINE_GATE_PASS`.
- Machine script SHA-256: `af7495baed948b4b2a0260a66ab6ea305b2c1a3374726cbdd0a4c22998bc4dc0`.
- Conflict ledger SHA-256: `ee57d60c2f8ff8aae99b0cd3d7a5d5176a44022e3983689be516d78d3e8fc7dc`.
- Runbook SHA-256: `10aa18790ab9c4a7a085416fbc3a8c48d2fc71488966e93fa38fec202f5a05ce`.

## Not verified

Installed services, real process/systemd/DB traces, Windows evidence, same-release artifact lineage, and freeze capture remain unverified. Requirements intake remains blocked.
