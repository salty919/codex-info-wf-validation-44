// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.VisualTree;

namespace CodexInfo.WindowsClient;

/// <summary>
/// Starts the platform-native move loop for borderless windows.
///
/// Updating <see cref="Window.Position"/> from pointer coordinates is
/// intentionally avoided: on a scaled Windows desktop the window move itself
/// changes the coordinate space used by pointer events and can produce visible
/// feedback/jitter. Avalonia delegates this operation to the native window
/// manager, which keeps the pointer anchor and DPI conversion in one place.
/// </summary>
internal static class WindowDragBehavior
{
    public static void Attach(Window window, double titleBarHeight = 88)
    {
        window.AddHandler(
            InputElement.PointerPressedEvent,
            (_, eventArgs) =>
            {
                if (eventArgs.Handled || eventArgs.GetCurrentPoint(window).Position.Y > titleBarHeight)
                {
                    return;
                }

                Begin(window, eventArgs);
            },
            RoutingStrategies.Tunnel);
    }

    public static bool Begin(Window window, PointerPressedEventArgs eventArgs)
    {
        if (IsButtonOrDescendant(eventArgs.Source) ||
            eventArgs.GetCurrentPoint(window).Properties.PointerUpdateKind != PointerUpdateKind.LeftButtonPressed)
        {
            return false;
        }

        window.BeginMoveDrag(eventArgs);
        eventArgs.Handled = true;
        return true;
    }

    private static bool IsButtonOrDescendant(object? source)
    {
        if (source is not Visual visual)
        {
            return false;
        }

        for (Visual? current = visual; current is not null; current = current.GetVisualParent())
        {
            if (current is Button)
            {
                return true;
            }
        }

        return false;
    }

}

internal static class WindowDragGeometry
{
    public static PixelPoint CalculatePosition(
        PixelPoint startPosition,
        PixelPoint startCursor,
        PixelPoint currentCursor)
    {
        return new PixelPoint(
            startPosition.X + currentCursor.X - startCursor.X,
            startPosition.Y + currentCursor.Y - startCursor.Y);
    }
}
