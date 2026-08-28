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
    private readonly IConnectionSupervisor? connectionSupervisor;
    private readonly Func<bool> authenticationLauncher;
    private readonly UpdateViewModel? update;
    private readonly object stateGate = new();
    private readonly CancellationTokenSource lifetime = new();
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
    private bool authLaunchSucceeded;
    private bool hasConnectionFailure;
    private bool disposed;
    private bool initialLoadPending = true;
    private bool explicitOperationActive;
    private GenerationContext? currentContext;
    private ClientSettings settingsSnapshot;
    private int started;

    public MainWindowViewModel(
        ILoopbackStatusClient client,
        ILoopbackDetailsClient? detailsClient = null,
        IConnectionSupervisor? connectionSupervisor = null,
        IWindowsUpdateCoordinator? updateCoordinator = null,
        Func<bool>? authenticationLauncher = null)
    {
        ArgumentNullException.ThrowIfNull(client);
        this.client = client;
        healthClient = client as ILoopbackHealthClient
            ?? throw new ArgumentException(
                "The status client must implement the fixed health boundary.",
                nameof(client));
        this.detailsClient = detailsClient;
        this.connectionSupervisor = connectionSupervisor;
        this.authenticationLauncher = authenticationLauncher ?? StartLinuxAuthenticationProcess;
        settingsSnapshot = App.CurrentSettings;
        update = updateCoordinator is null ? null : new UpdateViewModel(updateCoordinator);
        if (update is not null) update.PropertyChanged += OnUpdatePropertyChanged;
        refreshCommand = new AsyncCommand(RefreshManuallyAsync, () => CanRefresh);
        authCommand = new AsyncCommand(
            LaunchLinuxAuthenticationAsync,
            () => IsAuthRequired && !authLaunchSucceeded && !disposed);
        checkAuthCommand = new AsyncCommand(
            CheckAuthenticationAsync,
            () => IsAuthRequired && authLaunchSucceeded && CanRefresh);
        Models = new ReadOnlyObservableCollection<ModelUsageViewModel>(models);
        QuotaSegments = new ReadOnlyObservableCollection<QuotaSegmentViewModel>(quotaSegments);
        LocalizationService.LanguageChanged += OnLanguageChanged;
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    internal Task GenerationPipelineCompletionAsync(object context) =>
        context is GenerationContext generation
            ? generation.PipelineCompletion.Task
            : Task.FromException(new ArgumentException("Unknown generation context.", nameof(context)));

    public UiText Texts => LocalizationService.Current;

    public string ProductVersionText => ProductInfo.DisplayVersion;

    public ICommand RefreshCommand => refreshCommand;

    /// <summary>The StatusBanner's generic recovery command.</summary>
    public ICommand RetryCommand => refreshCommand;

    public ICommand AuthCommand => authCommand;

    public ICommand CheckAuthCommand => checkAuthCommand;

    public UpdateViewModel? Update => update;

    public ICommand? UpdateCommand => update?.UpdateCommand;

    public bool IsUpdateNotificationVisible
    {
        get
        {
            lock (stateGate)
            {
                return !IsAuthRequired &&
                    !hasConnectionFailure &&
                    !initialLoadPending &&
                    !refreshing &&
                    update?.IsNotificationVisible == true;
            }
        }
    }

    public bool IsUpdateActionVisible
    {
        get
        {
            lock (stateGate)
            {
                return !IsAuthRequired &&
                    !hasConnectionFailure &&
                    !initialLoadPending &&
                    !refreshing &&
                    update?.IsUpdateActionVisible == true;
            }
        }
    }

    public string UpdateNotificationText => update?.NotificationText ?? string.Empty;

    public string UpdateButtonText => update?.ActionText ?? Texts.UpdateButtonText;

    public bool ShowLastReceived => IsAuthenticated && !IsUpdateNotificationVisible;

    public ReadOnlyObservableCollection<ModelUsageViewModel> Models { get; }

    public ReadOnlyObservableCollection<QuotaSegmentViewModel> QuotaSegments { get; }

    public bool CanRefresh
    {
        get
        {
            lock (stateGate)
            {
                return !refreshing && !explicitOperationActive && !disposed;
            }
        }
    }

    public string RefreshButtonText => refreshing ? Texts.Refreshing : Texts.Refresh;

    /// <summary>True when the generic recovery CTA is the sole primary action.</summary>
    public bool IsRetryVisible
    {
        get
        {
            lock (stateGate)
            {
                return !disposed &&
                    !initialLoadPending &&
                    !refreshing &&
                    hasConnectionFailure &&
                    !IsAuthRequired;
            }
        }
    }

    /// <summary>True for an explicit retry in flight; the button is disabled.</summary>
    public bool IsRefreshingVisible
    {
        get
        {
            lock (stateGate)
            {
                return !disposed &&
                    !initialLoadPending &&
                    refreshing &&
                    explicitOperationActive &&
                    !IsAuthRequired;
            }
        }
    }

    public bool IsAuthStartVisible
    {
        get
        {
            lock (stateGate)
            {
                return IsAuthRequired && !authLaunchSucceeded;
            }
        }
    }

    public bool IsAuthCheckVisible
    {
        get
        {
            lock (stateGate)
            {
                return IsAuthRequired && authLaunchSucceeded;
            }
        }
    }

    public bool HasQuota => snapshot?.Quota is not null;

    public bool HasModels => models.Count > 0;

    public bool HasNoModels => !HasModels;

    public bool IsAuthRequired
    {
        get
        {
            lock (stateGate)
            {
                return presentationState == ClientPresentationState.AuthRequired;
            }
        }
    }

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

    /// <summary>Installs one startup context and starts the single polling timer.</summary>
    public void Start()
    {
        GenerationContext? startupContext = null;
        lock (stateGate)
        {
            if (started != 0 || disposed)
            {
                return;
            }

            started = 1;
            if (currentContext is null)
            {
                settingsSnapshot = App.CurrentSettings;
                startupContext = new GenerationContext(settingsSnapshot);
                currentContext = startupContext;
            }
        }

        update?.Start();
        if (startupContext is not null)
        {
            _ = RunStartupAsync(startupContext);
        }
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
        ArgumentNullException.ThrowIfNull(settings);
        if (!ConnectionSelectors.IsValid(settings) || connectionSupervisor is null)
        {
            return false;
        }

        var operation = BeginExplicitOperation(
            settings,
            ExplicitOperationKind.Restart,
            adoptSettings: true);
        if (operation is null)
        {
            return false;
        }

        _ = RunExplicitOperationAsync(operation);
        return true;
    }

    public void Dispose()
    {
        RetiredContext? retirement;
        lock (stateGate)
        {
            if (disposed)
            {
                return;
            }

            disposed = true;
            explicitOperationActive = false;
            retirement = RetireContextLocked(currentContext);
            currentContext = null;
            LocalizationService.LanguageChanged -= OnLanguageChanged;
            if (update is not null)
            {
                update.PropertyChanged -= OnUpdatePropertyChanged;
            }

            // The availability transition is the only synchronous UI change
            // owned by Dispose.  The post-Dispose baseline starts after this
            // lock exits, so late generations cannot add notifications.
            Notify(nameof(CanRefresh));
            refreshCommand.RaiseCanExecuteChanged();
            authCommand.RaiseCanExecuteChanged();
            checkAuthCommand.RaiseCanExecuteChanged();
            ClearModels();
        }

        lifetime.Cancel();
        CancelRetirement(retirement);
        if (update is not null)
        {
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
    }

    private async Task RunPollingAsync(CancellationToken cancellationToken)
    {
        try
        {
            using var timer = new PeriodicTimer(TimeSpan.FromSeconds(10));
            while (await timer.WaitForNextTickAsync(cancellationToken))
            {
                await RunPeriodicRefreshAsync();
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            // Normal shutdown.
        }
    }

    private Task RefreshManuallyAsync()
    {
        ClientSettings settings;
        lock (stateGate)
        {
            settings = settingsSnapshot;
        }

        var operation = BeginExplicitOperation(settings, ExplicitOperationKind.Restart);
        return operation is null
            ? Task.CompletedTask
            : RunExplicitOperationAsync(operation);
    }

    private Task CheckAuthenticationAsync()
    {
        ClientSettings settings;
        lock (stateGate)
        {
            settings = settingsSnapshot;
        }

        var operation = BeginExplicitOperation(settings, ExplicitOperationKind.Ensure);
        return operation is null
            ? Task.CompletedTask
            : RunExplicitOperationAsync(operation);
    }

    private Task LaunchLinuxAuthenticationAsync()
    {
        GenerationContext? context;
        lock (stateGate)
        {
            if (disposed)
            {
                return Task.CompletedTask;
            }
            context = currentContext;
        }

        bool launched;
        try { launched = authenticationLauncher(); }
        catch { launched = false; }

        if (context is not null)
        {
            MutateIfCurrent(context, () =>
            {
                authLaunchFailed = !launched;
                authLaunchSucceeded = launched;
                Notify(nameof(StatusDetail));
                Notify(nameof(IsAuthStartVisible));
                Notify(nameof(IsAuthCheckVisible));
                authCommand.RaiseCanExecuteChanged();
                checkAuthCommand.RaiseCanExecuteChanged();
            });
        }
        return Task.CompletedTask;
    }

    private void OnLanguageChanged(object? sender, EventArgs eventArgs)
    {
        lock (stateGate)
        {
            if (disposed) return;
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
            Notify(nameof(ResetAtText));
            Notify(nameof(ObservedAtText));
            Notify(nameof(ActiveThreadCountText));
            Notify(nameof(EstimatedCostText));
            Notify(nameof(ModelUsageUnavailableText));
        }
    }

    private static bool StartLinuxAuthenticationProcess()
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
        return true;
    }

    private async Task RunStartupAsync(GenerationContext context)
    {
        var lease = TryLease(context);
        if (lease is null)
        {
            return;
        }

        try
        {
            bool ready = connectionSupervisor is null
                ? ConnectionSelectors.IsValid(context.Settings)
                : connectionSupervisor.EnsureStarted(context.Settings);
            if (!ready)
            {
                MutateIfCurrent(context, () => ApplyFailure(StatusFetchFailure.Transport));
                return;
            }

            await FetchCycleAsync(context, lease.Token);
        }
        catch (OperationCanceledException)
        {
            // Lifetime/context retirement owns cancellation.  A transport
            // which ignores it is still fenced by MutateIfCurrent below.
        }
        catch
        {
            MutateIfCurrent(context, () => ApplyFailure(StatusFetchFailure.Transport));
        }
        finally
        {
            CompleteLease(context, lease, explicitOperation: false);
        }
    }

    private async Task RunPeriodicRefreshAsync()
    {
        GenerationContext? context;
        lock (stateGate)
        {
            context = currentContext;
        }

        if (context is null) return;
        var lease = TryLease(context);
        if (lease is null) return;
        try
        {
            await FetchCycleAsync(context, lease.Token);
        }
        finally
        {
            CompleteLease(context, lease, explicitOperation: false);
        }
    }

    private async Task RunExplicitOperationAsync(ExplicitOperation operation)
    {
        try
        {
            if (operation.Kind == ExplicitOperationKind.Restart)
            {
                var outcome = connectionSupervisor?.RestartExplicit(operation.Context.Settings)
                    ?? (ConnectionSelectors.IsValid(operation.Context.Settings)
                        ? ConnectionRestartOutcome.NoChildRequired
                        : ConnectionRestartOutcome.InvalidSettings);
                if (outcome is not ConnectionRestartOutcome.Started and
                    not ConnectionRestartOutcome.NoChildRequired)
                {
                    MutateIfCurrent(operation.Context, () => ApplyFailure(StatusFetchFailure.Transport));
                    return;
                }
            }
            else
            {
                var ready = connectionSupervisor is null
                    ? ConnectionSelectors.IsValid(operation.Context.Settings)
                    : connectionSupervisor.EnsureStarted(operation.Context.Settings);
                if (!ready)
                {
                    MutateIfCurrent(operation.Context, () => ApplyFailure(StatusFetchFailure.Transport));
                    return;
                }
            }

            await FetchCycleAsync(operation.Context, operation.Lease.Token);
        }
        catch (OperationCanceledException)
        {
            // Retired/disposed contexts are fenced by the state gate.
        }
        catch
        {
            MutateIfCurrent(operation.Context, () => ApplyFailure(StatusFetchFailure.Transport));
        }
        finally
        {
            CompleteLease(operation.Context, operation.Lease, explicitOperation: true);
        }
    }

    private async Task FetchCycleAsync(GenerationContext context, CancellationToken cancellationToken)
    {
        try
        {
            var health = await healthClient.FetchHealthAsync(cancellationToken);
            if (!health.IsSuccess)
            {
                MutateIfCurrent(context, () => ApplyFailure(health.Failure == HealthFetchFailure.Response
                    ? StatusFetchFailure.Response
                    : StatusFetchFailure.Transport));
                return;
            }

            var result = await client.FetchAsync(cancellationToken);
            if (!result.IsSuccess || result.Snapshot is not { } validatedSnapshot)
            {
                MutateIfCurrent(context, () => ApplyFailure(result.Failure ?? StatusFetchFailure.Transport));
                return;
            }

            // Details are account-scoped.  Never publish status alone while a
            // details client exists; only a matching pair is a complete root.
            if (detailsClient is not null && validatedSnapshot.Authenticated &&
                validatedSnapshot.State != ApiState.AuthRequired)
            {
                DetailsFetchResult detailsResult;
                try
                {
                    detailsResult = await detailsClient.FetchDetailsAsync(cancellationToken);
                }
                catch (OperationCanceledException)
                {
                    throw;
                }
                catch
                {
                    detailsResult = DetailsFetchResult.FromFailure(DetailsFetchFailure.Transport);
                }

                if (detailsResult.IsSuccess && detailsResult.Snapshot is { } validatedDetails &&
                    validatedDetails.Authenticated &&
                    validatedDetails.State != ApiState.AuthRequired &&
                    HasSamePublishedPair(validatedSnapshot, validatedDetails) &&
                    HasSamePublicCore(validatedSnapshot, validatedDetails))
                {
                    MutateIfCurrent(context, () => ApplyAuthenticatedPair(validatedSnapshot, validatedDetails));
                }
                else
                {
                    MutateIfCurrent(context, () =>
                    {
                        detailsFailure = detailsResult.Failure == DetailsFetchFailure.Transport
                            ? DetailsFetchFailure.Transport
                            : DetailsFetchFailure.Response;
                        Notify(nameof(DetailsStatusText));
                        Notify(nameof(DetailsStatusAutomationText));
                        ApplyFailure(detailsFailure == DetailsFetchFailure.Transport
                            ? StatusFetchFailure.Transport
                            : StatusFetchFailure.Response);
                    });
                }
            }
            else
            {
                MutateIfCurrent(context, () => ApplySnapshot(validatedSnapshot));
            }
        }
        catch (OperationCanceledException)
        {
            // Cancellation is a lifecycle event.  No UI mutation belongs to
            // this transaction, including its eventual finally path.
        }
        catch
        {
            MutateIfCurrent(context, () => ApplyFailure(StatusFetchFailure.Transport));
        }
    }

    private ExplicitOperation? BeginExplicitOperation(
        ClientSettings settings,
        ExplicitOperationKind kind,
        bool adoptSettings = false)
    {
        RetiredContext? retirement;
        ExplicitOperation operation;
        lock (stateGate)
        {
            if (disposed || explicitOperationActive)
            {
                return null;
            }

            explicitOperationActive = true;
            if (adoptSettings)
            {
                settingsSnapshot = settings;
            }

            var previous = currentContext;
            var next = new GenerationContext(settings)
            {
                IsExplicitOperation = true,
            };
            currentContext = next;
            retirement = RetireContextLocked(previous);
            var lease = TryLeaseLocked(next);
            if (lease is null)
            {
                // Linked-token creation can only fail during teardown. Keep
                // the old context authoritative if that rare race wins.
                currentContext = previous;
                explicitOperationActive = false;
                return null;
            }

            operation = new ExplicitOperation(next, lease, kind);
        }

        CancelRetirement(retirement);
        return operation;
    }

    private RefreshLease? TryLease(GenerationContext context)
    {
        lock (stateGate)
        {
            return TryLeaseLocked(context);
        }
    }

    private RefreshLease? TryLeaseLocked(GenerationContext context)
    {
        if (disposed || context.Retired || !ReferenceEquals(currentContext, context) ||
            context.RefreshInFlight)
        {
            return null;
        }

        context.RefreshInFlight = true;
        context.ActiveLeases++;
        try
        {
            var linked = CancellationTokenSource.CreateLinkedTokenSource(
                lifetime.Token,
                context.Retirement.Token);
            SetRefreshingLocked(true);
            return new RefreshLease(linked);
        }
        catch
        {
            context.ActiveLeases--;
            context.RefreshInFlight = false;
            return null;
        }
    }

    private void CompleteLease(
        GenerationContext context,
        RefreshLease lease,
        bool explicitOperation)
    {
        lease.Dispose();
        RetiredContext? retirement = null;
        lock (stateGate)
        {
            context.RefreshInFlight = false;
            if (context.ActiveLeases > 0)
            {
                context.ActiveLeases--;
            }

            if (ReferenceEquals(currentContext, context) && !context.Retired && !disposed)
            {
                if (explicitOperation && context.IsExplicitOperation)
                {
                    context.IsExplicitOperation = false;
                    explicitOperationActive = false;
                }

                if (initialLoadPending)
                {
                    initialLoadPending = false;
                    Notify(nameof(IsStartupLoading));
                    Notify(nameof(ShowAuthenticatedContent));
                }

                SetRefreshingLocked(false);
                Notify(nameof(IsRetryVisible));
                Notify(nameof(IsRefreshingVisible));
                Notify(nameof(IsUpdateNotificationVisible));
                Notify(nameof(IsUpdateActionVisible));
            }

            retirement = ReleaseRetirementLocked(context);
        }
        CancelRetirement(retirement);
        context.PipelineCompletion.TrySetResult(context);
    }

    private RetiredContext? RetireContextLocked(GenerationContext? context)
    {
        if (context is null || context.Retired)
        {
            return null;
        }

        context.Retired = true;
        var disposeAfterCancel = context.ActiveLeases == 0;
        if (disposeAfterCancel)
        {
            context.RetirementDisposed = true;
        }
        return new RetiredContext(context, disposeAfterCancel);
    }

    private RetiredContext? ReleaseRetirementLocked(GenerationContext context)
    {
        if (!context.Retired || context.ActiveLeases != 0 || context.RetirementDisposed)
        {
            return null;
        }

        context.RetirementDisposed = true;
        return new RetiredContext(context, DisposeAfterCancel: true);
    }

    private static void CancelRetirement(RetiredContext? retirement)
    {
        if (retirement is null)
        {
            return;
        }

        try
        {
            retirement.Context.Retirement.Cancel();
        }
        catch (ObjectDisposedException)
        {
            return;
        }
        finally
        {
            if (retirement.DisposeAfterCancel)
            {
                retirement.Context.Retirement.Dispose();
            }
        }
    }

    /// <summary>
    /// The only presentation commit gate.  The reference identity and
    /// retirement check remain adjacent to every mutation and notification,
    /// including late cancellation-ignoring continuations.
    /// </summary>
    private bool MutateIfCurrent(GenerationContext context, Action mutation)
    {
        lock (stateGate)
        {
            if (disposed || context.Retired || !ReferenceEquals(currentContext, context))
            {
                return false;
            }

            mutation();
            return true;
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

    private static bool HasSamePublishedPair(
        ApiStatusSnapshot status,
        ApiDetailsSnapshot details) =>
        status.PublishedPair is { } statusPair &&
        details.PublishedPair is { } detailsPair &&
        statusPair == detailsPair;

    private void ApplySnapshot(ApiStatusSnapshot validatedSnapshot)
    {
        snapshot = validatedSnapshot;
        lastReceivedAt = DateTimeOffset.Now;
        hasConnectionFailure = validatedSnapshot.State == ApiState.Error;
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
            authLaunchFailed = false;
            authLaunchSucceeded = false;
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
        Notify(nameof(IsRetryVisible));
        Notify(nameof(IsRefreshingVisible));
        Notify(nameof(IsUpdateNotificationVisible));
        Notify(nameof(IsUpdateActionVisible));
    }

    private void ApplyAuthenticatedPair(
        ApiStatusSnapshot validatedSnapshot,
        ApiDetailsSnapshot validatedDetails)
    {
        // All observable backing state is assigned before the first collection
        // or property notification.  The commit itself is synchronous so an
        // observer can never see status from one generation with details from
        // another.
        snapshot = validatedSnapshot;
        detailsSnapshot = validatedDetails;
        detailsFailure = null;
        lastReceivedAt = DateTimeOffset.Now;
        hasConnectionFailure = validatedSnapshot.State == ApiState.Error;
        authLaunchFailed = false;
        authLaunchSucceeded = false;
        presentationState = validatedSnapshot.State switch
        {
            ApiState.Ready => GetReadyPresentationState(validatedSnapshot),
            ApiState.Initializing => ClientPresentationState.Initializing,
            ApiState.AuthRequired => ClientPresentationState.AuthRequired,
            ApiState.Error => ClientPresentationState.ApiError,
            _ => ClientPresentationState.ResponseError,
        };

        ReplaceModels(validatedDetails.Models.OrderBy(ModelOrder), notify: false);
        RebuildQuotaSegments(notify: false);
        models.NotifyReset();
        quotaSegments.NotifyReset();

        Notify(nameof(HasDetails));
        Notify(nameof(DetailsSnapshot));
        Notify(nameof(DetailsStatusText));
        Notify(nameof(DetailsStatusAutomationText));
        Notify(nameof(ModelUsagePeriodText));
        Notify(nameof(EstimatedCostText));
        NotifySnapshotProperties(quotaAlreadyRebuilt: true);
        authCommand.RaiseCanExecuteChanged();
        checkAuthCommand.RaiseCanExecuteChanged();
        Notify(nameof(IsRetryVisible));
        Notify(nameof(IsRefreshingVisible));
        Notify(nameof(IsUpdateNotificationVisible));
        Notify(nameof(IsUpdateActionVisible));
    }

    private void ApplyFailure(StatusFetchFailure failure)
    {
        hasConnectionFailure = true;
        presentationState = failure == StatusFetchFailure.Response
            ? ClientPresentationState.ResponseError
            : ClientPresentationState.TransportError;
        NotifyStatusProperties();
        Notify(nameof(LastReceivedText));
        Notify(nameof(ShowLastReceived));
        authCommand.RaiseCanExecuteChanged();
        checkAuthCommand.RaiseCanExecuteChanged();
        Notify(nameof(IsAuthStartVisible));
        Notify(nameof(IsAuthCheckVisible));
        Notify(nameof(IsRetryVisible));
        Notify(nameof(IsRefreshingVisible));
        Notify(nameof(IsUpdateNotificationVisible));
        Notify(nameof(IsUpdateActionVisible));
    }

    private void SetRefreshingLocked(bool value)
    {
        refreshing = value;
        Notify(nameof(CanRefresh));
        Notify(nameof(RefreshButtonText));
        refreshCommand.RaiseCanExecuteChanged();
        checkAuthCommand.RaiseCanExecuteChanged();
        Notify(nameof(IsRetryVisible));
        Notify(nameof(IsRefreshingVisible));
        Notify(nameof(IsUpdateNotificationVisible));
        Notify(nameof(IsUpdateActionVisible));
        Notify(nameof(ShowLastReceived));
    }

    private void NotifySnapshotProperties(bool quotaAlreadyRebuilt = false)
    {
        if (quotaAlreadyRebuilt)
        {
            Notify(nameof(QuotaSegments));
        }
        else
        {
            RebuildQuotaSegments();
        }

        Notify(nameof(HasQuota));
        Notify(nameof(RemainingPercentText));
        Notify(nameof(RemainingPercentValue));
        Notify(nameof(QuotaWindowText));
        Notify(nameof(QuotaRemainingText));
        Notify(nameof(QuotaRemainingPeriodValue));
        Notify(nameof(AuthenticationText));
        Notify(nameof(IsAuthRequired));
        Notify(nameof(IsAuthenticated));
        Notify(nameof(IsAuthStartVisible));
        Notify(nameof(IsAuthCheckVisible));
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

    private void RebuildQuotaSegments(bool notify = true)
    {
        var fraction = snapshot?.Quota is { } quota
            ? Math.Clamp((quota.ResetAt - DateTimeOffset.UtcNow.ToUnixTimeSeconds()) /
                         (double)Math.Max(1, quota.WindowSeconds), 0, 1)
            : 0;
        quotaSegments.ReplaceAll(Enumerable.Range(0, 7)
            .Select(index => new QuotaSegmentViewModel(Math.Clamp(fraction * 7 - index, 0, 1))),
            notify: false);

        if (notify)
        {
            quotaSegments.NotifyReset();
            Notify(nameof(QuotaSegments));
        }
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

    private void ReplaceModels(IEnumerable<ApiDetailsModelUsage> source, bool notify = true)
    {
        var next = source.Select(static model => new ModelUsageViewModel(model)).ToArray();
        var previous = models.ToArray();
        models.ReplaceAll(next, notify);
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
        public void ReplaceAll(IEnumerable<T> values, bool notify = true)
        {
            ArgumentNullException.ThrowIfNull(values);
            CheckReentrancy();
            Items.Clear();
            foreach (var value in values)
            {
                Items.Add(value);
            }

            if (notify)
            {
                NotifyReset();
            }
        }

        public void NotifyReset()
        {
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
        lock (stateGate)
        {
            if (disposed) return;
            Notify(nameof(IsUpdateNotificationVisible));
            Notify(nameof(IsUpdateActionVisible));
            Notify(nameof(UpdateNotificationText));
            Notify(nameof(UpdateButtonText));
            Notify(nameof(ShowLastReceived));
        }
    }

    private enum ExplicitOperationKind
    {
        Restart,
        Ensure,
    }

    private sealed class GenerationContext(ClientSettings settings)
    {
        public ClientSettings Settings { get; } = settings;

        public CancellationTokenSource Retirement { get; } = new();

        public TaskCompletionSource<object?> PipelineCompletion { get; } = new(
            TaskCreationOptions.RunContinuationsAsynchronously);

        public bool Retired { get; set; }

        public bool RetirementDisposed { get; set; }

        public int ActiveLeases { get; set; }

        public bool RefreshInFlight { get; set; }

        public bool IsExplicitOperation { get; set; }
    }

    private sealed class RefreshLease(CancellationTokenSource linkedSource) : IDisposable
    {
        public CancellationToken Token => linkedSource.Token;

        public void Dispose() => linkedSource.Dispose();
    }

    private sealed record ExplicitOperation(
        GenerationContext Context,
        RefreshLease Lease,
        ExplicitOperationKind Kind);

    private sealed record RetiredContext(
        GenerationContext Context,
        bool DisposeAfterCancel);

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
