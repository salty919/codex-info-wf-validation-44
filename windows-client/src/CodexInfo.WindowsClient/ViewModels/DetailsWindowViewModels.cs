// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Globalization;
using System.Runtime.CompilerServices;
using CodexInfo.WindowsClient.Core;
using CodexInfo.WindowsClient.Localization;

namespace CodexInfo.WindowsClient.ViewModels;

public enum GraphMetric
{
    Dollars,
    Tokens,
}

public sealed class GraphPointViewModel
{
    private readonly GraphMetric metric;

    public GraphPointViewModel(ApiHistorySample sample, GraphMetric metric)
    {
        this.metric = metric;
        Timestamp = sample.Timestamp;
        TimestampText = TimeZoneInfo.ConvertTime(DateTimeOffset.FromUnixTimeSeconds(sample.Timestamp), LocalizationService.DisplayTimeZone)
            .ToString("g", CultureInfo.CurrentCulture);
        RemainingPercent = sample.RemainingPercent;

        foreach (var model in sample.Models)
        {
            var value = metric == GraphMetric.Dollars
                ? model.Dollars
                : model.InputTokens + model.CachedInputTokens + model.OutputTokens;
            switch (model.Name)
            {
                case "SOL":
                    SolValue = value;
                    break;
                case "TERRA":
                    TerraValue = value;
                    break;
                case "LUNA":
                    LunaValue = value;
                    break;
            }
        }
    }

    public long Timestamp { get; }

    public string TimestampText { get; }

    public double? RemainingPercent { get; }

    public double SolValue { get; }

    public double TerraValue { get; }

    public double LunaValue { get; }

    public string RemainingText => RemainingPercent is { } value
        ? string.Create(CultureInfo.CurrentCulture, $"{LocalizationService.Current.RemainingQuota} {value:0.#}%")
        : $"{LocalizationService.Current.RemainingQuota} —";

    public string ModelsText => metric == GraphMetric.Dollars
        ? string.Create(
            CultureInfo.CurrentCulture,
            $"SOL ${SolValue:N2} / TERRA ${TerraValue:N2} / LUNA ${LunaValue:N2}")
        : string.Create(
            CultureInfo.CurrentCulture,
            $"SOL {SolValue:N0} / TERRA {TerraValue:N0} / LUNA {LunaValue:N0}");
}

public sealed class GraphWindowViewModel : INotifyPropertyChanged, IDisposable
{
    private readonly MainWindowViewModel main;
    private readonly ObservableCollection<ApiHistoryPeriod> periods = [];
    private readonly ObservableCollection<GraphPointViewModel> points = [];
    private ApiHistoryPeriod? selectedPeriod;
    private GraphMetric selectedMetric = GraphMetric.Dollars;
    private bool showRemaining = true;
    private bool showModels = true;
    private bool showSol = true;
    private bool showTerra = true;
    private bool showLuna = true;
    private bool disposed;

