// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Diagnostics;
using CodexInfo.WindowsClient.Settings;

namespace CodexInfo.WindowsClient.Infrastructure;

/// <summary>Windows filesystem/process adapter for the setup connection port.</summary>
internal sealed class WindowsSetupConnectionEnvironment : ISetupConnectionEnvironment
{
    private readonly IConnectionChildProcessFactory processFactory;

    public WindowsSetupConnectionEnvironment()
        : this(new SystemConnectionChildProcessFactory())
    {
    }

    internal WindowsSetupConnectionEnvironment(IConnectionChildProcessFactory processFactory)
    {
        ArgumentNullException.ThrowIfNull(processFactory);
        this.processFactory = processFactory;
    }

    public IReadOnlyList<string> LoadSshConfigAliases()
    {
        try
        {
            var path = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
                ".ssh",
                "config");
            if (!File.Exists(path)) return [];
            return File.ReadLines(path)
                .Take(512)
                .Select(line => line.Trim())
                .Where(line => line.StartsWith("Host ", StringComparison.OrdinalIgnoreCase)
                    || line.StartsWith("Host\t", StringComparison.OrdinalIgnoreCase))
                .Select(line =>
                {
                    var comment = line.IndexOf('#');
                    return (comment >= 0 ? line[..comment] : line).Trim();
                })
                .SelectMany(line => line[(line.IndexOfAny([' ', '\t']) + 1)..]
                    .Split([' ', '\t'], StringSplitOptions.RemoveEmptyEntries))
                .Where(ConnectionSelectors.IsSshAlias)
                .Distinct(StringComparer.OrdinalIgnoreCase)
                .Take(32)
                .ToArray();
        }
        catch (IOException)
        {
            return [];
        }
        catch (UnauthorizedAccessException)
        {
            return [];
        }
    }

    public IReadOnlyList<string> LoadWslDistributions()
    {
        try
        {
            using var process = new Process
            {
                StartInfo = new ProcessStartInfo
                {
                    FileName = "wsl.exe",
                    UseShellExecute = false,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    CreateNoWindow = true,
                },
            };
            process.StartInfo.ArgumentList.Add("-l");
            process.StartInfo.ArgumentList.Add("-q");
            if (!process.Start()) return [];
            var output = process.StandardOutput.ReadToEnd();
            process.WaitForExit(3000);
            return output.Split(['\r', '\n'], StringSplitOptions.RemoveEmptyEntries)
                .Select(value => value.Trim())
                .Where(ConnectionSelectors.IsWslToken)
                .Distinct(StringComparer.Ordinal)
                .Take(32)
                .ToArray();
        }
        catch
        {
            return [];
        }
    }

    public IConnectionChildProcess CreateSshProcess(string target)
    {
        if (string.IsNullOrWhiteSpace(target))
        {
            throw new ArgumentException("An SSH target is required.", nameof(target));
        }

        var startInfo = new ProcessStartInfo
        {
            FileName = "ssh.exe",
            // Keep the explicit one-session recovery path on the same direct
            // executable + ArgumentList boundary as automatic connections.
            // `CreateNoWindow=false` still lets OpenSSH own its interactive
            // host-key/password prompt when the user explicitly starts it.
            UseShellExecute = false,
            CreateNoWindow = false,
            WorkingDirectory = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
        };
        startInfo.ArgumentList.Add("-N");
        startInfo.ArgumentList.Add("-L");
        startInfo.ArgumentList.Add("8787:127.0.0.1:8787");
        startInfo.ArgumentList.Add(target);
        return processFactory.Create(startInfo);
    }
}
