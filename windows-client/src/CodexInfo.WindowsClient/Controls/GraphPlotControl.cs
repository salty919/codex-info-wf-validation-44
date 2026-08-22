// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using CodexInfo.WindowsClient.Localization;
using CodexInfo.WindowsClient.ViewModels;
using System.Globalization;

namespace CodexInfo.WindowsClient.Controls;

/// <summary>Dependency-free graph surface for the history view.</summary>
public sealed class GraphPlotControl : Control
{
    // Keep the Windows graph palette in lockstep with ui/theme.slint.  The
    // order below is also the native legend/path ownership order.
    private static readonly Color RemainingColor = Color.Parse("#56B2F5");
    private static readonly Color SolColor = Color.Parse("#A88CF5");
    private static readonly Color TerraColor = Color.Parse("#5DC98A");
    private static readonly Color LunaColor = Color.Parse("#E6A23C");

    /// <summary>
    /// A percentage-based idle interval, matching the X client's plot
    /// coordinate space.  <paramref name="PreserveBoundary"/> keeps the
    /// synthetic reset-to-first-observation gap separate from an adjacent
    /// ordinary idle interval.
    /// </summary>
    internal readonly record struct IdleInterval(double Start, double Width, bool PreserveBoundary);

    public static readonly StyledProperty<IReadOnlyList<GraphPointViewModel>> PointsProperty =
        AvaloniaProperty.Register<GraphPlotControl, IReadOnlyList<GraphPointViewModel>>(
            nameof(Points), Array.Empty<GraphPointViewModel>());
    public static readonly StyledProperty<bool> ShowRemainingProperty =
        AvaloniaProperty.Register<GraphPlotControl, bool>(nameof(ShowRemaining), true);
    public static readonly StyledProperty<bool> ShowModelsProperty =
        AvaloniaProperty.Register<GraphPlotControl, bool>(nameof(ShowModels), true);
    public static readonly StyledProperty<bool> ShowSolProperty =
        AvaloniaProperty.Register<GraphPlotControl, bool>(nameof(ShowSol), true);
    public static readonly StyledProperty<bool> ShowTerraProperty =
        AvaloniaProperty.Register<GraphPlotControl, bool>(nameof(ShowTerra), true);
    public static readonly StyledProperty<bool> ShowLunaProperty =
        AvaloniaProperty.Register<GraphPlotControl, bool>(nameof(ShowLuna), true);
    public static readonly StyledProperty<bool> IsDollarsProperty =
        AvaloniaProperty.Register<GraphPlotControl, bool>(nameof(IsDollars), true);
    public static readonly StyledProperty<long> PeriodStartAtProperty =
        AvaloniaProperty.Register<GraphPlotControl, long>(nameof(PeriodStartAt));
    public static readonly StyledProperty<long> PeriodEndAtProperty =
        AvaloniaProperty.Register<GraphPlotControl, long>(nameof(PeriodEndAt));
    public static readonly StyledProperty<string> RemainingLabelProperty =
        AvaloniaProperty.Register<GraphPlotControl, string>(nameof(RemainingLabel), "Remaining");

    public IReadOnlyList<GraphPointViewModel> Points { get => GetValue(PointsProperty); set => SetValue(PointsProperty, value); }
    public bool ShowRemaining { get => GetValue(ShowRemainingProperty); set => SetValue(ShowRemainingProperty, value); }
    public bool ShowModels { get => GetValue(ShowModelsProperty); set => SetValue(ShowModelsProperty, value); }
    public bool ShowSol { get => GetValue(ShowSolProperty); set => SetValue(ShowSolProperty, value); }
    public bool ShowTerra { get => GetValue(ShowTerraProperty); set => SetValue(ShowTerraProperty, value); }
    public bool ShowLuna { get => GetValue(ShowLunaProperty); set => SetValue(ShowLunaProperty, value); }
    public bool IsDollars { get => GetValue(IsDollarsProperty); set => SetValue(IsDollarsProperty, value); }
    public long PeriodStartAt { get => GetValue(PeriodStartAtProperty); set => SetValue(PeriodStartAtProperty, value); }
    public long PeriodEndAt { get => GetValue(PeriodEndAtProperty); set => SetValue(PeriodEndAtProperty, value); }
    public string RemainingLabel { get => GetValue(RemainingLabelProperty); set => SetValue(RemainingLabelProperty, value); }

