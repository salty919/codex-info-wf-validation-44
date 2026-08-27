// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Collections.ObjectModel;
using System.Collections.Specialized;
using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.Runtime.CompilerServices;
using System.Text.RegularExpressions;
using System.Windows.Input;
using Avalonia.Media;
using CodexInfo.WindowsClient.Core;
using CodexInfo.WindowsClient.Localization;
using CodexInfo.WindowsClient.Settings;
using CodexInfo.WindowsClient.Updates;

namespace CodexInfo.WindowsClient.ViewModels;

/// <summary>
/// Presents only a validated <see cref="ApiStatusSnapshot"/>. HTTP, JSON, and
/// schema failures are classified by the Core client before reaching this UI.
/// </summary>
public sealed class MainWindowViewModel : INotifyPropertyChanged, IDisposable
{
    private static readonly IBrush NormalBackground = new SolidColorBrush(Color.Parse("#143426"));
    private static readonly IBrush NormalBorder = new SolidColorBrush(Color.Parse("#276C49"));
    private static readonly IBrush NormalAccent = new SolidColorBrush(Color.Parse("#4FB878"));
    private static readonly IBrush NoticeBackground = new SolidColorBrush(Color.Parse("#172C42"));
    private static readonly IBrush NoticeBorder = new SolidColorBrush(Color.Parse("#2D6193"));
    private static readonly IBrush NoticeAccent = new SolidColorBrush(Color.Parse("#5EA7E5"));
    private static readonly IBrush WarningBackground = new SolidColorBrush(Color.Parse("#3A2A13"));
    private static readonly IBrush WarningBorder = new SolidColorBrush(Color.Parse("#8A651F"));
    private static readonly IBrush WarningAccent = new SolidColorBrush(Color.Parse("#D5A43A"));
    private static readonly IBrush ErrorBackground = new SolidColorBrush(Color.Parse("#3A1D24"));
    private static readonly IBrush ErrorBorder = new SolidColorBrush(Color.Parse("#8E3D4D"));
    private static readonly IBrush ErrorAccent = new SolidColorBrush(Color.Parse("#E06B7A"));

    private readonly ILoopbackStatusClient client;
    private readonly ILoopbackHealthClient healthClient;
    private readonly ILoopbackDetailsClient? detailsClient;
    private readonly ConnectionSupervisor? connectionSupervisor;
    private readonly UpdateViewModel? update;
    private readonly CancellationTokenSource lifetime = new();
    private readonly SemaphoreSlim refreshGate = new(1, 1);
    private readonly AsyncCommand refreshCommand;
    private readonly AsyncCommand authCommand;
    private readonly AsyncCommand checkAuthCommand;
    private readonly SnapshotCollection<ModelUsageViewModel> models = [];
    private readonly SnapshotCollection<QuotaSegmentViewModel> quotaSegments = [];
    private ApiStatusSnapshot? snapshot;
    private ApiDetailsSnapshot? detailsSnapshot;
    private DetailsFetchFailure? detailsFailure;
    private DateTimeOffset? lastReceivedAt;
    private ClientPresentationState presentationState = ClientPresentationState.Connecting;
    private bool refreshing;
    private bool authLaunchFailed;
    private bool disposed;
    private bool initialLoadPending = true;
    private int started;

