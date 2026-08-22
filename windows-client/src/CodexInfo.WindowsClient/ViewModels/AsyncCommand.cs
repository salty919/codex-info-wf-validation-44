// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Windows.Input;

namespace CodexInfo.WindowsClient.ViewModels;

internal sealed class AsyncCommand : ICommand
{
    private readonly Func<Task> execute;
    private readonly Func<bool> canExecute;

    public AsyncCommand(Func<Task> execute, Func<bool> canExecute)
    {
        this.execute = execute;
        this.canExecute = canExecute;
    }

    public event EventHandler? CanExecuteChanged;

    public bool CanExecute(object? parameter)
    {
        return canExecute();
    }

    public async void Execute(object? parameter)
    {
        if (!CanExecute(parameter))
        {
            return;
        }

        try
        {
            await execute();
        }
        catch (OperationCanceledException)
        {
            // Closing the window cancels the active request; no UI error is needed.
        }
    }

    public void RaiseCanExecuteChanged()
    {
        CanExecuteChanged?.Invoke(this, EventArgs.Empty);
    }
}
