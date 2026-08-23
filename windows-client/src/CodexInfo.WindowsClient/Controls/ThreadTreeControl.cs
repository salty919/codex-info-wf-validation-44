// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;

namespace CodexInfo.WindowsClient.Controls;

/// <summary>Draws the parent/child rails in the dedicated thread gutter.</summary>
public sealed class ThreadTreeControl : Control
{
    public static readonly StyledProperty<int> TreeDepthProperty = AvaloniaProperty.Register<ThreadTreeControl, int>(nameof(TreeDepth));
    public static readonly StyledProperty<bool> ConnectedToParentProperty = AvaloniaProperty.Register<ThreadTreeControl, bool>(nameof(ConnectedToParent));
    public static readonly StyledProperty<bool> HasChildrenProperty = AvaloniaProperty.Register<ThreadTreeControl, bool>(nameof(HasChildren));
    public static readonly StyledProperty<bool> HasNextSiblingProperty = AvaloniaProperty.Register<ThreadTreeControl, bool>(nameof(HasNextSibling));
    public static readonly StyledProperty<bool> AncestorGuide1Property = AvaloniaProperty.Register<ThreadTreeControl, bool>(nameof(AncestorGuide1));
    public static readonly StyledProperty<bool> AncestorGuide2Property = AvaloniaProperty.Register<ThreadTreeControl, bool>(nameof(AncestorGuide2));
    public static readonly StyledProperty<bool> AncestorGuide3Property = AvaloniaProperty.Register<ThreadTreeControl, bool>(nameof(AncestorGuide3));

    public int TreeDepth { get => GetValue(TreeDepthProperty); set => SetValue(TreeDepthProperty, value); }
    public bool ConnectedToParent { get => GetValue(ConnectedToParentProperty); set => SetValue(ConnectedToParentProperty, value); }
    public bool HasChildren { get => GetValue(HasChildrenProperty); set => SetValue(HasChildrenProperty, value); }
    public bool HasNextSibling { get => GetValue(HasNextSiblingProperty); set => SetValue(HasNextSiblingProperty, value); }
    public bool AncestorGuide1 { get => GetValue(AncestorGuide1Property); set => SetValue(AncestorGuide1Property, value); }
    public bool AncestorGuide2 { get => GetValue(AncestorGuide2Property); set => SetValue(AncestorGuide2Property, value); }
    public bool AncestorGuide3 { get => GetValue(AncestorGuide3Property); set => SetValue(AncestorGuide3Property, value); }

    public override void Render(DrawingContext context)
    {
        base.Render(context);
        var rail = new Pen(new SolidColorBrush(Color.Parse("#D5A43A")), 2);
        const double baseX = 8;
        const double step = 12;
        var junctionY = Bounds.Height / 2;
        var junctionEndX = Math.Max(baseX + step, Bounds.Width - 5);
        var depth = Math.Clamp(TreeDepth, 0, 3);
        if (AncestorGuide1) context.DrawLine(rail, new Point(baseX, 0), new Point(baseX, Bounds.Height));
        if (AncestorGuide2) context.DrawLine(rail, new Point(baseX + step, 0), new Point(baseX + step, Bounds.Height));
        if (AncestorGuide3) context.DrawLine(rail, new Point(baseX + step * 2, 0), new Point(baseX + step * 2, Bounds.Height));
        if (ConnectedToParent)
        {
            var x = baseX + Math.Max(0, depth - 1) * step;
            context.DrawLine(rail, new Point(x, 0), new Point(x, HasNextSibling ? Bounds.Height : junctionY));
            context.DrawLine(rail, new Point(x, junctionY), new Point(junctionEndX, junctionY));
            context.DrawEllipse(rail.Brush, null, new Point(junctionEndX, junctionY), 3, 3);
        }
        if (HasChildren)
        {
            var x = baseX + depth * step;
            context.DrawLine(rail, new Point(x, junctionY), new Point(x, Bounds.Height));
        }
    }
}
