// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using Avalonia.Controls;
using Avalonia.Input;
using CodexInfo.WindowsClient.ViewModels;

namespace CodexInfo.WindowsClient;

public partial class LegalNoticesWindow : Window
{
    public LegalNoticesWindow()
    {
        InitializeComponent();
        WindowDragBehavior.Attach(this);
    }

    public LegalNoticesWindow(LegalNoticesWindowViewModel viewModel)
    {
        InitializeComponent();
        WindowDragBehavior.Attach(this);
        DataContext = viewModel;
        Closed += (_, _) => viewModel.Dispose();
    }

    private void OnTitlePointerPressed(object? sender, PointerPressedEventArgs eventArgs)
    {
        if (eventArgs.Source is not Button && eventArgs.GetCurrentPoint(this).Properties.PointerUpdateKind == PointerUpdateKind.LeftButtonPressed)
        {
            WindowDragBehavior.Begin(this, eventArgs);
        }
    }

    private void OnMinimizeWindow(object? sender, Avalonia.Interactivity.RoutedEventArgs eventArgs)
    {
        WindowState = WindowState.Minimized;
    }

    private void OnCloseWindow(object? sender, Avalonia.Interactivity.RoutedEventArgs eventArgs)
    {
        Close();
    }
}