    public override void Render(DrawingContext context)
    {
        base.Render(context);
        var bounds = Bounds;
        context.FillRectangle(new SolidColorBrush(Color.Parse("#101925")), bounds);
        var left = 64d;
        var top = 30d;
        var right = Math.Max(left + 1, bounds.Width - 120d);
        var bottom = Math.Max(top + 1, bounds.Height - 36d);
        var width = right - left;
        var height = bottom - top;
        var grid = new Pen(new SolidColorBrush(Color.Parse("#263548")), 1);
        for (var i = 0; i <= 4; i++)
        {
            var y = top + height * i / 4d;
            context.DrawLine(grid, new Point(left, y), new Point(right, y));
        }
        for (var i = 0; i <= 4; i++)
        {
            var x = left + width * i / 4d;
            context.DrawLine(grid, new Point(x, top), new Point(x, bottom));
        }

        var points = Points;
        if (points.Count == 0) return;
        var periodStart = PeriodStartAt > 0 ? PeriodStartAt : points[0].Timestamp;
        var periodEnd = PeriodEndAt > periodStart ? PeriodEndAt : points[^1].Timestamp;
        var mutedBand = new SolidColorBrush(Color.Parse("#A8B7CA")) { Opacity = 0.10 };
        foreach (var interval in BuildIdleIntervals(points, periodStart, periodEnd))
        {
            var x = left + width * interval.Start / 100d;
            var intervalWidth = width * interval.Width / 100d;
            context.FillRectangle(mutedBand, new Rect(x, top, Math.Max(1, intervalWidth), height));
        }
        var visibleValues = new List<double>();
        if (ShowSol) visibleValues.AddRange(points.Select(p => p.SolValue));
        if (ShowTerra) visibleValues.AddRange(points.Select(p => p.TerraValue));
        if (ShowLuna) visibleValues.AddRange(points.Select(p => p.LunaValue));
        // Dollar view follows visible model series. Token view keeps one
        // period-wide scale even when a series is hidden, so a toggle changes
        // visibility without changing the meaning of the y-axis.
        var allValues = points.SelectMany(point => new[] { point.SolValue, point.TerraValue, point.LunaValue });
        var maxModel = (IsDollars ? visibleValues : allValues).DefaultIfEmpty().Max();
        maxModel = Math.Max(1d, maxModel);
        var effectiveRemaining = BuildEffectiveRemaining(points);
        var hasVisibleModelData = ShowModels &&
            ((ShowSol && points.Any(point => point.SolValue > 0)) ||
             (ShowTerra && points.Any(point => point.TerraValue > 0)) ||
             (ShowLuna && points.Any(point => point.LunaValue > 0)));

        // Match the native layering: model lines first (LUNA, TERRA, SOL),
        // then the remaining-quota markers and continuous line on top.
        if (ShowModels && ShowLuna) DrawCumulativeSeries(context, points, p => p.LunaValue, maxModel, new SolidColorBrush(LunaColor), left, top, width, height, periodStart, periodEnd);
        if (ShowModels && ShowTerra) DrawCumulativeSeries(context, points, p => p.TerraValue, maxModel, new SolidColorBrush(TerraColor), left, top, width, height, periodStart, periodEnd);
        if (ShowModels && ShowSol) DrawCumulativeSeries(context, points, p => p.SolValue, maxModel, new SolidColorBrush(SolColor), left, top, width, height, periodStart, periodEnd);
        if (ShowRemaining)
        {
            DrawRemainingMarkers(context, points, effectiveRemaining, RemainingColor, left, top, width, height, periodStart, periodEnd);
            DrawRemainingSeries(context, points, effectiveRemaining, new SolidColorBrush(RemainingColor), left, top, width, height, periodStart, periodEnd);
        }
        DrawAxisLabels(context, points, effectiveRemaining, left, top, right, bottom, maxModel, hasVisibleModelData, IsDollars, periodStart, periodEnd, RemainingLabel, ShowRemaining, ShowModels && ShowLuna, ShowModels && ShowTerra, ShowModels && ShowSol);
    }

