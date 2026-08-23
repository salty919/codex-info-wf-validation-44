# RC-076 fresh independent audit (2026-08-23)

## Scope and verdict

- Scope: `RC-076 / WIN-I-006, WIN-J-010` local JSONL, internal snapshot, and REST transfer resource boundaries.
- Evaluator: fresh read-only evaluator, separate from the authoring agent.
- Verdict: `INCONCLUSIVE / HOLD`.
- Product runtime, same-release artifact lineage, and freeze capture are not asserted by this static record.

## Static projection under audit

- Local JSONL limits are line `4 MiB`, file `256 MiB`, and aggregate `2 GiB`, counted on received bytes before decode.
- Internal validated snapshot is a separate canonical JSON resource capped at `1 MiB`.
- REST transfer limits are response header `8 KiB`, status body `64 KiB`, and details body `32 MiB`; fixed Content-Length must equal UTF-8 body bytes and unknown-length streams stop at the first excess byte.
- Local invalid-record isolation requires a later validated cumulative snapshot; otherwise file/candidate rollback applies. Internal oversize rejects the candidate, and REST oversize rejects the whole PublishedPair.
- Prior valid state/hash/root is retained and partial rows, partial pairs, and guessed values are never published.

## Raw deterministic gate results

```text
git diff --check: exit 0
bash -n scripts/windows_requirements_extraction_check.sh: exit 0
bash scripts/windows_requirements_extraction_check.sh: exit 0 (MACHINE_GATE_PASS)
bash scripts/requirements_intake_guard.sh: exit 1 (requirements extraction incomplete; implementation/evaluation/release blocked)
```

## Required evidence not yet available

Resource-specific byte counters, Content-Length/stream cutoff, local/internal/REST state traces, same-release artifact lineage, and freeze capture remain required for product closure.

The fresh evaluator found the static resource boundaries internally consistent and confirmed the HOLD conditions; the required runtime, release-lineage, and freeze evidence is absent, so current-release PASS/CLOSED promotion is prohibited.

## Audited bytes at final record

```text
script     e36f963ebbd5d9e861f59d3c7a44e12d8786e16637b055afe3277e26269b98c2
conflicts  f1d6db5707b5d840c455ca7c28e6a4a37193b3a2788f7ed63c3e7c108c2eb6b3
contracts  540d1dc923b4e2999706ef8692ae1c8b698d6781ac3586f7e82d51d151c37b6b
data_rows  ab7cf49f29366611fcf1fe1e005ad299e6305b0fdc8b0986cff337aafa71668f
REST       ffa2b830b76b878737d124723996cd820da9be30233305b010666a86fe2a28df
policy     22dd15febc29548197bf76f59ee769b6ee62e2b557aa22f5a4f4406f1517ba0c
cross-scan fe99689f5e26e4cc9c2e6f7085fe24bc3cd1a86bd6962df465646a75d5ac20a5
tracker    967010974498bb5748e0ca2c3317269b4df662f79b092773fdd1d3b20a9174a9
```
