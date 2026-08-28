// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Diagnostics;
using CodexInfo.WindowsClient.Core;
using CodexInfo.WindowsClient.Updates;
using CodexInfo.WindowsClient.ViewModels;
using Xunit;

namespace CodexInfo.WindowsClient.Presentation.Tests;

public sealed class PreviewEnvironmentTests
{
    [Theory]
    [InlineData("700x480", 700, 480)]
    [InlineData(" 1200x900 ", 1200, 900)]
    public void PreviewSizeParserAcceptsBoundedIntegerSizes(string text, double width, double height)
    {
        Assert.True(CodexInfo.WindowsClient.PreviewEnvironment.TryParseSize(text, out var actualWidth, out var actualHeight));
        Assert.Equal(width, actualWidth);
        Assert.Equal(height, actualHeight);
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("700")]
    [InlineData("319x480")]
    [InlineData("700x239")]
    [InlineData("700X480")]
    public void PreviewSizeParserRejectsMalformedOrUnsafeSizes(string? text)
    {
        Assert.False(CodexInfo.WindowsClient.PreviewEnvironment.TryParseSize(text, out _, out _));
    }

    [Theory]
    [InlineData(null, 6)]
    [InlineData("", 6)]
    [InlineData("0", 0)]
    [InlineData("6", 6)]
    [InlineData("7", 7)]
    [InlineData("8", 6)]
    [InlineData("not-a-number", 6)]
    public void PreviewThreadCountIsBoundedToTheSixAndSevenRowAcceptanceFixtures(string? text, int expected)
    {
        Assert.Equal(expected, CodexInfo.WindowsClient.PreviewEnvironment.ParseThreadCount(text));
    }

    [Fact]
    public async Task GraphThreadsAndLegalPreviewFixtureHasCompleteAuthenticatedData()
    {
        using var client = new CodexInfo.WindowsClient.PreviewLoopbackClient();
        var status = await client.FetchAsync();
        var details = await client.FetchDetailsAsync();

        Assert.True(status.IsSuccess);
        Assert.True(status.Snapshot!.Authenticated);
        Assert.Equal(ApiState.Ready, status.Snapshot.State);
        Assert.Equal(6UL, status.Snapshot.ActiveThreadCount);
        Assert.Equal(3, status.Snapshot.Models.Count);

        Assert.True(details.IsSuccess);
        Assert.Equal(2, details.Snapshot!.HistoryPeriods.Count);
        Assert.Equal(3, details.Snapshot.HistoryPeriods.Single(period => period.Current).Samples.Count);
        Assert.Equal(2, details.Snapshot.HistoryPeriods.Single(period => !period.Current).Samples.Count);
        Assert.Equal(6, details.Snapshot.Threads.Count);
        Assert.Contains(details.Snapshot.Threads, thread => thread.IsOrphan);
        Assert.Contains(details.Snapshot.Threads, thread => thread.ParentId == "preview-root");
        Assert.True(details.Snapshot.EstimatedCostLabel.Length > 0);
    }

    [Fact]
    public async Task PreviewQuotaGaugeUsesTheExactHalfPeriodAcceptanceBoundary()
    {
        using var client = new CodexInfo.WindowsClient.PreviewLoopbackClient();
        var status = await client.FetchAsync();

        var quota = Assert.IsType<ApiQuota>(status.Snapshot!.Quota);
        var remainingSeconds = quota.ResetAt - status.Snapshot.ObservedAt;
        Assert.Equal(quota.WindowSeconds / 2, remainingSeconds);
    }