    private static void DrawAxisLabels(
        DrawingContext context, IReadOnlyList<GraphPointViewModel> points, IReadOnlyList<double?> effectiveRemaining,
        double left, double top, double right, double bottom, double maxModel, bool hasModelData, bool isDollars, long periodStart, long periodEnd, string remainingLabel,
        bool showRemaining, bool showLuna, bool showTerra, bool showSol)
    {
        var typeface = new Typeface("Noto Sans JP Medium", FontStyle.Normal, FontWeight.Medium);
        var muted = new SolidColorBrush(Color.Parse("#A8B7CA"));
        var labels = new[] { FormatAxisValue(maxModel, isDollars), FormatAxisValue(maxModel * .75, isDollars), FormatAxisValue(maxModel * .5, isDollars), FormatAxisValue(maxModel * .25, isDollars), FormatAxisValue(0, isDollars) };
        for (var i = 0; i < labels.Length; i++)
        {
            var text = new FormattedText(labels[i], CultureInfo.CurrentCulture, FlowDirection.LeftToRight, typeface, 11, muted);
            context.DrawText(text, new Point(4, top + (bottom - top) * i / 4d - 7));
        }
        if (points.Count == 0) return;
        var last = points[^1];
        var values = new List<(string label, Color color, double normalized, double endpointX)>();
        var endpointX = XAt(points, points.Count - 1, left, right - left, periodStart, periodEnd);
        if (showRemaining)
        {
            var remaining = effectiveRemaining[^1];
            var label = remaining is { } observed && double.IsFinite(observed)
                ? $"{observed:0.#}%"
                : "—";
            if (remaining is { } observedRemaining && double.IsFinite(observedRemaining))
                values.Add((label, RemainingColor, 1d - Math.Clamp(observedRemaining / 100d, 0, 1), endpointX));
        }
        if (hasModelData)
        {
            if (showLuna) values.Add((FormatValue(last.LunaValue), LunaColor, 1d - Math.Clamp(last.LunaValue / maxModel, 0, 1), endpointX));
            if (showTerra) values.Add((FormatValue(last.TerraValue), TerraColor, 1d - Math.Clamp(last.TerraValue / maxModel, 0, 1), endpointX));
            if (showSol) values.Add((FormatValue(last.SolValue), SolColor, 1d - Math.Clamp(last.SolValue / maxModel, 0, 1), endpointX));
        }
        var arranged = values.OrderBy(value => value.normalized).ToList();
        var desiredFirstY = arranged.Count == 0 ? top : top + (bottom - top) * arranged[0].normalized - 7;
        var firstLabelY = Math.Clamp(desiredFirstY, top, bottom - 16 - Math.Max(0, arranged.Count - 1) * 16);
        for (var i = 0; i < arranged.Count; i++)
        {
            var value = arranged[i];
            var brush = new SolidColorBrush(value.color);
            var text = new FormattedText(value.label, CultureInfo.CurrentCulture, FlowDirection.LeftToRight, typeface, 11, brush);
            var y = firstLabelY + i * 16;
            var actualY = top + (bottom - top) * value.normalized;
            context.DrawLine(new Pen(brush, 1), new Point(value.endpointX, actualY), new Point(right + 8, y + 7));
            context.DrawText(text, new Point(right + 12, y));
        }
        // Match the X client: label the period boundaries, not arbitrary raw
        // sample timestamps. This keeps sparse histories and the right edge
        // semantically stable.
        var timestampCandidates = new List<(FormattedText Text, double X)>();
        for (var i = 0; i < 5; i++)
        {
            var timestamp = periodStart + (long)Math.Round((periodEnd - periodStart) * i / 4d);
            var label = new FormattedText(
                TimeZoneInfo.ConvertTime(DateTimeOffset.FromUnixTimeSeconds(timestamp), LocalizationService.DisplayTimeZone)
                    .ToString("MM/dd HH:mm", CultureInfo.CurrentCulture),
                CultureInfo.CurrentCulture, FlowDirection.LeftToRight, typeface, 10, muted);
            timestampCandidates.Add((label, left + (right - left) * i / 4d));
        }

        var placedTimestampLabels = new List<(FormattedText Text, double Left, double Right)>();
        for (var index = 0; index < timestampCandidates.Count; index++)
        {
            var candidate = timestampCandidates[index];
            var labelLeft = Math.Clamp(candidate.X - candidate.Text.Width / 2, left, right - candidate.Text.Width);
            var labelRight = labelLeft + candidate.Text.Width;
            if (placedTimestampLabels.Count > 0 && labelLeft < placedTimestampLabels[^1].Right + 8)
            {
                // The final label is the end-of-period anchor.  Replace a
                // colliding intermediate label so that both anchors remain
                // available to the reader.
                if (index == timestampCandidates.Count - 1)
                    placedTimestampLabels.RemoveAt(placedTimestampLabels.Count - 1);
                else
                    continue;
            }

            placedTimestampLabels.Add((candidate.Text, labelLeft, labelRight));
        }

        foreach (var timestamp in placedTimestampLabels)
            context.DrawText(timestamp.Text, new Point(timestamp.Left, bottom + 8));

        string FormatValue(double value) => isDollars ? $"${value:0.00}" : value.ToString("N0", CultureInfo.CurrentCulture);
    }

