// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Diagnostics;

namespace CodexInfo.WindowsClient.Settings;

internal interface IConnectionChildProcess : IDisposable
{
    event EventHandler? Exited;

    bool HasExited { get; }

    bool Start();

    void Kill();

    void WaitForExit(int milliseconds);
}

internal interface IConnectionChildProcessFactory
{
    IConnectionChildProcess Create(ProcessStartInfo startInfo);
}
