// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Diagnostics;
using System.IO.Compression;
using System.Reflection;
using System.Runtime.Versioning;
using Microsoft.Win32;

namespace CodexInfo.WindowsClient.Installer;

internal static class Program
{
    private const string ProductName = "Codex Info Monitor";
    private const string ProductKey = "CodexInfo.WindowsClient";
    private const string ClientExecutable = "CodexInfo.WindowsClient.exe";
    private const string UninstallerExecutable = "CodexInfo.WindowsClient.Uninstaller.exe";

    [SupportedOSPlatform("windows")]
    private static int Main(string[] args)
    {
        try
        {
            var options = InstallerOptions.Parse(args);
            if (!OperatingSystem.IsWindows())
            {
                throw new PlatformNotSupportedException("The Windows installer must run on Windows.");
            }

            return options.Uninstall
                ? Uninstall(options)
                : Install(options);
        }
        catch (Exception exception)
        {
            Console.Error.WriteLine($"{ProductName} installer failed: {exception.Message}");
            return 1;
        }
    }

    [SupportedOSPlatform("windows")]
    private static int Install(InstallerOptions options)
    {
        var installDirectory = Path.GetFullPath(options.InstallDirectory ?? DefaultInstallDirectory());
        EnsureSafeInstallDirectory(installDirectory);
        var parent = Directory.GetParent(installDirectory)?.FullName
            ?? throw new InvalidOperationException("The install directory has no parent.");
        Directory.CreateDirectory(parent);

        var staging = Path.Combine(parent, $".{Path.GetFileName(installDirectory)}.staging-{Guid.NewGuid():N}");
        var backup = Path.Combine(parent, $".{Path.GetFileName(installDirectory)}.previous-{Guid.NewGuid():N}");
        var createdShortcuts = new List<string>();
        var existingMoved = false;
        var stagingMoved = false;
        try
        {
            Directory.CreateDirectory(staging);
            ExtractEmbeddedPayload(staging);
            var clientPath = Path.Combine(staging, ClientExecutable);
            if (!File.Exists(clientPath))
            {
                throw new InvalidDataException($"Embedded payload does not contain {ClientExecutable}.");
            }

            var installerPath = CurrentExecutablePath();
            File.Copy(installerPath, Path.Combine(staging, UninstallerExecutable), overwrite: true);

            if (Directory.Exists(installDirectory))
            {
                Directory.Move(installDirectory, backup);
                existingMoved = true;
            }

            Directory.Move(staging, installDirectory);
            stagingMoved = true;
            var installedClientPath = Path.Combine(installDirectory, ClientExecutable);

            var startMenuShortcut = CreateShortcut(
                Path.Combine(StartMenuDirectory(), $"{ProductName}.lnk"),
                installedClientPath,
                installDirectory,
                "Codex Info Windows monitoring client");
            createdShortcuts.Add(startMenuShortcut);
            string? desktopShortcut = null;
            if (options.DesktopShortcut)
            {
                desktopShortcut = CreateShortcut(
                    Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), $"{ProductName}.lnk"),
                    installedClientPath,
                    installDirectory,
                    "Codex Info Windows monitoring client");
                createdShortcuts.Add(desktopShortcut);
            }