    private static string FormatAxisValue(double value, bool isDollars)
    {
        if (isDollars) return $"${value:0.00}";
        if (Math.Abs(value) >= 1_000_000_000) return $"{value / 1_000_000_000:0.0}B";
        if (Math.Abs(value) >= 1_000_000) return $"{value / 1_000_000:0.0}M";
        if (Math.Abs(value) >= 1_000) return $"{value / 1_000:0.0}K";
        return value.ToString("N0", CultureInfo.CurrentCulture);
    }

    private static void DrawCumulativeSeries(
        DrawingContext context, IReadOnlyList<GraphPointViewModel> points,
        Func<GraphPointViewModel, double> valueSelector, double maximum, IBrush brush,
        double left, double top, double width, double height, long periodStart, long periodEnd)
    {
        var previous = double.NaN;
        Point? previousPoint = null;
        long previousTimestamp = 0;
        for (var i = 0; i < points.Count; i++)
        {
            var value = Math.Max(double.IsFinite(previous) && previous >= 0 ? previous : 0, valueSelector(points[i]));
            var point = new Point(
                XAt(points, i, left, width, periodStart, periodEnd),
                top + height * (1 - Math.Clamp(value / maximum, 0, 1)));
            if (previousPoint is { } prior)
            {
                var rising = value > previous;
                var lineBrush = brush is SolidColorBrush solid
                    ? new SolidColorBrush(solid.Color) { Opacity = rising ? 0.95 : 0.5 }
                    : brush;
                var pen = new Pen(lineBrush, rising ? 3 : 1) { LineCap = PenLineCap.Round, LineJoin = PenLineJoin.Round };
                var unobservedStart = previousTimestamp == periodStart &&
                    points[i].Timestamp - previousTimestamp > 60 &&
                    previous <= 0 && value > 0;
                if (unobservedStart)
                {
                    // Match X: the synthetic zero anchor is held flat until
                    // the first real measurement, then the increase is shown
                    // at its actual timestamp instead of as a false diagonal.
                    var flatBrush = brush is SolidColorBrush flatSolid
                        ? new SolidColorBrush(flatSolid.Color) { Opacity = 0.5 }
                        : brush;
                    var flatPen = new Pen(flatBrush, 1) { LineCap = PenLineCap.Round, LineJoin = PenLineJoin.Round };
                    context.DrawLine(flatPen, prior, new Point(point.X, prior.Y));
                    context.DrawLine(pen, new Point(point.X, prior.Y), point);
                }
                else
                {
                    context.DrawLine(pen, prior, point);
                }
            }
            previous = value;
            previousPoint = point;
            previousTimestamp = points[i].Timestamp;
        }
    }

