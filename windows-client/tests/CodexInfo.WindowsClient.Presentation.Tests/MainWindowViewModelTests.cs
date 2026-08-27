// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Diagnostics;
using System.Collections.Specialized;
using CodexInfo.WindowsClient.Core;
using CodexInfo.WindowsClient.Settings;
using CodexInfo.WindowsClient.ViewModels;
using Xunit;

namespace CodexInfo.WindowsClient.Presentation.Tests;

public sealed class MainWindowViewModelTests
{
    [Fact]
    public async Task Startup_keeps_content_hidden_until_the_first_snapshot_is_complete()
    {
        var client = new BlockingClient();
        using var viewModel = new MainWindowViewModel(client);

        viewModel.Start();
        await EventuallyAsync(() => client.CallCount == 1);

        Assert.True(viewModel.IsStartupLoading);
        Assert.False(viewModel.ShowAuthenticatedContent);

        client.Complete(ValidSnapshot());
        await EventuallyAsync(() => viewModel.IsAuthenticated && !viewModel.IsStartupLoading);

        Assert.True(viewModel.ShowAuthenticatedContent);
    }

    [Fact]
    public async Task Startup_failure_releases_spinner_and_exposes_retry_state()
    {
        using var viewModel = new MainWindowViewModel(new SequenceClient(
            StatusFetchResult.FromFailure(StatusFetchFailure.Transport)));

        viewModel.Start();
        await EventuallyAsync(() => !viewModel.IsStartupLoading);

        Assert.False(viewModel.IsStartupLoading);
        Assert.False(viewModel.ShowAuthenticatedContent);
        Assert.Equal("接続エラー", viewModel.StatusTitle);
        Assert.True(viewModel.CanRefresh);
    }

    [Fact]
    public async Task Startup_waits_for_the_matching_details_generation_before_publishing_content()
    {
        var details = new BlockingDetailsClient();
        using var viewModel = new MainWindowViewModel(
            new SequenceClient(StatusFetchResult.Success(ValidSnapshot())),
            details);

        viewModel.Start();
        await EventuallyAsync(() => details.CallCount == 1);

        Assert.True(viewModel.IsStartupLoading);
        Assert.False(viewModel.IsAuthenticated);

        details.Complete(DetailsFetchResult.Success(DetailsSnapshot(1)));
        await EventuallyAsync(() => viewModel.IsAuthenticated && !viewModel.IsStartupLoading);

        Assert.True(viewModel.ShowAuthenticatedContent);
    }

    [Fact]
    public void ApplyingSavedConnectionStartsTheSelectedWslService()
    {
        var child = new TestConnectionChildProcess();
        var factory = new TestConnectionChildProcessFactory(child);
        using var supervisor = new ConnectionSupervisor(factory);
        using var viewModel = new MainWindowViewModel(new NeverCalledClient(), null, supervisor);

        var settings = new ClientSettings("ja", false)
        {
            ConnectionConfigured = true,
            ConnectionProfile = ConnectionProfiles.Wsl,
            ConnectionSelector = "Ubuntu-24.04",
        };

        Assert.True(viewModel.ApplyConnectionSettings(settings));
        Assert.Single(factory.StartInfos);
        Assert.Equal(
            ["--distribution", "Ubuntu-24.04", "--", "codex_info", "--port", "8787"],
            factory.StartInfos[0].ArgumentList);
    }

    [Fact]
    public async Task NullableValuesAndEmptyModelsHaveExplicitPresentation()
    {
        var noDataSnapshot = new ApiStatusSnapshot(
            ApiState.Ready,
            null,
            true,
            null,
            null,
            [],
            3);
        using var viewModel = new MainWindowViewModel(new SequenceClient(
            StatusFetchResult.Success(noDataSnapshot)));

        viewModel.Start();
        await EventuallyAsync(() => viewModel.StatusTitle == "正常");

        Assert.Equal("未取得", viewModel.RemainingPercentText);
        Assert.Equal("未取得", viewModel.PlanText);
        Assert.Equal("未取得", viewModel.ResetAtText);
        Assert.Equal("未取得", viewModel.ObservedAtText);
        Assert.True(viewModel.HasNoModels);
        Assert.Empty(viewModel.Models);
    }

