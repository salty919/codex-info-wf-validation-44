// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

namespace CodexInfo.WindowsClient.Settings;

/// <summary>
/// Port used by setup state logic to discover safe connection selectors and
/// create its one transient manual SSH child.
/// </summary>
internal interface ISetupConnectionEnvironment
{
    IReadOnlyList<string> LoadSshConfigAliases();

    IReadOnlyList<string> LoadWslDistributions();

    IConnectionChildProcess CreateSshProcess(string target);
}
