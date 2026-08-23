<!-- Copyright (C) 2026 salty919 -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

# Codex Info 運用手順

## 起動構成

同一release binary `codex_info`は、次の3モードだけを提供する。

| モード | command | 常駐所有者 |
| --- | --- | --- |
| daemon+REST | `codex_info --service --listen 127.0.0.1:8787` | 1 processがrecorder workerとloopback RESTを所有 |
| X UIのみ | `codex_info --ui-only` | なし。daemon/RESTを起動せず、終了後にも残さない |
| 全体起動 | `codex_info --all` または引数なし | healthyな既存serviceを再利用。なければserviceを1つ起動してX UIを追加 |

`--listen`はnumeric loopback addressだけを受理する。`0.0.0.0`、`::`、LAN address、hostnameは拒否する。
`--ui-only`は`CODEX_INFO_API_LISTEN`を継承していてもservice modeへ変化しない。
後方互換のため、引数なしで`CODEX_INFO_API_LISTEN`だけを指定した起動はWindowを生成しないservice modeになる。
同じ環境変数のaddressで全体起動するときは`--all`を明示し、X UIを必ず追加する。

repositoryから起動する場合は同じ引数を`run.sh`へ渡す。

```bash
./run.sh --service
./run.sh --ui-only
./run.sh --all
```

## WSL / Ubuntu user-systemd自動起動

release build後に次を実行する。

```bash
cargo build --release --locked
bash scripts/install_systemd_recorder.sh
systemctl --user status codex-info.service
curl --fail http://127.0.0.1:8787/v1/health
```

installerはrelease binaryを`%h/.local/bin/codex_info`へ置き、`codex-info.service`だけを有効化する。
旧分離unitが存在する場合は競合防止のため停止・無効化する。

## daemon自動起動から外す

```bash
bash scripts/install_systemd_recorder.sh --remove
```

この操作は`codex-info.service`を停止・無効化し、unit fileだけを削除する。次は削除しない。

- `%h/.local/bin/codex_info`
- `history/usage_history.sqlite3`
- DB backup
- `history/usage_reset_hint.json`
- Codex session JSONL

履歴データ自体の削除は、この自動起動解除とは別の明示操作として扱う。

## 停止と確認

```bash
systemctl --user stop codex-info.service
systemctl --user is-active codex-info.service
curl --max-time 1 http://127.0.0.1:8787/v1/health
```

正常停止ではREST listenerを閉じ、recorder workerを停止し、singleton lockを解放する。
停止中の未取得値を推測・補間せず、既存DBとlast-good値を保持する。

## Windowsクライアント

WindowsクライアントはWSL/Ubuntu側のserviceへSSH local port forwarding経由で接続する。
X UIを併用する場合もserviceを増やさず、`--ui-only`を追加起動する。
保持期間、1回の取得上限、REST SLOは[REST API v1](REST_API_V1.md)と
[データ保護規約](DATA_PROTECTION_POLICY.md)を正本とする。
