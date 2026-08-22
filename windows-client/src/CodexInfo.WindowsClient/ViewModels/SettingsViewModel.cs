// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.CompilerServices;
using CodexInfo.WindowsClient.Localization;
using CodexInfo.WindowsClient.Settings;

namespace CodexInfo.WindowsClient.ViewModels;

public sealed class SettingsViewModel : INotifyPropertyChanged
{
    private readonly ClientSettingsStore store;
    private readonly MainWindowViewModel? main;
    private string selectedLanguageCode;
    private string selectedTimeZoneId;

    public SettingsViewModel(ClientSettingsStore store, MainWindowViewModel? main = null)
    {
        this.store = store;
        this.main = main;
        selectedLanguageCode = LocalizationService.Current.LanguageCode;
        selectedTimeZoneId = App.CurrentSettings.TimeZoneId;
        LanguageOptions = new ReadOnlyCollection<UiText>(LocalizationService.Languages.ToList());
        LocalizationService.LanguageChanged += OnLanguageChanged;
        if (main is not null) main.PropertyChanged += OnMainPropertyChanged;
    }

    public event PropertyChangedEventHandler? PropertyChanged;
    public event EventHandler? Saved;

    public ReadOnlyCollection<UiText> LanguageOptions { get; }
    public UiText Texts => LocalizationService.Current;
    public string SelectedLanguageCode
    {
        get => selectedLanguageCode;
        set
        {
            if (string.Equals(selectedLanguageCode, value, StringComparison.OrdinalIgnoreCase)) return;
            selectedLanguageCode = value;
            Notify();
            Notify(nameof(SelectedLanguage));
        }
    }

    public UiText? SelectedLanguage => LanguageOptions.FirstOrDefault(option => option.LanguageCode == selectedLanguageCode);
    public IReadOnlyList<TimeZoneOption> TimeZoneOptions =>
    [
        new("local", Texts.LocalTimeZone),
        new("UTC", Texts.UtcTimeZone),
    ];
    public string SelectedTimeZoneId { get => selectedTimeZoneId; set { if (selectedTimeZoneId == value) return; selectedTimeZoneId = value; Notify(); Notify(nameof(SelectedTimeZone)); } }
    public string SelectedTimeZone => selectedTimeZoneId == "UTC" ? Texts.UtcTimeZone : Texts.LocalTimeZone;
    public string CurrentEndpoint => Texts.ConnectionEndpoint;
    public string StatusTitle => main?.StatusTitle ?? Texts.Unavailable;
    public string StatusDetail => main?.StatusDetail ?? Texts.UnavailableDetails;
    public bool CanAuthenticate => main?.IsAuthRequired == true;
    public void Refresh() => main?.RefreshCommand.Execute(null);
    public void StartAuthentication() => main?.AuthCommand.Execute(null);

    public void Save()
    {
        LocalizationService.SetLanguage(selectedLanguageCode);
        selectedTimeZoneId = string.Equals(selectedTimeZoneId, "UTC", StringComparison.OrdinalIgnoreCase)
            ? "UTC"
            : "local";
        LocalizationService.SetTimeZone(selectedTimeZoneId);
        var current = store.Load();
        var updated = current with { Language = LocalizationService.Current.LanguageCode, TimeZoneId = selectedTimeZoneId };
        store.Save(updated);
        App.CurrentSettings = updated;
        Saved?.Invoke(this, EventArgs.Empty);
    }

    public void Dispose()
    {
        LocalizationService.LanguageChanged -= OnLanguageChanged;
        if (main is not null) main.PropertyChanged -= OnMainPropertyChanged;
    }

    private void OnLanguageChanged(object? sender, EventArgs eventArgs)
    {
        Notify(nameof(Texts));
        Notify(nameof(CurrentEndpoint));
        Notify(nameof(SelectedTimeZone));
        Notify(nameof(TimeZoneOptions));
        Notify(nameof(StatusTitle));
        Notify(nameof(StatusDetail));
        Notify(nameof(CanAuthenticate));
    }

    private void OnMainPropertyChanged(object? sender, PropertyChangedEventArgs eventArgs)
    {
        if (eventArgs.PropertyName is nameof(MainWindowViewModel.StatusTitle) or nameof(MainWindowViewModel.StatusDetail) or nameof(MainWindowViewModel.IsAuthRequired))
        {
            Notify(nameof(StatusTitle)); Notify(nameof(StatusDetail)); Notify(nameof(CanAuthenticate));
        }
    }

