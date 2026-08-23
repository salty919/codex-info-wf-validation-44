// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using CodexInfo.WindowsClient.Settings;

namespace CodexInfo.WindowsClient.ViewModels;

public enum SetupAdvanceOutcome
{
    StayOpen,
    CloseRequested,
}

public static class SetupLaunchPolicy
{
    public static bool ShouldOpen(ClientSettings settings)
    {
        ArgumentNullException.ThrowIfNull(settings);
        return !settings.SettingsCorrupt
            && !settings.SetupCompleted
            && !settings.ConnectionConfigured;
    }
}
