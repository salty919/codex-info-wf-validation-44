// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Security.Cryptography;
using CodexInfo.WindowsClient.Core;
using CodexInfo.WindowsClient.Infrastructure;
using CodexInfo.WindowsClient.Updates;
using Xunit;

namespace CodexInfo.WindowsClient.Presentation.Tests;

public sealed class WindowsUpdateCoordinatorTests
{
    [Fact]
    public async Task CheckOnlyPublishesNoticeAndDoesNotDownloadOrLaunch()
    {
        using var directory = new TemporaryDirectory();
        var release = ReleaseFor([1, 2, 3]);
        var client = new FakeUpdateClient(WindowsUpdateCheckResult.Success(release));
        var launcher = new RecordingLauncher();
        using var coordinator = new WindowsUpdateCoordinator(
            client, launcher, new Version(1, 0, 0), directory.Path);

        var result = await coordinator.CheckAsync(CancellationToken.None);

        Assert.Equal("1.2.3", result.AvailableVersion);
        Assert.False(result.IsFailure);
        Assert.Equal(0, client.DownloadCount);
        Assert.Empty(launcher.Paths);
        Assert.Empty(Directory.EnumerateFileSystemEntries(directory.Path));
    }

    [Fact]
    public async Task ExplicitStartDownloadsVerifiedBytesAndLaunchesOrdinarySetup()
    {
        using var directory = new TemporaryDirectory();
        var payload = new byte[] { 1, 3, 5, 7 };
        var release = ReleaseFor(payload);
        var client = new FakeUpdateClient(
            WindowsUpdateCheckResult.Success(release),
            async (destination, cancellationToken) =>
            {
                await destination.WriteAsync(payload, cancellationToken);
                return WindowsUpdateDownloadResult.Success();
            });
        var launcher = new RecordingLauncher();
        using var coordinator = new WindowsUpdateCoordinator(
            client, launcher, new Version(1, 0, 0), directory.Path);
        await coordinator.CheckAsync(CancellationToken.None);

        var result = await coordinator.StartAvailableUpdateAsync(CancellationToken.None);

        Assert.Equal(UpdateStartStatus.Started, result);
        var path = Assert.Single(launcher.Paths);
        Assert.Equal("CodexInfo.WindowsClient.Setup.exe", System.IO.Path.GetFileName(path));
        Assert.Equal(payload, await File.ReadAllBytesAsync(path));
        Assert.False(File.Exists(path + ".download"));
    }

    [Fact]
    public async Task StartWithoutAvailableReleaseHasNoFilesystemOrLauncherMutation()
    {
        using var directory = new TemporaryDirectory();
        var client = new FakeUpdateClient(WindowsUpdateCheckResult.NoUpdate());
        var launcher = new RecordingLauncher();
        using var coordinator = new WindowsUpdateCoordinator(
            client, launcher, new Version(1, 0, 0), directory.Path);

        var result = await coordinator.StartAvailableUpdateAsync(CancellationToken.None);

        Assert.Equal(UpdateStartStatus.NoAvailableUpdate, result);
        Assert.Equal(0, client.DownloadCount);
        Assert.Empty(launcher.Paths);
        Assert.Empty(Directory.EnumerateFileSystemEntries(directory.Path));
    }

    [Theory]
    [InlineData(WindowsUpdateFailure.Transport, UpdateStartStatus.DownloadFailed)]
    [InlineData(WindowsUpdateFailure.Response, UpdateStartStatus.DownloadFailed)]
    [InlineData(WindowsUpdateFailure.Integrity, UpdateStartStatus.IntegrityFailed)]
    public async Task FailedDownloadDeletesPartialFileAndNeverLaunches(
        WindowsUpdateFailure failure,
        UpdateStartStatus expected)
    {
        using var directory = new TemporaryDirectory();
        var payload = new byte[] { 2, 4, 6 };
        var release = ReleaseFor(payload);
        var client = new FakeUpdateClient(
            WindowsUpdateCheckResult.Success(release),
            async (destination, cancellationToken) =>
            {
                await destination.WriteAsync(payload.AsMemory(0, 1), cancellationToken);
                return WindowsUpdateDownloadResult.FromFailure(failure);
            });
        var launcher = new RecordingLauncher();
        using var coordinator = new WindowsUpdateCoordinator(
            client, launcher, new Version(1, 0, 0), directory.Path);
        await coordinator.CheckAsync(CancellationToken.None);

        var result = await coordinator.StartAvailableUpdateAsync(CancellationToken.None);

        Assert.Equal(expected, result);
        Assert.Empty(launcher.Paths);
        Assert.Empty(Directory.EnumerateFiles(directory.Path, "*.download", SearchOption.AllDirectories));
    }

