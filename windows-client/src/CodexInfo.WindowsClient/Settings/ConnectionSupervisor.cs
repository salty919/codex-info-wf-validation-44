// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.ComponentModel;
using System.Diagnostics;
using CodexInfo.WindowsClient.Infrastructure;

namespace CodexInfo.WindowsClient.Settings;

/// <summary>
/// Owns the single automatic bootstrap/tunnel child for one client
/// generation. It deliberately performs no retry loop: a failed child is
/// reaped and the UI remains disconnected until an explicit refresh/setup
/// action starts a new generation.
/// </summary>
public sealed class ConnectionSupervisor : IDisposable
{
    private readonly object gate = new();
    private readonly IConnectionChildProcessFactory processFactory;
    private IConnectionChildProcess? child;
    private bool disposed;

    public ConnectionSupervisor()
        : this(new SystemConnectionChildProcessFactory())
    {
    }

    internal ConnectionSupervisor(IConnectionChildProcessFactory processFactory)
    {
        ArgumentNullException.ThrowIfNull(processFactory);
        this.processFactory = processFactory;
    }

    public bool IsRunning
    {
        get
        {
            lock (gate)
            {
                return child is { HasExited: false };
            }
        }
    }

    public bool EnsureStarted(ClientSettings settings)
    {
        lock (gate)
        {
            if (disposed) return false;
            if (child is { HasExited: false }) return true;
            ReapChildLocked();
            if (!ConnectionSelectors.IsValid(settings)) return false;
            if (settings.ConnectionProfile is ConnectionProfiles.None)
            {
                return true;
            }

            ProcessStartInfo startInfo;
            try
            {
                startInfo = settings.ConnectionProfile switch
                {
                    ConnectionProfiles.SshConfigAlias =>
                        ConnectionProcessFactory.BuildAutomaticSsh(settings.ConnectionSelector),
                    ConnectionProfiles.Wsl =>
                        ConnectionProcessFactory.BuildAutomaticWsl(settings.ConnectionSelector),
                    _ => throw new InvalidOperationException("Unsupported connection profile."),
                };
            }
            catch (ArgumentException)
            {
                return false;
            }

            try
            {
                var next = processFactory.Create(startInfo);
                next.Exited += OnChildExited;
                if (!next.Start())
                {
                    next.Dispose();
                    return false;
                }

                child = next;
                return true;
            }
            catch (Win32Exception)
            {
                return false;
            }
            catch (InvalidOperationException)
            {
                return false;
            }
        }
    }

    public void Stop()
    {
        lock (gate)
        {
            if (child is not { } process) return;
            child = null;
            try
            {
                if (!process.HasExited) process.Kill();
            }
            catch (InvalidOperationException)
            {
                // The process exited between HasExited and Kill.
            }
            catch (Win32Exception)
            {
                // The OS has already reaped or denied a non-running child.
            }
            finally
            {
                process.Dispose();
            }
        }
    }

    public void Dispose()
    {
        lock (gate)
        {
            if (disposed) return;
            disposed = true;
        }
        Stop();
    }

    private void OnChildExited(object? sender, EventArgs eventArgs)
    {
        lock (gate)
        {
            if (sender is not IConnectionChildProcess exited || !ReferenceEquals(child, exited)) return;
            child = null;
            exited.Dispose();
        }
    }

    private void ReapChildLocked()
    {
        if (child is not { } process) return;
        child = null;
        try { process.WaitForExit(0); } catch (InvalidOperationException) { }
        process.Dispose();
    }
}
