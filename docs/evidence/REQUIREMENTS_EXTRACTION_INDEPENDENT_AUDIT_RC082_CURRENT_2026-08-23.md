# RC-082 current-source independent audit (2026-08-23)

## Scope and verdict

- Scope: `RC-082 / WIN-M-004, WIN-G-014, WIN-F-007, WIN-E-011, WIN-E-016`, Setup Cancel/Back/Close boundaries.
- Verdict: `INCONCLUSIVE / HOLD`; RC-082 remains `OPEN / SETUP-UX`.
- This record supersedes no prior status record and does not infer missing behavior.

## Confirmed static join

- First-launch Setup has the existing projection `visible_cancel=true`, owned product-process reap, confirmation before exit, and `setup_complete=false`.
- Setup reopened from Settings has `visible_cancel=true`, unsaved-input discard, `route=Settings`, `setup_complete=true`, and `write_count=0`.
- Existing WIN-E-011/WIN-E-016 failure oracles cover orphan process/tunnel/reap ownership and settings-byte/secret-persistence boundaries.

## Explicitly unresolved (kept OPEN)

- The current authorities do not uniquely define Back's prior-step rule for every Setup step.
- The current authorities do not uniquely define first-launch Cancel/Close's Main-disconnected plus Settings-recovery route.
- The current authorities do not uniquely define complete retention of the prior six-key bytes on reopen Cancel/Close.

No value, route, or retention rule is invented to close these gaps.

## Reproducible checks

```text
git diff --check: exit 0
bash -n scripts/windows_requirements_extraction_check.sh: exit 0
bash scripts/windows_requirements_extraction_check.sh: exit 0 (MACHINE_GATE_PASS)
bash scripts/requirements_intake_guard.sh: exit 1 (requirements extraction incomplete; implementation/evaluation/release blocked)
```

## Required evidence not available

Independent UIA/keyboard/process/settings runtime traces, same-release artifact lineage, and freeze capture are absent. RC-082 cannot be promoted to FIXED or CLOSED.

## Audited SHA-256

```text
machine script     e36f963ebbd5d9e861f59d3c7a44e12d8786e16637b055afe3277e26269b98c2
intake script      9470eb4107e7ca61329b580b4c3ec5873111227aab8141536b6ebab734e75867
conflict ledger    f1d6db5707b5d840c455ca7c28e6a4a37193b3a2788f7ed63c3e7c108c2eb6b3
cross-scan         b59bb32e01d2eec7edc26d05dab0d267f03b1cd682f74743a164be7418d953a2
tracker            ccddd949dc12dc2978d4251e541cb11f743f9b03a7c7550d7a8dd1f14265a4a6
```
