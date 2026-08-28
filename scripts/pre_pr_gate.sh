#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

bash scripts/regression_guard.sh
bash scripts/data_protection_gate.sh
bash scripts/windows_client_contract_gate.sh

echo 'pre-pr-gate: PASS'