    [Fact]
    public async Task ConcurrentSecondStartIsDroppedWithoutQueueingAnotherDownload()
    {
        using var directory = new TemporaryDirectory();
        var payload = new byte[] { 9, 8, 7 };
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var releaseDownload = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = ReleaseFor(payload);
        var client = new FakeUpdateClient(
            WindowsUpdateCheckResult.Success(release),
            async (destination, cancellationToken) =>
            {
                entered.SetResult();
                await releaseDownload.Task.WaitAsync(cancellationToken);
                await destination.WriteAsync(payload, cancellationToken);
                return WindowsUpdateDownloadResult.Success();
            });
        var launcher = new RecordingLauncher();
        using var coordinator = new WindowsUpdateCoordinator(
            client, launcher, new Version(1, 0, 0), directory.Path);
        await coordinator.CheckAsync(CancellationToken.None);

        var first = coordinator.StartAvailableUpdateAsync(CancellationToken.None);
        await entered.Task.WaitAsync(TimeSpan.FromSeconds(2));
        var second = await coordinator.StartAvailableUpdateAsync(CancellationToken.None);
        releaseDownload.SetResult();

        Assert.Equal(UpdateStartStatus.Busy, second);
        Assert.Equal(UpdateStartStatus.Started, await first);
        Assert.Equal(1, client.DownloadCount);
        Assert.Single(launcher.Paths);
    }

    [Fact]
    public async Task LaunchFailureIsTypedAndVerifiedInstallerRemainsForExplicitRetry()
    {
        using var directory = new TemporaryDirectory();
        var payload = new byte[] { 4, 2 };
        var release = ReleaseFor(payload);
        var client = new FakeUpdateClient(
            WindowsUpdateCheckResult.Success(release),
            async (destination, cancellationToken) =>
            {
                await destination.WriteAsync(payload, cancellationToken);
                return WindowsUpdateDownloadResult.Success();
            });
        var launcher = new RecordingLauncher { Result = false };
        using var coordinator = new WindowsUpdateCoordinator(
            client, launcher, new Version(1, 0, 0), directory.Path);
        await coordinator.CheckAsync(CancellationToken.None);

        var result = await coordinator.StartAvailableUpdateAsync(CancellationToken.None);

        Assert.Equal(UpdateStartStatus.LaunchFailed, result);
        var path = Assert.Single(launcher.Paths);
        Assert.True(File.Exists(path));
    }

    [Theory]
    [InlineData("relative.exe")]
    [InlineData("")]
    public void SystemLauncherRejectsUntrustedPathWithoutStartingIt(string path)
    {
        var launcher = new WindowsInstallerLauncher();

        Assert.False(launcher.TryLaunch(path));
    }

    private static WindowsUpdateRelease ReleaseFor(byte[] payload)
    {
        var hash = Convert.ToHexString(SHA256.HashData(payload)).ToLowerInvariant();
        return new WindowsUpdateRelease(
            new Version(1, 2, 3),
            new Uri("https://github.com/salty919/codex_info_v2/releases/download/windows-v1.2.3/CodexInfo.WindowsClient.Setup.exe"),
            hash,
            payload.Length);
    }

    private sealed class FakeUpdateClient : IWindowsUpdateClient, IDisposable
    {
        private readonly WindowsUpdateCheckResult checkResult;
        private readonly Func<Stream, CancellationToken, Task<WindowsUpdateDownloadResult>> download;

        public FakeUpdateClient(
            WindowsUpdateCheckResult checkResult,
            Func<Stream, CancellationToken, Task<WindowsUpdateDownloadResult>>? download = null)
        {
            this.checkResult = checkResult;
            this.download = download ?? ((_, _) => Task.FromResult(WindowsUpdateDownloadResult.Success()));
        }

        public int DownloadCount { get; private set; }

        public Task<WindowsUpdateCheckResult> CheckAsync(
            Version current,
            CancellationToken cancellationToken = default) => Task.FromResult(checkResult);

        public Task<WindowsUpdateDownloadResult> DownloadAsync(
            WindowsUpdateRelease release,
            Stream destination,
            CancellationToken cancellationToken = default)
        {
            DownloadCount++;
            return download(destination, cancellationToken);
        }

        public void Dispose()
        {
        }
    }

    private sealed class RecordingLauncher : IWindowsInstallerLauncher
    {
        public List<string> Paths { get; } = [];

        public bool Result { get; init; } = true;

        public bool TryLaunch(string installerPath)
        {
            Paths.Add(installerPath);
            return Result;
        }
    }

    private sealed class TemporaryDirectory : IDisposable
    {
        public TemporaryDirectory()
        {
            Path = System.IO.Path.Combine(
                System.IO.Path.GetTempPath(),
                "codex-info-update-tests-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(Path);
        }

        public string Path { get; }

        public void Dispose()
        {
            if (Directory.Exists(Path))
            {
                Directory.Delete(Path, recursive: true);
            }
        }
    }
}
