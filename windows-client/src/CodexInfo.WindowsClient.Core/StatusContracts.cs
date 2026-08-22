// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

namespace CodexInfo.WindowsClient.Core;

/// <summary>The state reported by the local status endpoint.</summary>
public enum ApiState
{
    Initializing,
    Ready,
    AuthRequired,
    Error,
}

/// <summary>The deliberately small set of failure details exposed to the UI.</summary>
public enum StatusFetchFailure
{
    Transport,
    Response,
}

/// <summary>A validated quota snapshot.</summary>
public sealed record ApiQuota(
    double RemainingPercent,
    long ResetAt,
    long WindowSeconds,
    bool Monthly);

/// <summary>A validated model token-usage snapshot.</summary>
public sealed record ApiModelUsage(
    string Name,
    ulong InputTokens,
    ulong CachedInputTokens,
    ulong OutputTokens);

/// <summary>A validated immutable snapshot returned by the status endpoint.</summary>
public sealed record ApiStatusSnapshot(
    ApiState State,
    long? ObservedAt,
    bool Authenticated,
    string? PlanLabel,
    ApiQuota? Quota,
    IReadOnlyList<ApiModelUsage> Models,
    ulong ActiveThreadCount);

/// <summary>A status result that never carries transport details or response text.</summary>
public sealed record StatusFetchResult(
    ApiStatusSnapshot? Snapshot,
    StatusFetchFailure? Failure)
{
    public bool IsSuccess => Snapshot is not null && Failure is null;

    public static StatusFetchResult Success(ApiStatusSnapshot snapshot)
    {
        ArgumentNullException.ThrowIfNull(snapshot);
        return new StatusFetchResult(snapshot, null);
    }

    public static StatusFetchResult FromFailure(StatusFetchFailure failure) =>
        new(null, failure);
}
