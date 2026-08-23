// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using CodexInfo.WindowsClient.Updates;

namespace CodexInfo.WindowsClient;

/// <summary>
/// Network-free fixture for the conditional update notice. It never downloads
/// or launches anything, even when the visual fixture button is activated.
/// </summary>
internal sealed class PreviewUpdateCoordinator : IWindowsUpdateCoordinator
{
    public Task<UpdateCheckResult> CheckAsync(CancellationToken cancellationToken) =>
        Task.FromResult(new UpdateCheckResult(
            PreviewEnvironment.Scenario == "update" ? "1.1.0" : null,
            false));

    public Task<UpdateStartStatus> StartAvailableUpdateAsync(CancellationToken cancellationToken) =>
        Task.FromResult(UpdateStartStatus.LaunchFailed);

    public void Dispose()
    {
    }
}
