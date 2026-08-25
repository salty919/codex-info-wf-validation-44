// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

namespace CodexInfo.WindowsClient.Infrastructure;

/// <summary>
/// Performs the bounded path checks required before a Windows-side mutation.
/// Existing path components are inspected without resolving or accepting a
/// reparse point; missing components are created one at a time and inspected
/// again immediately. Callers still perform their final target check directly
/// before replace/launch to narrow the check-to-use window.
/// </summary>
internal static class WindowsPathSafety
{
    public static bool EnsureDirectoryTreeWithoutReparse(string directory)
    {
        var fullPath = Path.GetFullPath(directory);
        var root = Path.GetPathRoot(fullPath);
        if (string.IsNullOrEmpty(root))
        {
            return false;
        }

        var current = root;
        var remainder = fullPath[root.Length..];
        var components = remainder.Split(
            [Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar],
            StringSplitOptions.RemoveEmptyEntries);
        foreach (var component in components)
        {
            current = Path.Combine(current, component);
            if (TryGetAttributes(current, out var attributes))
            {
                if ((attributes & FileAttributes.ReparsePoint) != 0 ||
                    (attributes & FileAttributes.Directory) == 0)
                {
                    return false;
                }

                continue;
            }

            Directory.CreateDirectory(current);
            if (!TryGetAttributes(current, out attributes) ||
                (attributes & FileAttributes.ReparsePoint) != 0 ||
                (attributes & FileAttributes.Directory) == 0)
            {
                return false;
            }
        }

        return true;
    }

    public static bool IsMissingOrRegularFile(string path)
    {
        if (!TryGetAttributes(path, out var attributes))
        {
            return !Directory.Exists(path);
        }

        return (attributes & (FileAttributes.ReparsePoint | FileAttributes.Directory)) == 0;
    }

    public static bool ContainsReparsePoint(string path)
    {
        var fullPath = Path.GetFullPath(path);
        var root = Path.GetPathRoot(fullPath);
        if (string.IsNullOrEmpty(root))
        {
            return true;
        }

        var current = root;
        var remainder = fullPath[root.Length..];
        var components = remainder.Split(
            [Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar],
            StringSplitOptions.RemoveEmptyEntries);
        foreach (var component in components)
        {
            current = Path.Combine(current, component);
            if (!TryGetAttributes(current, out var attributes))
            {
                // A missing tail cannot be a traversed reparse point.
                break;
            }

            if ((attributes & FileAttributes.ReparsePoint) != 0)
            {
                return true;
            }
        }

        return false;
    }

    private static bool TryGetAttributes(string path, out FileAttributes attributes)
    {
        try
        {
            attributes = File.GetAttributes(path);
            return true;
        }
        catch (FileNotFoundException)
        {
            return TryGetDanglingLinkAttributes(path, out attributes);
        }
        catch (DirectoryNotFoundException)
        {
            return TryGetDanglingLinkAttributes(path, out attributes);
        }
    }

    private static bool TryGetDanglingLinkAttributes(string path, out FileAttributes attributes)
    {
        try
        {
            if (new FileInfo(path).LinkTarget is not null ||
                new DirectoryInfo(path).LinkTarget is not null)
            {
                attributes = FileAttributes.ReparsePoint;
                return true;
            }
        }
        catch
        {
            // An inaccessible path remains fail closed at the caller.
        }

        attributes = default;
        return false;
    }
}