    public GraphWindowViewModel(MainWindowViewModel main)
    {
        this.main = main;
        Periods = new ReadOnlyObservableCollection<ApiHistoryPeriod>(periods);
        Points = new ReadOnlyObservableCollection<GraphPointViewModel>(points);
        main.PropertyChanged += OnMainPropertyChanged;
        Rebuild();
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public ReadOnlyObservableCollection<ApiHistoryPeriod> Periods { get; }

    public ReadOnlyObservableCollection<GraphPointViewModel> Points { get; }

    public UiText Texts => LocalizationService.Current;

    public IReadOnlyList<string> MetricOptions => [Texts.Dollars, Texts.Tokens];

    public string SelectedMetric
    {
        get => selectedMetric == GraphMetric.Dollars ? Texts.Dollars : Texts.Tokens;
        set
        {
            var metric = value == Texts.Tokens ? GraphMetric.Tokens : GraphMetric.Dollars;
            if (selectedMetric == metric)
            {
                return;
            }

            selectedMetric = metric;
            RebuildPoints();
            Notify(nameof(IsDollars));
            Notify();
        }
    }

    public ApiHistoryPeriod? SelectedPeriod
    {
        get => selectedPeriod;
        set
        {
            if (ReferenceEquals(selectedPeriod, value))
            {
                return;
            }

            selectedPeriod = value;
            RebuildPoints();
            Notify();
            Notify(nameof(HasPoints));
            Notify(nameof(SelectedPeriodText));
            Notify(nameof(SelectedPeriodStartAt));
            Notify(nameof(SelectedPeriodEndAt));
        }
    }

    public bool HasPoints => points.Count > 0;

    public bool HasNoPoints => !HasPoints;

    public bool HasPeriods => periods.Count > 0;

    public string SelectedPeriodText => selectedPeriod?.Label ?? Texts.UnavailableValue;

    public long SelectedPeriodStartAt => selectedPeriod?.StartAt ?? 0;

    // The API keeps the canonical reset boundary in end_at so clients can
    // label the period consistently.  For the active period the X client
    // clips the plot's right edge to the observation time; using the future
    // reset boundary here leaves an empty tail and changes the graph meaning.
    public long SelectedPeriodEndAt => selectedPeriod is { } period
        ? EffectiveGraphEnd(period, DateTimeOffset.UtcNow.ToUnixTimeSeconds())
        : 0;

    internal static long EffectiveGraphEnd(ApiHistoryPeriod period, long now)
    {
        if (!period.Current)
        {
            return period.EndAt;
        }

        return Math.Max(period.StartAt, Math.Min(period.EndAt, now));
    }

    internal static IReadOnlyList<ApiHistorySample> BuildGraphSamples(ApiHistoryPeriod period, long now)
    {
        var end = EffectiveGraphEnd(period, now);
        var observed = period.Samples
            .Where(sample => sample.Timestamp >= period.StartAt && sample.Timestamp < end)
            .OrderBy(sample => sample.Timestamp)
            .ToList();
        if (observed.Count == 0)
        {
            return [];
        }

        // The native client treats history rows as cumulative snapshots.  A
        // stale/out-of-order reread must therefore never make a later graph
        // segment move backwards.  Keep the latest remaining observation but
        // take the greatest model/token value for duplicate timestamps.
        var normalized = observed
            // Native history is rendered in sixty-second buckets.  Keep the
            // bucket anchored at the period start when a first sample falls
            // before that boundary, then merge cumulative snapshots by max.
            .GroupBy(sample => Math.Max(period.StartAt, sample.Timestamp - sample.Timestamp % 60))
            .Select(group =>
            {
                var rows = group.ToList();
                var latestRemaining = rows
                    .Where(sample => sample.RemainingPercent is { } value && double.IsFinite(value))
                    .Select(sample => sample.RemainingPercent)
                    .LastOrDefault();
                var latest = rows[^1];
                return new ApiHistorySample(
                    group.Key,
                    latest.ResetAt,
                    latestRemaining,
                    rows.Max(sample => sample.SolDollars),
                    rows.Max(sample => sample.TerraDollars),
                    rows.Max(sample => sample.LunaDollars),
                    rows.Max(sample => sample.SolTokens),
                    rows.Max(sample => sample.TerraTokens),
                    rows.Max(sample => sample.LunaTokens));
            })
            .OrderBy(sample => sample.Timestamp)
            .ToList();

        // Each history row is a cumulative snapshot, not a per-minute
        // increment.  Carry the greatest value through later buckets as the
        // native client does; this keeps both the model paths and the
        // activity-aware remaining interpolation monotonic after a stale
        // reread or an out-of-order API response.
        var cumulativeDollars = new double[3];
        var cumulativeTokens = new ulong[3];
        for (var index = 0; index < normalized.Count; index++)
        {
            var sample = normalized[index];
            cumulativeDollars[0] = Math.Max(cumulativeDollars[0], sample.SolDollars);
            cumulativeDollars[1] = Math.Max(cumulativeDollars[1], sample.TerraDollars);
            cumulativeDollars[2] = Math.Max(cumulativeDollars[2], sample.LunaDollars);
            cumulativeTokens[0] = Math.Max(cumulativeTokens[0], sample.SolTokens);
            cumulativeTokens[1] = Math.Max(cumulativeTokens[1], sample.TerraTokens);
            cumulativeTokens[2] = Math.Max(cumulativeTokens[2], sample.LunaTokens);
            normalized[index] = sample with
            {
                SolDollars = cumulativeDollars[0],
                TerraDollars = cumulativeDollars[1],
                LunaDollars = cumulativeDollars[2],
                SolTokens = cumulativeTokens[0],
                TerraTokens = cumulativeTokens[1],
                LunaTokens = cumulativeTokens[2],
            };
        }

        var hasRemainingObservation = normalized.Any(sample => sample.RemainingPercent is { } value && double.IsFinite(value));
        if (normalized[0].Timestamp == period.StartAt &&
            normalized[0].RemainingPercent is null &&
            hasRemainingObservation)
        {
            // raw_graph_points() always starts with a 100% reset anchor and
            // only replaces it when a quota observation exists at the same
            // timestamp.  Keep that distinction when the first model row is
            // present but its quota field is missing.
            normalized[0] = normalized[0] with { RemainingPercent = 100 };
        }
        var result = new List<ApiHistorySample>(normalized.Count + 2);
        if (normalized[0].Timestamp > period.StartAt)
        {
            result.Add(new ApiHistorySample(period.StartAt, normalized[0].ResetAt, hasRemainingObservation ? 100 : null, 0, 0, 0, 0, 0, 0));
        }

        result.AddRange(normalized);
        var last = result[^1];
        if (last.Timestamp < end)
        {
            result.Add(last with { Timestamp = end });
        }

        return result;
    }

    public string DetailsStatusText => main.DetailsStatusText;

    public string MetricAxisText => selectedMetric == GraphMetric.Dollars
        ? $"{Texts.Dollars} ({Texts.ModelUsage})"
        : $"{Texts.Tokens} ({Texts.ModelUsage})";

    public bool IsDollars => selectedMetric == GraphMetric.Dollars;

    public bool ShowRemaining
    {
        get => showRemaining;
        set
        {
            if (showRemaining == value) return;
            showRemaining = value;
            Notify();
        }
    }

    public bool ShowModels
    {
        get => showModels;
        set
        {
            if (showModels == value) return;
            showModels = value;
            Notify();
        }
    }

    public bool ShowSol { get => showSol; set { if (showSol == value) return; showSol = value; Notify(); } }
    public bool ShowTerra { get => showTerra; set { if (showTerra == value) return; showTerra = value; Notify(); } }
    public bool ShowLuna { get => showLuna; set { if (showLuna == value) return; showLuna = value; Notify(); } }

    public void Dispose()
    {
        if (disposed)
        {
            return;
        }

        disposed = true;
        main.PropertyChanged -= OnMainPropertyChanged;
    }

    private void OnMainPropertyChanged(object? sender, PropertyChangedEventArgs eventArgs)
    {
        if (eventArgs.PropertyName is nameof(MainWindowViewModel.DetailsSnapshot) or
            nameof(MainWindowViewModel.DetailsStatusText) or nameof(MainWindowViewModel.Texts))
        {
            Rebuild();
            Notify(nameof(DetailsStatusText));
            Notify(nameof(Texts));
            Notify(nameof(MetricOptions));
            Notify(nameof(SelectedMetric));
            Notify(nameof(MetricAxisText));
        }
    }

    private void Rebuild()
    {
        var previousId = selectedPeriod?.Id;
        periods.Clear();
        if (main.DetailsSnapshot is { } details)
        {
            foreach (var period in details.History)
            {
                periods.Add(period);
            }
        }

        selectedPeriod = periods.FirstOrDefault(period => period.Id == previousId)
            ?? periods.FirstOrDefault(period => period.Current)
            ?? periods.FirstOrDefault();

        RebuildPoints();
        Notify(nameof(HasPeriods));
        Notify(nameof(SelectedPeriod));
        Notify(nameof(SelectedPeriodText));
        Notify(nameof(SelectedPeriodStartAt));
        Notify(nameof(SelectedPeriodEndAt));
    }

    private void RebuildPoints()
    {
        points.Clear();
        if (selectedPeriod is { } period)
        {
            foreach (var sample in BuildGraphSamples(period, DateTimeOffset.UtcNow.ToUnixTimeSeconds()))
            {
                points.Add(new GraphPointViewModel(sample, selectedMetric));
            }
        }

        Notify(nameof(HasPoints));
        Notify(nameof(HasNoPoints));
        Notify(nameof(MetricAxisText));
    }

    private void Notify([CallerMemberName] string? propertyName = null)
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}

public sealed class ThreadsWindowViewModel : INotifyPropertyChanged, IDisposable
{
    private readonly MainWindowViewModel main;
    private readonly ObservableCollection<ThreadItemViewModel> threads = [];
    private bool disposed;

