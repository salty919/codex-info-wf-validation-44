// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Net;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;

namespace CodexInfo.WindowsClient.Core;

/// <summary>Fetches and strictly validates the loopback v1 status document.</summary>
public interface ILoopbackStatusClient
{
    Task<StatusFetchResult> FetchAsync(CancellationToken cancellationToken = default);
}

/// <remarks>
/// The endpoint and all network policy are fixed here so callers cannot accidentally
/// turn the client into a general-purpose HTTP client.  A handler constructor is
/// provided solely to make the transport boundary testable.
/// </remarks>
public sealed class LoopbackStatusClient : ILoopbackStatusClient, ILoopbackHealthClient, ILoopbackDetailsClient, IDisposable
{
    private const string Endpoint = "http://127.0.0.1:8787/v1/status";
    private const string DetailsEndpoint = "http://127.0.0.1:8787/v1/details";
    private const string HealthEndpoint = "http://127.0.0.1:8787/v1/health";
    private const int MaxResponseHeaderBytes = 8 * 1024;
    private const int MaxBodyBytes = 64 * 1024;
    private const int MaxHealthBodyBytes = 1024;
    // SQLite retains three months, but one details response is bounded to one
    // 31-day month of minute buckets. The byte envelope is independent.
    private const int MaxDetailsBodyBytes = 32 * 1024 * 1024;
    private const long MaxUnixSeconds = 253_402_300_799;
    private const int MaxHistoryPeriods = 128;
    private const int MaxHistorySamples = 31 * 24 * 60;
    private const int MaxThreads = 256;
    private const long ResetAtToleranceSeconds = 60;

    private static readonly HashSet<string> TopLevelProperties = CreatePropertySet(
        "api_version",
        "state",
        "observed_at",
        "authenticated",
        "plan_label",
        "quota",
        "models",
        "active_thread_count");

    private static readonly HashSet<string> HealthProperties = CreatePropertySet(
        "api_version",
        "service");

    private static readonly HashSet<string> QuotaProperties = CreatePropertySet(
        "remaining_percent",
        "reset_at",
        "window_seconds",
        "monthly");

    private static readonly HashSet<string> ModelProperties = CreatePropertySet(
        "name",
        "input_tokens",
        "cached_input_tokens",
        "output_tokens");

    private static readonly HashSet<string> DetailsTopLevelProperties = CreatePropertySet(
        "api_version",
        "state",
        "observed_at",
        "authenticated",
        "plan_label",
        "quota",
        "models",
        "active_thread_count",
        "history_periods",
        "history_samples",
        "threads",
        "estimated_cost_label");

    private static readonly HashSet<string> DetailsModelProperties = CreatePropertySet(
        "name",
        "input_tokens",
        "cached_input_tokens",
        "output_tokens",
        "input_dollars",
        "cached_input_dollars",
        "output_dollars");

    private static readonly HashSet<string> HistoryPeriodProperties = CreatePropertySet(
        "id",
        "start_at",
        "end_at",
        "reset_at",
        "label",
        "current");

    private static readonly HashSet<string> HistorySampleProperties = CreatePropertySet(
        "timestamp",
        "reset_at",
        "remaining_percent",
        "sol_dollars",
        "terra_dollars",
        "luna_dollars",
        "sol_tokens",
        "terra_tokens",
        "luna_tokens");

    private static readonly HashSet<string> ThreadProperties = CreatePropertySet(
        "id",
        "title",
        "parent_thread_id",
        "model",
        "model_label",
        "total_tokens",
        "context_usage_tokens",
        "context_window_tokens",
        "created_at",
        "last_user_message_at",
        "is_subagent",
        "depth");

    private readonly HttpClient _httpClient;

    public LoopbackStatusClient()
        : this(CreateDefaultHandler())
    {
    }

    public LoopbackStatusClient(HttpMessageHandler handler)
    {
        ArgumentNullException.ThrowIfNull(handler);
        if (handler is HttpClientHandler httpClientHandler)
        {
            // Keep the policy true even when a caller supplies an
            // HttpClientHandler as a test seam.
            httpClientHandler.UseProxy = false;
            httpClientHandler.AllowAutoRedirect = false;
            httpClientHandler.AutomaticDecompression = DecompressionMethods.None;
            httpClientHandler.UseCookies = false;
            httpClientHandler.MaxResponseHeadersLength = MaxResponseHeaderBytes / 1024;
        }

        _httpClient = new HttpClient(handler, disposeHandler: true)
        {
            Timeout = TimeSpan.FromSeconds(1),
        };
    }

    public async Task<StatusFetchResult> FetchAsync(CancellationToken cancellationToken = default)
    {
        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, Endpoint);
            request.Headers.Accept.Clear();
            request.Headers.Accept.Add(new MediaTypeWithQualityHeaderValue("application/json"));

            using var response = await _httpClient.SendAsync(
                    request,
                    HttpCompletionOption.ResponseHeadersRead,
                    cancellationToken)
                .ConfigureAwait(false);

            // HTTP status failures are transport failures by the public contract;
            // no response body is ever exposed to the caller.
            if (response.StatusCode != HttpStatusCode.OK)
            {
                return Failure(StatusFetchFailure.Transport);
            }

            if (!HasAcceptableHeaderSize(response))
            {
                return Failure(StatusFetchFailure.Response);
            }

            if (!HasRequiredResponseHeaders(response))
            {
                return Failure(StatusFetchFailure.Response);
            }

            if (!TryGetContentLength(response.Content, out var contentLength))
            {
                return Failure(StatusFetchFailure.Response);
            }

