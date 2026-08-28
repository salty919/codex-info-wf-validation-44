// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Diagnostics;

namespace CodexInfo.WindowsClient.Settings;

/// <summary>
/// Builds the only automatic child-process command lines allowed by the
/// Windows connection contract. Callers receive an argv-backed
/// <see cref="ProcessStartInfo"/>; no shell parser is involved.
/// </summary>
public static class ConnectionProcessFactory
{
    public static ProcessStartInfo BuildAutomaticSsh(string selector)
    {
        if (!ConnectionSelectors.IsSshAlias(selector))
        {
            throw new ArgumentException("A literal OpenSSH Host alias is required.", nameof(selector));
        }

        var startInfo = HiddenDirect("ssh.exe");
        startInfo.ArgumentList.Add("-o");
        startInfo.ArgumentList.Add("BatchMode=yes");
        startInfo.ArgumentList.Add("-N");
        startInfo.ArgumentList.Add("-L");
        startInfo.ArgumentList.Add("8787:127.0.0.1:8787");
        startInfo.ArgumentList.Add(selector);
        return startInfo;
    }

    public static ProcessStartInfo BuildAutomaticWsl(string selector)
    {
        if (!ConnectionSelectors.IsWslToken(selector))
        {
            throw new ArgumentException("An installed WSL distribution token is required.", nameof(selector));
        }

        var startInfo = HiddenDirect("wsl.exe");
        startInfo.ArgumentList.Add("--distribution");
        startInfo.ArgumentList.Add(selector);
        startInfo.ArgumentList.Add("--");
        startInfo.ArgumentList.Add("codex_info");
        startInfo.ArgumentList.Add("--port");
        startInfo.ArgumentList.Add("8787");
        return startInfo;
    }

    private static ProcessStartInfo HiddenDirect(string executable) => new()
    {
        FileName = executable,
        UseShellExecute = false,
        CreateNoWindow = true,
    };
}
