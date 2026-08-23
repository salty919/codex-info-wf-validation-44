// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

namespace CodexInfo.WindowsClient.Settings;

/// <summary>
/// Owns the atomic transition from persisted settings to the in-memory
/// settings generation used by the running application.
/// </summary>
public interface IClientSettingsSession
{
    ClientSettings Current { get; }

    void Save(ClientSettings settings);
}

public sealed class ClientSettingsSession : IClientSettingsSession
{
    private readonly ClientSettingsStore store;
    private readonly Func<ClientSettings> readCurrent;
    private readonly Action<ClientSettings> publish;

    public ClientSettingsSession(
        ClientSettingsStore store,
        Func<ClientSettings> readCurrent,
        Action<ClientSettings> publish)
    {
        ArgumentNullException.ThrowIfNull(store);
        ArgumentNullException.ThrowIfNull(readCurrent);
        ArgumentNullException.ThrowIfNull(publish);
        this.store = store;
        this.readCurrent = readCurrent;
        this.publish = publish;
    }

    public ClientSettings Current => readCurrent();

    public void Save(ClientSettings settings)
    {
        ArgumentNullException.ThrowIfNull(settings);
        store.Save(settings);
        publish(settings);
    }
}
