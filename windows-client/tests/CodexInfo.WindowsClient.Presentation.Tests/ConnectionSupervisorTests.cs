// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.ComponentModel;
using System.Diagnostics;
using CodexInfo.WindowsClient.Settings;
using Xunit;

namespace CodexInfo.WindowsClient.Presentation.Tests;

public sealed class ConnectionSupervisorTests
{
    [Fact]
    public void ValidChildIsSingleOwnedStoppedAndRestartableAfterExit()
    {
        var first = new FakeChildProcess();
        var second = new FakeChildProcess();
        var factory = new FakeChildProcessFactory(first, second);
        using var supervisor = new ConnectionSupervisor(factory);
        var settings = WslSettings();

        Assert.True(supervisor.EnsureStarted(settings));
        Assert.True(supervisor.IsRunning);
        Assert.Single(factory.StartInfos);
        Assert.Equal("wsl.exe", factory.StartInfos[0].FileName);
        Assert.Equal(
            ["--distribution", "Ubuntu-24.04", "--", "codex_info", "--service", "--listen", "127.0.0.1:8787"],
            factory.StartInfos[0].ArgumentList);

        Assert.True(supervisor.EnsureStarted(settings));
        Assert.Single(factory.StartInfos);

        first.SignalExit();
        Assert.False(supervisor.IsRunning);
        Assert.True(first.Disposed);

        Assert.True(supervisor.EnsureStarted(settings));
        Assert.Equal(2, factory.StartInfos.Count);
        supervisor.Stop();
        Assert.True(second.Killed);
        Assert.True(second.Disposed);
        Assert.False(supervisor.IsRunning);
        supervisor.Stop();
    }

    [Fact]
    public void InvalidNoneAndFailedChildrenHaveFiniteFailClosedOutcomes()
    {
        var rejected = new FakeChildProcess { StartResult = false };
        var factory = new FakeChildProcessFactory(rejected);
        using var supervisor = new ConnectionSupervisor(factory);

        Assert.True(supervisor.EnsureStarted(ClientSettings.Default));
        Assert.Empty(factory.StartInfos);
        Assert.False(supervisor.EnsureStarted(new ClientSettings("xx", true)));
        Assert.Empty(factory.StartInfos);

        Assert.False(supervisor.EnsureStarted(WslSettings()));
        Assert.True(rejected.Disposed);
        Assert.False(supervisor.IsRunning);
    }

    [Theory]
    [InlineData("win32")]
    [InlineData("invalid")]
    public void ProcessCreationAndStartExceptionsBecomeConnectionFailure(string failure)
    {
        var factory = new ThrowingChildProcessFactory(failure);
        using var supervisor = new ConnectionSupervisor(factory);

        Assert.False(supervisor.EnsureStarted(WslSettings()));
        Assert.False(supervisor.IsRunning);
    }

    [Fact]
    public void StopReapsChildEvenWhenTheOsRacesKill()
    {
        var child = new FakeChildProcess { KillException = new Win32Exception() };
        using var supervisor = new ConnectionSupervisor(new FakeChildProcessFactory(child));
        Assert.True(supervisor.EnsureStarted(WslSettings()));

        supervisor.Stop();

        Assert.True(child.Disposed);
        Assert.False(supervisor.IsRunning);
    }

    [Fact]
    public void DisposedSupervisorRejectsEveryLaterGeneration()
    {
        var child = new FakeChildProcess();
        var supervisor = new ConnectionSupervisor(new FakeChildProcessFactory(child));
        supervisor.Dispose();
        supervisor.Dispose();

        Assert.False(supervisor.EnsureStarted(WslSettings()));
        Assert.False(child.StartCalled);
    }

    private static ClientSettings WslSettings() => new("ja", true)
    {
        ConnectionConfigured = true,
        ConnectionProfile = ConnectionProfiles.Wsl,
        ConnectionSelector = "Ubuntu-24.04",
    };

    private sealed class FakeChildProcessFactory(params FakeChildProcess[] children) : IConnectionChildProcessFactory
    {
        private int index;

        public List<ProcessStartInfo> StartInfos { get; } = [];

        public IConnectionChildProcess Create(ProcessStartInfo startInfo)
        {
            StartInfos.Add(startInfo);
            return children[Math.Min(index++, children.Length - 1)];
        }
    }

    private sealed class ThrowingChildProcessFactory(string failure) : IConnectionChildProcessFactory
    {
        public IConnectionChildProcess Create(ProcessStartInfo startInfo) => failure switch
        {
            "win32" => throw new Win32Exception(),
            _ => throw new InvalidOperationException(),
        };
    }

    private sealed class FakeChildProcess : IConnectionChildProcess
    {
        public event EventHandler? Exited;

        public bool StartResult { get; init; } = true;
        public Exception? KillException { get; init; }
        public bool StartCalled { get; private set; }
        public bool Killed { get; private set; }
        public bool Disposed { get; private set; }
        public bool HasExited { get; private set; }

        public bool Start()
        {
            StartCalled = true;
            return StartResult;
        }

        public void Kill()
        {
            Killed = true;
            if (KillException is not null) throw KillException;
            HasExited = true;
        }

        public void WaitForExit(int milliseconds)
        {
        }

        public void SignalExit()
        {
            HasExited = true;
            Exited?.Invoke(this, EventArgs.Empty);
        }

        public void Dispose() => Disposed = true;
    }
}