    [Fact]
    public async Task TransportFailureKeepsSnapshotAndMarksItStale()
    {
        var success = StatusFetchResult.Success(ValidSnapshot());
        var failure = StatusFetchResult.FromFailure(StatusFetchFailure.Transport);
        using var viewModel = new MainWindowViewModel(new SequenceClient(success, failure));

        viewModel.Start();
        await EventuallyAsync(() => viewModel.StatusTitle == "正常");
        var initialRemaining = viewModel.RemainingPercentText;

        viewModel.RefreshCommand.Execute(null);
        await EventuallyAsync(() => viewModel.StatusTitle == "接続エラー");

        Assert.Equal(initialRemaining, viewModel.RemainingPercentText);
        Assert.Contains("前回受信の値", viewModel.StatusDetail, StringComparison.Ordinal);
        Assert.Contains("現在は更新できていません", viewModel.LastReceivedText, StringComparison.Ordinal);
    }

    [Fact]
    public async Task ApiErrorIsNotPresentedAsTransportFailure()
    {
        using var viewModel = new MainWindowViewModel(new SequenceClient(
            StatusFetchResult.Success(ValidSnapshot(state: ApiState.Error))));

        viewModel.Start();
        await EventuallyAsync(() => viewModel.StatusTitle == "Linux 側の取得エラー");

        Assert.DoesNotContain("更新できていません", viewModel.StatusDetail, StringComparison.Ordinal);
        Assert.Contains("接続経路", viewModel.StatusDetail, StringComparison.Ordinal);
        Assert.Equal("98.5%", viewModel.RemainingPercentText);
    }

    [Theory]
    [InlineData(2, "残量不足")]
    [InlineData(10, "残量警告")]
    public async Task ReadySnapshotWarnsAtQuotaThresholds(
        double remainingPercent,
        string expectedStatusTitle)
    {
        var quota = new ApiQuota(
            remainingPercent,
            DateTimeOffset.UtcNow.AddDays(2).ToUnixTimeSeconds(),
            604800,
            false);
        using var viewModel = new MainWindowViewModel(new SequenceClient(
            StatusFetchResult.Success(ValidSnapshot(quota: quota))));

        viewModel.Start();
        await EventuallyAsync(() => viewModel.StatusTitle == expectedStatusTitle);

        Assert.Equal($"{remainingPercent:0.#}%", viewModel.RemainingPercentText);
    }

    [Fact]
    public async Task ReadySnapshotWarnsBeforeResetWhenQuotaIsNotLow()
    {
        var quota = new ApiQuota(
            98.5,
            DateTimeOffset.UtcNow.AddHours(1).ToUnixTimeSeconds(),
            604800,
            false);
        using var viewModel = new MainWindowViewModel(new SequenceClient(
            StatusFetchResult.Success(ValidSnapshot(quota: quota))));

        viewModel.Start();
        await EventuallyAsync(() => viewModel.StatusTitle == "リセット警告");

        Assert.Contains("24 時間以内", viewModel.StatusDetail, StringComparison.Ordinal);
    }

    [Fact]
    public async Task QuotaDangerTakesPriorityOverResetWarning()
    {
        var quota = new ApiQuota(
            0,
            DateTimeOffset.UtcNow.AddHours(1).ToUnixTimeSeconds(),
            604800,
            false);
        using var viewModel = new MainWindowViewModel(new SequenceClient(
            StatusFetchResult.Success(ValidSnapshot(quota: quota))));

        viewModel.Start();
        await EventuallyAsync(() => viewModel.StatusTitle == "残量不足");
    }

    [Theory]
    [InlineData(ApiState.Initializing, "Linux 側で準備中")]
    [InlineData(ApiState.AuthRequired, "Linux 側で認証が必要です")]
    public async Task NonReadyWireStatesHaveTheirOwnPresentation(
        ApiState state,
        string expectedStatusTitle)
    {
        var snapshot = new ApiStatusSnapshot(state, null, false, null, null, [], 0);
        using var viewModel = new MainWindowViewModel(new SequenceClient(
            StatusFetchResult.Success(snapshot)));

        viewModel.Start();
        await EventuallyAsync(() => viewModel.StatusTitle == expectedStatusTitle);

        Assert.Equal("未取得", viewModel.RemainingPercentText);
        Assert.DoesNotContain("更新できていません", viewModel.StatusDetail, StringComparison.Ordinal);
        if (state == ApiState.Initializing)
        {
            Assert.Contains("自動で更新", viewModel.StatusDetail, StringComparison.Ordinal);
        }
    }

    [Fact]
    public async Task FirstTransportFailureShowsNoSyntheticValues()
    {
        using var viewModel = new MainWindowViewModel(new SequenceClient(
            StatusFetchResult.FromFailure(StatusFetchFailure.Transport)));

        viewModel.Start();
        await EventuallyAsync(() => viewModel.StatusTitle == "接続エラー");

        Assert.Equal("未取得", viewModel.RemainingPercentText);
        Assert.Equal("前回受信: 未取得", viewModel.LastReceivedText);
        Assert.Contains("接続できません", viewModel.StatusDetail, StringComparison.Ordinal);
    }

