// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Diagnostics;
using CodexInfo.WindowsClient;
using CodexInfo.WindowsClient.Core;
using CodexInfo.WindowsClient.Graphing;
using CodexInfo.WindowsClient.Localization;
using CodexInfo.WindowsClient.Settings;
using CodexInfo.WindowsClient.ViewModels;
using Xunit;

[assembly: CollectionBehavior(DisableTestParallelization = true)]

namespace CodexInfo.WindowsClient.Presentation.Tests;

public sealed class PresentationBoundaryTests
{
    [Fact]
    public void UiTextRemainingBoundariesHaveExplicitJapaneseMeaning()
    {
        var text = LocalizationService.Languages.Single(language => language.LanguageCode == "ja");

        Assert.Equal("まもなくリセット", text.FormatRemaining(0, 0, 0));
        Assert.Equal("まもなくリセット", text.FormatRemaining(0, 0, 0, immediate: true));
        Assert.Equal("残り 1分未満", text.FormatRemaining(0, 0, 0, lessThanMinute: true));
        Assert.Equal("残り 2日 3時間 4分", text.FormatRemaining(2, 3, 4));
        Assert.Equal("残り 2日", text.FormatRemaining(2, 0, 0));
    }

    [Fact]
    public void UiTextStatusDetailsDistinguishLaunchFailureAndRetainedSnapshot()
    {
        var text = LocalizationService.Languages.Single(language => language.LanguageCode == "en");

        Assert.Contains("Could not start authentication", text.StatusDetailFor("AuthRequired", true, false), StringComparison.Ordinal);
        Assert.Contains("Start Linux authentication", text.StatusDetailFor("AuthRequired", false, false), StringComparison.Ordinal);
        Assert.DoesNotContain("last received", text.StatusDetailFor("TransportError", false, false), StringComparison.OrdinalIgnoreCase);
        Assert.Contains("last received", text.StatusDetailFor("TransportError", false, true), StringComparison.OrdinalIgnoreCase);
        Assert.Contains("valid response", text.StatusDetailFor("ResponseError", false, false), StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void UiTextCatalogIsUniqueAndEveryLocaleHasCoreLabels()
    {
        var languages = LocalizationService.Languages;

        Assert.Equal(languages.Count, languages.Select(language => language.LanguageCode).Distinct(StringComparer.Ordinal).Count());
        Assert.All(languages, language =>
        {
            Assert.False(string.IsNullOrWhiteSpace(language.LanguageName));
            Assert.False(string.IsNullOrWhiteSpace(language.UsageStatus));
            Assert.False(string.IsNullOrWhiteSpace(language.Refresh));
            Assert.False(string.IsNullOrWhiteSpace(language.UnavailableValue));
            Assert.False(string.IsNullOrWhiteSpace(language.ConnectionEndpoint));
        });
    }

    [Fact]
    public void SettingsViewModelSaveNormalizesLocaleTimezoneAndRaisesSaved()
    {
        var originalSettings = App.CurrentSettings;
        var originalLanguage = LocalizationService.Current.LanguageCode;
        var originalTimeZone = LocalizationService.DisplayTimeZone.Id;
        var root = Directory.CreateTempSubdirectory("codex-info-settings-vm-test");
        SettingsViewModel? viewModel = null;
        try
        {
            var path = Path.Combine(root.FullName, "settings.json");
            var store = new ClientSettingsStore(path);
            store.Save(new ClientSettings("ja", true));
            App.CurrentSettings = store.Load();
            LocalizationService.SetLanguage("ja");
            LocalizationService.SetTimeZone("local");

            viewModel = new SettingsViewModel(store);
            var savedCount = 0;
            viewModel.Saved += (_, _) => savedCount++;
            viewModel.SelectedLanguageCode = "en-US";
            viewModel.SelectedTimeZoneId = "utc";

            viewModel.Save();

            var loaded = store.Load();
            Assert.Equal("en", loaded.Language);
            Assert.Equal("UTC", loaded.TimeZoneId);
            Assert.Equal(loaded, App.CurrentSettings);
            Assert.Equal(1, savedCount);
            Assert.Equal("en", LocalizationService.Current.LanguageCode);
            Assert.Equal("UTC", LocalizationService.DisplayTimeZone.Id);
        }
        finally
        {
            viewModel?.Dispose();
            App.CurrentSettings = originalSettings;
            LocalizationService.SetLanguage(originalLanguage);
            LocalizationService.SetTimeZone(string.Equals(originalTimeZone, "UTC", StringComparison.OrdinalIgnoreCase) ? "UTC" : "local");
            root.Delete(recursive: true);
        }
    }

    [Fact]
    public async Task SettingsViewModelMirrorsMainAuthenticationCapability()
    {
        var auth = new ApiStatusSnapshot(ApiState.AuthRequired, 1, false, null, null, [], 0);
        var ready = ValidStatus(DateTimeOffset.UtcNow.AddHours(2).ToUnixTimeSeconds());
        var client = new SequenceStatusClient(StatusFetchResult.Success(auth), StatusFetchResult.Success(ready));
        using var main = new MainWindowViewModel(client);
        var root = Directory.CreateTempSubdirectory("codex-info-settings-status-test");
        SettingsViewModel? settings = null;
        try
        {
            settings = new SettingsViewModel(new ClientSettingsStore(Path.Combine(root.FullName, "settings.json")), main);
            main.Start();
            await EventuallyAsync(() => main.IsAuthRequired);

            Assert.True(settings.CanAuthenticate);
            Assert.Equal(main.StatusTitle, settings.StatusTitle);
            Assert.Equal(main.StatusDetail, settings.StatusDetail);

            main.RefreshCommand.Execute(null);
            await EventuallyAsync(() => main.IsAuthenticated);
            Assert.False(settings.CanAuthenticate);
            Assert.Equal(main.StatusTitle, settings.StatusTitle);
        }
        finally
        {
            settings?.Dispose();
            root.Delete(recursive: true);
        }
    }

    [Fact]
    public async Task SetupNoneProfileRequiresObservedConnectionAndBuildsFailClosedSettings()
    {
        var auth = new ApiStatusSnapshot(ApiState.AuthRequired, 1, false, null, null, [], 0);
        using var main = new MainWindowViewModel(new SequenceStatusClient(StatusFetchResult.Success(auth)));
        var originalSettings = App.CurrentSettings;
        try
        {
            App.CurrentSettings = ClientSettings.Default;
            main.Start();
            await EventuallyAsync(() => main.IsAuthRequired);

            using var setup = new SetupViewModel(main);
            Assert.Equal(ConnectionProfiles.None, setup.SelectedConnectionProfile);
            Assert.Equal([ConnectionSelectors.None], setup.ConnectionSelectorOptions);
            Assert.True(setup.IsConnectionSelectionValid);
            Assert.True(setup.CanContinue);

            var configured = setup.BuildSettings(ClientSettings.Default);
            Assert.True(configured.ConnectionConfigured);
            Assert.Equal(ConnectionProfiles.None, configured.ConnectionProfile);
            Assert.Equal(ConnectionSelectors.None, configured.ConnectionSelector);

            // Transient SSH input must not silently become a durable selector
            // for the none profile.
            setup.SshHost = "linux.example";
            setup.SshUser = "salty";
            Assert.False(setup.IsConnectionSelectionValid);
            var rejected = setup.BuildSettings(ClientSettings.Default);
            Assert.False(rejected.ConnectionConfigured);
            Assert.Equal(ConnectionProfiles.None, rejected.ConnectionProfile);
            Assert.Equal(ConnectionSelectors.None, rejected.ConnectionSelector);
        }
        finally
        {
            App.CurrentSettings = originalSettings;
        }
    }

    [Fact]
    public async Task SetupSshInputUsesBoundedSafeHostAndUserGrammar()
    {
        var auth = new ApiStatusSnapshot(ApiState.AuthRequired, 1, false, null, null, [], 0);
        using var main = new MainWindowViewModel(new SequenceStatusClient(StatusFetchResult.Success(auth)));
        main.Start();
        await EventuallyAsync(() => main.IsAuthRequired);

        using var setup = new SetupViewModel(main);
        setup.SshHost = new string('h', 255);
        Assert.True(setup.CanStartSsh);
        setup.SshHost = new string('h', 256);
        Assert.False(setup.CanStartSsh);

        setup.SshHost = "linux.example";
        setup.SshUser = new string('u', 128);
        Assert.True(setup.CanStartSsh);
        setup.SshUser = new string('u', 129);
        Assert.False(setup.CanStartSsh);

        setup.SshUser = "salty";
        setup.SshHost = "host;whoami";
        Assert.False(setup.CanStartSsh);
        Assert.Contains("user@linux-host", setup.SshCommand, StringComparison.Ordinal);
    }

    [Fact]
    public async Task MainQuotaSegmentsExposeSevenBoundedCellsAndCurrentPeriod()
    {
        var now = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        var quota = new ApiQuota(40, now + 500, 1_000, false);
        using var viewModel = new MainWindowViewModel(new SequenceStatusClient(
            StatusFetchResult.Success(new ApiStatusSnapshot(ApiState.Ready, now, true, "Pro", quota, [], 0))));

        viewModel.Start();
        await EventuallyAsync(() => viewModel.IsAuthenticated);

        Assert.Equal(7, viewModel.QuotaSegments.Count);
        Assert.All(viewModel.QuotaSegments, segment => Assert.InRange(segment.Fill, 0, 1));
        Assert.True(viewModel.QuotaSegments[0].Fill > 0.95);
        Assert.True(viewModel.QuotaSegments[3].Fill is > 0 and < 1);
        Assert.Equal(0, viewModel.QuotaSegments[4].Fill);
        Assert.InRange(viewModel.QuotaRemainingPeriodValue, 45, 55);
        Assert.Equal(LocalizationService.Current.WeeklyQuota, viewModel.QuotaWindowText);
    }

    [Fact]
    public async Task MainDetailsFailureDoesNotPublishAPartialAccountGeneration()
    {
        var status = ValidStatus(DateTimeOffset.UtcNow.AddHours(2).ToUnixTimeSeconds());
        using var viewModel = new MainWindowViewModel(
            new SequenceStatusClient(StatusFetchResult.Success(status)),
            new SequenceDetailsClient(DetailsFetchResult.FromFailure(DetailsFetchFailure.Response)));

        viewModel.Start();
        await EventuallyAsync(() => viewModel.StatusTitle == LocalizationService.Current.Unavailable);

        Assert.False(viewModel.IsAuthenticated);
        Assert.False(viewModel.HasDetails);
        Assert.True(viewModel.HasNoModels);
        Assert.Equal(LocalizationService.Current.UnavailableValue, viewModel.RemainingPercentText);
    }

    [Fact]
    public async Task MainActivitySummaryClassifiesExactlyOneKnownModelToken()
    {
        var threads = new ApiThreadDetails[]
        {
            new("sol", "sol", null, "gpt-sol", "SOL", null, null, null, null, null, false, null, false),
            new("terra", "terra", null, "gpt-terra", "TERRA", null, null, null, null, null, false, null, false),
            new("luna", "luna", null, "gpt-luna", "LUNA", null, null, null, null, null, false, null, false),
            new("other", "other", null, "gpt-sol-terra", "SOL TERRA", null, null, null, null, null, false, null, false),
        };
        var details = new ApiDetailsSnapshot(ApiState.Ready, 1, true, "Pro", null, [], 4, [], [], threads, "概算 —");
        using var viewModel = new MainWindowViewModel(
            new SequenceStatusClient(StatusFetchResult.Success(new ApiStatusSnapshot(ApiState.Ready, 1, true, "Pro", null, [], 4))),
            new SequenceDetailsClient(DetailsFetchResult.Success(details)));

        viewModel.Start();
        await EventuallyAsync(() => viewModel.HasDetails);

        Assert.Equal(4UL, viewModel.ActiveThreadCount);
        Assert.Equal(1, viewModel.ActiveSolCount);
        Assert.Equal(1, viewModel.ActiveTerraCount);
        Assert.Equal(1, viewModel.ActiveLunaCount);
        Assert.Equal(1, viewModel.ActiveOtherCount);
    }

    [Fact]
    public void GraphPointFormatsMissingQuotaAndBothMetricFamilies()
    {
        var sample = new ApiHistorySample(100, 200, null, 1.25, 2, 3, 100, 200, 300);

        var dollars = new GraphPointViewModel(sample, GraphMetric.Dollars);
        var tokens = new GraphPointViewModel(sample, GraphMetric.Tokens);

        Assert.Null(dollars.RemainingPercent);
        Assert.Contains("—", dollars.RemainingText, StringComparison.Ordinal);
        Assert.Equal(1.25, dollars.SolValue);
        Assert.Contains("$1.25", dollars.ModelsText, StringComparison.Ordinal);
        Assert.Equal(100, tokens.SolValue);
        Assert.Contains("SOL 100", tokens.ModelsText, StringComparison.Ordinal);
    }

    [Fact]
    public void GraphReductionPreservesEndpointsAndRejectsAnUnrenderableBudget()
    {
        var samples = Enumerable.Range(0, 5)
            .Select(index => new ApiHistorySample(index + 1, 10, 100 - index, index, index * 2, index * 3, (ulong)index, (ulong)(index * 2), (ulong)(index * 3)))
            .ToArray();

        var reduced = GraphWindowViewModel.ReduceGraphSamples(samples, 3);

        Assert.Equal(3, reduced.Count);
        Assert.Equal(samples[0], reduced[0]);
        Assert.Equal(samples[^1], reduced[^1]);
        Assert.Throws<ArgumentOutOfRangeException>(() => GraphWindowViewModel.ReduceGraphSamples(samples, 1));
    }

    [Fact]
    public async Task ThreadPresentationUsesExplicitFallbacksForMissingContextTokensAndParent()
    {
        var thread = new ApiThreadDetails(
            "orphan",
            "orphan task",
            null,
            "gpt-5.6-luna",
            "",
            null,
            null,
            null,
            null,
            null,
            true,
            null,
            true);
        var details = new ApiDetailsSnapshot(ApiState.Ready, 1, true, "Pro", null, [], 1, [], [], [thread], "概算 —");
        using var main = new MainWindowViewModel(
            new SequenceStatusClient(StatusFetchResult.Success(new ApiStatusSnapshot(ApiState.Ready, 1, true, "Pro", null, [], 1))),
            new SequenceDetailsClient(DetailsFetchResult.Success(details)));
        main.Start();
        await EventuallyAsync(() => main.HasDetails);

        using var viewModel = new ThreadsWindowViewModel(main);
        var item = Assert.Single(viewModel.Threads);

        Assert.Contains(LocalizationService.Current.SubThread, item.RoleText, StringComparison.Ordinal);
        Assert.Contains(LocalizationService.Current.ParentUnavailable, item.ParentText, StringComparison.Ordinal);
        Assert.Equal("gpt-5.6-luna", item.ModelText);
        Assert.EndsWith("—", item.ContextText, StringComparison.Ordinal);
        Assert.EndsWith("—", item.TokenText, StringComparison.Ordinal);
        Assert.EndsWith("—", item.AgeText, StringComparison.Ordinal);
        Assert.EndsWith("—", item.InstructionAgeText, StringComparison.Ordinal);
    }

    private static ApiStatusSnapshot ValidStatus(long observedAt) => new(
        ApiState.Ready,
        observedAt,
        true,
        "Pro",
        new ApiQuota(75, observedAt + 604_800, 604_800, false),
        [],
        0);

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

    private sealed class SequenceStatusClient(params StatusFetchResult[] results) : ILoopbackStatusClient
    {
        private int index;

        public Task<StatusFetchResult> FetchAsync(CancellationToken cancellationToken = default)
        {
            var result = results[Math.Min(index, results.Length - 1)];
            index++;
            return Task.FromResult(result);
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
}
