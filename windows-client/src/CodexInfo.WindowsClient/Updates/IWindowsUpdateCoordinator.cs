// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

namespace CodexInfo.WindowsClient.Updates;

/// <summary>Read-only update discovery and explicitly initiated update start.</summary>
public interface IWindowsUpdateCoordinator : IDisposable
{
    Task<UpdateCheckResult> CheckAsync(CancellationToken cancellationToken = default);

    Task<UpdateStartStatus> StartAvailableUpdateAsync(CancellationToken cancellationToken = default);
}

public sealed record UpdateCheckResult(string? AvailableVersion, bool IsFailure);

public enum UpdateStartStatus
{
    Started,
    Busy,
    NoAvailableUpdate,
    DownloadFailed,
    IntegrityFailed,
    LaunchFailed,
}