    [Fact]
    public async Task HealthFailureStopsTheCycleBeforeStatusAndKeepsValuesUnavailable()
    {
        var client = new HealthAwareClient(
            HealthFetchResult.FromFailure(HealthFetchFailure.Response),
            StatusFetchResult.Success(ValidSnapshot()));
        using var viewModel = new MainWindowViewModel(client);

        viewModel.Start();
        await EventuallyAsync(() => viewModel.StatusDetail.Contains("有効な応答", StringComparison.Ordinal));

        Assert.Equal(["health"], client.Calls);
        Assert.Equal("未取得", viewModel.RemainingPercentText);
    }

    [Fact]
    public async Task HealthyCycleRequestsHealthBeforeStatus()
    {
        var client = new HealthAwareClient(
            HealthFetchResult.Success(new ApiHealthSnapshot("v1", "codex-info")),
            StatusFetchResult.Success(ValidSnapshot()));
        using var viewModel = new MainWindowViewModel(client);

        viewModel.Start();
        await EventuallyAsync(() => viewModel.StatusTitle == "正常");

        Assert.Equal(["health", "status"], client.Calls);
    }

    [Fact]
    public async Task ManualRefreshDoesNotQueueBehindAnActiveRequest()
    {
        var client = new BlockingClient();
        using var viewModel = new MainWindowViewModel(client);

        viewModel.Start();
        await EventuallyAsync(() => client.CallCount == 1);
        Assert.False(viewModel.CanRefresh);

        viewModel.RefreshCommand.Execute(null);
        await Task.Delay(50);
        Assert.Equal(1, client.CallCount);

        client.Complete(ValidSnapshot());
        await EventuallyAsync(() => viewModel.StatusTitle == "正常");
    }

    [Fact]
    public async Task RefreshPublishesModelAndQuotaCollectionsAsOneAtomicReset()
    {
        var firstDetails = DetailsSnapshot(1.25);
        var secondDetails = DetailsSnapshot(2.5);
        using var viewModel = new MainWindowViewModel(
            new SequenceClient(
                StatusFetchResult.Success(ValidSnapshot()),
                StatusFetchResult.Success(ValidSnapshot())),
            new SequenceDetailsClient(
                DetailsFetchResult.Success(firstDetails),
                DetailsFetchResult.Success(secondDetails)));

        viewModel.Start();
        await EventuallyAsync(() => viewModel.HasDetails);

        var modelChanges = 0;
        var quotaChanges = 0;
        NotifyCollectionChangedEventHandler modelHandler = (_, _) => modelChanges++;
        NotifyCollectionChangedEventHandler quotaHandler = (_, _) => quotaChanges++;
        ((INotifyCollectionChanged)viewModel.Models).CollectionChanged += modelHandler;
        ((INotifyCollectionChanged)viewModel.QuotaSegments).CollectionChanged += quotaHandler;

        viewModel.RefreshCommand.Execute(null);
        await EventuallyAsync(() => viewModel.Models[0].InputDollarsText == "$2.50");

        ((INotifyCollectionChanged)viewModel.Models).CollectionChanged -= modelHandler;
        ((INotifyCollectionChanged)viewModel.QuotaSegments).CollectionChanged -= quotaHandler;
        Assert.Equal(1, modelChanges);
        Assert.Equal(1, quotaChanges);
    }

    [Fact]
    public async Task DetailsFailureKeepsTheLastDetailsAndHasIndependentStatus()
    {
        var details = new ApiDetailsSnapshot(
            ApiState.Ready,
            1,
            true,
            "Pro",
            new ApiQuota(98.5, 2, 604800, false),
            [new ApiDetailsModelUsage("SOL", 1, 2, 3, 1.25, 0.25, 0.5)],
            3,
            [],
            [],
            [],
            "概算 $2");
        using var viewModel = new MainWindowViewModel(
            new SequenceClient(StatusFetchResult.Success(ValidSnapshot()), StatusFetchResult.Success(ValidSnapshot())),
            new SequenceDetailsClient(
                DetailsFetchResult.Success(details),
                DetailsFetchResult.FromFailure(DetailsFetchFailure.Response)));

        viewModel.Start();
        await EventuallyAsync(() => viewModel.HasDetails);
        Assert.Equal("ready", viewModel.DetailsStatusAutomationText);
        Assert.Equal("$1.25", viewModel.Models[0].InputDollarsText);

        viewModel.RefreshCommand.Execute(null);
        await EventuallyAsync(() => viewModel.StatusDetail.Contains("前回受信の値", StringComparison.Ordinal));

        Assert.True(viewModel.HasDetails);
        Assert.Equal("error", viewModel.DetailsStatusAutomationText);
        Assert.Equal("$1.25", viewModel.Models[0].InputDollarsText);
        Assert.Contains("前回受信の値", viewModel.StatusDetail, StringComparison.Ordinal);
    }

