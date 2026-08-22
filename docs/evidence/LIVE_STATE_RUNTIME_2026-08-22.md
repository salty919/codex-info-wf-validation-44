# Live state runtime evidence (2026-08-22)

対象release binary SHA-256: `124a41aa7e20a24bc1ec2adac2948f2b89be8d158adf89696f4a9c61162983d4`

`CODEX_INFO_DEBUG=1 ./run.sh` の実プロセスで次を確認した。

```text
thread active paths=4
thread active owner roots=1
thread root snapshots=1
thread descendant snapshots=0
thread descendants skipped inactive=58
thread snapshot rows=1
state thread result rows=1
```

これは、DBに残る停止済みchild 58件を履歴だけで実行中に昇格させず、現在のroot 1件だけを公開した実行ログである。`local session files=470`、`local timeline files=470`、`local collect succeeded` も同一実行で確認した。

新規X11 capture（900x480）は [`latest-normal.png`](live-state-runtime-2026-08-22/latest-normal.png)、SHA-256は `e0fd364fc559f9c4a624a929ab408f9e56002c5ed8aceaffabf91f0f97613db7`。ただし目視では、画面左右の内容がviewport内で切れて見え、既存の設計正本（単一縦積み、外周余白、全主要部品の同一グリッド）をこのcaptureだけでは合格と判定できない。X11/WSLgの複数surface合成の影響を切り分ける追加captureが必要であり、UI受入はHOLDとする。

停止・再起動、複数app-serverのcross-server admission非混入、実RPC timeout/EOFについては独立監査V4の指摘どおり未取得であり、LIVE-001をPASSにはしない。