    public ThreadsWindowViewModel(MainWindowViewModel main)
    {
        this.main = main;
        Threads = new ReadOnlyObservableCollection<ThreadItemViewModel>(threads);
        main.PropertyChanged += OnMainPropertyChanged;
        Rebuild();
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public ReadOnlyObservableCollection<ThreadItemViewModel> Threads { get; }

    public UiText Texts => LocalizationService.Current;

    public bool HasThreads => threads.Count > 0;

    public bool HasNoThreads => !HasThreads;

    public string EmptyText => Texts.NoRunningThreads;

    public string DetailsStatusText => main.DetailsStatusText;

    public string ThreadRole(ApiThreadDetails thread)
    {
        if (thread.IsOrphan)
        {
            return thread.IsSubAgent ? $"{Texts.SubThread} ({Texts.UnavailableValue})" : $"{Texts.MainThread} ({Texts.UnavailableValue})";
        }

        var prefix = thread.Depth is { } depth && depth > 0 ? new string('│', Math.Min(depth, 3)) + " " : string.Empty;
        return prefix + (thread.IsSubAgent
            ? thread.Depth is { } nestedDepth ? $"{Texts.SubThread} D{nestedDepth}" : Texts.SubThread
            : Texts.MainThread);
    }

    public string ParentText(ApiThreadDetails thread) => thread.ParentId is { } parent
        ? $"{Texts.Parent}: {parent}"
        : thread.IsOrphan && thread.IsSubAgent
            ? Texts.ParentUnavailable
            : Texts.UnavailableValue;

    public string ModelText(ApiThreadDetails thread) =>
        string.IsNullOrWhiteSpace(thread.ModelLabel) ? thread.Model : thread.ModelLabel;

    public string ContextText(ApiThreadDetails thread)
    {
        if (thread.ContextPercent is not { } percent)
        {
            return $"{Texts.Context} —";
        }

        return thread.ContextLimit is { } limit
            ? string.Create(CultureInfo.CurrentCulture, $"{Texts.Context} {percent:0.#}% / {limit:N0}")
            : string.Create(CultureInfo.CurrentCulture, $"{Texts.Context} {percent:0.#}%");
    }

    public string TokenText(ApiThreadDetails thread) => thread.CumulativeTokens is { } tokens
        ? string.Create(CultureInfo.CurrentCulture, $"{Texts.Tokens} {tokens:N0}")
        : $"{Texts.Tokens} —";

    public void Dispose()
    {
        if (disposed)
        {
            return;
        }

        disposed = true;
        main.PropertyChanged -= OnMainPropertyChanged;
    }

    private void OnMainPropertyChanged(object? sender, PropertyChangedEventArgs eventArgs)
    {
        if (eventArgs.PropertyName is nameof(MainWindowViewModel.DetailsSnapshot) or
            nameof(MainWindowViewModel.DetailsStatusText) or nameof(MainWindowViewModel.Texts))
        {
            Rebuild();
            Notify(nameof(DetailsStatusText));
            Notify(nameof(Texts));
        }
    }

    private void Rebuild()
    {
        threads.Clear();
        if (main.DetailsSnapshot is { } details)
        {
            var ordered = ParentFirst(details.Threads);
            var byId = details.Threads.ToDictionary(thread => thread.Id, StringComparer.Ordinal);
            for (var index = 0; index < ordered.Count; index++)
            {
                var thread = ordered[index];
                var parentExists = thread.ParentId is { } parentId && byId.ContainsKey(parentId);
                var hasChildren = ordered.Any(candidate => candidate.ParentId == thread.Id);
                var hasNextSibling = ordered.Skip(index + 1).Any(candidate => candidate.ParentId == thread.ParentId);
                var depth = thread.Depth ?? CalculateDepth(thread, byId);
                var currentChain = AncestorChain(thread, byId);
                var ancestorGuides = new bool[3];
                for (var guide = 1; guide <= 3; guide++)
                {
                    ancestorGuides[guide - 1] = currentChain.Count >= guide && ordered.Skip(index + 1).Any(candidate =>
                    {
                        var candidateChain = AncestorChain(candidate, byId);
                        return candidateChain.Count >= guide && candidateChain[guide - 1] == currentChain[guide - 1];
                    });
                }
                var parentTitle = thread.ParentId is { } id && byId.TryGetValue(id, out var parent)
                    ? parent.Title
                    : string.Empty;
                threads.Add(new ThreadItemViewModel(this, thread, Math.Min(depth, 3), parentExists && !thread.IsOrphan,
                    hasChildren, hasNextSibling, ancestorGuides[0], ancestorGuides[1], ancestorGuides[2], parentTitle));
            }
        }

        Notify(nameof(HasThreads));
        Notify(nameof(HasNoThreads));
    }

    private static int CalculateDepth(ApiThreadDetails thread, IReadOnlyDictionary<string, ApiThreadDetails> byId)
    {
        var depth = 0;
        var current = thread;
        var seen = new HashSet<string>(StringComparer.Ordinal);
        while (current.ParentId is { } parentId && byId.TryGetValue(parentId, out var parent) && seen.Add(parentId))
        {
            depth++;
            current = parent;
        }
        return depth;
    }

    private static IReadOnlyList<string> AncestorChain(ApiThreadDetails thread, IReadOnlyDictionary<string, ApiThreadDetails> byId)
    {
        var reverse = new List<string> { thread.Id };
        var current = thread;
        var seen = new HashSet<string>(StringComparer.Ordinal) { thread.Id };
        while (current.ParentId is { } parentId && byId.TryGetValue(parentId, out var parent) && seen.Add(parentId))
        {
            reverse.Add(parent.Id);
            current = parent;
        }
        reverse.Reverse();
        return reverse;
    }

    private static IReadOnlyList<ApiThreadDetails> ParentFirst(IReadOnlyList<ApiThreadDetails> source)
    {
        var byId = source.ToDictionary(thread => thread.Id, StringComparer.Ordinal);
        var children = source.Where(thread => thread.ParentId is not null).GroupBy(thread => thread.ParentId!, StringComparer.Ordinal)
            .ToDictionary(group => group.Key, group => group.OrderBy(thread => thread.Id, StringComparer.Ordinal).ToList(), StringComparer.Ordinal);
        var result = new List<ApiThreadDetails>(source.Count);
        var visited = new HashSet<string>(StringComparer.Ordinal);
        void Visit(ApiThreadDetails item)
        {
            if (!visited.Add(item.Id)) return;
            result.Add(item);
            if (children.TryGetValue(item.Id, out var nested))
                foreach (var child in nested) Visit(child);
        }
        foreach (var root in source.Where(thread => thread.ParentId is null || !byId.ContainsKey(thread.ParentId)).OrderBy(thread => thread.Id, StringComparer.Ordinal)) Visit(root);
        foreach (var item in source) Visit(item);
        return result;
    }

    private void Notify([CallerMemberName] string? propertyName = null)
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}

public sealed class ThreadItemViewModel
{
    public ThreadItemViewModel(ThreadsWindowViewModel owner, ApiThreadDetails thread, int treeDepth,
        bool connectedToParent, bool hasChildren, bool hasNextSibling,
        bool ancestorGuide1, bool ancestorGuide2, bool ancestorGuide3, string parentTitle)
    {
        Id = thread.Id;
        Title = thread.Title;
        RoleText = owner.ThreadRole(thread);
        ParentText = string.IsNullOrWhiteSpace(parentTitle)
            ? owner.ParentText(thread)
            : $"{owner.ParentText(thread)} / {parentTitle}";
        ModelText = owner.ModelText(thread);
        ContextText = owner.ContextText(thread);
        TokenText = owner.TokenText(thread);
        DepthText = thread.Depth is { } depth ? $"{owner.Texts.Depth} {depth}" : $"{owner.Texts.Depth} —";
        AgeText = owner.Texts.FormatElapsed(thread.CreatedAt, owner.Texts.Elapsed);
        InstructionAgeText = owner.Texts.FormatElapsed(thread.LastUserMessageAt, owner.Texts.Instruction);
        TreeDepth = treeDepth;
        ConnectedToParent = connectedToParent;
        HasChildren = hasChildren;
        HasNextSibling = hasNextSibling;
        AncestorGuide1 = ancestorGuide1;
        AncestorGuide2 = ancestorGuide2;
        AncestorGuide3 = ancestorGuide3;
        ParentTitle = parentTitle;
    }

