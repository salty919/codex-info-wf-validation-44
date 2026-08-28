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

/// <summary>
/// The opaque identity published by the daemon for one immutable status/details
/// pair.  Its wire representation is retained only for equality and diagnostics;
/// the client never interprets any portion of the value.
/// </summary>
public readonly record struct PublishedPairIdentity
{
    private const int WireLength = 67;
    private const string Prefix = "v1:";
    private readonly string? _value;

    private PublishedPairIdentity(string value)
    {
        _value = value;
    }

    internal static bool TryCreate(string value, out PublishedPairIdentity identity)
    {
        identity = default;
        if (value.Length != WireLength || !value.StartsWith(Prefix, StringComparison.Ordinal))
        {
            return false;
        }

        for (var index = Prefix.Length; index < value.Length; index++)
        {
            var character = value[index];
            if (!IsLowerHex(character))
            {
                return false;
            }
        }

        identity = new PublishedPairIdentity(value);
        return true;
    }

    /// <summary>
    /// Creates an identity through the same canonical validation used by the
    /// loopback wire parser.  This is intended for trusted in-process
    /// fixtures/adapters; untrusted response values still enter through the
    /// parser's fail-closed boundary.
    /// </summary>
    public static PublishedPairIdentity Create(string value)
    {
        ArgumentNullException.ThrowIfNull(value);
        return TryCreate(value, out var identity)
            ? identity
            : throw new ArgumentException("The published-pair identity is not canonical.", nameof(value));
    }

    public override string ToString() => _value ?? string.Empty;

    private static bool IsLowerHex(char character) =>
        character is >= '0' and <= '9' or >= 'a' and <= 'f';
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
    ulong ActiveThreadCount)
{
    /// <summary>The opaque identity of the accepted status response pair.</summary>
    public PublishedPairIdentity? PublishedPair { get; init; }
}

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
