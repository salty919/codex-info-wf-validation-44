# Windowsインストーラ再発行・再導入証跡（2026-08-22）

現行ソース（グラフ分バケット・欠測処理・初回導線・ウィンドウ移動ハンドラ修正を含む）から固定SDKで再発行した成果物を、ホストWindowsへ再導入した。物理カーソル、フォーカス奪取、キー送信は行っていない。

```text
SDK: /home/salty/.codex_info_dotnet_sdk/dotnet
Core tests: 28 passed / 0 failed
Presentation tests: 41 passed / 0 failed
installer SHA-256: b5cccbb2b949b6c57a8641b0beb96374315b5004e97bd0c3fe67ed16237b563d
installed client SHA-256: not revalidated for current b5ccc build (previous installed generation is not current evidence)
installed managed client DLL SHA-256: not revalidated for current b5ccc build
installer result: NOT INSTALLED (host client PID 6244 held the installed executable; no process termination was performed)
Start menu result: previous shortcut remains; new payload installation pending a user-safe shutdown
physical input: NOT RUN (explicit opt-in required)
default move smoke raw: `window-move-smoke: SKIP (physical cursor input is opt-in; rerun with -AllowPhysicalInput)`
corrupt-settings payload runtime: process remained alive for 3s and was stopped by the test harness; no mouse, key, focus, or window-move input was sent. Host install was not replaced because PID 6244 remained user-owned.
release decision: HOLD
```

インストーラ本体とpublish payloadのSHAは別物であり、受入スクリプトはインストーラSHAを固定照合する。独立監査、fresh画像、物理操作なしの移動証拠が揃うまで納品判定は変更しない。