            if (contentLength is > MaxBodyBytes)
            {
                return Failure(StatusFetchFailure.Response);
            }

            var bodyStatus = await ReadBodyAsync(
                    response.Content,
                    contentLength,
                    cancellationToken)
                .ConfigureAwait(false);

            if (bodyStatus.Kind is BodyReadKind.Oversize)
            {
                return Failure(StatusFetchFailure.Response);
            }

            if (bodyStatus.Kind is BodyReadKind.Transport || bodyStatus.Body is null)
            {
                return Failure(StatusFetchFailure.Transport);
            }

            if (!TryParseSnapshot(bodyStatus.Body, out var snapshot) || snapshot is null)
            {
                return Failure(StatusFetchFailure.Response);
            }

            return StatusFetchResult.Success(snapshot);
        }
        catch (OperationCanceledException)
        {
            return Failure(StatusFetchFailure.Transport);
        }
        catch (HttpRequestException)
        {
            return Failure(StatusFetchFailure.Transport);
        }
        catch (IOException)
        {
            return Failure(StatusFetchFailure.Transport);
        }
        catch (Exception)
        {
            // This is intentionally generic.  No exception object or message can
            // cross the boundary, including failures from a custom test handler.
            return Failure(StatusFetchFailure.Transport);
        }
    }

    public async Task<HealthFetchResult> FetchHealthAsync(
        CancellationToken cancellationToken = default)
    {
        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, HealthEndpoint);
            request.Headers.Accept.Clear();
            request.Headers.Accept.Add(new MediaTypeWithQualityHeaderValue("application/json"));

            using var response = await _httpClient.SendAsync(
                    request,
                    HttpCompletionOption.ResponseHeadersRead,
                    cancellationToken)
                .ConfigureAwait(false);

            if (response.StatusCode != HttpStatusCode.OK)
            {
                return HealthFetchResult.FromFailure(HealthFetchFailure.Response);
            }

            if (!HasAcceptableHeaderSize(response) ||
                !HasRequiredResponseHeaders(response) ||
                !TryGetContentLength(response.Content, out var contentLength) ||
                contentLength is not long declaredLength)
            {
                return HealthFetchResult.FromFailure(HealthFetchFailure.Response);
            }

            if (declaredLength > MaxHealthBodyBytes)
            {
                return HealthFetchResult.FromFailure(HealthFetchFailure.Response);
            }

            var bodyStatus = await ReadBodyAsync(
                    response.Content,
                    declaredLength,
                    cancellationToken,
                    MaxHealthBodyBytes)
                .ConfigureAwait(false);

            if (bodyStatus.Kind is BodyReadKind.Oversize)
            {
                return HealthFetchResult.FromFailure(HealthFetchFailure.Response);
            }

            if (bodyStatus.Kind is BodyReadKind.Transport || bodyStatus.Body is null)
            {
                return HealthFetchResult.FromFailure(HealthFetchFailure.Transport);
            }

            if (bodyStatus.Body.LongLength != declaredLength)
            {
                return HealthFetchResult.FromFailure(HealthFetchFailure.Response);
            }

            if (!TryParseHealth(bodyStatus.Body, out var snapshot) || snapshot is null)
            {
                return HealthFetchResult.FromFailure(HealthFetchFailure.Response);
            }

            return HealthFetchResult.Success(snapshot);
        }
        catch (OperationCanceledException)
        {
            return HealthFetchResult.FromFailure(HealthFetchFailure.Transport);
        }
        catch (HttpRequestException)
        {
            return HealthFetchResult.FromFailure(HealthFetchFailure.Transport);
        }
        catch (IOException)
        {
            return HealthFetchResult.FromFailure(HealthFetchFailure.Transport);
        }
        catch (Exception)
        {
            return HealthFetchResult.FromFailure(HealthFetchFailure.Transport);
        }
    }

    /// <summary>
    /// Reads the independent details document.  This deliberately has a
    /// separate public result type so callers cannot accidentally turn a
    /// details failure into a status failure.
    /// </summary>
    public async Task<DetailsFetchResult> FetchDetailsAsync(
        CancellationToken cancellationToken = default)
    {
        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, DetailsEndpoint);
            request.Headers.Accept.Clear();
            request.Headers.Accept.Add(new MediaTypeWithQualityHeaderValue("application/json"));

            using var response = await _httpClient.SendAsync(
                    request,
                    HttpCompletionOption.ResponseHeadersRead,
                    cancellationToken)
                .ConfigureAwait(false);

            if (response.StatusCode != HttpStatusCode.OK)
            {
                return DetailsFailure(DetailsFetchFailure.Transport);
            }

            if (!HasAcceptableHeaderSize(response) ||
                !HasRequiredResponseHeaders(response) ||
                !TryGetContentLength(response.Content, out var contentLength))
            {
                return DetailsFailure(DetailsFetchFailure.Response);
            }

            if (contentLength is > MaxDetailsBodyBytes)
            {
                return DetailsFailure(DetailsFetchFailure.Response);
            }

            var bodyStatus = await ReadBodyAsync(
                    response.Content,
                    contentLength,
                    cancellationToken,
                    MaxDetailsBodyBytes)
                .ConfigureAwait(false);

            if (bodyStatus.Kind is BodyReadKind.Oversize)
            {
                return DetailsFailure(DetailsFetchFailure.Response);
            }

            if (bodyStatus.Kind is BodyReadKind.Transport || bodyStatus.Body is null)
            {
                return DetailsFailure(DetailsFetchFailure.Transport);
            }

            if (!TryParseDetails(bodyStatus.Body, out var snapshot) || snapshot is null)
            {
                return DetailsFailure(DetailsFetchFailure.Response);
            }

            return DetailsFetchResult.Success(snapshot);
        }
        catch (OperationCanceledException)
        {
            return DetailsFailure(DetailsFetchFailure.Transport);
        }
        catch (HttpRequestException)
        {
            return DetailsFailure(DetailsFetchFailure.Transport);
        }
        catch (IOException)
        {
            return DetailsFailure(DetailsFetchFailure.Transport);
        }
        catch (Exception)
        {
            return DetailsFailure(DetailsFetchFailure.Transport);
        }
    }

    public void Dispose() => _httpClient.Dispose();

    private static HttpMessageHandler CreateDefaultHandler() => new HttpClientHandler
    {
        UseProxy = false,
        AllowAutoRedirect = false,
        AutomaticDecompression = DecompressionMethods.None,
        UseCookies = false,
        MaxResponseHeadersLength = MaxResponseHeaderBytes / 1024,
    };

    private static HashSet<string> CreatePropertySet(params string[] properties) =>
        new(properties, StringComparer.Ordinal);

    private static StatusFetchResult Failure(StatusFetchFailure failure) =>
        StatusFetchResult.FromFailure(failure);

    private static DetailsFetchResult DetailsFailure(DetailsFetchFailure failure) =>
        DetailsFetchResult.FromFailure(failure);

    private static bool HasAcceptableHeaderSize(HttpResponseMessage response)
    {
        try
        {
            long bytes = 2; // final CRLF
            bytes = CountHeaders(response.Headers, bytes);
            bytes = CountHeaders(response.Content?.Headers, bytes);
            return bytes <= MaxResponseHeaderBytes;
        }
        catch (Exception)
        {
            return false;
        }
    }

    private static long CountHeaders(HttpContentHeaders? headers, long current)
    {
        if (headers is null)
        {
            return current;
        }

        return CountHeaders((IEnumerable<KeyValuePair<string, IEnumerable<string>>>)headers, current);
    }

    private static long CountHeaders(HttpResponseHeaders headers, long current) =>
        CountHeaders((IEnumerable<KeyValuePair<string, IEnumerable<string>>>)headers, current);

    private static long CountHeaders(
        IEnumerable<KeyValuePair<string, IEnumerable<string>>> headers,
        long current)
    {
        var bytes = current;
        foreach (var header in headers)
        {
            var values = header.Value.ToArray();
            if (values.Length == 0)
            {
                bytes = checked(bytes + Encoding.UTF8.GetByteCount(header.Key) + 4);
                continue;
            }

            foreach (var value in values)
            {
                // Each value is counted as an independent wire header field;
                // this is conservative for coalesced multi-value headers.
                bytes = checked(
                    bytes +
                    Encoding.UTF8.GetByteCount(header.Key) +
                    2 +
                    Encoding.UTF8.GetByteCount(value) +
                    2);
            }
        }

        return bytes;
    }

    private static bool HasRequiredResponseHeaders(HttpResponseMessage response)
    {
        try
        {
            var contentType = response.Content?.Headers.ContentType;
            var mediaType = contentType?.MediaType;
            var charset = contentType?.CharSet?.Trim('"');
            return mediaType is not null &&
                   mediaType.Equals("application/json", StringComparison.OrdinalIgnoreCase) &&
                   charset is not null &&
                   charset.Equals("utf-8", StringComparison.OrdinalIgnoreCase) &&
                   response.Headers.CacheControl?.NoStore == true;
        }
        catch (Exception)
        {
            return false;
        }
    }

    private static bool TryGetContentLength(HttpContent content, out long? contentLength)
    {
        try
        {
            contentLength = content.Headers.ContentLength;
            return contentLength is null or >= 0;
        }
        catch (Exception)
        {
            contentLength = null;
            return false;
        }
    }

    private static async Task<BodyReadResult> ReadBodyAsync(
        HttpContent content,
        long? contentLength,
        CancellationToken cancellationToken,
        int maximumBodyBytes = MaxBodyBytes)
    {
        try
        {
            await using var stream = await content.ReadAsStreamAsync(cancellationToken).ConfigureAwait(false);
            using var body = new MemoryStream(
                capacity: contentLength is long length && length >= 0 && length <= maximumBodyBytes
                    ? (int)length
                    : maximumBodyBytes);
            var buffer = new byte[8 * 1024];
            var count = 0;

            while (true)
            {
                var read = await stream.ReadAsync(buffer.AsMemory(), cancellationToken).ConfigureAwait(false);
                if (read == 0)
                {
                    return BodyReadResult.Success(body.ToArray());
                }

                if (read > maximumBodyBytes - count)
                {
                    return BodyReadResult.Oversize();
                }

                body.Write(buffer, 0, read);
                count += read;
            }
        }
        catch (OperationCanceledException)
        {
            return BodyReadResult.Transport();
        }
        catch (HttpRequestException)
        {
            return BodyReadResult.Transport();
        }
        catch (IOException)
        {
            return BodyReadResult.Transport();
        }
        catch (Exception)
        {
            return BodyReadResult.Transport();
        }
    }

    private static bool TryParseSnapshot(byte[] body, out ApiStatusSnapshot? snapshot)
    {
        snapshot = null;

        try
        {
            using var document = JsonDocument.Parse(
                body,
                new JsonDocumentOptions
                {
                    AllowTrailingCommas = false,
                    CommentHandling = JsonCommentHandling.Disallow,
                    MaxDepth = 16,
                });

            var root = document.RootElement;
            if (!HasExactlyProperties(root, TopLevelProperties, 8))
            {
                return false;
            }

            if (!TryGetString(root, "api_version", out var apiVersion) || apiVersion != "v1")
            {
                return false;
            }

            if (!TryGetState(root, out var state))
            {
                return false;
            }

            if (!TryGetNullableUnixSeconds(root, "observed_at", out var observedAt))
            {
                return false;
            }

            if (!TryGetBoolean(root, "authenticated", out var authenticated))
            {
                return false;
            }

            if (!TryGetNullablePlanLabel(root, out var planLabel))
            {
                return false;
            }

            if (!TryGetQuota(root, out var quota))
            {
                return false;
            }

            if (!TryGetModels(root, out var models))
            {
                return false;
            }

            if (!TryGetUInt64(root, "active_thread_count", out var activeThreadCount))
            {
                return false;
            }

            snapshot = new ApiStatusSnapshot(
                state,
                observedAt,
                authenticated,
                planLabel,
                quota,
                new System.Collections.ObjectModel.ReadOnlyCollection<ApiModelUsage>(models),
                activeThreadCount);
            return true;
        }
        catch (JsonException)
        {
            return false;
        }
        catch (ArgumentException)
        {
            return false;
        }
        catch (InvalidOperationException)
        {
            return false;
        }
        catch (Exception)
        {
            return false;
        }
    }

    private static bool TryParseHealth(byte[] body, out ApiHealthSnapshot? snapshot)
    {
        snapshot = null;

        try
        {
            using var document = JsonDocument.Parse(
                body,
                new JsonDocumentOptions
                {
                    AllowTrailingCommas = false,
                    CommentHandling = JsonCommentHandling.Disallow,
                    MaxDepth = 4,
                });

            var root = document.RootElement;
            if (!HasExactlyProperties(root, HealthProperties, 2) ||
                !TryGetString(root, "api_version", out var apiVersion) ||
                apiVersion != "v1" ||
                !TryGetBoundedString(root, "service", 1, 64, out var service) ||
                service != "codex-info")
            {
                return false;
            }

            snapshot = new ApiHealthSnapshot(apiVersion, service);
            return true;
        }
        catch (JsonException)
        {
            return false;
        }
        catch (ArgumentException)
        {
            return false;
        }
        catch (InvalidOperationException)
        {
            return false;
        }
        catch (Exception)
        {
            return false;
        }
    }

    private static bool TryParseDetails(byte[] body, out ApiDetailsSnapshot? snapshot)
    {
        snapshot = null;

        try
        {
            using var document = JsonDocument.Parse(
                body,
                new JsonDocumentOptions
                {
                    AllowTrailingCommas = false,
                    CommentHandling = JsonCommentHandling.Disallow,
                    MaxDepth = 24,
                });

            var root = document.RootElement;
            if (!HasExactlyProperties(root, DetailsTopLevelProperties, 12) ||
                !TryGetString(root, "api_version", out var apiVersion) ||
                apiVersion != "v1" ||
                !TryGetState(root, out var state) ||
                !TryGetNullableUnixSeconds(root, "observed_at", out var observedAt) ||
                !TryGetBoolean(root, "authenticated", out var authenticated) ||
                !TryGetNullablePlanLabel(root, out var planLabel) ||
                !TryGetQuota(root, out var quota) ||
                !TryGetDetailsModels(root, out var models) ||
                !TryGetUInt64(root, "active_thread_count", out var activeThreadCount) ||
                !TryGetHistoryPeriods(root, out var historyPeriods) ||
                !TryGetFlatHistorySamples(root, out var historySamples) ||
                !TryGetThreads(root, out var threads) ||
                !TryGetBoundedString(root, "estimated_cost_label", 1, 160, out var estimatedCostLabel))
            {
                return false;
            }

            historyPeriods = historyPeriods
                .Select(period => period with
                {
                    Samples = SamplesForCanonicalPeriod(period, historySamples),
                })
                .ToList();

            snapshot = new ApiDetailsSnapshot(
                state,
                observedAt,
                authenticated,
                planLabel,
                quota,
                new System.Collections.ObjectModel.ReadOnlyCollection<ApiDetailsModelUsage>(models),
                activeThreadCount,
                new System.Collections.ObjectModel.ReadOnlyCollection<ApiHistoryPeriod>(historyPeriods),
                new System.Collections.ObjectModel.ReadOnlyCollection<ApiHistorySample>(historySamples),
                new System.Collections.ObjectModel.ReadOnlyCollection<ApiThreadDetails>(threads),
                estimatedCostLabel);
            return true;
        }
        catch (JsonException)
        {
            return false;
        }
        catch (ArgumentException)
        {
            return false;
        }
        catch (InvalidOperationException)
        {
            return false;
        }
        catch (Exception)
        {
            return false;
        }
    }

    private static IReadOnlyList<ApiHistorySample> SamplesForCanonicalPeriod(
        ApiHistoryPeriod period,
        IReadOnlyList<ApiHistorySample> samples)
    {
        var minimumResetAt = period.ResetAt - ResetAtToleranceSeconds;
        var selected = samples
            .Where(sample => sample.ResetAt >= minimumResetAt && sample.ResetAt <= period.ResetAt)
            .OrderBy(sample => sample.Timestamp)
            .ThenBy(sample => sample.ResetAt);
        var merged = new List<ApiHistorySample>();
        foreach (var group in selected.GroupBy(sample => sample.Timestamp))
        {
            var rows = group.ToList();
            var canonical = rows[^1] with { ResetAt = period.ResetAt };
            var remainingValues = rows
                .Where(sample => sample.RemainingPercent is { } value && double.IsFinite(value))
                .Select(sample => sample.RemainingPercent!.Value)
                .ToArray();
            // Different reset IDs can legitimately be aliases of one
            // canonical period, but two different quota values at the same
            // observation timestamp are ambiguous.  Choosing the last row
            // makes row order manufacture a vertical 88% -> 14% drop.  Keep
            // the model maxima, but fail closed for the conflicting quota
            // field so the graph cannot invent a loss of remaining credit.
            var mergedRemaining = remainingValues.Length == 0
                ? canonical.RemainingPercent
                : remainingValues.All(value => Math.Abs(value - remainingValues[0]) <= double.Epsilon)
                    ? remainingValues[0]
                    : null;
            merged.Add(canonical with
            {
                RemainingPercent = mergedRemaining,
                SolDollars = rows.Max(sample => sample.SolDollars),
                TerraDollars = rows.Max(sample => sample.TerraDollars),
                LunaDollars = rows.Max(sample => sample.LunaDollars),
                SolTokens = rows.Max(sample => sample.SolTokens),
                TerraTokens = rows.Max(sample => sample.TerraTokens),
                LunaTokens = rows.Max(sample => sample.LunaTokens),
            });
        }

        return new System.Collections.ObjectModel.ReadOnlyCollection<ApiHistorySample>(merged);
    }

    private static bool TryGetDetailsModels(
        JsonElement parent,
        out List<ApiDetailsModelUsage> models)
    {
        models = new List<ApiDetailsModelUsage>();
        if (!parent.TryGetProperty("models", out var property) ||
            property.ValueKind != JsonValueKind.Array ||
            property.GetArrayLength() > 3)
        {
            return false;
        }

        var names = new HashSet<string>(StringComparer.Ordinal);
        foreach (var model in property.EnumerateArray())
        {
            if (!HasExactlyProperties(model, DetailsModelProperties, 7) ||
                !TryGetString(model, "name", out var name) ||
                !names.Add(name) ||
                !IsSupportedModel(name) ||
                !TryGetUInt64(model, "input_tokens", out var inputTokens) ||
                !TryGetUInt64(model, "cached_input_tokens", out var cachedInputTokens) ||
                !TryGetUInt64(model, "output_tokens", out var outputTokens) ||
                !TryGetNonNegativeFiniteDouble(model, "input_dollars", out var inputDollars) ||
                !TryGetNonNegativeFiniteDouble(model, "cached_input_dollars", out var cachedInputDollars) ||
                !TryGetNonNegativeFiniteDouble(model, "output_dollars", out var outputDollars))
            {
                return false;
            }

            models.Add(new ApiDetailsModelUsage(
                name,
                inputTokens,
                cachedInputTokens,
                outputTokens,
                inputDollars,
                cachedInputDollars,
                outputDollars));
        }

        return true;
    }

    private static bool TryGetHistoryPeriods(
        JsonElement parent,
        out List<ApiHistoryPeriod> periods)
    {
        periods = new List<ApiHistoryPeriod>();
        if (!parent.TryGetProperty("history_periods", out var property) ||
            property.ValueKind != JsonValueKind.Array ||
            property.GetArrayLength() > MaxHistoryPeriods)
        {
            return false;
        }

        var periodIds = new HashSet<string>(StringComparer.Ordinal);
        foreach (var period in property.EnumerateArray())
        {
            if (!HasExactlyProperties(period, HistoryPeriodProperties, 6) ||
                !TryGetBoundedString(period, "id", 1, 512, out var id) ||
                !periodIds.Add(id) ||
                !TryGetUnixSeconds(period, "start_at", out var startAt) ||
                !TryGetUnixSeconds(period, "end_at", out var endAt) ||
                !TryGetUnixSeconds(period, "reset_at", out var resetAt) ||
                endAt < startAt ||
                resetAt < endAt ||
                !TryGetBoundedString(period, "label", 1, 512, out var label) ||
                !TryGetBoolean(period, "current", out var current))
            {
                return false;
            }

            periods.Add(new ApiHistoryPeriod(
                id,
                startAt,
                endAt,
                current,
                label)
            {
                ResetAt = resetAt,
            });
        }

        return true;
    }

    private static bool TryGetFlatHistorySamples(
        JsonElement parent,
        out List<ApiHistorySample> samples)
    {
        samples = new List<ApiHistorySample>();
        if (!parent.TryGetProperty("history_samples", out var property) ||
            property.ValueKind != JsonValueKind.Array ||
            property.GetArrayLength() > MaxHistorySamples)
        {
            return false;
        }

        foreach (var sample in property.EnumerateArray())
        {
            if (!HasExactlyProperties(sample, HistorySampleProperties, 9) ||
                !TryGetUnixSeconds(sample, "timestamp", out var timestamp) ||
                !TryGetUnixSeconds(sample, "reset_at", out var resetAt) ||
                !TryGetNullableRemainingPercent(sample, out var remainingPercent) ||
                !TryGetNonNegativeFiniteDouble(sample, "sol_dollars", out var solDollars) ||
                !TryGetNonNegativeFiniteDouble(sample, "terra_dollars", out var terraDollars) ||
                !TryGetNonNegativeFiniteDouble(sample, "luna_dollars", out var lunaDollars) ||
                !TryGetUInt64(sample, "sol_tokens", out var solTokens) ||
                !TryGetUInt64(sample, "terra_tokens", out var terraTokens) ||
                !TryGetUInt64(sample, "luna_tokens", out var lunaTokens))
            {
                return false;
            }

            samples.Add(new ApiHistorySample(
                timestamp,
                resetAt,
                remainingPercent,
                solDollars,
                terraDollars,
                lunaDollars,
                solTokens,
                terraTokens,
                lunaTokens));
        }

        // The recorder can publish two rows for one (reset_at,timestamp)
        // identity when separate model observations land in the same minute.
        // Rejecting the identity as a duplicate makes an otherwise reachable
        // server appear disconnected on Windows. Coalesce only this bounded
        // array-record collision: model values use maxima, while conflicting
        // remaining values become unavailable instead of being chosen by row
        // order. Duplicate JSON keys inside one object remain rejected above.
        samples = samples
            .GroupBy(sample => (sample.ResetAt, sample.Timestamp))
            .Select(group => MergeHistorySampleIdentity(group))
            .OrderBy(sample => sample.Timestamp)
            .ThenBy(sample => sample.ResetAt)
            .ToList();

        return true;
    }

    private static ApiHistorySample MergeHistorySampleIdentity(
        IEnumerable<ApiHistorySample> source)
    {
        var rows = source.ToList();
        var canonical = rows[^1];
        var remainingValues = rows
            .Where(sample => sample.RemainingPercent is { } value && double.IsFinite(value))
            .Select(sample => sample.RemainingPercent!.Value)
            .ToArray();
        var remaining = remainingValues.Length == 0
            ? canonical.RemainingPercent
            : remainingValues.All(value => Math.Abs(value - remainingValues[0]) <= double.Epsilon)
                ? remainingValues[0]
                : null;
        return canonical with
        {
            RemainingPercent = remaining,
            SolDollars = rows.Max(sample => sample.SolDollars),
            TerraDollars = rows.Max(sample => sample.TerraDollars),
            LunaDollars = rows.Max(sample => sample.LunaDollars),
            SolTokens = rows.Max(sample => sample.SolTokens),
            TerraTokens = rows.Max(sample => sample.TerraTokens),
            LunaTokens = rows.Max(sample => sample.LunaTokens),
        };
    }

    private static bool TryGetThreads(
        JsonElement parent,
        out List<ApiThreadDetails> threads)
    {
        threads = new List<ApiThreadDetails>();
        if (!parent.TryGetProperty("threads", out var property) ||
            property.ValueKind != JsonValueKind.Array ||
            property.GetArrayLength() > MaxThreads)
        {
            return false;
        }

        var ids = new HashSet<string>(StringComparer.Ordinal);
        var pending = new List<(string Id, string Title, string? ParentId, string Model, string ModelLabel, ulong? TotalTokens, ulong? ContextTokens, ulong? ContextLimit, long? CreatedAt, long? LastUserMessageAt, bool IsSubAgent, int? Depth)>();
        foreach (var thread in property.EnumerateArray())
        {
            if (!HasExactlyProperties(thread, ThreadProperties, 12) ||
                !TryGetBoundedString(thread, "id", 1, 512, out var id) ||
                !ids.Add(id) ||
                !TryGetBoundedString(thread, "title", 1, 512, out var title) ||
                !TryGetNullableBoundedString(thread, "parent_thread_id", 1, 512, out var parentId) ||
                !TryGetBoundedString(thread, "model", 1, 128, out var model) ||
                !TryGetBoundedString(thread, "model_label", 1, 24, out var modelLabel) ||
                !TryGetNullableUInt64(thread, "total_tokens", out var totalTokens) ||
                !TryGetNullableUInt64(thread, "context_usage_tokens", out var contextTokens) ||
                !TryGetNullableUInt64(thread, "context_window_tokens", out var contextLimit) ||
                !TryGetNullableUnixSeconds(thread, "created_at", out var createdAt) ||
                !TryGetNullableUnixSeconds(thread, "last_user_message_at", out var lastUserMessageAt) ||
                !TryGetBoolean(thread, "is_subagent", out var isSubAgent) ||
                !TryGetNullableDepth(thread, out var depth))
            {
                return false;
            }

            pending.Add((id, title, parentId, model, modelLabel, totalTokens, contextTokens, contextLimit, createdAt, lastUserMessageAt, isSubAgent, depth));
        }

        foreach (var item in pending)
        {
            var isOrphan = item.ParentId is { } parentId && !ids.Contains(parentId);
            threads.Add(new ApiThreadDetails(item.Id, item.Title, item.ParentId, item.Model, item.ModelLabel,
                item.TotalTokens, item.ContextTokens, item.ContextLimit, item.CreatedAt,
                item.LastUserMessageAt, item.IsSubAgent, item.Depth, isOrphan));
        }

        // A cycle has no valid parent-first projection and must reject the
        // complete details generation. Orphans remain representable because
        // their missing parent is an explicit display state.
        var parentById = pending.ToDictionary(item => item.Id, item => item.ParentId, StringComparer.Ordinal);
        foreach (var id in parentById.Keys)
        {
            var seen = new HashSet<string>(StringComparer.Ordinal) { id };
            var current = id;
            while (parentById.TryGetValue(current, out var parentId) && parentId is not null &&
                   parentById.ContainsKey(parentId))
            {
                if (!seen.Add(parentId))
                {
                    threads.Clear();
                    return false;
                }
                current = parentId;
            }
        }

        return true;
    }

    private static bool HasExactlyProperties(
        JsonElement value,
        HashSet<string> expected,
        int expectedCount)
    {
        if (value.ValueKind != JsonValueKind.Object)
        {
            return false;
        }

        var seen = new HashSet<string>(StringComparer.Ordinal);
        var count = 0;
        foreach (var property in value.EnumerateObject())
        {
            count++;
            if (!seen.Add(property.Name) || !expected.Contains(property.Name))
            {
                return false;
            }
        }

        return count == expectedCount;
    }

    private static bool TryGetString(JsonElement parent, string name, out string value)
    {
        value = string.Empty;
        if (!parent.TryGetProperty(name, out var property) || property.ValueKind != JsonValueKind.String)
        {
            return false;
        }

        value = property.GetString() ?? string.Empty;
        return true;
    }

    private static bool TryGetBoundedString(
        JsonElement parent,
        string name,
        int minimum,
        int maximum,
        out string value)
    {
        value = string.Empty;
        if (!TryGetString(parent, name, out var candidate) ||
            !IsSafeText(candidate, minimum, maximum))
        {
            return false;
        }

        value = candidate;
        return true;
    }

    private static bool TryGetNullableBoundedString(
        JsonElement parent,
        string name,
        int minimum,
        int maximum,
        out string? value)
    {
        value = null;
        if (!parent.TryGetProperty(name, out var property))
        {
            return false;
        }

        if (property.ValueKind == JsonValueKind.Null)
        {
            return true;
        }

        if (property.ValueKind != JsonValueKind.String)
        {
            return false;
        }

        var candidate = property.GetString();
        if (candidate is null || !IsSafeText(candidate, minimum, maximum))
        {
            return false;
        }

        value = candidate;
        return true;
    }

    private static bool TryGetNullableRemainingPercent(
        JsonElement parent,
        out double? value)
    {
        return TryGetNullableFiniteDouble(parent, "remaining_percent", 0, 100, out value);
    }

    private static bool TryGetNullableFiniteDouble(
        JsonElement parent,
        string name,
        double minimum,
        double maximum,
        out double? value)
    {
        value = null;
        if (!parent.TryGetProperty(name, out var property))
        {
            return false;
        }

        if (property.ValueKind == JsonValueKind.Null)
        {
            return true;
        }

        if (property.ValueKind != JsonValueKind.Number ||
            !property.TryGetDouble(out var candidate) ||
            !double.IsFinite(candidate) ||
            candidate < minimum ||
            candidate > maximum)
        {
            return false;
        }

        value = candidate;
        return true;
    }

    private static bool TryGetNonNegativeFiniteDouble(
        JsonElement parent,
        string name,
        out double value)
    {
        value = default;
        if (!parent.TryGetProperty(name, out var property) ||
            property.ValueKind != JsonValueKind.Number ||
            !property.TryGetDouble(out value) ||
            !double.IsFinite(value) ||
            value < 0 ||
            value > 1_000_000_000_000)
        {
            value = default;
            return false;
        }

        return true;
    }

    private static bool TryGetNullableUInt64(JsonElement parent, string name, out ulong? value)
    {
        value = null;
        if (!parent.TryGetProperty(name, out var property))
        {
            return false;
        }

        if (property.ValueKind == JsonValueKind.Null)
        {
            return true;
        }

        if (!TryGetUInt64(property, out var candidate))
        {
            return false;
        }

        value = candidate;
        return true;
    }

    private static bool TryGetDepth(JsonElement parent, out int value)
    {
        value = default;
        if (!TryGetUInt64(parent, "depth", out var candidate) || candidate > 64)
        {
            return false;
        }

        value = (int)candidate;
        return true;
    }

    private static bool TryGetNullableDepth(JsonElement parent, out int? value)
    {
        value = null;
        if (!parent.TryGetProperty("depth", out var property))
        {
            return false;
        }

        if (property.ValueKind == JsonValueKind.Null)
        {
            return true;
        }

        if (!TryGetUInt64(property, out var candidate) || candidate > 1024)
        {
            return false;
        }

        value = (int)candidate;
        return true;
    }

    private static bool IsSupportedModel(string name) =>
        name is "SOL" or "TERRA" or "LUNA";

    private static bool IsSafeText(string value, int minimum, int maximum)
    {
        if (!HasUnicodeScalarLength(value, minimum, maximum))
        {
            return false;
        }

        foreach (var character in value)
        {
            if (character <= '\u001F' ||
                character is >= '\u007F' and <= '\u009F' ||
                character is '\u2028' or '\u2029' ||
                character is >= '\u202A' and <= '\u202E' ||
                character is >= '\u2066' and <= '\u2069')
            {
                return false;
            }
        }

        return true;
    }

    private static bool TryGetState(JsonElement parent, out ApiState state)
    {
        state = default;
        if (!TryGetString(parent, "state", out var value))
        {
            return false;
        }

        state = value switch
        {
            "initializing" => ApiState.Initializing,
            "ready" => ApiState.Ready,
            "auth_required" => ApiState.AuthRequired,
            "error" => ApiState.Error,
            _ => default,
        };

        return value is "initializing" or "ready" or "auth_required" or "error";
    }

    private static bool TryGetBoolean(JsonElement parent, string name, out bool value)
    {
        value = default;
        if (!parent.TryGetProperty(name, out var property) ||
            (property.ValueKind is not JsonValueKind.True and not JsonValueKind.False))
        {
            return false;
        }

        value = property.GetBoolean();
        return true;
    }

    private static bool TryGetNullableUnixSeconds(
        JsonElement parent,
        string name,
        out long? value)
    {
        value = null;
        if (!parent.TryGetProperty(name, out var property))
        {
            return false;
        }

        if (property.ValueKind == JsonValueKind.Null)
        {
            return true;
        }

        if (!TryGetUInt64(property, out var unsigned) ||
            unsigned is < 1 or > MaxUnixSeconds)
        {
            return false;
        }

        value = (long)unsigned;
        return true;
    }

    private static bool TryGetNullablePlanLabel(JsonElement parent, out string? value)
    {
        value = null;
        if (!parent.TryGetProperty("plan_label", out var property))
        {
            return false;
        }

        if (property.ValueKind == JsonValueKind.Null)
        {
            return true;
        }

        if (property.ValueKind != JsonValueKind.String)
        {
            return false;
        }

        var text = property.GetString();
        if (text is null || !IsSafePlanLabel(text))
        {
            return false;
        }

        value = text;
        return true;
    }

    private static bool TryGetQuota(JsonElement parent, out ApiQuota? quota)
    {
        quota = null;
        if (!parent.TryGetProperty("quota", out var property))
        {
            return false;
        }

        if (property.ValueKind == JsonValueKind.Null)
        {
            return true;
        }

        if (!HasExactlyProperties(property, QuotaProperties, 4) ||
            !TryGetFiniteDouble(property, "remaining_percent", out var remainingPercent) ||
            remainingPercent is < 0 or > 100 ||
            !TryGetUnixSeconds(property, "reset_at", out var resetAt) ||
            !TryGetPositiveInt64(property, "window_seconds", out var windowSeconds) ||
            !TryGetBoolean(property, "monthly", out var monthly))
        {
            return false;
        }

        quota = new ApiQuota(remainingPercent, resetAt, windowSeconds, monthly);
        return true;
    }

    private static bool TryGetModels(JsonElement parent, out List<ApiModelUsage> models)
    {
        models = new List<ApiModelUsage>();
        if (!parent.TryGetProperty("models", out var property) ||
            property.ValueKind != JsonValueKind.Array ||
            property.GetArrayLength() > 3)
        {
            return false;
        }

        var names = new HashSet<string>(StringComparer.Ordinal);
        foreach (var model in property.EnumerateArray())
        {
            if (!HasExactlyProperties(model, ModelProperties, 4) ||
                !TryGetString(model, "name", out var name) ||
                !names.Add(name) ||
                name is not ("SOL" or "TERRA" or "LUNA") ||
                !TryGetUInt64(model, "input_tokens", out var inputTokens) ||
                !TryGetUInt64(model, "cached_input_tokens", out var cachedInputTokens) ||
                !TryGetUInt64(model, "output_tokens", out var outputTokens))
            {
                return false;
            }

            models.Add(new ApiModelUsage(name, inputTokens, cachedInputTokens, outputTokens));
        }

        return true;
    }

    private static bool TryGetUnixSeconds(JsonElement parent, string name, out long value)
    {
        value = default;
        if (!TryGetUInt64(parent, name, out var unsigned) || unsigned is < 1 or > MaxUnixSeconds)
        {
            return false;
        }

        value = (long)unsigned;
        return true;
    }

    private static bool TryGetPositiveInt64(JsonElement parent, string name, out long value)
    {
        value = default;
        if (!TryGetUInt64(parent, name, out var unsigned) || unsigned is < 1 or > long.MaxValue)
        {
            return false;
        }

        value = (long)unsigned;
        return true;
    }

    private static bool TryGetUInt64(JsonElement parent, string name, out ulong value)
    {
        value = default;
        return parent.TryGetProperty(name, out var property) && TryGetUInt64(property, out value);
    }

    private static bool TryGetUInt64(JsonElement property, out ulong value)
    {
        value = default;
        if (property.ValueKind != JsonValueKind.Number || !IsIntegerLexeme(property.GetRawText()))
        {
            return false;
        }

        return property.TryGetUInt64(out value);
    }

    private static bool TryGetFiniteDouble(JsonElement parent, string name, out double value)
    {
        value = default;
        if (!parent.TryGetProperty(name, out var property) || property.ValueKind != JsonValueKind.Number)
        {
            return false;
        }

        return property.TryGetDouble(out value) && double.IsFinite(value);
    }

    private static bool IsIntegerLexeme(string raw)
    {
        if (raw.Length == 0)
        {
            return false;
        }

        foreach (var character in raw)
        {
            if (character is < '0' or > '9')
            {
                return false;
            }
        }

        return true;
    }

    private static bool HasUnicodeScalarLength(string value, int minimum, int maximum)
    {
        var scalarCount = 0;
        for (var index = 0; index < value.Length; index++)
        {
            var character = value[index];
            if (char.IsHighSurrogate(character))
            {
                if (index + 1 >= value.Length || !char.IsLowSurrogate(value[index + 1]))
                {
                    return false;
                }

                index++;
            }
            else if (char.IsLowSurrogate(character))
            {
                return false;
            }

            scalarCount++;
            if (scalarCount > maximum)
            {
                return false;
            }
        }

        return scalarCount >= minimum;
    }

    private static bool IsSafePlanLabel(string value)
    {
        if (!HasUnicodeScalarLength(value, 1, 64))
        {
            return false;
        }

        foreach (var character in value)
        {
            if (character <= '\u001F' ||
                character is >= '\u007F' and <= '\u009F' ||
                character is '\u2028' or '\u2029' ||
                character is >= '\u202A' and <= '\u202E' ||
                character is >= '\u2066' and <= '\u2069')
            {
                return false;
            }
        }

        return true;
    }

    private enum BodyReadKind
    {
        Success,
        Oversize,
        Transport,
    }

    private readonly record struct BodyReadResult(BodyReadKind Kind, byte[]? Body)
    {
        public static BodyReadResult Success(byte[] body) => new(BodyReadKind.Success, body);

        public static BodyReadResult Oversize() => new(BodyReadKind.Oversize, null);

        public static BodyReadResult Transport() => new(BodyReadKind.Transport, null);
    }
}
