// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Net;
using System.Net.Http.Headers;
using System.Reflection;
using System.Text;
using CodexInfo.WindowsClient.Core;
using Xunit;

namespace CodexInfo.WindowsClient.Core.Tests;

public sealed class LoopbackStatusClientTests
{
    [Theory]
    [InlineData("ready", ApiState.Ready)]
    [InlineData("auth_required", ApiState.AuthRequired)]
    [InlineData("error", ApiState.Error)]
    [InlineData("initializing", ApiState.Initializing)]
    public async Task ValidStatesProduceValidatedSnapshots(string state, ApiState expectedState)
    {
        var json = ValidJson(state);
        var handler = new StubHandler(_ => JsonResponse(json));

        using var client = new LoopbackStatusClient(handler);
        var result = await client.FetchAsync(CancellationToken.None);

        Assert.True(result.IsSuccess);
        Assert.Null(result.Failure);
        Assert.NotNull(result.Snapshot);
        Assert.Equal(expectedState, result.Snapshot.State);
        Assert.Equal("Pro", result.Snapshot.PlanLabel);
        Assert.Equal(98.5, result.Snapshot.Quota!.RemainingPercent);
        Assert.Single(result.Snapshot.Models);
        Assert.Equal("SOL", result.Snapshot.Models[0].Name);
        Assert.Equal((ulong)3, result.Snapshot.ActiveThreadCount);
    }

    [Fact]
    public async Task NullableQuotaAndEmptyModelsAreAccepted()
    {
        var json = ValidJson("ready")
            .Replace("\"plan_label\":\"Pro\"", "\"plan_label\":null", StringComparison.Ordinal)
            .Replace("\"quota\":{\"remaining_percent\":98.5,\"reset_at\":253402300799,\"window_seconds\":1,\"monthly\":false}", "\"quota\":null", StringComparison.Ordinal)
            .Replace("\"models\":[{\"name\":\"SOL\",\"input_tokens\":1,\"cached_input_tokens\":2,\"output_tokens\":3}]", "\"models\":[]", StringComparison.Ordinal);
        var result = await Fetch(json);

        Assert.True(result.IsSuccess);
        Assert.Null(result.Snapshot!.PlanLabel);
        Assert.Null(result.Snapshot.Quota);
        Assert.Empty(result.Snapshot.Models);
    }

    [Fact]
    public async Task EndpointAndMethodAreFixed()
    {
        var handler = new StubHandler(request =>
        {
            Assert.Equal(HttpMethod.Get, request.Method);
            Assert.Equal("http://127.0.0.1:8787/v1/status", request.RequestUri!.AbsoluteUri);
            return JsonResponse(ValidJson("ready"));
        });

        using var client = new LoopbackStatusClient(handler);
        var result = await client.FetchAsync(CancellationToken.None);

        Assert.True(result.IsSuccess);
        Assert.NotNull(handler.LastRequest);
    }

    [Fact]
    public async Task HealthEndpointAndMethodAreFixedAndStrictlyParsed()
    {
        var handler = new StubHandler(request =>
        {
            Assert.Equal(HttpMethod.Get, request.Method);
            Assert.Equal("http://127.0.0.1:8787/v1/health", request.RequestUri!.AbsoluteUri);
            return JsonResponse(HealthJson());
        });

        using var client = new LoopbackStatusClient(handler);
        var result = await client.FetchHealthAsync(CancellationToken.None);

        Assert.True(result.IsSuccess);
        Assert.Equal(new ApiHealthSnapshot("v1", "codex-info"), result.Snapshot);
    }

    [Fact]
    public async Task HealthRequiresHttp200()
    {
        var handler = new StubHandler(_ =>
        {
            var response = JsonResponse(HealthJson());
            response.StatusCode = HttpStatusCode.Created;
            return response;
        });

        using var client = new LoopbackStatusClient(handler);
        var result = await client.FetchHealthAsync(CancellationToken.None);

        Assert.Equal(HealthFetchFailure.Response, result.Failure);
        Assert.Null(result.Snapshot);
    }