    public string Id { get; }
    public string Title { get; }
    public string RoleText { get; }
    public string ParentText { get; }
    public string ModelText { get; }
    public string ContextText { get; }
    public string TokenText { get; }
    public string DepthText { get; }
    public string AgeText { get; }
    public string InstructionAgeText { get; }
    public int TreeDepth { get; }
    public bool ConnectedToParent { get; }
    public bool HasChildren { get; }
    public bool HasNextSibling { get; }
    public bool AncestorGuide1 { get; }
    public bool AncestorGuide2 { get; }
    public bool AncestorGuide3 { get; }
    public string ParentTitle { get; }

}

public sealed class LegalNoticesWindowViewModel : INotifyPropertyChanged, IDisposable
{
    private readonly MainWindowViewModel main;
    private readonly ObservableCollection<ApiLegalNotice> notices = [];
    private bool disposed;

    public LegalNoticesWindowViewModel(MainWindowViewModel main)
    {
        this.main = main;
        Notices = new ReadOnlyObservableCollection<ApiLegalNotice>(notices);
        main.PropertyChanged += OnMainPropertyChanged;
        Rebuild();
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public ReadOnlyObservableCollection<ApiLegalNotice> Notices { get; }

    public UiText Texts => LocalizationService.Current;

    public bool HasNotices => notices.Count > 0;

    public string DetailsStatusText => main.DetailsStatusText;

    public void Dispose()
    {
        if (disposed)
        {
            return;
        }

        disposed = true;
        main.PropertyChanged -= OnMainPropertyChanged;
    }

    private void OnMainPropertyChanged(object? sender, PropertyChangedEventArgs eventArgs)
    {
        if (eventArgs.PropertyName is nameof(MainWindowViewModel.DetailsSnapshot) or
            nameof(MainWindowViewModel.DetailsStatusText) or nameof(MainWindowViewModel.Texts))
        {
            Rebuild();
            Notify(nameof(DetailsStatusText));
            Notify(nameof(Texts));
        }
    }

    private void Rebuild()
    {
        notices.Clear();
        var japanese = Texts.LanguageCode == "ja";
        // Legal information remains reachable before authentication and when
        // the auxiliary endpoint is unavailable.  It is intentionally static
        // and contains no account/backend data.
        if (notices.Count == 0)
        {
            notices.Add(new ApiLegalNotice(
                Texts.LegalCodeName,
                japanese ? "Copyright (C) 2026 salty919。Codex Info は GPL-3.0-only で提供されます。無保証です。詳細は GNU General Public License version 3 を参照してください。" : "Copyright (C) 2026 salty919. Codex Info is provided under GPL-3.0-only without warranty. See the GNU General Public License version 3 for details."));
            notices.Add(new ApiLegalNotice(
                Texts.LegalFontName,
                japanese ? "Noto Sans JPおよびNoto Sans CJK KRを埋め込んでいます。Copyright (c) 2014-2021 Adobe。SIL Open Font License 1.1 に基づきます。" : "Noto Sans JP and Noto Sans CJK KR are embedded. Copyright (c) 2014-2021 Adobe. Distributed under SIL Open Font License 1.1."));
            notices.Add(new ApiLegalNotice(
                Texts.LegalProtocolName,
                japanese ? "Windowsクライアントは SSH ローカルポート転送で保護された 127.0.0.1 の読み取り専用 REST v1 だけを使用します。認証情報やアクセストークンは取得・表示しません。" : "The Windows client uses only the read-only REST v1 endpoint on 127.0.0.1 protected by an SSH local forward. Credentials and tokens are never collected or displayed."));
            notices.Add(new ApiLegalNotice(
                Texts.LegalSchemaName,
                japanese ? "REST v1 の status/details JSON スキーマは docs/REST_API_V1.md に固定されています。未知キーや不正値は拒否し、最後に検証できたスナップショットを保持します。" : "The REST v1 status/details JSON schema is fixed in docs/REST_API_V1.md. Unknown keys and invalid values are rejected while the last valid snapshot is retained."));
            notices.Add(new ApiLegalNotice(
                Texts.LegalThirdPartyName,
                japanese ? "Avalonia、Skia、HarfBuzz、ANGLE、.NET ランタイムその他の依存物の通知は THIRD_PARTY_NOTICES.md と LICENSES/ を参照してください。" : "See THIRD_PARTY_NOTICES.md and LICENSES/ for notices covering Avalonia, Skia, HarfBuzz, ANGLE, the .NET runtime, and other dependencies."));
            notices.Add(new ApiLegalNotice(
                Texts.LegalDistributionName,
                japanese ? "配布前に windows-client/tools/Collect-ThirdPartyNotices.ps1 を publish ディレクトリへ実行し、依存物の通知を同梱してください。" : "Before distribution, run windows-client/tools/Collect-ThirdPartyNotices.ps1 against the publish directory and include all dependency notices."));
        }

        Notify(nameof(HasNotices));
    }

    private void Notify([CallerMemberName] string? propertyName = null)
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}
