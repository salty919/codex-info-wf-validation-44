// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Input.Platform;
using CodexInfo.WindowsClient.Settings;
using CodexInfo.WindowsClient.Core;
using CodexInfo.WindowsClient.ViewModels;

namespace CodexInfo.WindowsClient;

public partial class SetupWindow : Window
{
    public SetupWindow() : this(new SetupViewModel(new MainWindowViewModel(new LoopbackStatusClient(), new LoopbackStatusClient()))) { }

    public SetupWindow(SetupViewModel viewModel)
    {
        InitializeComponent();
        WindowDragBehavior.Attach(this);
        DataContext = viewModel;
        Closed += (_, _) => viewModel.Dispose();
    }

    private void OnTitlePointerPressed(object? sender, PointerPressedEventArgs e)
    {
        if (e.Source is not Button && e.GetCurrentPoint(this).Properties.PointerUpdateKind == PointerUpdateKind.LeftButtonPressed)
        {
            WindowDragBehavior.Begin(this, e);
        }
    }

    private void OnRefresh(object? sender, Avalonia.Interactivity.RoutedEventArgs e) => (DataContext as SetupViewModel)?.Refresh();

    private void OnContinue(object? sender, Avalonia.Interactivity.RoutedEventArgs e)
    {
        if (DataContext is not SetupViewModel vm) return;
        if (vm.IsConnectionStep && !vm.CanContinue) return;
        if (vm.IsAuthStep && !vm.CanContinue)
        {
            vm.StartAuthentication();
            return;
        }
        if (vm.IsDoneStep)
        {
            var updated = vm.BuildSettings(App.CurrentSettings) with { SetupCompleted = true, Language = vm.Texts.LanguageCode };
            App.SettingsStore.Save(updated);
            App.CurrentSettings = updated;
            Close();
            return;
        }
        if (vm.IsConnectionStep)
        {
            // A reachable API/auth-required response proves the local
            // forwarding route, even when Linux authentication is still
            // pending.  Persist only this non-sensitive completion marker;
            // SSH host/user remain transient by design.
            var updated = vm.BuildSettings(App.CurrentSettings) with { Language = vm.Texts.LanguageCode };
            App.SettingsStore.Save(updated);
            App.CurrentSettings = updated;
        }
        vm.Continue();
    }

    private async void OnCopySsh(object? sender, Avalonia.Interactivity.RoutedEventArgs e) => await CopyTextAsync((DataContext as SetupViewModel)?.SshCommand);

    private void OnStartOrStopSsh(object? sender, Avalonia.Interactivity.RoutedEventArgs e) => (DataContext as SetupViewModel)?.StartOrStopSsh();

    private async void OnCopyApi(object? sender, Avalonia.Interactivity.RoutedEventArgs e) => await CopyTextAsync((DataContext as SetupViewModel)?.ApiCommand);

    private async Task CopyTextAsync(string? text)
    {
        if (!string.IsNullOrEmpty(text) && Clipboard is { } clipboard) await clipboard.SetTextAsync(text);
    }

    private void OnClose(object? sender, Avalonia.Interactivity.RoutedEventArgs e) => Close();
}