    [Fact]
    public async Task HealthRequiresDeclaredContentLength()
    {
        var response = new HttpResponseMessage(HttpStatusCode.OK)
        {
            Content = new UnknownLengthContent(Encoding.UTF8.GetBytes(HealthJson())),
        };
        response.Headers.CacheControl = new CacheControlHeaderValue { NoStore = true };

        using var client = new LoopbackStatusClient(new StubHandler(_ => response));
        var result = await client.FetchHealthAsync(CancellationToken.None);

        Assert.Equal(HealthFetchFailure.Response, result.Failure);
        Assert.Null(result.Snapshot);
    }

    [Fact]
    public async Task HealthRejectsDeclaredContentLengthMismatch()
    {
        var response = new HttpResponseMessage(HttpStatusCode.OK)
        {
            Content = new DeclaredLengthContent(HealthJson().Length),
        };
        response.Headers.CacheControl = new CacheControlHeaderValue { NoStore = true };

        using var client = new LoopbackStatusClient(new StubHandler(_ => response));
        var result = await client.FetchHealthAsync(CancellationToken.None);

        Assert.Equal(HealthFetchFailure.Response, result.Failure);
        Assert.Null(result.Snapshot);
    }

    [Theory]
    [InlineData("api_version", "api_version2")]
    [InlineData("service", "Service")]
    public async Task HealthRejectsUnknownOrWrongFixedValues(string property, string replacement)
    {
        var json = HealthJson().Replace($"\"{property}\":", $"\"{replacement}\":", StringComparison.Ordinal);
        var result = await FetchHealth(json);

        Assert.Equal(HealthFetchFailure.Response, result.Failure);
        Assert.Null(result.Snapshot);
    }

    [Theory]
    [InlineData("status")]
    [InlineData("details")]
    public async Task StatusAndDetailsRequireHttp200(string endpoint)
    {
        var handler = new StubHandler(request =>
        {
            var response = endpoint == "status"
                ? JsonResponse(ValidJson("ready"))
                : JsonResponse(ValidDetailsJson());
            response.StatusCode = HttpStatusCode.Created;
            return response;
        });

        using var client = new LoopbackStatusClient(handler);
        if (endpoint == "status")
        {
            var result = await client.FetchAsync(CancellationToken.None);
            Assert.Equal(StatusFetchFailure.Transport, result.Failure);
            Assert.Null(result.Snapshot);
        }
        else
        {
            var result = await client.FetchDetailsAsync(CancellationToken.None);
            Assert.Equal(DetailsFetchFailure.Transport, result.Failure);
            Assert.Null(result.Snapshot);
        }
    }

    [Fact]
    public async Task ContentTypeMustBeApplicationJson()
    {
        var handler = new StubHandler(_ => new HttpResponseMessage(HttpStatusCode.OK)
        {
            Content = new StringContent(ValidJson("ready"), Encoding.UTF8, "text/plain"),
        });

        using var client = new LoopbackStatusClient(handler);
        var result = await client.FetchAsync(CancellationToken.None);

        Assert.Equal(StatusFetchFailure.Response, result.Failure);
        Assert.Null(result.Snapshot);
    }

    [Theory]
    [InlineData(false, true)]
    [InlineData(true, false)]
    public async Task Utf8AndNoStoreAreBothRequired(bool includeUtf8, bool includeNoStore)
    {
        var response = new HttpResponseMessage(HttpStatusCode.OK)
        {
            Content = new StringContent(ValidJson("ready"), Encoding.UTF8, "application/json"),
        };
        if (!includeUtf8)
        {
            response.Content.Headers.ContentType!.CharSet = null;
        }
        if (includeNoStore)
        {
            response.Headers.CacheControl = new CacheControlHeaderValue { NoStore = true };
        }
        using var client = new LoopbackStatusClient(new StubHandler(_ => response));

        var result = await client.FetchAsync(CancellationToken.None);

        Assert.Equal(StatusFetchFailure.Response, result.Failure);
        Assert.Null(result.Snapshot);
    }

