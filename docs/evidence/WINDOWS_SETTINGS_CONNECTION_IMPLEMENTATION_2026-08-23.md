<!-- Copyright (C) 2026 salty919 -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

# Windows 接続設定実装メモ（2026-08-23）

## 対象

`ClientSettingsStore` の6キー契約（`language`、`setupCompleted`、
`connectionConfigured`、`timeZoneId`、`connectionProfile`、
`connectionSelector`）と、直接 `ArgumentList` を使う自動 SSH/WSL 起動情報を実装した。
旧4キー、JSON破損、プロファイルまたはselector不正は、Welcome再表示ではなく
切断状態（`SettingsCorrupt=true`）へフォールバックする。手動SSHのraw host/userは
設定オブジェクトに含まれない。

## 検証

```text
/home/salty/.codex_info_dotnet_sdk/dotnet test windows-client/CodexInfo.WindowsClient.sln --configuration Release --no-restore
  Core: 28 passed, Presentation: 59 passed
PATH=/home/salty/.codex_info_dotnet_sdk:$PATH bash scripts/windows_client_contract_gate.sh
  PASS
PATH=/home/salty/.codex_info_dotnet_sdk:$PATH bash scripts/windows_acceptance_e2e.sh
  PASS (physical cursor smoke is opt-in and was skipped)
```

このメモは実装単位の証跡であり、Windows physical host上の全要件、保存selectorからの
自動再接続 supervisor、fresh画像の独立評価を完了したことを意味しない。