    [Fact]
    public async Task MismatchedStatusDetailsPairKeepsTheLastCompleteGeneration()
    {
        var completeDetails = new ApiDetailsSnapshot(
            ApiState.Ready, 1, true, "Pro",
            new ApiQuota(98.5, 2, 604800, false),
            [new ApiDetailsModelUsage("SOL", 1, 2, 3, 1.25, 0.25, 0.5)],
            3, [], [], [], "概算 $2");
        var mismatchedDetails = completeDetails with { ObservedAt = 99 };
        using var viewModel = new MainWindowViewModel(
            new SequenceClient(
                StatusFetchResult.Success(ValidSnapshot()),
                StatusFetchResult.Success(ValidSnapshot(observedAt: 2))),
            new SequenceDetailsClient(
                DetailsFetchResult.Success(completeDetails),
                DetailsFetchResult.Success(mismatchedDetails)));

        viewModel.Start();
        await EventuallyAsync(() => viewModel.HasDetails);
        var lastCompleteObservedAt = viewModel.ObservedAtText;
        var lastCompleteDollars = viewModel.Models[0].InputDollarsText;

        viewModel.RefreshCommand.Execute(null);
        await EventuallyAsync(() => viewModel.StatusDetail.Contains("前回受信の値", StringComparison.Ordinal));

        Assert.Equal(lastCompleteObservedAt, viewModel.ObservedAtText);
        Assert.Equal(lastCompleteDollars, viewModel.Models[0].InputDollarsText);
    }

    [Fact]
    public async Task StatusActiveThreadCountRemainsAuthoritativeWhenDetailsRowsAreBounded()
    {
        var details = new ApiDetailsSnapshot(
            ApiState.Ready,
            1,
            true,
            "Pro",
            null,
            [],
            3,
            [],
            [],
            [],
            "概算 —");
        using var viewModel = new MainWindowViewModel(
            new SequenceClient(StatusFetchResult.Success(new ApiStatusSnapshot(
                ApiState.Ready, 1, true, "Pro", null, [], 3))),
            new SequenceDetailsClient(DetailsFetchResult.Success(details)));

        viewModel.Start();
        await EventuallyAsync(() => viewModel.HasDetails);

        // Details rows are intentionally bounded and may contain fewer rows
        // than the scalar status count. The summary must not invent a lower
        // count from the presentation list.
        Assert.Equal(3UL, viewModel.ActiveThreadCount);
        Assert.Contains("3", viewModel.ActiveThreadCountText, StringComparison.Ordinal);
    }

    [Fact]
    public async Task ValidAuthRequiredStatusClearsAccountDetails()
    {
        var details = new ApiDetailsSnapshot(
            ApiState.Ready,
            1,
            true,
            "Pro",
            new ApiQuota(98.5, 2, 604800, false),
            [new ApiDetailsModelUsage("SOL", 1, 2, 3, 1.25, 0.25, 0.5)],
            3,
            [],
            [],
            [],
            "概算 $2");
        using var viewModel = new MainWindowViewModel(
            new SequenceClient(
                StatusFetchResult.Success(ValidSnapshot()),
                StatusFetchResult.Success(new ApiStatusSnapshot(ApiState.AuthRequired, null, false, null, null, [], 0))),
            new SequenceDetailsClient(
                DetailsFetchResult.Success(details),
                DetailsFetchResult.Success(details)));

        viewModel.Start();
        await EventuallyAsync(() => viewModel.HasDetails);
        viewModel.RefreshCommand.Execute(null);
        await EventuallyAsync(() => viewModel.StatusTitle == "Linux 側で認証が必要です");

        Assert.False(viewModel.HasDetails);
        Assert.True(viewModel.HasNoModels);
        Assert.Contains("Linux 側", viewModel.StatusDetail, StringComparison.Ordinal);
    }