    [Theory]
    [InlineData("api_version", "api_version2")]
    [InlineData("state", "State")]
    [InlineData("models", "model")]
    public async Task UnknownOrCaseChangedPropertyIsRejected(string oldName, string newName)
    {
        var result = await Fetch(ValidJson("ready").Replace($"\"{oldName}\":", $"\"{newName}\":", StringComparison.Ordinal));

        Assert.Equal(StatusFetchFailure.Response, result.Failure);
    }

    [Fact]
    public async Task MissingPropertyAndWrongTypeAreRejected()
    {
        var missing = await Fetch(ValidJson("ready").Replace(",\"active_thread_count\":3", "", StringComparison.Ordinal));
        var wrongType = await Fetch(ValidJson("ready").Replace("\"authenticated\":true", "\"authenticated\":1", StringComparison.Ordinal));

        Assert.Equal(StatusFetchFailure.Response, missing.Failure);
        Assert.Equal(StatusFetchFailure.Response, wrongType.Failure);
    }

    [Theory]
    [InlineData("1.0")]
    [InlineData("1e0")]
    [InlineData("-1")]
    public async Task IntegerFieldsRejectNonIntegerOrNegativeNumbers(string value)
    {
        var json = ValidJson("ready").Replace("\"active_thread_count\":3", $"\"active_thread_count\":{value}", StringComparison.Ordinal);
        var result = await Fetch(json);

        Assert.Equal(StatusFetchFailure.Response, result.Failure);
    }

    [Fact]
    public async Task IntegerBoundariesAreAcceptedAndOverflowIsRejected()
    {
        var max = ValidJson("ready")
            .Replace("\"observed_at\":1", "\"observed_at\":253402300799", StringComparison.Ordinal)
            .Replace("\"reset_at\":253402300799", "\"reset_at\":253402300799", StringComparison.Ordinal)
            .Replace("\"window_seconds\":1", "\"window_seconds\":9223372036854775807", StringComparison.Ordinal)
            .Replace("\"input_tokens\":1", "\"input_tokens\":18446744073709551615", StringComparison.Ordinal)
            .Replace("\"active_thread_count\":3", "\"active_thread_count\":18446744073709551615", StringComparison.Ordinal);
        var valid = await Fetch(max);
        var overflow = await Fetch(max.Replace("18446744073709551615", "18446744073709551616", StringComparison.Ordinal));

        Assert.True(valid.IsSuccess);
        Assert.Equal(StatusFetchFailure.Response, overflow.Failure);
    }

    [Fact]
    public async Task PlanLabelCountsUnicodeScalars()
    {
        var sixtyFourScalars = string.Concat(Enumerable.Repeat("😀", 64));
        var sixtyFiveScalars = string.Concat(Enumerable.Repeat("😀", 65));
        var valid = await Fetch(ValidJson("ready").Replace("Pro", sixtyFourScalars, StringComparison.Ordinal));
        var invalid = await Fetch(ValidJson("ready").Replace("Pro", sixtyFiveScalars, StringComparison.Ordinal));

        Assert.True(valid.IsSuccess);
        Assert.Equal(StatusFetchFailure.Response, invalid.Failure);
    }

    [Fact]
    public async Task PlanLabelRejectsLayoutAndBidiControls()
    {
        var newline = await Fetch(ValidJson("ready").Replace("Pro", "Pro\\nInjected", StringComparison.Ordinal));
        var bidi = await Fetch(ValidJson("ready").Replace("Pro", "Pro\\u202Eevil", StringComparison.Ordinal));

        Assert.Equal(StatusFetchFailure.Response, newline.Failure);
        Assert.Equal(StatusFetchFailure.Response, bidi.Failure);
    }

    [Fact]
    public async Task DuplicateKeysAreRejectedAtEveryKnownObjectLevel()
    {
        var top = await Fetch(ValidJson("ready").Replace(",\"active_thread_count\":3", ",\"active_thread_count\":3,\"state\":\"ready\"", StringComparison.Ordinal));
        var quota = await Fetch(ValidJson("ready").Replace("\"monthly\":false}", "\"monthly\":false,\"monthly\":true}", StringComparison.Ordinal));
        var model = await Fetch(ValidJson("ready").Replace("\"output_tokens\":3}]", "\"output_tokens\":3,\"name\":\"SOL\"}]", StringComparison.Ordinal));

        Assert.Equal(StatusFetchFailure.Response, top.Failure);
        Assert.Equal(StatusFetchFailure.Response, quota.Failure);
        Assert.Equal(StatusFetchFailure.Response, model.Failure);
    }

