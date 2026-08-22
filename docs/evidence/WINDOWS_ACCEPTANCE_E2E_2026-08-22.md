# Windows client acceptance evidence (2026-08-22)

The historical acceptance transcript below is retained for traceability only; it is not a current release PASS. The current gate is reproducible and fail-closed. It runs the Core and
Presentation tests, the source contract gate, AXAML parse, exact SHA-256
checks for every fresh Windows state image, installer identity checks, and a
real Start-menu launch/stop smoke on the WSL2 Windows host when
`powershell.exe` is available.

```text
scripts/windows_acceptance_e2e.sh
Core:         28 passed / 0 failed
Presentation: 41 passed / 0 failed
windows-client-contract-gate: PASS
windows-start-menu-smoke: PASS
windows-keyboard-focus-smoke: PASS
windows-acceptance-e2e: HISTORICAL PASS (not current release evidence; current decision is HOLD)
```

The installed host path is `%LOCALAPPDATA%\Programs\Codex Info Monitor`; the
Start-menu shortcut target and working directory both resolve to that install
directory. The final setup artifact is present at
`artifacts/windows-installer/CodexInfo.WindowsClient.Setup.exe` and the
workspace publish copy has the same SHA-256:

```text
b5cccbb2b949b6c57a8641b0beb96374315b5004e97bd0c3fe67ed16237b563d
```

最新変更後の物理移動スモークは、ユーザーのカーソルを奪わないため明示指定なしでは実行しない。`-AllowPhysicalInput`を付けない受入ゲートはこの項目をSKIPし、未検証をPASSへ変換しない。

## 旧版の全ボーダーレス画面の実移動・クローズ証跡（fresh host、受入根拠から撤回）

以下は旧スクリプトがホストの物理カーソルを操作して取得した履歴であり、ユーザーの明示許可なしに再実行しない。現行受入ではこの節をPASS根拠に使わず、未検証として扱う。旧証跡を残すのは、過去の検証が現在の合格証拠へすり替わることを防ぐためである。

`scripts/windows_window_move_smoke.ps1`をインストール済みクライアントへ実行し、各ケースを新規プロセスで起動してタイトル領域を左ボタンでドラッグした後、右上の閉じる操作を実Win32入力で行う。矩形の`before`/`after`が変化し、対象HWNDが非表示になったものだけをPASSとする。合成入力は相対移動イベントを使用し、入力キュー由来の見かけ上の座標逆戻りも検出する。最新acceptanceのraw出力:

```text
virtual-desktop: PASS origin=0,0 size=9600x2160
window-center: PASS normal center=1920,1044
window-move: PASS normal before=1245,504 after=1527,691
window-cross-monitor: PASS normal before=1245,504 after=9519,1073
window-close: PASS normal
window-center: PASS setup center=1920,1044
window-move: PASS setup before=1350,534 after=1644,729
window-close: PASS setup
window-center: PASS graph center=1920,1044
window-move: PASS graph before=1350,534 after=1643,730
window-close: PASS graph
window-center: PASS threads center=1920,1044
window-move: PASS threads before=1350,534 after=1644,730
window-close: PASS threads
window-center: PASS legal center=1920,1044
window-move: PASS legal before=1350,534 after=1643,729
window-close: PASS legal
window-center: PASS settings center=1920,1044
window-move: PASS settings before=1455,579 after=1749,775
window-close: PASS settings
```

Main/Setup/Graph/Threads/Legal/Settingsの6画面で実マウス移動と閉じる操作を確認し、ソース上のハンドラ存在だけでは合格にしていない。仮想デスクトップは9600x2160、全6画面の初期中心は作業領域中心(1920,1044)と一致し、通常画面では別モニタへ跨ぐ長距離ドラッグも確認した。DPI仮想化による座標ずれを避けるため、試験プロセス自身も`SetProcessDPIAware`を有効にしている。

Native-drag fresh host images (same installed SHA) were recaptured after the move-loop change: `windows-main-native-drag.png` (`11a100b921be21464de2c44b1e13177ef13664bdc4ae8e63a619e4111d5eb53e`), `windows-graph-native-drag.png` (`0e16d6303c69f343a16966071541797449e5611cf91908e333ed99e661b1acc7`), `windows-threads-native-drag.png` (`fa47c9ae40b38a8e1deac3b9bc32bafd83a2abb12e37d58c2cd6d12beb1a035f`), `windows-legal-native-drag.png` (`dca32b094a09bbf0e5c43db2e07aa6508c65899f89d848e65017f5eb530b26d7`), `windows-settings-final-native-drag.png` (`d93d5846e3c61c3b93b27e846d13cd585848c840146af51fa729f9e5005ea766`), and `windows-setup-native-drag.png` (existing SSH-guidance SHA `229e01e71ca162a65df4ccd72ef825979a62aa630b6c2fe550ef57c1f132d9ff`).

Fresh state images (13 exact SHA-256 entries) cover normal, warning (10%), critical (2%), zero, full,
API error, authentication-required, Graph, Threads, Legal, Setup, and
Settings. Their hashes are recorded in
[WINDOWS_RUNTIME_2026-08-22.md](WINDOWS_RUNTIME_2026-08-22.md). The images
were captured from newly started host processes after the final publish; no
previous window was reused.

This evidence proves the bounded Windows acceptance surface. Native regression
rows and the database-protection policy are included in the current integrated
gates; no row is waived or silently downgraded.