    /// <summary>
    /// Returns the same neutral bands as the X graph.  A band is present when
    /// all three cumulative model snapshots are unchanged.  A long zero
    /// interval from the reset anchor to the first real observation is also
    /// marked as idle, but its boundary is preserved so it cannot disappear
    /// into a neighbouring flat interval.
    /// </summary>
    internal static IReadOnlyList<IdleInterval> BuildIdleIntervals(
        IReadOnlyList<GraphPointViewModel> points, long periodStart, long periodEnd)
    {
        var span = Math.Max(1d, periodEnd - periodStart);
        var intervals = new List<IdleInterval>();
        if (points.Count < 2 || periodEnd <= periodStart)
        {
            return intervals;
        }

        static bool IsFinite(double value) => double.IsFinite(value);
        static bool ModelValuesUnchanged(GraphPointViewModel before, GraphPointViewModel after) =>
            IsFinite(before.SolValue) && IsFinite(after.SolValue) && before.SolValue == after.SolValue &&
            IsFinite(before.TerraValue) && IsFinite(after.TerraValue) && before.TerraValue == after.TerraValue &&
            IsFinite(before.LunaValue) && IsFinite(after.LunaValue) && before.LunaValue == after.LunaValue;

        static bool IsZero(GraphPointViewModel point) =>
            point.SolValue == 0 && point.TerraValue == 0 && point.LunaValue == 0;

        for (var index = 1; index < points.Count; index++)
        {
            var before = points[index - 1];
            var after = points[index];
            if (after.Timestamp <= before.Timestamp)
            {
                continue;
            }

            var intervalStart = Math.Max(before.Timestamp, periodStart);
            var intervalEnd = Math.Min(after.Timestamp, periodEnd);
            if (intervalEnd <= intervalStart)
            {
                continue;
            }

            var syntheticZeroGap = before.Timestamp == periodStart &&
                after.Timestamp - before.Timestamp > 60 &&
                IsZero(before) &&
                (IsFinite(after.SolValue) && after.SolValue > 0 ||
                 IsFinite(after.TerraValue) && after.TerraValue > 0 ||
                 IsFinite(after.LunaValue) && after.LunaValue > 0);
            if (!ModelValuesUnchanged(before, after) && !syntheticZeroGap)
            {
                continue;
            }

            var start = Math.Clamp((intervalStart - periodStart) / span * 100d, 0, 100);
            var end = Math.Clamp((intervalEnd - periodStart) / span * 100d, 0, 100);
            if (end <= start)
            {
                continue;
            }

            if (intervals.Count > 0)
            {
                var previous = intervals[^1];
                var previousEnd = previous.Start + previous.Width;
                if (!previous.PreserveBoundary && !syntheticZeroGap &&
                    Math.Abs(previousEnd - start) <= double.Epsilon)
                {
                    intervals[^1] = previous with { Width = end - previous.Start };
                    continue;
                }
            }

            intervals.Add(new IdleInterval(start, end - start, syntheticZeroGap));
        }

        return intervals;
    }

