// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Diagnostics;
using CodexInfo.WindowsClient.Settings;

namespace CodexInfo.WindowsClient.Infrastructure;

/// <summary>OS process adapter for the connection-supervisor port.</summary>
internal sealed class SystemConnectionChildProcessFactory : IConnectionChildProcessFactory
{
    public IConnectionChildProcess Create(ProcessStartInfo startInfo) =>
        new SystemConnectionChildProcess(startInfo);
}

internal sealed class SystemConnectionChildProcess : IConnectionChildProcess
{
    private readonly Process process;

    public SystemConnectionChildProcess(ProcessStartInfo startInfo)
    {
        ArgumentNullException.ThrowIfNull(startInfo);
        process = new Process
        {
            StartInfo = startInfo,
            EnableRaisingEvents = true,
        };
        process.Exited += (_, eventArgs) => Exited?.Invoke(this, eventArgs);
    }

    public event EventHandler? Exited;

    public bool HasExited => process.HasExited;

    public bool Start() => process.Start();

    public void Kill() => process.Kill(entireProcessTree: true);

    public void WaitForExit(int milliseconds) => process.WaitForExit(milliseconds);

    public void Dispose() => process.Dispose();
}