    [Fact]
    public async Task HeaderLimitIsEnforcedBeforeBodyParsing()
    {
        var handler = new StubHandler(_ =>
        {
            var response = JsonResponse(ValidJson("ready"));
            response.Headers.TryAddWithoutValidation("X-Large", new string('x', 8_200));
            return response;
        });

        using var client = new LoopbackStatusClient(handler);
        var result = await client.FetchAsync(CancellationToken.None);

        Assert.Equal(StatusFetchFailure.Response, result.Failure);
    }

    [Fact]
    public async Task UnknownLengthBodyIsRejectedWhileStreaming()
    {
        var payload = Encoding.UTF8.GetBytes(new string('x', 65_537));
        var handler = new StubHandler(_ => new HttpResponseMessage(HttpStatusCode.OK)
        {
            Content = new UnknownLengthContent(payload),
        });

        // The custom content sets Content-Type but deliberately has no Content-Length.
        using var client = new LoopbackStatusClient(handler);
        var result = await client.FetchAsync(CancellationToken.None);

        Assert.Equal(StatusFetchFailure.Response, result.Failure);
    }

    [Fact]
    public async Task DeclaredOversizeBodyIsRejectedBeforeParsing()
    {
        var handler = new StubHandler(_ => new HttpResponseMessage(HttpStatusCode.OK)
        {
            Content = new StringContent(new string('x', 65_537), Encoding.UTF8, "application/json"),
        });

        using var client = new LoopbackStatusClient(handler);
        var result = await client.FetchAsync(CancellationToken.None);

        Assert.Equal(StatusFetchFailure.Response, result.Failure);
        Assert.Null(result.Snapshot);
    }

    [Fact]
    public async Task Non2xxAndTransportFailuresAreClassifiedWithoutDetails()
    {
        var nonSuccess = new StubHandler(_ => new HttpResponseMessage(HttpStatusCode.BadGateway)
        {
            Content = new StringContent("not public", Encoding.UTF8, "text/plain"),
        });
        var thrown = new StubHandler(_ => throw new HttpRequestException("private detail"));

        using var nonSuccessClient = new LoopbackStatusClient(nonSuccess);
        using var thrownClient = new LoopbackStatusClient(thrown);
        var nonSuccessResult = await nonSuccessClient.FetchAsync(CancellationToken.None);
        var thrownResult = await thrownClient.FetchAsync(CancellationToken.None);

        Assert.Equal(StatusFetchFailure.Transport, nonSuccessResult.Failure);
        Assert.Null(nonSuccessResult.Snapshot);
        Assert.Equal(StatusFetchFailure.Transport, thrownResult.Failure);
        Assert.Null(thrownResult.Snapshot);
    }

    [Fact]
    public async Task DetailsEndpointParsesHistoryThreadsAndSamplesStrictly()
    {
        var handler = new StubHandler(request =>
        {
            Assert.Equal(HttpMethod.Get, request.Method);
            Assert.Equal("http://127.0.0.1:8787/v1/details", request.RequestUri!.AbsoluteUri);
            return JsonResponse(ValidDetailsJson());
        });

        using var client = new LoopbackStatusClient(handler);
        var result = await client.FetchDetailsAsync(CancellationToken.None);

        Assert.True(result.IsSuccess);
        var details = Assert.IsType<ApiDetailsSnapshot>(result.Snapshot);
        Assert.Single(details.Models);
        Assert.Equal(1.25, details.Models[0].TotalDollars);
        Assert.Single(details.HistoryPeriods);
        Assert.Single(details.HistoryPeriods[0].Samples);
        Assert.Equal(42.5, details.HistoryPeriods[0].Samples[0].RemainingPercent);
        Assert.Single(details.Threads);
        Assert.Equal("SOL", details.Threads[0].Model);
        Assert.Equal("Pro", details.PlanLabel);
        Assert.Equal("概算 $1", details.EstimatedCostLabel);
    }