    private void Notify([CallerMemberName] string? propertyName = null) => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
}

public sealed record TimeZoneOption(string Id, string Label);

public sealed class SetupViewModel : INotifyPropertyChanged, IDisposable
{
    private readonly MainWindowViewModel main;
    private Process? sshProcess;
    private string sshUser = string.Empty;
    private string sshHost = string.Empty;
    private string? selectedSshConfigAlias;
    private bool sshLaunchFailed;
    private string selectedConnectionProfile;
    private string selectedConnectionSelector;
    private int step;

    public SetupViewModel(MainWindowViewModel main)
    {
        this.main = main;
        main.PropertyChanged += OnMainPropertyChanged;
        LocalizationService.LanguageChanged += OnLanguageChanged;
        SshConfigAliases = LoadSshConfigAliases();
        WslDistributions = LoadWslDistributions();
        var saved = App.CurrentSettings;
        selectedConnectionProfile = saved.ConnectionProfile is ConnectionProfiles.Wsl or ConnectionProfiles.SshConfigAlias
            ? saved.ConnectionProfile
            : ConnectionProfiles.None;
        selectedConnectionSelector = selectedConnectionProfile == ConnectionProfiles.None
            ? ConnectionSelectors.None
            : saved.ConnectionSelector;
    }

    public event PropertyChangedEventHandler? PropertyChanged;
    public UiText Texts => LocalizationService.Current;
    public IReadOnlyList<ConnectionProfileOption> ConnectionProfileOptions =>
    [
        new(ConnectionProfiles.None, Texts.ConnectionProfileNone),
        new(ConnectionProfiles.Wsl, Texts.ConnectionProfileWsl),
        new(ConnectionProfiles.SshConfigAlias, Texts.ConnectionProfileSsh),
    ];
    public string SelectedConnectionProfile
    {
        get => selectedConnectionProfile;
        set
        {
            var next = value is ConnectionProfiles.Wsl or ConnectionProfiles.SshConfigAlias
                ? value
                : ConnectionProfiles.None;
            if (selectedConnectionProfile == next) return;
            selectedConnectionProfile = next;
            selectedConnectionSelector = next switch
            {
                ConnectionProfiles.Wsl when WslDistributions.Count > 0 => WslDistributions[0],
                ConnectionProfiles.SshConfigAlias when SshConfigAliases.Count > 0 => SshConfigAliases[0],
                _ => ConnectionSelectors.None,
            };
            Notify();
            Notify(nameof(SelectedConnectionSelector));
            Notify(nameof(ConnectionSelectorOptions));
            Notify(nameof(CanContinue));
            Notify(nameof(SshCommand));
        }
    }
    public string SelectedConnectionSelector
    {
        get => selectedConnectionSelector;
        set
        {
            var next = value ?? ConnectionSelectors.None;
            if (selectedConnectionSelector == next) return;
            selectedConnectionSelector = next;
            if (selectedConnectionProfile == ConnectionProfiles.SshConfigAlias) SshHost = next;
            Notify();
            Notify(nameof(CanContinue));
            Notify(nameof(SshCommand));
        }
    }
    public IReadOnlyList<string> ConnectionSelectorOptions => selectedConnectionProfile switch
    {
        ConnectionProfiles.Wsl => WslDistributions,
        ConnectionProfiles.SshConfigAlias => SshConfigAliases,
        _ => [ConnectionSelectors.None],
    };
    public IReadOnlyList<string> WslDistributions { get; }
    public int Step { get => step; private set { step = value; Notify(); Notify(nameof(IsConnectionStep)); Notify(nameof(IsAuthStep)); Notify(nameof(IsDoneStep)); } }
    public bool IsConnectionStep => Step == 0;
    public bool IsAuthStep => Step == 1;
    public bool IsDoneStep => Step == 2;
    public IReadOnlyList<string> SshConfigAliases { get; }
    public string? SelectedSshConfigAlias
    {
        get => selectedSshConfigAlias;
        set
        {
            if (selectedSshConfigAlias == value) return;
            selectedSshConfigAlias = value;
            if (!string.IsNullOrWhiteSpace(value)) SshHost = value;
            Notify();
        }
    }
    public string SshUser { get => sshUser; set { if (sshUser == value) return; sshUser = value; sshLaunchFailed = false; Notify(); Notify(nameof(SshCommand)); Notify(nameof(CanStartSsh)); Notify(nameof(SshStatusText)); } }
    public string SshHost { get => sshHost; set { if (sshHost == value) return; sshHost = value; sshLaunchFailed = false; Notify(); Notify(nameof(SshCommand)); Notify(nameof(CanStartSsh)); Notify(nameof(SshStatusText)); } }
    public bool SshRunning => sshProcess is { HasExited: false };
    public bool CanStartSsh => IsSafeSshHost(SshHost) && (string.IsNullOrWhiteSpace(SshUser) || IsSafeSshUser(SshUser));
    public string SshActionText => SshRunning ? Texts.SshStop : Texts.SshStart;
    public string SshStatusText => sshLaunchFailed
        ? Texts.SshLaunchFailedStatus
        : SshRunning
            ? Texts.SshRunningStatus
            : CanStartSsh
                ? Texts.SshReadyStatus
                : Texts.SshNotReady;
    public bool CanContinue => Step switch
    {
        0 => IsConnectionSelectionValid && (main.HasQuota || main.IsAuthRequired || main.HasDetails),
        // Starting `wsl.exe codex login` is not authentication completion.
        // The user must explicitly re-check and receive an authenticated
        // status snapshot before the setup flow can be marked complete.
        1 => main.IsAuthenticated,
        _ => true,
    };
    public string SshCommand => CanStartSsh
        ? $"ssh -N -L 8787:127.0.0.1:8787 {(string.IsNullOrWhiteSpace(SshUser) ? (SelectedConnectionSelector == ConnectionSelectors.None ? SshHost : SelectedConnectionSelector) : $"{SshUser}@{(SelectedConnectionSelector == ConnectionSelectors.None ? SshHost : SelectedConnectionSelector)}")}"
        : Texts.SshCommand;
    public string ApiCommand => Texts.ApiCommand;
    public string StatusTitle => main.StatusTitle;
    public string StatusDetail => main.StatusDetail;
    public string ContinueText => Step == 2 ? Texts.Close : Texts.Continue;

