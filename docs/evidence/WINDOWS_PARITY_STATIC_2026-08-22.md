# Windows parity static evidence (2026-08-22)

この環境の標準 `PATH` には `dotnet` が無いが、リポジトリ外の固定SDK
`/home/salty/.codex_info_dotnet_sdk` を明示して .NET のテストを実行した。
Windows OSの実画像（Graph/Threads/Legal、quota全状態、Setup、Settings）は別証拠として回収済みである。以下の静的証拠は、実Windows画像・Start Menu smoke・keyboard smokeと組み合わせて
WIN-PAR-01..10/12/13、WIN-DES-01..02、WIN-I18N-01..02、WIN-SET-01..02、WIN-ACC-01 の受入根拠を構成する。

## 実行結果

```text
$ /home/salty/.codex_info_dotnet_sdk/dotnet test windows-client/CodexInfo.WindowsClient.sln --no-restore --configuration Release
Core: 28 passed / 0 failed
Presentation: 31 passed / 0 failed

$ bash scripts/windows_client_contract_gate.sh
windows-client-contract-gate: PASS

$ perl -MXML::Parser -e 'XML::Parser->new->parsefile($_) for @ARGV' windows-client/src/CodexInfo.WindowsClient/*.axaml
axaml-parse: PASS
```

契約gateが検出する固定証拠には、Main の graph/threads/legal/settings 操作、Graph の期間・
metric・系列切替、Threads tree、Legal の ScrollViewer、Setup の SSH/API command、認証完了条件、
フォント、installer payload、第三者通知、ならびに今回の bounded 修正が含まれる。

ホストWindowsのfresh window-only画像は `docs/evidence/visual-2026-08-22/` の
`windows-graph-fresh-window-only.png`、`windows-threads-fresh-1000x800.png`、
`windows-legal-fresh-window-only.png` に保存し、別の独立監査で目視した。

今回のコード上の追加証拠:

- `LocalizationService.NormalizeLanguageCode` が未知言語を英語へ決定的にfallbackし、設定読込時に
  言語と timezone (`UTC` / `local`) を正規化する。
- timezone変更を既存の表示変更通知へ流し、日時を選択文化の `g` 形式で再描画できる。
- モデルのtoken/dollar文字列は保存済みraw値から再計算され、言語変更後の桁区切りを保持しない。
- 経過時間は選択言語の単位を使い、残り時間は値が0の単位を省略する。
- 詳細endpointの行数ではなく status snapshot の `active_thread_count` を概要の正本にする。
- Graph の空履歴選択を無効化し、期間・metric selectorへ accessible name/tooltip を付与する。

実WindowsでのGraph window-only fresh画像は別証拠（`windows-graph-fresh-window-only.png`）で確認済み。
実Windowsの高DPI/最小サイズ、キーボード実操作、各locale、Threads/Legal、Setup最小幅は
`WINDOWS_ACCEPTANCE_E2E_2026-08-22.md`のfresh画像SHAとsmokeで確認済み。ブラウザ認証は資格情報を取得せず、WSL login起動・認証済みstatus再確認・未認証完了拒否の安全境界をfixture/contractで確認する。