    [Fact]
    public async Task OpaqueHistoryPeriodIdStillJoinsSamplesByResetBoundary()
    {
        var json = ValidDetailsJson().Replace(
            "\"id\":\"253402300799\"",
            "\"id\":\"current-period\"",
            StringComparison.Ordinal);

        var result = await FetchDetails(json);

        Assert.True(result.IsSuccess);
        Assert.Single(result.Snapshot!.HistoryPeriods[0].Samples);
        Assert.Equal(253_402_300_799, result.Snapshot.HistoryPeriods[0].ResetAt);
    }

    [Fact]
    public async Task ResetJitterSamplesJoinAndMergeIntoTheCanonicalPeriodLikeTheNativeGraph()
    {
        const string original = "{\"timestamp\":1,\"reset_at\":253402300799,\"remaining_percent\":42.5,\"sol_dollars\":1.25,\"terra_dollars\":0.0,\"luna_dollars\":0.0,\"sol_tokens\":6,\"terra_tokens\":0,\"luna_tokens\":0}";
        const string jittered = "{\"timestamp\":1,\"reset_at\":253402300739,\"remaining_percent\":null,\"sol_dollars\":2.0,\"terra_dollars\":3.0,\"luna_dollars\":4.0,\"sol_tokens\":12,\"terra_tokens\":13,\"luna_tokens\":14}";
        const string outsideTolerance = "{\"timestamp\":2,\"reset_at\":253402300738,\"remaining_percent\":40.0,\"sol_dollars\":9.0,\"terra_dollars\":9.0,\"luna_dollars\":9.0,\"sol_tokens\":99,\"terra_tokens\":99,\"luna_tokens\":99}";
        var json = ValidDetailsJson().Replace(
            original,
            jittered + "," + outsideTolerance + "," + original,
            StringComparison.Ordinal);

        var result = await FetchDetails(json);

        Assert.True(result.IsSuccess);
        var sample = Assert.Single(result.Snapshot!.HistoryPeriods[0].Samples);
        Assert.Equal(253_402_300_799, sample.ResetAt);
        Assert.Equal(42.5, sample.RemainingPercent);
        Assert.Equal(2.0, sample.SolDollars);
        Assert.Equal(3.0, sample.TerraDollars);
        Assert.Equal(4.0, sample.LunaDollars);
        Assert.Equal(12UL, sample.SolTokens);
        Assert.Equal(13UL, sample.TerraTokens);
        Assert.Equal(14UL, sample.LunaTokens);
    }

    [Fact]
    public async Task ConflictingRemainingValuesAtOneTimestampAreUnavailableInsteadOfLastRowWins()
    {
        const string original = "{\"timestamp\":1,\"reset_at\":253402300799,\"remaining_percent\":42.5,\"sol_dollars\":1.25,\"terra_dollars\":0.0,\"luna_dollars\":0.0,\"sol_tokens\":6,\"terra_tokens\":0,\"luna_tokens\":0}";
        const string conflicting = "{\"timestamp\":1,\"reset_at\":253402300739,\"remaining_percent\":14.0,\"sol_dollars\":9.0,\"terra_dollars\":0.0,\"luna_dollars\":0.0,\"sol_tokens\":9,\"terra_tokens\":0,\"luna_tokens\":0}";
        var json = ValidDetailsJson().Replace(
            original,
            original + "," + conflicting,
            StringComparison.Ordinal);

        var result = await FetchDetails(json);

        Assert.True(result.IsSuccess);
        var sample = Assert.Single(result.Snapshot!.HistoryPeriods[0].Samples);
        Assert.Null(sample.RemainingPercent);
        Assert.Equal(9.0, sample.SolDollars);
        Assert.Equal(9UL, sample.SolTokens);
    }