            RegisterUninstall(installDirectory, startMenuShortcut, desktopShortcut);
            try
            {
                TryDeleteDirectory(backup);
            }
            catch
            {
                // A stale previous-generation directory is harmless; the
                // newly installed generation and its registration are valid.
            }
            Console.WriteLine($"Installed {ProductName} to {installDirectory}");
            Console.WriteLine($"Start menu entry: {startMenuShortcut}");
            return 0;
        }
        catch
        {
            foreach (var shortcut in createdShortcuts)
            {
                RemoveShortcut(shortcut);
            }
            TryDeleteDirectory(staging);
            if (stagingMoved && Directory.Exists(installDirectory))
            {
                TryDeleteDirectory(installDirectory);
            }
            if (existingMoved && Directory.Exists(backup))
            {
                Directory.Move(backup, installDirectory);
            }

            throw;
        }
    }

    [SupportedOSPlatform("windows")]
    private static int Uninstall(InstallerOptions options)
    {
        var installDirectory = Path.GetFullPath(options.InstallDirectory ?? ReadRegisteredInstallDirectory());
        EnsureSafeInstallDirectory(installDirectory);
        RemoveShortcut(Path.Combine(StartMenuDirectory(), $"{ProductName}.lnk"));
        RemoveShortcut(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), $"{ProductName}.lnk"));
        Registry.CurrentUser.DeleteSubKeyTree(UninstallRegistryPath, throwOnMissingSubKey: false);
        var startMenuDirectory = StartMenuDirectory();
        if (Directory.Exists(startMenuDirectory) && !Directory.EnumerateFileSystemEntries(startMenuDirectory).Any())
        {
            Directory.Delete(startMenuDirectory);
        }

        if (options.PurgeSettings)
        {
            var settingsPath = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "CodexInfo", "settings.json");
            if (File.Exists(settingsPath))
            {
                File.Delete(settingsPath);
            }
        }

        if (Directory.Exists(installDirectory))
        {
            ScheduleDirectoryRemoval(installDirectory);
        }

        Console.WriteLine($"Uninstalled {ProductName}; user settings and server history were preserved.");
        return 0;
    }

    private static string UninstallRegistryPath => $@"Software\Microsoft\Windows\CurrentVersion\Uninstall\{ProductKey}";

    private static string DefaultInstallDirectory() => Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "Programs",
        ProductName);

    private static string StartMenuDirectory() => Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.Programs),
        "Codex Info");

    [SupportedOSPlatform("windows")]
    private static string ReadRegisteredInstallDirectory()
    {
        using var key = Registry.CurrentUser.OpenSubKey(UninstallRegistryPath, writable: false);
        var value = key?.GetValue("InstallLocation") as string;
        return string.IsNullOrWhiteSpace(value)
            ? throw new InvalidOperationException("Codex Info Monitor is not registered as installed.")
            : value;
    }

    [SupportedOSPlatform("windows")]
    private static void RegisterUninstall(string installDirectory, string startMenuShortcut, string? desktopShortcut)
    {
        using var key = Registry.CurrentUser.CreateSubKey(UninstallRegistryPath, writable: true)
            ?? throw new InvalidOperationException("Could not create the per-user uninstall registration.");
        var uninstaller = Path.Combine(installDirectory, UninstallerExecutable);
        key.SetValue("DisplayName", ProductName);
        key.SetValue("DisplayVersion", "1.0.0");
        key.SetValue("Publisher", "salty919");
        key.SetValue("InstallLocation", installDirectory);
        key.SetValue("UninstallString", $"\"{uninstaller}\" --uninstall");
        key.SetValue("DisplayIcon", Path.Combine(installDirectory, ClientExecutable));
        key.SetValue("NoModify", 1, RegistryValueKind.DWord);
        key.SetValue("NoRepair", 1, RegistryValueKind.DWord);
        key.SetValue("StartMenuShortcut", startMenuShortcut);
        if (desktopShortcut is not null)
        {
            key.SetValue("DesktopShortcut", desktopShortcut);
        }
    }

    [SupportedOSPlatform("windows")]
    private static string CreateShortcut(string path, string target, string workingDirectory, string description)
    {
        var directory = Path.GetDirectoryName(path) ?? throw new InvalidOperationException("Shortcut has no directory.");
        Directory.CreateDirectory(directory);
        var shellType = Type.GetTypeFromProgID("WScript.Shell")
            ?? throw new InvalidOperationException("Windows shortcut support is unavailable.");
        dynamic shell = Activator.CreateInstance(shellType)
            ?? throw new InvalidOperationException("Could not create the Windows shell shortcut object.");
        dynamic shortcut = shell.CreateShortcut(path);
        shortcut.TargetPath = target;
        shortcut.WorkingDirectory = workingDirectory;
        shortcut.Description = description;
        shortcut.IconLocation = $"{target},0";
        shortcut.Save();
        return path;
    }

    private static void RemoveShortcut(string path)
    {
        if (File.Exists(path))
        {
            File.Delete(path);
        }
    }

    private static void ExtractEmbeddedPayload(string destination)
    {
        using var archiveStream = Assembly.GetExecutingAssembly().GetManifestResourceStream("CodexInfo.WindowsClient.Installer.Payload.zip")
            ?? throw new InvalidOperationException("This setup executable has no embedded client payload.");
        using var archive = new ZipArchive(archiveStream, ZipArchiveMode.Read);
        var root = Path.GetFullPath(destination) + Path.DirectorySeparatorChar;
        foreach (var entry in archive.Entries)
        {
            var target = Path.GetFullPath(Path.Combine(destination, entry.FullName));
            if (!target.StartsWith(root, StringComparison.OrdinalIgnoreCase))
            {
                throw new InvalidDataException("The embedded payload contains an unsafe path.");
            }

            if (string.IsNullOrEmpty(entry.Name))
            {
                Directory.CreateDirectory(target);
                continue;
            }

            Directory.CreateDirectory(Path.GetDirectoryName(target)!);
            entry.ExtractToFile(target, overwrite: true);
        }
    }

    private static string CurrentExecutablePath() => Process.GetCurrentProcess().MainModule?.FileName
        ?? throw new InvalidOperationException("Could not resolve the setup executable path.");

    private static void ScheduleDirectoryRemoval(string directory)
    {
        var script = Path.Combine(Path.GetTempPath(), $"codex-info-uninstall-{Guid.NewGuid():N}.cmd");
        var quotedDirectory = directory.Replace("\"", "\"\"");
        File.WriteAllText(script, $"@echo off\r\ntimeout /t 2 /nobreak >nul\r\nrmdir /s /q \"{quotedDirectory}\"\r\ndel /q \"%~f0\"\r\n");
        Process.Start(new ProcessStartInfo
        {
            FileName = "cmd.exe",
            Arguments = $"/d /c \"{script}\"",
            CreateNoWindow = true,
            UseShellExecute = false,
            WindowStyle = ProcessWindowStyle.Hidden,
        });
    }

    private static void EnsureSafeInstallDirectory(string directory)
    {
        var programsRoot = Path.GetFullPath(Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "Programs"));
        var requiredPrefix = programsRoot.TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar;
        if (directory.IndexOfAny(Path.GetInvalidPathChars()) >= 0 ||
            !directory.StartsWith(requiredPrefix, StringComparison.OrdinalIgnoreCase) ||
            !string.Equals(Path.GetFileName(directory), ProductName, StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidOperationException("The install directory must be the per-user Codex Info Monitor directory.");
        }
    }

    private static void TryDeleteDirectory(string directory)
    {
        if (Directory.Exists(directory))
        {
            Directory.Delete(directory, recursive: true);
        }
    }
}

internal sealed record InstallerOptions(
    bool Uninstall,
    bool DesktopShortcut,
    bool PurgeSettings,
    string? InstallDirectory)
{
    public static InstallerOptions Parse(IEnumerable<string> arguments)
    {
        var values = arguments.ToArray();
        string? installDirectory = null;
        var uninstall = false;
        var desktopShortcut = false;
        var purgeSettings = false;
        for (var index = 0; index < values.Length; index++)
        {
            switch (values[index].ToLowerInvariant())
            {
                case "--uninstall":
                    uninstall = true;
                    break;
                case "--desktop-shortcut":
                    desktopShortcut = true;
                    break;
                case "--purge-settings":
                    purgeSettings = true;
                    break;
                case "--install-dir" when index + 1 < values.Length:
                    installDirectory = values[++index];
                    break;
                default:
                    throw new ArgumentException($"Unknown or incomplete installer option: {values[index]}");
            }
        }

        return new InstallerOptions(uninstall, desktopShortcut, purgeSettings, installDirectory);
    }
}
