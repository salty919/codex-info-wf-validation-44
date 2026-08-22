# Graph 境界テスト証跡（2026-08-22）

担当エージェント `implement_graph_boundary_tests` が製品コードを変更せず、
`windows-client/tests/CodexInfo.WindowsClient.Presentation.Tests/WindowDragGeometryTests.cs`
へ境界テストを追加した。

確認範囲:

- `now < start`、`now == start`、`now == end`、`now > end`
- 履歴なし
- 期間外サンプル除外
- 期間終端アンカーと終端サンプル除外
- 既存の分バケット最大累積値・`null remaining`

固定SDK実行結果:

```text
SDK: /home/salty/.codex_info_dotnet_sdk/dotnet (10.0.400)
WindowDragGeometryTests: 18 passed, 0 failed
Presentation.Tests 全体: 50 passed, 0 failed
git diff --check: PASS
```

これは単体テストの証跡であり、同一成果物SHAの実Windows描画、X版との画像同等性、
実インストール後の挙動を証明しない。独立監査がPASSするまで要求行は`HOLD`とする。