    [Fact]
    public async Task DuplicateHistorySampleIdentitiesAreCoalescedWithoutRejectingTheServer()
    {
        const string original = "{\"timestamp\":1,\"reset_at\":253402300799,\"remaining_percent\":42.5,\"sol_dollars\":1.25,\"terra_dollars\":0.0,\"luna_dollars\":0.0,\"sol_tokens\":6,\"terra_tokens\":0,\"luna_tokens\":0}";
        const string secondObservation = "{\"timestamp\":1,\"reset_at\":253402300799,\"remaining_percent\":42.5,\"sol_dollars\":0.0,\"terra_dollars\":0.0,\"luna_dollars\":2.0,\"sol_tokens\":0,\"terra_tokens\":0,\"luna_tokens\":9}";
        var json = ValidDetailsJson().Replace(
            original,
            original + "," + secondObservation,
            StringComparison.Ordinal);

        var result = await FetchDetails(json);

        Assert.True(result.IsSuccess);
        var sample = Assert.Single(result.Snapshot!.HistoryPeriods[0].Samples);
        Assert.Equal(42.5, sample.RemainingPercent);
        Assert.Equal(1.25, sample.SolDollars);
        Assert.Equal(2.0, sample.LunaDollars);
        Assert.Equal(6UL, sample.SolTokens);
        Assert.Equal(9UL, sample.LunaTokens);
    }

    [Fact]
    public async Task DuplicateIdentityWithQuotaOnlyConflictCannotInventHistoricalDrop()
    {
        const string original = "{\"timestamp\":1,\"reset_at\":253402300799,\"remaining_percent\":42.5,\"sol_dollars\":1.25,\"terra_dollars\":0.0,\"luna_dollars\":0.0,\"sol_tokens\":6,\"terra_tokens\":0,\"luna_tokens\":0}";
        const string quotaOnlyConflict = "{\"timestamp\":1,\"reset_at\":253402300799,\"remaining_percent\":14.0,\"sol_dollars\":0.0,\"terra_dollars\":0.0,\"luna_dollars\":0.0,\"sol_tokens\":0,\"terra_tokens\":0,\"luna_tokens\":0}";
        var json = ValidDetailsJson().Replace(
            original,
            original + "," + quotaOnlyConflict,
            StringComparison.Ordinal);

        var result = await FetchDetails(json);

        Assert.True(result.IsSuccess);
        var sample = Assert.Single(result.Snapshot!.HistoryPeriods[0].Samples);
        Assert.Null(sample.RemainingPercent);
        Assert.Equal(1.25, sample.SolDollars);
        Assert.Equal(6UL, sample.SolTokens);
    }

    [Theory]
    [InlineData(30)]
    [InlineData(60)]
    public async Task MovingResetCollisionWithLaterObservationFailsClosed(int driftSeconds)
    {
        const string original = "{\"timestamp\":1,\"reset_at\":253402300799,\"remaining_percent\":42.5,\"sol_dollars\":1.25,\"terra_dollars\":0.0,\"luna_dollars\":0.0,\"sol_tokens\":6,\"terra_tokens\":0,\"luna_tokens\":0}";
        const string spend = "{\"timestamp\":1,\"reset_at\":253402300799,\"remaining_percent\":88.0,\"sol_dollars\":1.0,\"terra_dollars\":0.0,\"luna_dollars\":0.0,\"sol_tokens\":6,\"terra_tokens\":0,\"luna_tokens\":0}";
        var collision = $"{{\"timestamp\":1,\"reset_at\":{253402300799L - driftSeconds},\"remaining_percent\":14.0,\"sol_dollars\":0.0,\"terra_dollars\":0.0,\"luna_dollars\":0.0,\"sol_tokens\":0,\"terra_tokens\":0,\"luna_tokens\":0}}";
        var later = $"{{\"timestamp\":2,\"reset_at\":{253402300799L - driftSeconds + 30},\"remaining_percent\":87.0,\"sol_dollars\":2.0,\"terra_dollars\":0.0,\"luna_dollars\":0.0,\"sol_tokens\":7,\"terra_tokens\":0,\"luna_tokens\":0}}";
        var json = ValidDetailsJson().Replace(
            original,
            spend + "," + collision + "," + later,
            StringComparison.Ordinal);

        var result = await FetchDetails(json);

        Assert.True(result.IsSuccess);
        var samples = result.Snapshot!.HistoryPeriods[0].Samples;
        Assert.Equal(2, samples.Count);
        Assert.Null(samples[0].RemainingPercent);
        Assert.Equal(87.0, samples[1].RemainingPercent);
        Assert.DoesNotContain(samples, sample => sample.RemainingPercent == 14.0);
    }

