# RC-079 fresh independent audit (2026-08-23)

## Scope and verdict

- Scope: `RC-079 / WIN-I-001, WIN-I-006, WIN-J-001`, all JSON response headers and route/error matrix.
- Verdict: `INCONCLUSIVE / HOLD`.
- No runtime, release-lineage, or freeze evidence is inferred from static documents.

## Static authority join

- Every response, including 404/405, uses exact `Content-Type: application/json; charset=utf-8`, `Cache-Control: no-store`, and response-header aggregate at most `8 KiB`.
- Fixed bodies require `Content-Length` equal to UTF-8 body bytes; the response-header allowlist does not permit cookies, redirects, compression, authentication, or proxy headers.
- Exact known GET routes are `/v1/health`, `/v1/status`, and `/v1/details`; known non-GET returns JSON 405 and unknown/case-altered/normalized/query paths return JSON 404.
- Success and rejection routes share the read-only effect set and must not mutate SQLite, WAL/SHM, backup, migration, checkpoint, PublishedPair, or filesystem state.
- Status/details transfer limits remain `64 KiB` and `32 MiB`; these are separate from the internal validated snapshot `1 MiB` limit.

## Reproducible checks

```text
git diff --check: exit 0
bash -n scripts/windows_requirements_extraction_check.sh: exit 0
bash scripts/windows_requirements_extraction_check.sh: exit 0 (MACHINE_GATE_PASS)
bash scripts/requirements_intake_guard.sh: exit 1 (requirements extraction incomplete; implementation/evaluation/release blocked)
```

## Required evidence not available

Per-route runtime header/route/effect traces, first-excess cutoff and body-byte measurements, same-release artifact lineage, and freeze capture remain absent. RC-079 cannot be promoted to current-release PASS/CLOSED.

## Audited SHA-256

```text
machine script     e36f963ebbd5d9e861f59d3c7a44e12d8786e16637b055afe3277e26269b98c2
intake script      9470eb4107e7ca61329b580b4c3ec5873111227aab8141536b6ebab734e75867
conflict ledger    f1d6db5707b5d840c455ca7c28e6a4a37193b3a2788f7ed63c3e7c108c2eb6b3
REST authority     ffa2b830b76b878737d124723996cd820da9be30233305b010666a86fe2a28df
data policy        22dd15febc29548197bf76f59ee769b6ee62e2b557aa22f5a4f4406f1517ba0c
cross-scan         22f51ccde4653f0ca26fc193f1ba7b3f246fca8323be5204db852ff0b01b7755
tracker            94ad5587fccade8579f6273ad5c7559ad0cbc3e45bb01355f63d1284e556204c
```
