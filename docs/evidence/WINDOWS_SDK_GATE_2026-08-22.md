# Windows client SDK gate evidence (2026-08-22)

固定SDK `/home/salty/.codex_info_dotnet_sdk/dotnet`（10.0.400）で実行した。

```text
dotnet test windows-client/tests/CodexInfo.WindowsClient.Core.Tests/CodexInfo.WindowsClient.Core.Tests.csproj --no-restore -v minimal
Passed: 28, Failed: 0, Skipped: 0

dotnet test windows-client/tests/CodexInfo.WindowsClient.Presentation.Tests/CodexInfo.WindowsClient.Presentation.Tests.csproj --no-restore -v minimal
Passed: 50, Failed: 0, Skipped: 0
```

固定SDK単体試験の証拠であり、Windowsホストへの現行成果物の再導入、実画像、物理移動、X版画像同等性、REST/daemon/SQLite異常系を証明しない。これらが未確認のため、受入判定は`HOLD`である。