    public MainWindowViewModel(
        ILoopbackStatusClient client,
        ILoopbackDetailsClient? detailsClient = null,
        ConnectionSupervisor? connectionSupervisor = null,
        IWindowsUpdateCoordinator? updateCoordinator = null)
    {
        ArgumentNullException.ThrowIfNull(client);
        this.client = client;
        healthClient = client as ILoopbackHealthClient
            ?? throw new ArgumentException(
                "The status client must implement the fixed health boundary.",
                nameof(client));
        this.detailsClient = detailsClient;
        this.connectionSupervisor = connectionSupervisor;
        update = updateCoordinator is null ? null : new UpdateViewModel(updateCoordinator);
        if (update is not null) update.PropertyChanged += OnUpdatePropertyChanged;
        refreshCommand = new AsyncCommand(RefreshManuallyAsync, () => CanRefresh);
        authCommand = new AsyncCommand(LaunchLinuxAuthenticationAsync, () => IsAuthRequired && !disposed);
        checkAuthCommand = new AsyncCommand(RefreshManuallyAsync, () => IsAuthRequired && CanRefresh);
        Models = new ReadOnlyObservableCollection<ModelUsageViewModel>(models);
        QuotaSegments = new ReadOnlyObservableCollection<QuotaSegmentViewModel>(quotaSegments);
        LocalizationService.LanguageChanged += OnLanguageChanged;
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public UiText Texts => LocalizationService.Current;

    public string ProductVersionText => ProductInfo.DisplayVersion;

    public ICommand RefreshCommand => refreshCommand;

    public ICommand AuthCommand => authCommand;

    public ICommand CheckAuthCommand => checkAuthCommand;

    public UpdateViewModel? Update => update;

    public ICommand? UpdateCommand => update?.UpdateCommand;

    public bool IsUpdateNotificationVisible => !IsAuthRequired && update?.IsNotificationVisible == true;

    public bool IsUpdateActionVisible => !IsAuthRequired && update?.IsUpdateActionVisible == true;

    public string UpdateNotificationText => update?.NotificationText ?? string.Empty;

    public string UpdateButtonText => update?.ActionText ?? Texts.UpdateButtonText;

    public bool ShowLastReceived => IsAuthenticated && !IsUpdateNotificationVisible;

    public ReadOnlyObservableCollection<ModelUsageViewModel> Models { get; }

    public ReadOnlyObservableCollection<QuotaSegmentViewModel> QuotaSegments { get; }

    public bool CanRefresh => !refreshing && !disposed;

    public string RefreshButtonText => refreshing ? Texts.Refreshing : Texts.Refresh;

    public bool HasQuota => snapshot?.Quota is not null;

    public bool HasModels => models.Count > 0;

    public bool HasNoModels => !HasModels;

    public bool IsAuthRequired => presentationState == ClientPresentationState.AuthRequired;

    /// <summary>
    /// True only after the status owner has accepted an authenticated snapshot.
    /// Setup uses this instead of treating a reachable API or an old details
    /// document as proof that the current account is ready.
    /// </summary>
    public bool IsAuthenticated => snapshot is { Authenticated: true } && !IsAuthRequired;

    /// <summary>
    /// Keeps the first frame stable while the health, status, and auxiliary
    /// details generation is being assembled.  Subsequent polls update the
    /// already-published generation without hiding the content.
    /// </summary>
    public bool IsStartupLoading => initialLoadPending;

    public bool ShowAuthenticatedContent => IsAuthenticated && !IsStartupLoading;

    public bool HasActiveThreads => ActiveThreadCount > 0;

    public bool HasNoActiveThreads => !HasActiveThreads;

    /// <summary>
    /// The scalar status count is authoritative even when the details endpoint
    /// returns only a bounded row sample.  Details rows are still used for the
    /// model breakdown and the child window.
    /// </summary>
    public ulong ActiveThreadCount => snapshot?.ActiveThreadCount ?? detailsSnapshot?.ActiveThreadCount ?? 0;

    public string ActiveThreadCountLabel => string.Create(CultureInfo.CurrentCulture, $"{ActiveThreadCount:N0}{(string.IsNullOrEmpty(Texts.CountUnit) ? "" : " " + Texts.CountUnit)}");

    public int ActiveSolCount => CountThreads("SOL");

    public int ActiveTerraCount => CountThreads("TERRA");

    public int ActiveLunaCount => CountThreads("LUNA");

    public int ActiveOtherCount => Math.Max(0, (int)ActiveThreadCount - ActiveSolCount - ActiveTerraCount - ActiveLunaCount);

    /// <summary>Whether at least one details document has been accepted.</summary>
    public bool HasDetails => detailsSnapshot is not null;

    public ApiDetailsSnapshot? DetailsSnapshot => detailsSnapshot;

    public string DetailsStatusText
    {
        get
        {
            if (Texts.LanguageCode == "ja")
            {
                return detailsFailure switch
                {
                    null when detailsSnapshot is not null => "詳細データ: 最新",
                    DetailsFetchFailure.Transport when detailsSnapshot is not null => "詳細データ: 前回値を表示（接続エラー）",
                    DetailsFetchFailure.Response when detailsSnapshot is not null => "詳細データ: 前回値を表示（応答エラー）",
                    DetailsFetchFailure.Transport => "詳細データ: 未取得（接続エラー）",
                    DetailsFetchFailure.Response => "詳細データ: 未取得（応答エラー）",
                    _ => "詳細データ: 未取得",
                };
            }
            return detailsFailure switch
            {
                null when detailsSnapshot is not null => $"{Texts.Details}: {Texts.Latest}",
                DetailsFetchFailure.Transport when detailsSnapshot is not null => $"{Texts.Details}: {Texts.Unavailable} ({Texts.TransportError})",
                DetailsFetchFailure.Response when detailsSnapshot is not null => $"{Texts.Details}: {Texts.Unavailable} ({Texts.ApiError})",
                DetailsFetchFailure.Transport => $"{Texts.Details}: {Texts.Unavailable} ({Texts.TransportError})",
                DetailsFetchFailure.Response => $"{Texts.Details}: {Texts.Unavailable} ({Texts.ApiError})",
                _ => $"{Texts.Details}: {Texts.UnavailableValue}",
            };
        }
    }

    /// <summary>
    /// Locale-independent UI Automation contract for the details generation.
    /// The visible status remains localized, while UI tests consume this
    /// stable value instead of decoding rendered text.
    /// </summary>
    public string DetailsStatusAutomationText => detailsSnapshot is not null && detailsFailure is null
        ? "ready"
        : detailsFailure is null
            ? "pending"
            : "error";

    public string RemainingPercentText
    {
        get
        {
            return snapshot?.Quota is { } quota
                ? string.Create(CultureInfo.CurrentCulture, $"{quota.RemainingPercent:0.#}%")
                : Texts.UnavailableValue;
        }
    }

    public double RemainingPercentValue => snapshot?.Quota?.RemainingPercent ?? 0;

    public string QuotaWindowText => (snapshot?.Quota) switch
    {
        null => Texts.QuotaWaiting,
        { Monthly: true } => Texts.MonthlyQuota,
        _ => Texts.WeeklyQuota,
    };

    public string QuotaRemainingText => snapshot?.Quota is { } quota
        ? FormatRemainingDuration(quota.ResetAt)
        : Texts.UnavailableValue;

    public double QuotaRemainingPeriodValue => snapshot?.Quota is { } quota
        ? Math.Clamp(
            (quota.ResetAt - DateTimeOffset.UtcNow.ToUnixTimeSeconds()) * 100.0 /
            Math.Max(1, quota.WindowSeconds),
            0,
            100)
        : 0;

    public string ModelUsagePeriodText =>
        detailsSnapshot?.History.FirstOrDefault(period => period.Current)?.Label
        ?? detailsSnapshot?.History.FirstOrDefault()?.Label
        ?? QuotaWindowText;

    public string ModelUsageUnavailableText => $"{Texts.ModelUsage}: {Texts.UnavailableValue}";

    public string EstimatedCostText => detailsSnapshot?.EstimatedCostLabel ?? Texts.EstimatedUnavailable;

    public ReadOnlyObservableCollection<ModelUsageViewModel> CurrentModels => Models;

    public string AuthenticationText => snapshot switch
    {
        null => Texts.UnavailableValue,
        { Authenticated: true } => Texts.Connected,
        _ => Texts.AuthRequired,
    };

    public string PlanText => snapshot?.PlanLabel ?? Texts.UnavailableValue;

    public string ActiveThreadCountText => snapshot is null && detailsSnapshot is null
        ? Texts.UnavailableValue
        : string.Create(CultureInfo.CurrentCulture, $"{ActiveThreadCount:N0}{(string.IsNullOrEmpty(Texts.CountUnit) ? "" : " " + Texts.CountUnit)}");

    public string ResetAtText => snapshot?.Quota is { } quota
        ? FormatUnixTime(quota.ResetAt)
        : Texts.UnavailableValue;

    public string ObservedAtText => snapshot?.ObservedAt is { } observedAt
        ? FormatUnixTime(observedAt)
        : Texts.UnavailableValue;

    public string LastReceivedText => lastReceivedAt is { } receivedAt
        ? $"{Texts.LastReceivedPrefix}: {TimeZoneInfo.ConvertTime(receivedAt, LocalizationService.DisplayTimeZone).ToString("g", CultureInfo.CurrentCulture)}{StaleSuffix}"
        : Texts.LastReceivedUnavailable;

    public string StatusTitle => presentationState switch
    {
        ClientPresentationState.Connecting => Texts.Connecting,
        ClientPresentationState.Ready => Texts.Ready,
        ClientPresentationState.QuotaDanger => Texts.QuotaDanger,
        ClientPresentationState.QuotaWarning => Texts.QuotaWarning,
        ClientPresentationState.ResetWarning => Texts.ResetWarning,
        ClientPresentationState.Initializing => Texts.Initializing,
        ClientPresentationState.AuthRequired => Texts.AuthRequired,
        ClientPresentationState.ApiError => Texts.ApiError,
        ClientPresentationState.TransportError => Texts.TransportError,
        ClientPresentationState.ResponseError => Texts.Unavailable,
        _ => Texts.Connecting,
    };

    public string StatusDetail => Texts.StatusDetailFor(presentationState.ToString(), authLaunchFailed, snapshot is not null);

    public IBrush StatusBackground => presentationState switch
    {
        ClientPresentationState.Ready => NormalBackground,
        ClientPresentationState.Initializing or ClientPresentationState.Connecting => NoticeBackground,
        ClientPresentationState.AuthRequired or ClientPresentationState.QuotaWarning or ClientPresentationState.ResetWarning => WarningBackground,
        _ => ErrorBackground,
    };

    public IBrush StatusBorder => presentationState switch
    {
        ClientPresentationState.Ready => NormalBorder,
        ClientPresentationState.Initializing or ClientPresentationState.Connecting => NoticeBorder,
        ClientPresentationState.AuthRequired or ClientPresentationState.QuotaWarning or ClientPresentationState.ResetWarning => WarningBorder,
        _ => ErrorBorder,
    };

    public IBrush StatusAccent => presentationState switch
    {
        ClientPresentationState.Ready => NormalAccent,
        ClientPresentationState.Initializing or ClientPresentationState.Connecting => NoticeAccent,
        ClientPresentationState.AuthRequired or ClientPresentationState.QuotaWarning or ClientPresentationState.ResetWarning => WarningAccent,
        _ => ErrorAccent,
    };

    /// <summary>
    /// Starts exactly one initial request and then one request ten seconds after
    /// every completed polling request. Manual refreshes use the same gate and
    /// are dropped while a request is active.
    /// </summary>
    public void Start()
    {
        if (Interlocked.Exchange(ref started, 1) != 0 || disposed)
        {
            return;
        }

        connectionSupervisor?.EnsureStarted(App.CurrentSettings);
        update?.Start();
        _ = RunPollingAsync(lifetime.Token);
    }

    /// <summary>
    /// Applies a newly saved connection profile and performs one explicit
    /// health/status refresh. Setup uses this boundary after saving its
    /// selector; without it the supervisor would retain the pre-setup
    /// <c>none</c> profile for the lifetime of the process.
    /// </summary>
    internal bool ApplyConnectionSettings(ClientSettings settings)
    {
        if (disposed || connectionSupervisor is null)
        {
            return false;
        }

        var started = connectionSupervisor.EnsureStarted(settings);
        if (started)
        {
            _ = TryRefreshAsync(lifetime.Token);
        }

        return started;
    }

    public void Dispose()
    {
        if (disposed)
        {
            return;
        }

        disposed = true;
        LocalizationService.LanguageChanged -= OnLanguageChanged;
        lifetime.Cancel();
        if (update is not null)
        {
            update.PropertyChanged -= OnUpdatePropertyChanged;
            update.Dispose();
        }
        if (client is IDisposable disposableClient)
        {
            disposableClient.Dispose();
        }
        if (detailsClient is IDisposable disposableDetailsClient &&
            !ReferenceEquals(detailsClient, client))
        {
            disposableDetailsClient.Dispose();
        }
        connectionSupervisor?.Dispose();
        ClearModels();

        Notify(nameof(CanRefresh));
        refreshCommand.RaiseCanExecuteChanged();
        authCommand.RaiseCanExecuteChanged();
        checkAuthCommand.RaiseCanExecuteChanged();
    }

    private async Task RunPollingAsync(CancellationToken cancellationToken)
    {
        try
        {
            await TryRefreshAsync(cancellationToken);
            using var timer = new PeriodicTimer(TimeSpan.FromSeconds(10));
            while (await timer.WaitForNextTickAsync(cancellationToken))
            {
                await TryRefreshAsync(cancellationToken);
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            // Normal shutdown.
        }
    }

    private Task RefreshManuallyAsync()
    {
        return TryRefreshAsync(lifetime.Token);
    }

    private Task LaunchLinuxAuthenticationAsync()
    {
        try
        {
            // The command contains no account data or credentials.  The Linux
            // Codex CLI owns the browser flow and the server remains the sole
            // authority for the resulting authenticated state.
            var startInfo = new ProcessStartInfo
            {
                FileName = "wsl.exe",
                UseShellExecute = false,
                CreateNoWindow = false,
            };
            startInfo.ArgumentList.Add("--");
            startInfo.ArgumentList.Add("codex");
            startInfo.ArgumentList.Add("login");
            Process.Start(startInfo);
            authLaunchFailed = false;
        }
        catch
        {
            authLaunchFailed = true;
            Notify(nameof(StatusDetail));
        }

        return Task.CompletedTask;
    }

    private void OnLanguageChanged(object? sender, EventArgs eventArgs)
    {
        Notify(nameof(Texts));
        Notify(nameof(RefreshButtonText));
        Notify(nameof(StatusTitle));
        Notify(nameof(StatusDetail));
        Notify(nameof(UpdateNotificationText));
        Notify(nameof(UpdateButtonText));
        Notify(nameof(LastReceivedText));
        Notify(nameof(ShowLastReceived));
        Notify(nameof(DetailsStatusText));
        Notify(nameof(DetailsStatusAutomationText));
        Notify(nameof(QuotaWindowText));
        Notify(nameof(QuotaRemainingText));
        Notify(nameof(RemainingPercentText));
        Notify(nameof(ActiveThreadCountLabel));
        Notify(nameof(AuthenticationText));
        Notify(nameof(PlanText));
        Notify(nameof(LastReceivedText));
        Notify(nameof(ResetAtText));
        Notify(nameof(ObservedAtText));
        Notify(nameof(ActiveThreadCountText));
        Notify(nameof(EstimatedCostText));
        Notify(nameof(ModelUsageUnavailableText));
    }

    private async Task TryRefreshAsync(CancellationToken cancellationToken)
    {
        bool acquired;
        try
        {
            acquired = await refreshGate.WaitAsync(0, cancellationToken);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            return;
        }

        if (!acquired)
        {
            return;
        }

        SetRefreshing(true);
        try
        {
            // A client generation is admitted only after the fixed listener
            // health document has passed its own schema and header boundary.
            HealthFetchResult health = await healthClient
                .FetchHealthAsync(cancellationToken);
            if (cancellationToken.IsCancellationRequested)
            {
                return;
            }

            if (!health.IsSuccess)
            {
                ApplyFailure(health.Failure == HealthFetchFailure.Response
                    ? StatusFetchFailure.Response
                    : StatusFetchFailure.Transport);
                return;
            }

            StatusFetchResult result = await client.FetchAsync(cancellationToken);
            if (cancellationToken.IsCancellationRequested)
            {
                return;
            }

            if (result.Snapshot is not { } validatedSnapshot)
            {
                ApplyFailure(result.Failure ?? StatusFetchFailure.Transport);
                return;
            }

            // Details are account-scoped.  Never issue or apply an auxiliary
            // snapshot after an unauthenticated status, otherwise a slower
            // details response could repopulate cleared rows from the prior
            // account generation.
            if (detailsClient is not null && validatedSnapshot.Authenticated &&
                validatedSnapshot.State != ApiState.AuthRequired)
            {
                DetailsFetchResult detailsResult;
                try
                {
                    detailsResult = await detailsClient.FetchDetailsAsync(cancellationToken);
                }
                catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
                {
                    return;
                }
                catch
                {
                    detailsResult = DetailsFetchResult.FromFailure(DetailsFetchFailure.Transport);
                }

                if (cancellationToken.IsCancellationRequested)
                {
                    return;
                }

                if (detailsResult.Snapshot is { } validatedDetails &&
                    validatedDetails.Authenticated &&
                    validatedDetails.State != ApiState.AuthRequired &&
                    HasSamePublicCore(validatedSnapshot, validatedDetails))
                {
                    // Commit the pair only after both documents have passed
                    // validation and their shared projection is identical.
                    // Until this point the last complete UI generation stays
                    // untouched.
                    ApplySnapshot(validatedSnapshot);
                    ApplyDetails(validatedDetails);
                }
                else
                {
                    detailsFailure = detailsResult.Failure == DetailsFetchFailure.Transport
                        ? DetailsFetchFailure.Transport
                        : DetailsFetchFailure.Response;
                    Notify(nameof(DetailsStatusText));
                    Notify(nameof(DetailsStatusAutomationText));
                    ApplyFailure(detailsFailure == DetailsFetchFailure.Transport
                        ? StatusFetchFailure.Transport
                        : StatusFetchFailure.Response);
                }
            }
            else
            {
                ApplySnapshot(validatedSnapshot);
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            // The close path owns cancellation and does not change the UI.
        }
        catch
        {
            if (!cancellationToken.IsCancellationRequested)
            {
                ApplyFailure(StatusFetchFailure.Transport);
            }
        }
        finally
        {
            if (initialLoadPending)
            {
                initialLoadPending = false;
                Notify(nameof(IsStartupLoading));
                Notify(nameof(ShowAuthenticatedContent));
            }
            SetRefreshing(false);
            refreshGate.Release();
        }
    }

    private static bool HasSamePublicCore(
        ApiStatusSnapshot status,
        ApiDetailsSnapshot details)
    {
        if (status.State != details.State ||
            status.ObservedAt != details.ObservedAt ||
            status.Authenticated != details.Authenticated ||
            !string.Equals(status.PlanLabel, details.PlanLabel, StringComparison.Ordinal) ||
            status.Quota != details.Quota ||
            status.ActiveThreadCount != details.ActiveThreadCount ||
            status.Models.Count != details.Models.Count)
        {
            return false;
        }

        for (var index = 0; index < status.Models.Count; index++)
        {
            var statusModel = status.Models[index];
            var detailsModel = details.Models[index];
            if (!string.Equals(statusModel.Name, detailsModel.Name, StringComparison.Ordinal) ||
                statusModel.InputTokens != detailsModel.InputTokens ||
                statusModel.CachedInputTokens != detailsModel.CachedInputTokens ||
                statusModel.OutputTokens != detailsModel.OutputTokens)
            {
                return false;
            }
        }

        return true;
    }

    private void ApplySnapshot(ApiStatusSnapshot validatedSnapshot)
    {
        snapshot = validatedSnapshot;
        lastReceivedAt = DateTimeOffset.Now;
        presentationState = validatedSnapshot.State switch
        {
            ApiState.Ready => GetReadyPresentationState(validatedSnapshot),
            ApiState.Initializing => ClientPresentationState.Initializing,
            ApiState.AuthRequired => ClientPresentationState.AuthRequired,
            ApiState.Error => ClientPresentationState.ApiError,
            _ => ClientPresentationState.ResponseError,
        };

        if (validatedSnapshot.State == ApiState.AuthRequired || !validatedSnapshot.Authenticated)
        {
            // A valid authentication transition is not an auxiliary fetch
            // failure: clear account-scoped details so old rows cannot remain
            // visible while Linux asks the user to authenticate.
            detailsSnapshot = null;
            detailsFailure = null;
            ClearModels();
            Notify(nameof(HasDetails));
            Notify(nameof(DetailsSnapshot));
            Notify(nameof(DetailsStatusText));
            Notify(nameof(DetailsStatusAutomationText));
            Notify(nameof(EstimatedCostText));
            NotifyActiveThreadProperties();
        }
        else if (detailsSnapshot is null)
        {
            ReplaceStatusModels(validatedSnapshot.Models.OrderBy(ModelOrder));
        }

        NotifySnapshotProperties();
        authCommand.RaiseCanExecuteChanged();
        checkAuthCommand.RaiseCanExecuteChanged();
    }

    private void ApplyFailure(StatusFetchFailure failure)
    {
        presentationState = failure == StatusFetchFailure.Response
            ? ClientPresentationState.ResponseError
            : ClientPresentationState.TransportError;
        NotifyStatusProperties();
        Notify(nameof(LastReceivedText));
        authCommand.RaiseCanExecuteChanged();
        checkAuthCommand.RaiseCanExecuteChanged();
    }

    private void ApplyDetails(ApiDetailsSnapshot validatedDetails)
    {
        detailsSnapshot = validatedDetails;
        detailsFailure = null;

        // Details are the only source for dollar columns.  Keep the existing
        // status model rows until a valid details snapshot arrives; once it
        // does, replace the entire collection so rows never mix generations.
        ReplaceModels(validatedDetails.Models.OrderBy(ModelOrder));

        Notify(nameof(HasDetails));
        Notify(nameof(DetailsSnapshot));
        Notify(nameof(DetailsStatusText));
        Notify(nameof(DetailsStatusAutomationText));
        Notify(nameof(ModelUsagePeriodText));
        Notify(nameof(EstimatedCostText));
        Notify(nameof(HasModels));
        Notify(nameof(HasNoModels));
        NotifyActiveThreadProperties();
    }

    private void ApplyDetailsFailure(DetailsFetchFailure failure)
    {
        // Deliberately do not clear detailsSnapshot or Models.  This is the
        // auxiliary owner fault and is independent of the status banner.
        detailsFailure = failure;
        Notify(nameof(DetailsStatusText));
        Notify(nameof(DetailsStatusAutomationText));
    }

    private void SetRefreshing(bool value)
    {
        refreshing = value;
        Notify(nameof(CanRefresh));
        Notify(nameof(RefreshButtonText));
        refreshCommand.RaiseCanExecuteChanged();
    }

    private void NotifySnapshotProperties()
    {
        RebuildQuotaSegments();
        Notify(nameof(HasQuota));
        Notify(nameof(RemainingPercentText));
        Notify(nameof(RemainingPercentValue));
        Notify(nameof(QuotaWindowText));
        Notify(nameof(QuotaRemainingText));
        Notify(nameof(QuotaRemainingPeriodValue));
        Notify(nameof(AuthenticationText));
        Notify(nameof(IsAuthRequired));
        Notify(nameof(IsAuthenticated));
        Notify(nameof(ShowAuthenticatedContent));
        Notify(nameof(PlanText));
        Notify(nameof(ActiveThreadCountText));
        Notify(nameof(ResetAtText));
        Notify(nameof(ObservedAtText));
        Notify(nameof(LastReceivedText));
        Notify(nameof(HasModels));
        Notify(nameof(HasNoModels));
        Notify(nameof(ModelUsagePeriodText));
        NotifyStatusProperties();
        NotifyActiveThreadProperties();
    }

    private void RebuildQuotaSegments()
    {
        var fraction = snapshot?.Quota is { } quota
            ? Math.Clamp((quota.ResetAt - DateTimeOffset.UtcNow.ToUnixTimeSeconds()) /
                         (double)Math.Max(1, quota.WindowSeconds), 0, 1)
            : 0;
        quotaSegments.ReplaceAll(Enumerable.Range(0, 7)
            .Select(index => new QuotaSegmentViewModel(Math.Clamp(fraction * 7 - index, 0, 1))));

        Notify(nameof(QuotaSegments));
    }

    private void NotifyStatusProperties()
    {
        Notify(nameof(StatusTitle));
        Notify(nameof(StatusDetail));
        Notify(nameof(StatusBackground));
        Notify(nameof(StatusBorder));
        Notify(nameof(StatusAccent));
    }

    private void NotifyActiveThreadProperties()
    {
        Notify(nameof(HasActiveThreads));
        Notify(nameof(HasNoActiveThreads));
        Notify(nameof(ActiveThreadCount));
        Notify(nameof(ActiveThreadCountLabel));
        Notify(nameof(ActiveThreadCountText));
        Notify(nameof(ActiveSolCount));
        Notify(nameof(ActiveTerraCount));
        Notify(nameof(ActiveLunaCount));
        Notify(nameof(ActiveOtherCount));
    }

    private void ClearModels()
    {
        var previous = models.ToArray();
        models.ReplaceAll([]);
        foreach (var model in previous)
        {
            model.Dispose();
        }
    }

    private void ReplaceModels(IEnumerable<ApiDetailsModelUsage> source)
    {
        var next = source.Select(static model => new ModelUsageViewModel(model)).ToArray();
        var previous = models.ToArray();
        models.ReplaceAll(next);
        foreach (var model in previous)
        {
            model.Dispose();
        }
    }

    private void ReplaceStatusModels(IEnumerable<ApiModelUsage> source)
    {
        var next = source.Select(static model => new ModelUsageViewModel(model)).ToArray();
        var previous = models.ToArray();
        models.ReplaceAll(next);
        foreach (var model in previous)
        {
            model.Dispose();
        }
    }

    /// <summary>
    /// Publishes a complete snapshot as one collection reset.  Clearing and
    /// re-adding rows individually makes Avalonia remove and recreate the
    /// whole model table during every poll, which is visible as a full-screen
    /// flicker.  Items is mutated silently and one Reset is sent after the
    /// new immutable row set is ready.
    /// </summary>
    private sealed class SnapshotCollection<T> : ObservableCollection<T>
    {
        public void ReplaceAll(IEnumerable<T> values)
        {
            ArgumentNullException.ThrowIfNull(values);
            CheckReentrancy();
            Items.Clear();
            foreach (var value in values)
            {
                Items.Add(value);
            }

            OnPropertyChanged(new PropertyChangedEventArgs(nameof(Count)));
            OnPropertyChanged(new PropertyChangedEventArgs("Item[]"));
            OnCollectionChanged(new NotifyCollectionChangedEventArgs(NotifyCollectionChangedAction.Reset));
        }
    }

    private int CountThreads(string model)
    {
        if (detailsSnapshot is null)
        {
            return 0;
        }

        return detailsSnapshot.Threads.Count(thread => ClassifyThreadModel(thread.Model, thread.ModelLabel) == model);
    }

    private static string ClassifyThreadModel(string model, string label)
    {
        var tokens = Regex.Split(model + " " + label, "[^\\p{L}\\p{N}]+")
            .Select(static token => token.ToUpperInvariant())
            .Where(static token => token.Length > 0)
            .Where(static token => token is "SOL" or "TERRA" or "LUNA")
            .Distinct(StringComparer.Ordinal)
            .ToArray();
        return tokens.Length == 1 ? tokens[0] : "その他";
    }

    private static int ModelOrder(ApiModelUsage model)
    {
        return ModelOrder(model.Name);
    }

    private static int ModelOrder(ApiDetailsModelUsage model)
    {
        return ModelOrder(model.Name);
    }

    private static int ModelOrder(string name)
    {
        return name switch
        {
            "SOL" => 0,
            "TERRA" => 1,
            "LUNA" => 2,
            _ => int.MaxValue,
        };
    }

    private static ClientPresentationState GetReadyPresentationState(ApiStatusSnapshot validatedSnapshot)
    {
        if (validatedSnapshot.Quota is not { } quota)
        {
            return ClientPresentationState.Ready;
        }

        if (quota.RemainingPercent <= 2)
        {
            return ClientPresentationState.QuotaDanger;
        }

        if (quota.RemainingPercent <= 10)
        {
            return ClientPresentationState.QuotaWarning;
        }

        long now = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        if (quota.ResetAt > now && quota.ResetAt <= now + (long)TimeSpan.FromHours(24).TotalSeconds)
        {
            return ClientPresentationState.ResetWarning;
        }

        return ClientPresentationState.Ready;
    }

    private static string FormatUnixTime(long unixSeconds)
    {
        var utc = DateTimeOffset.FromUnixTimeSeconds(unixSeconds);
        return TimeZoneInfo.ConvertTime(utc, LocalizationService.DisplayTimeZone)
            .ToString("g", CultureInfo.CurrentCulture);
    }

    private string FormatRemainingDuration(long resetAt)
    {
        var seconds = Math.Max(0, resetAt - DateTimeOffset.UtcNow.ToUnixTimeSeconds());
        var days = seconds / 86_400;
        var hours = seconds % 86_400 / 3_600;
        var minutes = seconds % 3_600 / 60;
        if (seconds == 0)
        {
            return Texts.FormatRemaining(0, 0, 0, immediate: true);
        }

        if (days == 0 && hours == 0 && minutes == 0)
        {
            return Texts.FormatRemaining(0, 0, 0, lessThanMinute: true);
        }
        return Texts.FormatRemaining(days, hours, minutes);
    }

    private string StaleSuffix => presentationState is ClientPresentationState.TransportError or ClientPresentationState.ResponseError
        ? Texts.StaleValueSuffix
        : string.Empty;

    private void Notify([CallerMemberName] string? propertyName = null)
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        if (propertyName is nameof(IsAuthRequired) or nameof(IsAuthenticated))
        {
            Notify(nameof(IsUpdateNotificationVisible));
            Notify(nameof(IsUpdateActionVisible));
            Notify(nameof(ShowLastReceived));
        }
    }

    private void OnUpdatePropertyChanged(object? sender, PropertyChangedEventArgs eventArgs)
    {
        Notify(nameof(IsUpdateNotificationVisible));
        Notify(nameof(IsUpdateActionVisible));
        Notify(nameof(UpdateNotificationText));
        Notify(nameof(UpdateButtonText));
        Notify(nameof(ShowLastReceived));
    }

    private enum ClientPresentationState
    {
        Connecting,
        Ready,
        QuotaDanger,
        QuotaWarning,
        ResetWarning,
        Initializing,
        AuthRequired,
        ApiError,
        TransportError,
        ResponseError,
    }
}

public sealed class QuotaSegmentViewModel
{
    public QuotaSegmentViewModel(double fill)
    {
        Fill = fill;
    }

    public double Fill { get; }
}