    [Theory]
    [InlineData("normal", 48d, false)]
    [InlineData("update", 48d, true)]
    [InlineData("warning", 10d, false)]
    [InlineData("danger", 2d, false)]
    [InlineData("zero", 0d, false)]
    [InlineData("full", 100d, false)]
    public async Task PreviewReadyScenariosCommitOneCompletePublishedPairGeneration(
        string scenario,
        double expectedRemaining,
        bool expectedUpdate)
    {
        await WithPreviewScenarioAsync(scenario, async () =>
        {
            using var client = new CodexInfo.WindowsClient.PreviewLoopbackClient();
            using var coordinator = new PreviewUpdateCoordinatorForTests(expectedUpdate);
            using var viewModel = new MainWindowViewModel(
                client,
                client,
                updateCoordinator: coordinator);

            var status = await client.FetchAsync(CancellationToken.None);
            var details = await client.FetchDetailsAsync(CancellationToken.None);
            Assert.True(status.IsSuccess);
            Assert.True(details.IsSuccess);
            Assert.NotNull(status.Snapshot!.PublishedPair);
            Assert.Equal(status.Snapshot.PublishedPair, details.Snapshot!.PublishedPair);

            viewModel.Start();
            await EventuallyAsync(() => viewModel.IsAuthenticated &&
                !viewModel.IsStartupLoading &&
                viewModel.DetailsStatusAutomationText == "ready");
            if (expectedUpdate)
            {
                await EventuallyAsync(() => viewModel.IsUpdateNotificationVisible &&
                    viewModel.IsUpdateActionVisible);
            }
            else
            {
                await EventuallyAsync(() => viewModel.Update is not null &&
                    viewModel.Update.AvailableVersion is null);
            }

            Assert.True(viewModel.ShowAuthenticatedContent);
            Assert.True(viewModel.HasDetails);
            Assert.Equal(expectedRemaining, viewModel.RemainingPercentValue);
            Assert.Equal(status.Snapshot.PublishedPair, viewModel.DetailsSnapshot!.PublishedPair);
            Assert.NotEmpty(viewModel.Models);
            Assert.Equal(expectedUpdate, viewModel.IsUpdateNotificationVisible);
        });
    }

    [Theory]
    [InlineData("auth", ApiState.AuthRequired)]
    [InlineData("error", ApiState.Error)]
    public async Task PreviewAuthAndErrorKeepStatusOnlySemantics(string scenario, ApiState expectedState)
    {
        await WithPreviewScenarioAsync(scenario, async () =>
        {
            using var client = new CodexInfo.WindowsClient.PreviewLoopbackClient();
            using var coordinator = new PreviewUpdateCoordinatorForTests(available: true);
            using var viewModel = new MainWindowViewModel(
                client,
                client,
                updateCoordinator: coordinator);

            viewModel.Start();
            await EventuallyAsync(() => !viewModel.IsStartupLoading);

            var status = await client.FetchAsync(CancellationToken.None);
            Assert.Equal(expectedState, status.Snapshot!.State);
            Assert.False(status.Snapshot.Authenticated);
            Assert.False(viewModel.IsAuthenticated);
            Assert.False(viewModel.ShowAuthenticatedContent);
            Assert.False(viewModel.HasDetails);
            Assert.Empty(viewModel.Models);
            // Auth/error are status-only visibility transitions; the
            // validated status quota remains available while account-scoped
            // details/models are cleared.
            Assert.Equal(48d, viewModel.RemainingPercentValue);
            Assert.Equal("48%", viewModel.RemainingPercentText);
            Assert.False(viewModel.IsUpdateNotificationVisible);
            if (expectedState == ApiState.AuthRequired)
            {
                Assert.True(viewModel.IsAuthRequired);
            }
            else
            {
                Assert.True(viewModel.IsRetryVisible);
            }
        });
    }

    private static async Task WithPreviewScenarioAsync(string scenario, Func<Task> action)
    {
        var original = Environment.GetEnvironmentVariable("CODEX_INFO_WINDOWS_PREVIEW");
        Environment.SetEnvironmentVariable("CODEX_INFO_WINDOWS_PREVIEW", scenario);
        try
        {
            await action();
        }
        finally
        {
            Environment.SetEnvironmentVariable("CODEX_INFO_WINDOWS_PREVIEW", original);
        }
    }

    private static async Task EventuallyAsync(Func<bool> condition)
    {
        var stopwatch = Stopwatch.StartNew();
        while (!condition())
        {
            if (stopwatch.Elapsed > TimeSpan.FromSeconds(2))
            {
                throw new TimeoutException("Expected preview presentation state was not reached.");
            }

            await Task.Delay(10);
        }
    }

    private sealed class PreviewUpdateCoordinatorForTests(bool available) : IWindowsUpdateCoordinator
    {
        public Task<UpdateCheckResult> CheckAsync(CancellationToken cancellationToken = default) =>
            Task.FromResult(new UpdateCheckResult(available ? "1.1.0" : null, false));

        public Task<UpdateStartStatus> StartAvailableUpdateAsync(CancellationToken cancellationToken = default) =>
            Task.FromResult(UpdateStartStatus.LaunchFailed);

        public void Dispose()
        {
        }
    }
}