    public void Refresh() => main.RefreshCommand.Execute(null);
    public void StartAuthentication() => main.AuthCommand.Execute(null);
    public void StartOrStopSsh()
    {
        if (SshRunning)
        {
            try { sshProcess?.Kill(entireProcessTree: true); } catch { /* process may have exited */ }
            return;
        }

        if (!CanStartSsh) return;
        try
        {
            sshLaunchFailed = false;
            var process = new Process
            {
                StartInfo = new ProcessStartInfo
                {
                    FileName = "ssh.exe",
                    UseShellExecute = true,
                    WorkingDirectory = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
                },
                EnableRaisingEvents = true,
            };
            process.StartInfo.ArgumentList.Add("-N");
            process.StartInfo.ArgumentList.Add("-L");
            process.StartInfo.ArgumentList.Add("8787:127.0.0.1:8787");
            process.StartInfo.ArgumentList.Add(string.IsNullOrWhiteSpace(SshUser) ? SshHost : $"{SshUser}@{SshHost}");
            process.Exited += OnSshExited;
            if (process.Start())
            {
                sshProcess = process;
                Notify(nameof(SshRunning)); Notify(nameof(SshActionText));
                Refresh();
            }
            else
            {
                process.Dispose();
                sshLaunchFailed = true;
                Notify(nameof(SshStatusText));
            }
        }
        catch
        {
            sshProcess = null;
            sshLaunchFailed = true;
            Notify(nameof(SshRunning)); Notify(nameof(SshActionText));
            Notify(nameof(SshStatusText));
        }
    }

    public bool IsConnectionSelectionValid => selectedConnectionProfile switch
    {
        ConnectionProfiles.Wsl => ConnectionSelectors.IsWslToken(selectedConnectionSelector)
            && WslDistributions.Contains(selectedConnectionSelector, StringComparer.Ordinal),
        ConnectionProfiles.SshConfigAlias => ConnectionSelectors.IsSshAlias(selectedConnectionSelector)
            && SshConfigAliases.Contains(selectedConnectionSelector, StringComparer.OrdinalIgnoreCase),
        // `none` is a supported profile for an already-running local
        // listener (for example a manually managed WSL service). It becomes
        // configured only after the health/status path has been observed.
        ConnectionProfiles.None => string.IsNullOrWhiteSpace(SshHost)
            && string.IsNullOrWhiteSpace(SshUser)
            && (main.HasQuota || main.IsAuthRequired || main.HasDetails),
        _ => false,
    };

