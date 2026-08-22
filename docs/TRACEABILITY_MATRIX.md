# 要求トレーサビリティ

要求を受入可能な単位へ分解し、実装・自動テスト・実機証拠・独立判定を一行で追跡する。未確認の行は納品対象から除外する。

現行のWindows要求ID正本は
`docs/WINDOWS_REQUIREMENTS_TRACEABILITY_MATRIX_2026-08-22.md` のWIN-A〜M 226行である。
下表のDP/WIN-INSTALL/WIN-PAR等は移行元の旧ID索引であり、現行226行の代替・合否根拠にはしない。
旧IDは現行行への対応が確定するまで `LEGACY_REFERENCE_ONLY` とする。

| 要求群 | 正本 | 実装範囲 | 自動検証 | 実機証拠 | 独立判定 |
| --- | --- | --- | --- | --- | --- |
| DB保護/daemon | `docs/REQUIREMENTS_LEDGER.md` DP-001〜010 | `src/usage_store.rs`, `src/daemon.rs`, `packaging/` | cargo tests, daemon E2E, data gate | DB hash/quick_check/systemd logs | 必須 |
| Windows installer | `docs/WINDOWS_CLIENT_REQUIREMENTS.md` WIN-INSTALL-01〜04 | `windows-client/installer/` | publish/contract gate | host Windows install/start/uninstall | 必須 |
| Windows parity/design | 同 WIN-PAR/WIN-DES | `windows-client/src/` | Core/Presentation tests | fresh normal/warning/error/DPI images | 必須 |
| 多言語/導入 | 同 WIN-I18N/WIN-SET/WIN-ACC | ViewModels/Settings/Setup | culture/UI tests | 各言語・再起動・接続復旧 | 必須 |
| 全Windowsウィンドウ操作 | `docs/WINDOWS_CLIENT_REQUIREMENTS.md` WIN-ACC-02 | Main/Setup/Settings/Graph/Threads/Legal title regions | `windows_window_move_smoke.ps1`, contract gate | fresh host drag smoke | 必須 |
| native回帰 | 同 REG-01〜11 | `src/`, `run.sh` | fmt/check/test/build/gates | fresh X11 PID/XID, runtime trace | 必須 |

更新時は、変更された行と影響境界を先に記録し、証拠を取得してからstatusを変更する。
