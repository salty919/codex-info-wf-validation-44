# Windows グラフ残量系列再監査（2026-08-22）

## 指摘

旧現行画像では、残量線がアイドル中の再取得値をそのまま結んだため、利用量が変化していない区間にも下降線が入り、再取得ノイズによって上向きに見える折れが発生し得た。X版の契約は「モデル累積値が進んだ区間だけ残量を変化させ、アイドル区間は水平、欠測・終端は推測しない」である。

## 修正

`GraphPlotControl.BuildEffectiveRemaining` を X版 `remaining_graph_points_for_metric` / `smooth_remaining_points_with_activity` と突合し、次を実装した。

- SOL/TERRA/LUNA の累積値が増えない区間では、quota の低下再取得を無視して水平保持。
- リセット開始アンカーから最初の観測までの未観測区間を消費と解釈しない。
- 活動区間で同じ quota が続き、後続の実測低下がある場合だけ活動時間で補間。
- 活動中の内部変化点は X版と同じ重み付き平滑化を適用し、残量が増えない単調制約を維持。
- 欠測値・終端値を利用量から推測しない。
- リセット直後の初回観測は、X版と同じく水平保持後に観測時刻で段差表示し、期間開始から斜めに接続しない。

## 検証

```text
dotnet test windows-client/CodexInfo.WindowsClient.sln --no-restore --configuration Release
Core: 28/28 PASS
Presentation: 57/57 PASS
合計: 85/85 PASS
```

追加した回帰試験は、利用量が不変の区間で quota が 90→70 と再取得されても表示値を 90 に保持するケースを含む。

## 現行ホスト画像

修正後の現行インストール（managed client DLL SHA-256
`84baa617f82bce3e981a2173d4041581939f930b94c23b62c0d8089afcf3f430`）から新規起動して取得した画像:

[windows-graph-current-v3.png](visual-2026-08-22-current-v3/windows-graph-current-v3.png)

SHA-256: `75b9b65f6b7c7699ddcf2c1f8f8a4abf9656a48033192d4d41e083ee364c75c6`

画像上、残量線は開始アンカーからアイドル帯で水平を保ち、モデル利用が進む区間だけ右下へ進み、右端で上向きに戻っていないことを目視確認した。

## 判定

残量系列の実装・回帰試験・現行画像は更新済み。ただし、物理マウス入力試験は実行しておらず、独立評価担当の現行SHA最終判定も別ゲートとして残るため、製品全体の完了扱いにはしない。
