// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.ComponentModel;
using System.Diagnostics;
using CodexInfo.WindowsClient.Infrastructure;

namespace CodexInfo.WindowsClient.Settings;

/// <summary>The finite outcomes of one user-explicit connection restart.</summary>
public enum ConnectionRestartOutcome
{
    Started,
    NoChildRequired,
    InvalidSettings,
    StartFailed,
    Disposed,
}

/// <summary>
/// The narrow supervisor boundary consumed by the presentation state owner.
/// It is public so deterministic presentation fakes can observe the same
/// startup and explicit-operation contract as the production supervisor.
/// </summary>
public interface IConnectionSupervisor : IDisposable
{
    bool EnsureStarted(ClientSettings settings);

    ConnectionRestartOutcome RestartExplicit(ClientSettings settings);
}

/// <summary>
/// Owns the single automatic bootstrap/tunnel child for one client
/// generation. It deliberately performs no retry loop: a failed child is
/// reaped and the UI remains disconnected until an explicit refresh/setup
/// action starts a new generation.
/// </summary>
public sealed class ConnectionSupervisor : IConnectionSupervisor
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

            return TryStartChildLocked(settings) == ConnectionRestartOutcome.Started;
        }
    }

    /// <summary>
    /// Detaches and disposes the current child before attempting at most one
    /// new child.  This is the only forced restart boundary; callbacks and
    /// polling never call it implicitly.
    /// </summary>
    public ConnectionRestartOutcome RestartExplicit(ClientSettings settings)
    {
        lock (gate)
        {
            if (disposed) return ConnectionRestartOutcome.Disposed;

            var previous = child;
            child = null;
            if (previous is not null)
            {
                StopAndDispose(previous);
            }

            if (!ConnectionSelectors.IsValid(settings))
            {
                return ConnectionRestartOutcome.InvalidSettings;
            }

            if (settings.ConnectionProfile is ConnectionProfiles.None)
            {
                return ConnectionRestartOutcome.NoChildRequired;
            }

            return TryStartChildLocked(settings);
        }
    }

    public void Stop()
    {
        lock (gate)
        {
            if (child is not { } process) return;
            child = null;
            StopAndDispose(process);
        }
    }

    public void Dispose()
    {
        IConnectionChildProcess? process;
        lock (gate)
        {
            if (disposed) return;
            disposed = true;
            process = child;
            child = null;
        }
        if (process is not null)
        {
            StopAndDispose(process);
        }
    }

    private void OnChildExited(object? sender, EventArgs eventArgs)
    {
        IConnectionChildProcess? exited = null;
        lock (gate)
        {
            if (sender is not IConnectionChildProcess candidate || !ReferenceEquals(child, candidate)) return;
            child = null;
            exited = candidate;
        }
        if (exited is not null)
        {
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

    private ConnectionRestartOutcome TryStartChildLocked(ClientSettings settings)
    {
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
            return ConnectionRestartOutcome.InvalidSettings;
        }

        IConnectionChildProcess? next = null;
        try
        {
            next = processFactory.Create(startInfo);
            if (next is null)
            {
                return ConnectionRestartOutcome.StartFailed;
            }
            next.Exited += OnChildExited;
            if (!next.Start())
            {
                next.Exited -= OnChildExited;
                next.Dispose();
                return ConnectionRestartOutcome.StartFailed;
            }

            child = next;
            return ConnectionRestartOutcome.Started;
        }
        catch (Win32Exception)
        {
            if (next is not null)
            {
                next.Exited -= OnChildExited;
                next.Dispose();
            }
            return ConnectionRestartOutcome.StartFailed;
        }
        catch (InvalidOperationException)
        {
            if (next is not null)
            {
                next.Exited -= OnChildExited;
                next.Dispose();
            }
            return ConnectionRestartOutcome.StartFailed;
        }
        catch (Exception)
        {
            if (next is not null)
            {
                next.Exited -= OnChildExited;
                next.Dispose();
            }
            return ConnectionRestartOutcome.StartFailed;
        }
    }

    private static void StopAndDispose(IConnectionChildProcess process)
    {
        try
        {
            if (!process.HasExited)
            {
                process.Kill();
                process.WaitForExit(0);
            }
        }
        catch (InvalidOperationException)
        {
            // The process exited between HasExited and Kill/WaitForExit.
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
