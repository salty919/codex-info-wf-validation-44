// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Diagnostics;
using System.Globalization;
using CodexInfo.WindowsClient.Core;
using CodexInfo.WindowsClient.Localization;
using CodexInfo.WindowsClient.Settings;
using CodexInfo.WindowsClient.ViewModels;
using Xunit;

namespace CodexInfo.WindowsClient.Presentation.Tests;

public sealed class DetailsWindowViewModelTests
{
    [Fact]
    public void SettingsAreAtomicAndCorruptFilesFallBackWithoutCredentials()
    {
        var root = Directory.CreateTempSubdirectory("codex-info-settings-test");
        try
        {
            var path = Path.Combine(root.FullName, "settings.json");
            var store = new ClientSettingsStore(path);
            var expected = new ClientSettings("en", true) { ConnectionConfigured = true, TimeZoneId = "UTC" };
            store.Save(expected);
            Assert.Equal(expected, store.Load());
            Assert.False(File.Exists(path + ".tmp"));

            File.WriteAllText(path, "{not-json}");
            var corrupt = store.Load();
            Assert.True(corrupt.SettingsCorrupt);
            Assert.False(corrupt.ConnectionConfigured);
            Assert.False(corrupt.SetupCompleted);
            Assert.DoesNotContain("SettingsCorrupt", File.ReadAllText(path));

            foreach (var malformed in new[] { "", "{\"Language\":", "[]", "null" })
            {
                File.WriteAllText(path, malformed);
                Assert.True(store.Load().SettingsCorrupt);
                Assert.False(store.Load().ConnectionConfigured);
                Assert.False(store.Load().SetupCompleted);
            }

            File.WriteAllText(path, "{\"language\":\"xx\",\"setupCompleted\":true,\"connectionConfigured\":false,\"timeZoneId\":\"remote\",\"connectionProfile\":\"none\",\"connectionSelector\":\"none\"}");
            var normalized = store.Load();
            Assert.Equal("en", normalized.Language);
            Assert.Equal("local", normalized.TimeZoneId);
            Assert.False(normalized.SettingsCorrupt);
        }
        finally
        {
            root.Delete(recursive: true);
        }
    }

    [Fact]
    public void SettingsPersist_exactly_six_non_secret_keys_and_reject_invalid_profiles()
    {
        var root = Directory.CreateTempSubdirectory("codex-info-settings-shape-test");
        try
        {
            var path = Path.Combine(root.FullName, "settings.json");
            var store = new ClientSettingsStore(path);
            store.Save(new ClientSettings("ja", true)
            {
                ConnectionConfigured = true,
                TimeZoneId = "UTC",
                ConnectionProfile = ConnectionProfiles.SshConfigAlias,
                ConnectionSelector = "work.example",
            });

            using var document = System.Text.Json.JsonDocument.Parse(File.ReadAllText(path));
            Assert.Equal(
                ["connectionConfigured", "connectionProfile", "connectionSelector", "language", "setupCompleted", "timeZoneId"],
                document.RootElement.EnumerateObject().Select(property => property.Name).OrderBy(name => name));
            Assert.DoesNotContain("host", File.ReadAllText(path), StringComparison.OrdinalIgnoreCase);
            Assert.DoesNotContain("password", File.ReadAllText(path), StringComparison.OrdinalIgnoreCase);
            Assert.Equal("work.example", store.Load().ConnectionSelector);

            Assert.Throws<ArgumentException>(() => store.Save(new ClientSettings("ja", true)
            {
                ConnectionProfile = ConnectionProfiles.SshConfigAlias,
                ConnectionSelector = "user@host",
            }));
        }
        finally
        {
            root.Delete(recursive: true);
        }
    }

    [Fact]
    public void Automatic_connection_commands_are_direct_and_use_validated_tokens()
    {
        var ssh = ConnectionProcessFactory.BuildAutomaticSsh("work.example");
        Assert.Equal("ssh.exe", ssh.FileName);
        Assert.False(ssh.UseShellExecute);
        Assert.True(ssh.CreateNoWindow);
        Assert.Equal(["-o", "BatchMode=yes", "-N", "-L", "8787:127.0.0.1:8787", "work.example"], ssh.ArgumentList);

        var wsl = ConnectionProcessFactory.BuildAutomaticWsl("Ubuntu-24.04");
        Assert.Equal("wsl.exe", wsl.FileName);
        Assert.False(wsl.UseShellExecute);
        Assert.Equal(["--distribution", "Ubuntu-24.04", "--", "env", "CODEX_INFO_API_LISTEN=127.0.0.1:8787", "./run.sh"], wsl.ArgumentList);
        Assert.Throws<ArgumentException>(() => ConnectionProcessFactory.BuildAutomaticSsh("user@host"));
    }

