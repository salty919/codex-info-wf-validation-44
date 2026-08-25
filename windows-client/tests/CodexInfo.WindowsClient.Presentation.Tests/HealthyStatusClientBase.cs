// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using CodexInfo.WindowsClient.Core;

namespace CodexInfo.WindowsClient.Presentation.Tests;

/// <summary>
/// Explicit test adapter for presentation-only status fakes. Production code
/// never uses this adapter; every real client must implement the health port.
/// </summary>
internal abstract class HealthyStatusClientBase : ILoopbackStatusClient, ILoopbackHealthClient
{
    public virtual Task<HealthFetchResult> FetchHealthAsync(
        CancellationToken cancellationToken = default) =>
        Task.FromResult(HealthFetchResult.Success(new ApiHealthSnapshot("v1", "codex-info")));

    public abstract Task<StatusFetchResult> FetchAsync(
        CancellationToken cancellationToken = default);
}