    /// <summary>
    /// Mirrors the native graph's missing-quota-sample rule. A repeated quota
    /// value is interpolated only when model usage advanced across the same
    /// interval; idle intervals remain horizontal and an unobserved terminal
    /// value is never fabricated.
    /// </summary>
    internal static IReadOnlyList<double?> BuildEffectiveRemaining(IReadOnlyList<GraphPointViewModel> points)
    {
        if (points.Count == 0)
        {
            return Array.Empty<double?>();
        }

        var rawValues = points.Select(point => point.RemainingPercent).ToArray();
        var firstObserved = Array.FindIndex(rawValues, value => value is { } observed && double.IsFinite(observed));
        if (firstObserved < 0)
        {
            return rawValues;
        }

        // The quota line is driven by the same cumulative model snapshots as
        // the native graph. A quota reread during an idle interval is not
        // evidence of consumption; accepting it would create the false
        // diagonal/step visible in the Windows graph. Keep idle segments
        // horizontal and only accept a lower observation on an active segment.
        var values = new double?[points.Count];
        var activeSegments = new bool[Math.Max(0, points.Count - 1)];
        var interpolated = new bool[points.Count];
        values[0] = rawValues[0] is { } first && double.IsFinite(first)
            ? Math.Clamp(first, 0, 100)
            : 100d;
        for (var index = 1; index < points.Count; index++)
        {
            var previous = values[index - 1] ?? 100d;
            var modelAdvanced = points[index].SolValue > points[index - 1].SolValue ||
                points[index].TerraValue > points[index - 1].TerraValue ||
                points[index].LunaValue > points[index - 1].LunaValue;
            var syntheticZeroGap = index == 1 &&
                points[index].Timestamp - points[index - 1].Timestamp > 60 &&
                points[index - 1].SolValue <= 0 &&
                points[index - 1].TerraValue <= 0 &&
                points[index - 1].LunaValue <= 0 &&
                modelAdvanced;
            var active = modelAdvanced && !syntheticZeroGap;
            activeSegments[index - 1] = active;
            // A synthetic reset-to-first-use gap is not an active interval,
            // but the first quota observation still belongs at its real
            // timestamp. Keep it as the vertical endpoint; DrawRemainingSeries
            // holds the preceding 100% value flat until that x coordinate.
            if (modelAdvanced && rawValues[index] is { } raw && double.IsFinite(raw))
            {
                values[index] = Math.Min(previous, Math.Clamp(raw, 0, 100));
            }
            else
            {
                values[index] = previous;
            }
        }

        // Complete a repeated active plateau only when it is bounded by a
        // later lower observation. Active duration, rather than wall-clock
        // duration, determines the interpolation; idle gaps stay horizontal.
        var source = values.ToArray();
        static double SegmentSeconds(GraphPointViewModel before, GraphPointViewModel after) =>
            Math.Max(0, after.Timestamp - before.Timestamp);
        for (var index = 1; index < points.Count; index++)
        {
            if (source[index - 1] is not { } previous || source[index] is not { } current ||
                Math.Abs(previous - current) > 0.0001 ||
                points[index - 1].Timestamp == points[index].Timestamp)
            {
                continue;
            }

            var right = index + 1;
            while (right < points.Count && source[right] is { } candidate && candidate >= previous)
            {
                right++;
            }

            if (right >= points.Count || source[right] is not { } next || next >= previous)
            {
                continue;
            }

            var totalActive = 0d;
            var elapsedActive = 0d;
            for (var segment = index - 1; segment < right; segment++)
            {
                var duration = SegmentSeconds(points[segment], points[segment + 1]);
                if (activeSegments[segment])
                {
                    totalActive += duration;
                    if (segment < index)
                    {
                        elapsedActive += duration;
                    }
                }
            }

            if (totalActive > 0)
            {
                values[index] = previous + (next - previous) * Math.Clamp(elapsedActive / totalActive, 0, 1);
                interpolated[index] = true;
            }
        }

        // Match the native smoothing at an internal active change point while
        // preserving explicitly interpolated plateaus and never allowing a
        // quota line to rise again.
        for (var index = 1; index + 1 < values.Length; index++)
        {
            if (!activeSegments[index - 1] || !activeSegments[index] ||
                interpolated[index - 1] || interpolated[index] || interpolated[index + 1] ||
                values[index - 1] is not { } before || values[index] is not { } current ||
                values[index + 1] is not { } after)
            {
                continue;
            }

            values[index] = Math.Min(before, (before + 2 * current + after) / 4d);
        }

        for (var index = 1; index < values.Length; index++)
        {
            if (!activeSegments[index - 1] &&
                points[index].Timestamp != points[index - 1].Timestamp &&
                !IsSyntheticRemainingGap(points, index, points[0].Timestamp))
            {
                values[index] = values[index - 1];
            }
            else if (values[index] is { } current && values[index - 1] is { } before)
            {
                values[index] = Math.Min(current, before);
            }
        }

        return values;
    }