    [Fact]
    public void ConnectionConfigured_marker_survives_without_persisting_an_endpoint()
    {
        var root = Directory.CreateTempSubdirectory("codex-info-settings-connection-test");
        try
        {
            var path = Path.Combine(root.FullName, "settings.json");
            var store = new ClientSettingsStore(path);
            store.Save(new ClientSettings("ja", false) { ConnectionConfigured = true });

            var raw = File.ReadAllText(path);
            Assert.Contains("connectionConfigured", raw, StringComparison.Ordinal);
            Assert.DoesNotContain("ssh", raw, StringComparison.OrdinalIgnoreCase);
            Assert.True(store.Load().ConnectionConfigured);
            Assert.False(store.Load().SetupCompleted);
        }
        finally
        {
            root.Delete(recursive: true);
        }
    }

    [Fact]
    public void JapaneseDurationsOmitZeroUnitsAndUseExplicitElapsedFallback()
    {
        var text = LocalizationService.Current;

        Assert.Equal("残り 2日 3時間 4分", text.FormatRemaining(2, 3, 4));
        Assert.Equal("残り 2日", text.FormatRemaining(2, 0, 0));
        Assert.Equal("残り 3時間", text.FormatRemaining(0, 3, 0));
        Assert.Equal("残り 4分", text.FormatRemaining(0, 0, 4));
        Assert.EndsWith("—", text.FormatElapsed(null, text.Elapsed), StringComparison.Ordinal);
    }

    [Fact]
    public void UnknownLanguageSettingsUseTheDeterministicEnglishFallback()
    {
        Assert.Equal("en", LocalizationService.NormalizeLanguageCode("xx-YY"));
        Assert.Equal("ja", LocalizationService.NormalizeLanguageCode(null));
        Assert.Equal("zh-Hans", LocalizationService.NormalizeLanguageCode("ZH-HANS"));
        Assert.Equal("en", LocalizationService.NormalizeLanguageCode("en-US"));
    }

    [Fact]
    public void ModelNumbersAreReformattedFromRawValuesWhenCultureChanges()
    {
        var previousCulture = CultureInfo.CurrentCulture;
        try
        {
            CultureInfo.CurrentCulture = CultureInfo.GetCultureInfo("en-US");
            using var model = new ModelUsageViewModel(
                new ApiDetailsModelUsage("SOL", 12_345, 678, 9, 1.25, 0.5, 0.01));
            Assert.Equal("12,345", model.InputTokensText);
            Assert.Equal("$1.25", model.InputDollarsText);

            CultureInfo.CurrentCulture = CultureInfo.GetCultureInfo("de-DE");
            Assert.Equal("12.345", model.InputTokensText);
            Assert.Equal("$1,25", model.InputDollarsText);
        }
        finally
        {
            CultureInfo.CurrentCulture = previousCulture;
        }
    }

    [Fact]
    public async Task GraphKeepsPeriodsAndSwitchesDollarAndTokenSeries()
    {
        var resetAt = DateTimeOffset.UtcNow.AddDays(3).ToUnixTimeSeconds();
        var observedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        var period = new ApiHistoryPeriod("current", resetAt - 604800, resetAt, true, "current")
        {
            Samples =
            [
                new ApiHistorySample(observedAt - 120, resetAt, 80, 1.25, 2.5, 3.75, 100, 200, 300),
                new ApiHistorySample(observedAt - 60, resetAt, 75, 1.5, 3, 4.5, 120, 240, 360),
            ],
        };
        var details = new ApiDetailsSnapshot(
            ApiState.Ready, resetAt - 30, true, "Pro",
            new ApiQuota(75, resetAt, 604800, false), [], 0,
            [period], period.Samples, [], "estimated");

        using var main = new MainWindowViewModel(
            new SingleStatusClient(StatusFetchResult.Success(ValidStatus(resetAt))),
            new SingleDetailsClient(DetailsFetchResult.Success(details)));
        main.Start();
        await EventuallyAsync(() => main.HasDetails);

        using var graph = new GraphWindowViewModel(main);
        Assert.Single(graph.Periods);
        Assert.Equal(4, graph.Points.Count);
        Assert.Equal(0, graph.Points[0].LunaValue);
        Assert.Equal(3.75, graph.Points[1].LunaValue);
        Assert.Equal(4.5, graph.Points[2].LunaValue);
        Assert.Equal(4.5, graph.Points[3].LunaValue);

        graph.SelectedMetric = graph.Texts.Tokens;
        Assert.Equal(0, graph.Points[0].LunaValue);
        Assert.Equal(300, graph.Points[1].LunaValue);
        Assert.Equal(360, graph.Points[2].LunaValue);
        Assert.Equal(360, graph.Points[3].LunaValue);
        graph.ShowLuna = false;
        Assert.False(graph.ShowLuna);
    }