    private static ApiStatusSnapshot ValidSnapshot(
        ApiState state = ApiState.Ready,
        long? observedAt = 1,
        string? planLabel = "Pro",
        ApiQuota? quota = null,
        IReadOnlyList<ApiModelUsage>? models = null)
    {
        return new ApiStatusSnapshot(
            state,
            observedAt,
            true,
            planLabel,
            quota ?? new ApiQuota(98.5, 2, 604800, false),
            models ?? [new ApiModelUsage("SOL", 1, 2, 3)],
            3);
    }

    private static ApiDetailsSnapshot DetailsSnapshot(double inputDollars) => new(
        ApiState.Ready,
        1,
        true,
        "Pro",
        new ApiQuota(98.5, 2, 604800, false),
        [new ApiDetailsModelUsage("SOL", 1, 2, 3, inputDollars, 0, 0)],
        3,
        [],
        [],
        [],
        "概算 —");

    private static async Task EventuallyAsync(Func<bool> condition)
    {
        var stopwatch = Stopwatch.StartNew();
        while (!condition())
        {
            if (stopwatch.Elapsed > TimeSpan.FromSeconds(2))
            {
                throw new TimeoutException("Expected presentation state was not reached.");
            }

            await Task.Delay(10);
        }
    }

    private sealed class SequenceClient(params StatusFetchResult[] results) : HealthyStatusClientBase
    {
        private int index;

        public override Task<StatusFetchResult> FetchAsync(CancellationToken cancellationToken = default)
        {
            var result = results[Math.Min(index, results.Length - 1)];
            index++;
            return Task.FromResult(result);
        }
    }

    private sealed class BlockingClient : HealthyStatusClientBase
    {
        private readonly TaskCompletionSource<StatusFetchResult> completion = new(
            TaskCreationOptions.RunContinuationsAsynchronously);

        public int CallCount { get; private set; }

        public override Task<StatusFetchResult> FetchAsync(CancellationToken cancellationToken = default)
        {
            CallCount++;
            return completion.Task.WaitAsync(cancellationToken);
        }

        public void Complete(ApiStatusSnapshot snapshot)
        {
            completion.TrySetResult(StatusFetchResult.Success(snapshot));
        }
    }

    private sealed class HealthAwareClient(
        HealthFetchResult healthResult,
        StatusFetchResult statusResult) : HealthyStatusClientBase
    {
        public List<string> Calls { get; } = [];

        public override Task<HealthFetchResult> FetchHealthAsync(CancellationToken cancellationToken = default)
        {
            Calls.Add("health");
            return Task.FromResult(healthResult);
        }

        public override Task<StatusFetchResult> FetchAsync(CancellationToken cancellationToken = default)
        {
            Calls.Add("status");
            return Task.FromResult(statusResult);
        }
    }

    private sealed class SequenceDetailsClient(params DetailsFetchResult[] results) : ILoopbackDetailsClient
    {
        private int index;

        public Task<DetailsFetchResult> FetchDetailsAsync(CancellationToken cancellationToken = default)
        {
            var result = results[Math.Min(index, results.Length - 1)];
            index++;
            return Task.FromResult(result);
        }
    }

    private sealed class BlockingDetailsClient : ILoopbackDetailsClient
    {
        private readonly TaskCompletionSource<DetailsFetchResult> completion = new(
            TaskCreationOptions.RunContinuationsAsynchronously);

        public int CallCount { get; private set; }

        public Task<DetailsFetchResult> FetchDetailsAsync(CancellationToken cancellationToken = default)
        {
            CallCount++;
            return completion.Task.WaitAsync(cancellationToken);
        }

        public void Complete(DetailsFetchResult result) => completion.TrySetResult(result);
    }

    private sealed class NeverCalledClient : HealthyStatusClientBase
    {
        public override Task<StatusFetchResult> FetchAsync(CancellationToken cancellationToken = default) =>
            throw new InvalidOperationException("The connection start test must not depend on HTTP.");
    }

    private sealed class TestConnectionChildProcessFactory(TestConnectionChildProcess child)
        : IConnectionChildProcessFactory
    {
        public List<ProcessStartInfo> StartInfos { get; } = [];

        public IConnectionChildProcess Create(ProcessStartInfo startInfo)
        {
            StartInfos.Add(startInfo);
            return child;
        }
    }

    private sealed class TestConnectionChildProcess : IConnectionChildProcess
    {
        public event EventHandler? Exited
        {
            add { }
            remove { }
        }
        public bool HasExited { get; private set; }

        public bool Start() => true;
        public void Kill() => HasExited = true;
        public void WaitForExit(int milliseconds) { }
        public void Dispose() { }
    }
}
