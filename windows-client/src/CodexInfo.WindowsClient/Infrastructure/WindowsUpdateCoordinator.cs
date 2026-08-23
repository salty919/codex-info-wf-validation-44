// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Diagnostics;
using CodexInfo.WindowsClient.Core;
using CodexInfo.WindowsClient.Updates;

namespace CodexInfo.WindowsClient.Infrastructure;

/// <summary>Starts the ordinary Windows installer UI after validation.</summary>
public interface IWindowsInstallerLauncher
{
    bool TryLaunch(string installerPath);
}

/// <summary>
/// Launches only an existing absolute executable. No unattended installer
/// argument is added: the user remains in control of the standard Setup UI.
/// </summary>
public sealed class WindowsInstallerLauncher : IWindowsInstallerLauncher
{
    public bool TryLaunch(string installerPath)
    {
        if (string.IsNullOrWhiteSpace(installerPath) ||
            !Path.IsPathFullyQualified(installerPath) ||
            !string.Equals(Path.GetExtension(installerPath), ".exe", StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        try
        {
            var file = new FileInfo(installerPath);
            if (!file.Exists || file.Length <= 0 ||
                (file.Attributes & FileAttributes.ReparsePoint) != 0)
            {
                return false;
            }

            using var process = Process.Start(new ProcessStartInfo
            {
                FileName = file.FullName,
                WorkingDirectory = file.DirectoryName!,
                UseShellExecute = true,
            });
            return process is not null;
        }
        catch
        {
            return false;
        }
    }
}

/// <summary>
/// Owns the user-initiated mutation boundary. <see cref="CheckAsync"/> only
/// stores validated metadata; bytes are downloaded and Setup is started only
/// from <see cref="StartAvailableUpdateAsync"/>.
/// </summary>
public sealed class WindowsUpdateCoordinator : IWindowsUpdateCoordinator
{
    private const string InstallerName = "CodexInfo.WindowsClient.Setup.exe";

    private readonly IWindowsUpdateClient client;
    private readonly IWindowsInstallerLauncher launcher;
    private readonly Version currentVersion;
    private readonly string updateRoot;
    private WindowsUpdateRelease? availableRelease;
    private int startInProgress;
    private int disposed;

    public WindowsUpdateCoordinator(
        IWindowsUpdateClient client,
        IWindowsInstallerLauncher launcher,
        Version currentVersion,
        string updateRoot)
    {
        this.client = client ?? throw new ArgumentNullException(nameof(client));
        this.launcher = launcher ?? throw new ArgumentNullException(nameof(launcher));
        this.currentVersion = currentVersion ?? throw new ArgumentNullException(nameof(currentVersion));
        if (string.IsNullOrWhiteSpace(updateRoot) || !Path.IsPathFullyQualified(updateRoot))
        {
            throw new ArgumentException("The update root must be an absolute path.", nameof(updateRoot));
        }

        this.updateRoot = Path.GetFullPath(updateRoot);
    }

    public async Task<UpdateCheckResult> CheckAsync(CancellationToken cancellationToken)
    {
        if (Volatile.Read(ref disposed) != 0)
        {
            return new UpdateCheckResult(null, true);
        }

        availableRelease = null;
        var result = await client.CheckAsync(currentVersion, cancellationToken).ConfigureAwait(false);
        if (!result.IsSuccess)
        {
            return new UpdateCheckResult(null, true);
        }

        availableRelease = result.Release;
        return new UpdateCheckResult(
            result.Release is null ? null : FormatVersion(result.Release.Version),
            false);
    }

    public async Task<UpdateStartStatus> StartAvailableUpdateAsync(CancellationToken cancellationToken)
    {
        if (Volatile.Read(ref disposed) != 0)
        {
            return UpdateStartStatus.DownloadFailed;
        }

        if (Interlocked.CompareExchange(ref startInProgress, 1, 0) != 0)
        {
            return UpdateStartStatus.Busy;
        }

        string? partialPath = null;
        try
        {
            var release = availableRelease;
            if (release is null)
            {
                return UpdateStartStatus.NoAvailableUpdate;
            }

            var versionDirectory = Path.Combine(updateRoot, FormatVersion(release.Version));
            Directory.CreateDirectory(versionDirectory);
            var finalPath = Path.Combine(versionDirectory, InstallerName);
            partialPath = finalPath + ".download";
            DeleteIfPresent(partialPath);

            WindowsUpdateDownloadResult download;
            await using (var destination = new FileStream(
                             partialPath,
                             FileMode.CreateNew,
                             FileAccess.Write,
                             FileShare.None,
                             64 * 1024,
                             FileOptions.Asynchronous | FileOptions.SequentialScan))
            {
                download = await client.DownloadAsync(release, destination, cancellationToken)
                    .ConfigureAwait(false);
                await destination.FlushAsync(cancellationToken).ConfigureAwait(false);
            }

            if (!download.IsSuccess)
            {
                DeleteIfPresent(partialPath);
                return download.Failure == WindowsUpdateFailure.Integrity
                    ? UpdateStartStatus.IntegrityFailed
                    : UpdateStartStatus.DownloadFailed;
            }

            File.Move(partialPath, finalPath, overwrite: true);
            partialPath = null;
            return launcher.TryLaunch(finalPath)
                ? UpdateStartStatus.Started
                : UpdateStartStatus.LaunchFailed;
        }
        catch (OperationCanceledException)
        {
            DeleteIfPresent(partialPath);
            return UpdateStartStatus.DownloadFailed;
        }
        catch
        {
            DeleteIfPresent(partialPath);
            return UpdateStartStatus.DownloadFailed;
        }
        finally
        {
            Volatile.Write(ref startInProgress, 0);
        }
    }

    public void Dispose()
    {
        if (Interlocked.Exchange(ref disposed, 1) != 0)
        {
            return;
        }

        availableRelease = null;
        if (client is IDisposable disposable)
        {
            disposable.Dispose();
        }
    }

    private static string FormatVersion(Version version) =>
        $"{version.Major}.{version.Minor}.{version.Build}";

    private static void DeleteIfPresent(string? path)
    {
        if (path is null)
        {
            return;
        }

        try
        {
            File.Delete(path);
        }
        catch
        {
            // Cleanup is best-effort. A stale CreateNew target still fails
            // closed on the next explicit attempt instead of being executed.
        }
    }
}