    [Fact]
    public async Task ThreadsAreParentFirstAndRetainOrphanAndContextFields()
    {
        var threads = new List<ApiThreadDetails>
        {
            new("child", "child", "root", "gpt-5.6-luna", "LUNA", 500, 100, 1000, 10, 20, true, 1, false),
            new("orphan", "orphan", "missing", "gpt-5.6-sol", "SOL", null, null, null, null, null, true, null, true),
            new("root", "root", null, "gpt-5.6-terra", "TERRA", 900, 200, 1000, 10, 30, false, 0, false),
        };
        var details = new ApiDetailsSnapshot(
            ApiState.Ready, 100, true, "Pro", null, [], 3, [], [], threads, "estimated");
        using var main = new MainWindowViewModel(
            new SingleStatusClient(StatusFetchResult.Success(ValidStatus(100))),
            new SingleDetailsClient(DetailsFetchResult.Success(details)));
        main.Start();
        await EventuallyAsync(() => main.HasDetails);

        using var viewModel = new ThreadsWindowViewModel(main);
        Assert.Equal(["orphan", "root", "child"], viewModel.Threads.Select(item => item.Id));
        Assert.Contains("900", viewModel.Threads[1].TokenText, StringComparison.Ordinal);
        Assert.Contains("500", viewModel.Threads[2].TokenText, StringComparison.Ordinal);
        Assert.False(viewModel.Threads[0].ConnectedToParent);
        Assert.Contains("missing", viewModel.Threads[0].ParentText, StringComparison.Ordinal);
    }

    [Fact]
    public async Task SetupCannotCompleteUntilAuthenticatedSnapshotIsAccepted()
    {
        var resetAt = DateTimeOffset.UtcNow.AddDays(3).ToUnixTimeSeconds();
        var client = new ToggleStatusClient(
            StatusFetchResult.Success(new ApiStatusSnapshot(
                ApiState.AuthRequired, resetAt - 30, false, null, null, [], 0)));
        using var main = new MainWindowViewModel(client);
        main.Start();
        await EventuallyAsync(() => main.IsAuthRequired);

        using var setup = new SetupViewModel(main);
        Assert.True(setup.CanContinue);
        setup.Continue();
        Assert.True(setup.IsAuthStep);
        Assert.False(setup.CanContinue);
        setup.Continue();
        Assert.True(setup.IsAuthStep);

        client.Result = StatusFetchResult.Success(new ApiStatusSnapshot(
            ApiState.Ready, resetAt - 10, true, "Pro",
            new ApiQuota(75, resetAt, 604800, false), [], 0));
        main.RefreshCommand.Execute(null);
        await EventuallyAsync(() => main.IsAuthenticated);
        Assert.True(setup.CanContinue);
        setup.Continue();
        Assert.True(setup.IsDoneStep);
    }

    private static ApiStatusSnapshot ValidStatus(long resetAt) => new(
        ApiState.Ready, 100, true, "Pro", new ApiQuota(75, resetAt, 604800, false), [], 0);

    private static async Task EventuallyAsync(Func<bool> condition)
    {
        var stopwatch = Stopwatch.StartNew();
        while (!condition())
        {
            if (stopwatch.Elapsed > TimeSpan.FromSeconds(2))
            {
                throw new TimeoutException("Expected details state was not reached.");
            }

            await Task.Delay(10);
        }
    }

    private sealed class SingleStatusClient(StatusFetchResult result) : ILoopbackStatusClient
    {
        public Task<StatusFetchResult> FetchAsync(CancellationToken cancellationToken = default) => Task.FromResult(result);
    }

    private sealed class SingleDetailsClient(DetailsFetchResult result) : ILoopbackDetailsClient
    {
        public Task<DetailsFetchResult> FetchDetailsAsync(CancellationToken cancellationToken = default) => Task.FromResult(result);
    }

    private sealed class ToggleStatusClient(StatusFetchResult initial) : ILoopbackStatusClient
    {
        public StatusFetchResult Result { get; set; } = initial;
        public Task<StatusFetchResult> FetchAsync(CancellationToken cancellationToken = default) => Task.FromResult(Result);
    }
}