    public ClientSettings BuildSettings(ClientSettings current) => current with
    {
        ConnectionConfigured = IsConnectionSelectionValid,
        ConnectionProfile = IsConnectionSelectionValid ? selectedConnectionProfile : ConnectionProfiles.None,
        ConnectionSelector = IsConnectionSelectionValid ? selectedConnectionSelector : ConnectionSelectors.None,
    };
    public void Continue()
    {
        if (!CanContinue)
        {
            if (IsAuthStep)
            {
                StartAuthentication();
            }
            return;
        }

        Step = Math.Min(2, Step + 1);
    }
    public void Dispose()
    {
        main.PropertyChanged -= OnMainPropertyChanged;
        LocalizationService.LanguageChanged -= OnLanguageChanged;
        sshProcess?.Dispose();
    }

    private void OnMainPropertyChanged(object? sender, PropertyChangedEventArgs eventArgs)
    {
        if (eventArgs.PropertyName is nameof(MainWindowViewModel.StatusTitle)
            or nameof(MainWindowViewModel.StatusDetail)
            or nameof(MainWindowViewModel.IsAuthRequired)
            or nameof(MainWindowViewModel.IsAuthenticated)
            or nameof(MainWindowViewModel.HasQuota)
            or nameof(MainWindowViewModel.HasDetails))
        {
            Notify(nameof(StatusTitle));
            Notify(nameof(StatusDetail));
            Notify(nameof(CanContinue));
        }
    }

    private void OnLanguageChanged(object? sender, EventArgs eventArgs)
    {
        Notify(nameof(Texts));
        Notify(nameof(ConnectionProfileOptions));
        Notify(nameof(ConnectionSelectorOptions));
        Notify(nameof(SshCommand));
        Notify(nameof(SshStatusText));
        Notify(nameof(ApiCommand));
        Notify(nameof(SshActionText));
        Notify(nameof(ContinueText));
    }

    private void OnSshExited(object? sender, EventArgs eventArgs)
    {
        sshProcess?.Dispose();
        sshProcess = null;
        Notify(nameof(SshRunning));
        Notify(nameof(SshActionText));
        Notify(nameof(SshStatusText));
    }

    private static bool IsSafeSshHost(string value)
    {
        if (string.IsNullOrWhiteSpace(value) || value.Length > 255) return false;
        return value.All(character => char.IsLetterOrDigit(character) || character is '.' or '-' or '_' or ':');
    }

    private static bool IsSafeSshUser(string value) =>
        value.Length <= 128 && value.All(character => char.IsLetterOrDigit(character) || character is '.' or '-' or '_');

    private static IReadOnlyList<string> LoadSshConfigAliases()
    {
        try
        {
            var path = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".ssh", "config");
            if (!File.Exists(path)) return [];
            return File.ReadLines(path)
                .Take(512)
                .Select(line => line.Trim())
                .Where(line => line.StartsWith("Host ", StringComparison.OrdinalIgnoreCase)
                    || line.StartsWith("Host\t", StringComparison.OrdinalIgnoreCase))
                .Select(line =>
                {
                    var comment = line.IndexOf('#');
                    return (comment >= 0 ? line[..comment] : line).Trim();
                })
                .SelectMany(line => line[(line.IndexOfAny([' ', '\t']) + 1)..]
                    .Split([' ', '\t'], StringSplitOptions.RemoveEmptyEntries))
                .Where(IsSafeSshHost)
                .Distinct(StringComparer.OrdinalIgnoreCase)
                .Take(32)
                .ToArray();
        }
        catch (IOException)
        {
            return [];
        }
        catch (UnauthorizedAccessException)
        {
            return [];
        }
    }

    private static IReadOnlyList<string> LoadWslDistributions()
    {
        try
        {
            using var process = new Process
            {
                StartInfo = new ProcessStartInfo
                {
                    FileName = "wsl.exe",
                    UseShellExecute = false,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    CreateNoWindow = true,
                },
            };
            process.StartInfo.ArgumentList.Add("-l");
            process.StartInfo.ArgumentList.Add("-q");
            if (!process.Start()) return [];
            var output = process.StandardOutput.ReadToEnd();
            process.WaitForExit(3000);
            return output.Split(['\r', '\n'], StringSplitOptions.RemoveEmptyEntries)
                .Select(value => value.Trim())
                .Where(ConnectionSelectors.IsWslToken)
                .Distinct(StringComparer.Ordinal)
                .Take(32)
                .ToArray();
        }
        catch
        {
            return [];
        }
    }

    private void Notify([CallerMemberName] string? propertyName = null) => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
}

public sealed record ConnectionProfileOption(string Id, string Label);