    private static void DrawRemainingSeries(
        DrawingContext context, IReadOnlyList<GraphPointViewModel> points,
        IReadOnlyList<double?> values, IBrush brush,
        double left, double top, double width, double height, long periodStart, long periodEnd)
    {
        var pen = new Pen(brush, 2) { LineCap = PenLineCap.Round, LineJoin = PenLineJoin.Round };
        Point? previous = null;
        for (var index = 0; index < points.Count; index++)
        {
            if (values[index] is not { } value || !double.IsFinite(value))
            {
                previous = null;
                continue;
            }

            var point = new Point(
                XAt(points, index, left, width, periodStart, periodEnd),
                top + height * (1 - Math.Clamp(value / 100d, 0, 1)));
            if (previous is { } prior)
            {
                if (IsSyntheticRemainingGap(points, index, periodStart))
                {
                    var vertical = new Point(point.X, prior.Y);
                    context.DrawLine(pen, prior, vertical);
                    context.DrawLine(pen, vertical, point);
                }
                else
                {
                    context.DrawLine(pen, prior, point);
                }
            }
            previous = point;
        }
    }

    internal static bool IsSyntheticRemainingGap(
        IReadOnlyList<GraphPointViewModel> points, int index, long periodStart)
    {
        if (index <= 0 || index >= points.Count)
        {
            return false;
        }

        var before = points[index - 1];
        var after = points[index];
        return before.Timestamp == periodStart &&
            after.Timestamp - before.Timestamp > 60 &&
            before.SolValue <= 0 && before.TerraValue <= 0 && before.LunaValue <= 0 &&
            (after.SolValue > 0 || after.TerraValue > 0 || after.LunaValue > 0);
    }

    /// <summary>
    /// Draws the native graph's remaining-quota boundary markers.  Markers
    /// are not sample dots: one 2x2 square is placed where each integer
    /// percentage boundary is crossed by the smoothed descending line.
    /// Model series intentionally have no markers.
    /// </summary>
    private static void DrawRemainingMarkers(
        DrawingContext context, IReadOnlyList<GraphPointViewModel> points,
        IReadOnlyList<double?> values, Color color,
        double left, double top, double width, double height, long periodStart, long periodEnd)
    {
        var span = Math.Max(1d, periodEnd - periodStart);
        var seenBoundaries = new HashSet<int>();
        for (var index = 1; index < points.Count; index++)
        {
            if (values[index - 1] is not { } previous ||
                values[index] is not { } current ||
                !double.IsFinite(previous) || !double.IsFinite(current) ||
                current >= previous)
            {
                continue;
            }

            var boundary = (int)Math.Floor(previous);
            if (Math.Abs(previous - boundary) <= double.Epsilon)
            {
                boundary--;
            }

            var lowestBoundary = (int)Math.Ceiling(current);
            while (boundary >= lowestBoundary)
            {
                var boundaryValue = (double)boundary;
                if (boundaryValue < previous && boundaryValue >= current && seenBoundaries.Add(boundary))
                {
                    var fraction = Math.Clamp((boundaryValue - previous) / (current - previous), 0, 1);
                    var timestamp = IsSyntheticRemainingGap(points, index, periodStart)
                        ? points[index].Timestamp
                        : points[index - 1].Timestamp +
                          (points[index].Timestamp - points[index - 1].Timestamp) * fraction;
                    var x = left + width * Math.Clamp((timestamp - periodStart) / span, 0, 1);
                    var y = top + height * (1 - Math.Clamp(boundaryValue / 100d, 0, 1));
                    context.FillRectangle(new SolidColorBrush(color), new Rect(x - 1, y - 1, 2, 2));
                }

                boundary--;
            }
        }
    }

    private static double XAt(IReadOnlyList<GraphPointViewModel> points, int index, double left, double width, long periodStart, long periodEnd)
    {
        if (points.Count <= 1) return left + width / 2;
        if (periodEnd <= periodStart) return left + width * index / (points.Count - 1d);
        return left + width * Math.Clamp((points[index].Timestamp - periodStart) / (double)(periodEnd - periodStart), 0, 1);
    }
}