    [Theory]
    [InlineData("unknown")]
    [InlineData("models")]
    public async Task DetailsUnknownOrDuplicateKeysAreRejected(string field)
    {
        var json = field == "unknown"
            ? ValidDetailsJson().Replace("\"estimated_cost_label\":", "\"unknown\":1,\"estimated_cost_label\":", StringComparison.Ordinal)
            : ValidDetailsJson().Replace(
                "\"models\":[",
                "\"models\":[",
                StringComparison.Ordinal)
                .Replace(
                    "],\"active_thread_count\":",
                    "],\"models\":[],\"active_thread_count\":",
                    StringComparison.Ordinal);
        var result = await FetchDetails(json);

        Assert.Equal(DetailsFetchFailure.Response, result.Failure);
        Assert.Null(result.Snapshot);
    }

    [Fact]
    public async Task ThreadCycleRejectsTheCompleteDetailsGeneration()
    {
        var cycle = ValidDetailsJson().Replace(
            "\"parent_thread_id\":null",
            "\"parent_thread_id\":\"thread-1\"",
            StringComparison.Ordinal);

        var result = await FetchDetails(cycle);

        Assert.Equal(DetailsFetchFailure.Response, result.Failure);
        Assert.Null(result.Snapshot);
    }

    [Fact]
    public async Task DetailsOversizeBodyIsRejected()
    {
        var handler = new StubHandler(_ => new HttpResponseMessage(HttpStatusCode.OK)
        {
            Content = new DeclaredLengthContent(32L * 1024 * 1024 + 1),
        });

        using var client = new LoopbackStatusClient(handler);
        var result = await client.FetchDetailsAsync(CancellationToken.None);

        Assert.Equal(DetailsFetchFailure.Response, result.Failure);
    }

    [Fact]
    public void DefaultHandlerDisablesProxyRedirectDecompressionAndCookies()
    {
        var method = typeof(LoopbackStatusClient).GetMethod(
            "CreateDefaultHandler",
            BindingFlags.NonPublic | BindingFlags.Static);
        Assert.NotNull(method);
        using var handler = Assert.IsType<HttpClientHandler>(method!.Invoke(null, null));

        Assert.False(handler.UseProxy);
        Assert.False(handler.AllowAutoRedirect);
        Assert.Equal(DecompressionMethods.None, handler.AutomaticDecompression);
        Assert.False(handler.UseCookies);
        Assert.Equal(8, handler.MaxResponseHeadersLength);
    }

    [Fact]
    public void ClientTimeoutIsFixedAtOneSecond()
    {
        using var client = new LoopbackStatusClient(new StubHandler(_ => JsonResponse(ValidJson("ready"))));
        var field = typeof(LoopbackStatusClient).GetField("_httpClient", BindingFlags.NonPublic | BindingFlags.Instance);
        Assert.NotNull(field);
        var httpClient = Assert.IsType<HttpClient>(field!.GetValue(client));

        Assert.Equal(TimeSpan.FromSeconds(1), httpClient.Timeout);
    }

    [Fact]
    public void DetailsHistoryCapacityIsExactlyOneThirtyOneDayMinuteWindow()
    {
        var field = typeof(LoopbackStatusClient).GetField(
            "MaxHistorySamples",
            BindingFlags.NonPublic | BindingFlags.Static);

        Assert.NotNull(field);
        Assert.Equal(44_640, field!.GetRawConstantValue());
    }

    private static async Task<StatusFetchResult> Fetch(string json)
    {
        using var client = new LoopbackStatusClient(new StubHandler(_ => JsonResponse(json)));
        return await client.FetchAsync(CancellationToken.None);
    }

    private static async Task<DetailsFetchResult> FetchDetails(string json)
    {
        using var client = new LoopbackStatusClient(new StubHandler(_ => JsonResponse(json)));
        return await client.FetchDetailsAsync(CancellationToken.None);
    }

