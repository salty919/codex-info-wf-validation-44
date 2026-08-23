# RC-066 fresh independent audit (2026-08-23)

## Scope

- RC-066 / WIN-J-011
- Read-only audit of the current conflict ledger, the RC-066 row projection, DATA_PROTECTION_POLICY, DESIGN, and the extraction machine gate.
- No product implementation, release, freeze, or runtime evidence was assumed.

## Independent result

`HOLD`: the document projection and machine structure are consistent, but product runtime and same-release/freeze artifact lineage are not available. RC-066 remains `FIXED_PENDING_FRESH_AUDIT`, not `CLOSED`.

## Evidence

- `docs/WINDOWS_REQUIREMENTS_OPEN_CONFLICTS_2026-08-23.md:76` records RC-066 as `FIXED_PENDING_FRESH_AUDIT / DATA-PROTECTION`.
- `docs/atomic-contracts/WIN_J_M_CONCRETE_CONTRACTS_2026-08-22.md:105-109` contains `WIN-J-011 unexpected-exit projection (RC-066)` and `source_id=WIN-J-011:unexpected-exit`.
- The projection explicitly covers explicit TERM, 2-second unexpected-exit detection, one restart after a 5-second backoff in the same epoch, Failed latch after restart failure or a second unexpected exit, explicit-start/systemd-new-activation epoch boundaries, retention of last-good/DB/hint/cursor/gap-ledger state, and prohibition on fabricated gaps.
- `docs/DATA_PROTECTION_POLICY.md:93,120-127,145-147` and `DESIGN.md:111` provide the selected authority values and retention boundary.

## Reproducible checks

- `git diff --check`: exit 0.
- `bash scripts/windows_requirements_extraction_check.sh`: exit 0, `MACHINE_GATE_PASS`.
- Machine script SHA-256: `4d9f60156bede0d6dd452ef524b283c93841da1df49c91393f194ac4784b9e11`.
- Conflict ledger SHA-256: `d6955de01f16620f2d1c12f5720f585472a749da903fdbbc3fa9ad725963daf4`.
- J-M contracts SHA-256: `c6d925209a66967b11976e670205ffc8c837c06e7f7cf4536728de20c19f47d4`.
- DATA_PROTECTION_POLICY SHA-256: `2a161b3313a3d6c4e0e6ba6354bd67609c7d7ab73513fff8ec5b4a365cb9fdad`.
- DESIGN SHA-256: `990d3b93d3acad36149b7de81b202ff88631b2e34cd29e9e63fcd970e594df40`.

## Not verified

Product runtime behavior, same-release artifact lineage, freeze capture, and real supervisor/process traces remain unverified. Requirements intake remains blocked.