    private static async Task<HealthFetchResult> FetchHealth(string json)
    {
        using var client = new LoopbackStatusClient(new StubHandler(_ => JsonResponse(json)));
        return await client.FetchHealthAsync(CancellationToken.None);
    }

    private static HttpResponseMessage JsonResponse(string json)
    {
        var response = new HttpResponseMessage(HttpStatusCode.OK)
        {
            Content = new StringContent(json, Encoding.UTF8, "application/json"),
        };
        response.Headers.CacheControl = new CacheControlHeaderValue { NoStore = true };
        return response;
    }

    private static string ValidJson(string state) =>
        $$"""{"api_version":"v1","state":"{{state}}","observed_at":1,"authenticated":true,"plan_label":"Pro","quota":{"remaining_percent":98.5,"reset_at":253402300799,"window_seconds":1,"monthly":false},"models":[{"name":"SOL","input_tokens":1,"cached_input_tokens":2,"output_tokens":3}],"active_thread_count":3}""";

    private static string HealthJson() =>
        "{\"api_version\":\"v1\",\"service\":\"codex-info\"}";

    private static string ValidDetailsJson() =>
        "{\"api_version\":\"v1\",\"state\":\"ready\",\"observed_at\":1,\"authenticated\":true,\"plan_label\":\"Pro\",\"quota\":{\"remaining_percent\":98.5,\"reset_at\":253402300799,\"window_seconds\":604800,\"monthly\":false},\"models\":[{\"name\":\"SOL\",\"input_tokens\":10,\"cached_input_tokens\":2,\"output_tokens\":3,\"input_dollars\":0.5,\"cached_input_dollars\":0.25,\"output_dollars\":0.5}],\"active_thread_count\":1,\"history_periods\":[{\"id\":\"253402300799\",\"start_at\":253341820799,\"end_at\":253402300799,\"reset_at\":253402300799,\"label\":\"2026/08/01 — 2026/08/08\",\"current\":true}],\"history_samples\":[{\"timestamp\":1,\"reset_at\":253402300799,\"remaining_percent\":42.5,\"sol_dollars\":1.25,\"terra_dollars\":0.0,\"luna_dollars\":0.0,\"sol_tokens\":6,\"terra_tokens\":0,\"luna_tokens\":0}],\"threads\":[{\"id\":\"thread-1\",\"title\":\"Task\",\"parent_thread_id\":null,\"model\":\"SOL\",\"model_label\":\"SOL\",\"total_tokens\":20,\"context_usage_tokens\":10,\"context_window_tokens\":80,\"created_at\":1,\"last_user_message_at\":1,\"is_subagent\":false,\"depth\":0}],\"estimated_cost_label\":\"概算 $1\"}";

    private sealed class StubHandler(Func<HttpRequestMessage, HttpResponseMessage> responder) : HttpMessageHandler
    {
        public HttpRequestMessage? LastRequest { get; private set; }

        protected override Task<HttpResponseMessage> SendAsync(
            HttpRequestMessage request,
            CancellationToken cancellationToken)
        {
            LastRequest = request;
            return Task.FromResult(responder(request));
        }
    }

    private sealed class UnknownLengthContent : HttpContent
    {
        private readonly byte[] _payload;

        public UnknownLengthContent(byte[] payload)
        {
            _payload = payload;
            Headers.ContentType = new MediaTypeHeaderValue("application/json");
        }

        protected override async Task SerializeToStreamAsync(Stream stream, TransportContext? context)
        {
            await stream.WriteAsync(_payload);
        }

        protected override bool TryComputeLength(out long length)
        {
            length = 0;
            return false;
        }

    }

    private sealed class DeclaredLengthContent : HttpContent
    {
        public DeclaredLengthContent(long length)
        {
            Headers.ContentType = new MediaTypeHeaderValue("application/json");
            Headers.ContentLength = length;
        }

        protected override Task SerializeToStreamAsync(Stream stream, TransportContext? context) =>
            Task.CompletedTask;

        protected override bool TryComputeLength(out long length)
        {
            length = Headers.ContentLength ?? 0;
            return false;
        }
    }
}
