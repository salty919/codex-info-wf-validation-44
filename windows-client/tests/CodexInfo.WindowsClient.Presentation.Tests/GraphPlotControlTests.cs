// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Globalization;
using CodexInfo.WindowsClient.Core;
using CodexInfo.WindowsClient.Graphing;
using CodexInfo.WindowsClient.ViewModels;
using Xunit;

namespace CodexInfo.WindowsClient.Presentation.Tests;

public sealed class GraphPlotControlTests
{
    [Fact]
    public void ContractMaximumReductionIsViewportBoundedAndPreservesExactEndpoints()
    {
        // Exercise the largest response permitted by the one-month transport
        // contract. The parser rejects larger responses before rendering.
        var samples = Enumerable.Range(0, 44_640)
            .Select(index => new ApiHistorySample(index + 1, 200_000, 100 - index / 1_000d, index, index * 2, index * 3, (ulong)index, (ulong)index * 2, (ulong)index * 3))
            .ToArray();

        var reduced = GraphWindowViewModel.ReduceGraphSamples(samples);

        Assert.Equal(GraphWindowViewModel.MaxRenderedGraphPoints, reduced.Count);
        Assert.Equal(samples[0], reduced[0]);
        Assert.Equal(samples[^1], reduced[^1]);
        Assert.True(reduced.Zip(reduced.Skip(1)).All(pair => pair.First.Timestamp < pair.Second.Timestamp));
    }

    [Fact]
    public void ScenePreservesAllIrregularSamplesAndExactEndpoints()
    {
        var source = Enumerable.Range(0, 2_048)
            .Select(index => new ApiHistorySample(
                index * index + 1,
                5_000_000,
                100 - index / 100d,
                index,
                index,
                index,
                (ulong)index,
                (ulong)index,
                (ulong)index))
            .ToArray();

        var scene = GraphScene.Create(source, GraphMetric.Dollars, 1, source[^1].Timestamp);

        Assert.Equal(source.Length, scene.Timestamps.Count);
        Assert.Equal(source[0].Timestamp, scene.Timestamps[0]);
        Assert.Equal(source[^1].Timestamp, scene.Timestamps[^1]);
        Assert.True(scene.Timestamps.Zip(scene.Timestamps.Skip(1)).All(pair => pair.First < pair.Second));
    }

    [Fact]
    public void EndpointLabelsStayNearTheirSeriesAndResolveOnlyActualCollisions()
    {
        var arranged = GraphScene.ArrangeEndpointLabelTops(
            [10, 80, 84, 170],
            top: 0,
            bottom: 200,
            labelHeight: 14,
            gap: 2);

        Assert.Equal(10, arranged[0]);
        Assert.Equal(80, arranged[1]);
        Assert.Equal(96, arranged[2]);
        Assert.Equal(170, arranged[3]);
        Assert.All(arranged, value => Assert.InRange(value, 0, 186));
        Assert.True(arranged.Zip(arranged.Skip(1)).All(pair => pair.Second - pair.First >= 16));
    }

    [Fact]
    public void EndpointLabelsAtBottomRemainBoundedAndNonCrossing()
    {
        var arranged = GraphScene.ArrangeEndpointLabelTops(
            [180, 181, 182, 183],
            top: 0,
            bottom: 200,
            labelHeight: 14,
            gap: 2);

        Assert.Equal(186, arranged[^1]);
        Assert.All(arranged, value => Assert.InRange(value, 0, 186));
        Assert.True(arranged.Zip(arranged.Skip(1)).All(pair => pair.Second - pair.First >= 16));
    }

    [Fact]
    public void PlotProjectionBuildsFrameworkIndependentAxisTicks()
    {
        var projection = GraphPlotProjection.BuildAxes(
            GraphScene.Empty(GraphMetric.Tokens),
            TimeZoneInfo.Utc,
            CultureInfo.InvariantCulture);

        Assert.Equal([0d, 0.25d, 0.5d, 0.75d, 1d], projection.BottomValues);
        Assert.Equal([0d, 0.25d, 0.5d, 0.75d, 1d], projection.ModelValues);
        Assert.Equal([0d, 25d, 50d, 75d, 100d], projection.RemainingValues);
        Assert.Equal(["0%", "25%", "50%", "75%", "100%"], projection.RemainingLabels);
        Assert.Equal(5, projection.BottomLabels.Count);
        Assert.Equal(5, projection.ModelLabels.Count);
    }

    [Fact]
    public void PlotProjectionReservesNativeHeadroomAndEndpointLabelGutter()
    {
        var scene = Scene(
            [
                Point(1_000, 100, 0, 0, 0),
                Point(2_000, 75, 2, 4, 6),
            ]);

        var projection = GraphPlotProjection.BuildAxes(
            scene,
            TimeZoneInfo.Utc,
            CultureInfo.InvariantCulture);

        Assert.True(projection.ModelDisplayMinimum < 0);
        Assert.True(projection.ModelDisplayMaximum > scene.ModelMaximum);
        Assert.Equal(0.01, (0 - projection.ModelDisplayMinimum) /
            (projection.ModelDisplayMaximum - projection.ModelDisplayMinimum), precision: 12);
        Assert.Equal(0.99, (scene.ModelMaximum - projection.ModelDisplayMinimum) /
            (projection.ModelDisplayMaximum - projection.ModelDisplayMinimum), precision: 12);
        Assert.True(projection.EndpointLabelAt > scene.PeriodEndAt);
        Assert.True(projection.DisplayEndAt > projection.EndpointLabelAt);
        Assert.Equal(scene.PeriodEndAt, projection.BottomValues[^1]);
    }

    [Fact]
    public void PlotProjectionSeparatesFlatAndRisingSegmentsLikeTheNativeGraph()
    {
        var scene = Scene(
            [
                Point(1_000, 100, 0, 0, 0),
                Point(1_060, 99, 0, 0, 1),
                Point(1_120, 99, 0, 0, 1),
                Point(1_180, 98, 0, 0, 2),
            ]);

        var lines = GraphPlotProjection.BuildModelLines(scene, scene.Luna);

        Assert.Equal([1_060d, 1_120d], lines.Flat.X);
        Assert.Equal([1d, 1d], lines.Flat.Y);
        Assert.Equal([1_000d, 1_060d, double.NaN, 1_120d, 1_180d], lines.Rising.X);
        Assert.Equal([0d, 1d, double.NaN, 1d, 2d], lines.Rising.Y);
    }

    [Fact]
    public void PlotProjectionHoldsSyntheticAnchorFlatThenChangesAtFirstObservation()
    {
        var scene = Scene(
            [
                Point(1_000, 100, 0, 0, 0),
                Point(1_120, 90, 1, 0, 0),
            ]);

        var model = GraphPlotProjection.BuildModelLines(scene, scene.Sol);
        var remaining = GraphPlotProjection.BuildRemainingLine(scene);

        Assert.Equal([1_000d, 1_120d], model.Flat.X);
        Assert.Equal([0d, 0d], model.Flat.Y);
        Assert.Equal([1_120d, 1_120d], model.Rising.X);
        Assert.Equal([0d, 1d], model.Rising.Y);
        Assert.Equal([1_000d, 1_120d, 1_120d], remaining.X);
        Assert.Equal([100d, 100d, 90d], remaining.Y);
    }

    [Fact]
    public void PlotProjectionDropsSubpixelIdleBandsButKeepsMeaningfulAndBoundaryBands()
    {
        var scene = Scene(
            [
                Point(1_000, 100, 0, 0, 0),
                Point(1_060, 100, 0, 0, 0),
                Point(2_000, 90, 1, 0, 0),
                Point(7_000, 90, 1, 0, 0),
            ],
            1_000,
            87_400);

        var visible = GraphPlotProjection.BuildVisibleIdleIntervals(scene);

        var retained = Assert.Single(visible);
        Assert.Equal((2_000L, 7_000L), (retained.StartAt, retained.EndAt));

        var boundaryScene = Scene(
            [
                Point(1_000, 100, 0, 0, 0),
                Point(1_120, 90, 1, 0, 0),
            ],
            1_000,
            87_400);
        Assert.True(Assert.Single(GraphPlotProjection.BuildVisibleIdleIntervals(boundaryScene)).PreserveBoundary);
    }

    [Fact]
    public void PlotProjectionPreservesAxisValueFormattingBoundaries()
    {
        var culture = CultureInfo.InvariantCulture;

        Assert.Equal("$12.30", GraphPlotProjection.FormatAxisValue(12.3, GraphMetric.Dollars, culture));
        Assert.Equal("999", GraphPlotProjection.FormatAxisValue(999, GraphMetric.Tokens, culture));
        Assert.Equal("1.0K", GraphPlotProjection.FormatAxisValue(1_000, GraphMetric.Tokens, culture));
        Assert.Equal("1.0M", GraphPlotProjection.FormatAxisValue(1_000_000, GraphMetric.Tokens, culture));
        Assert.Equal("1.0B", GraphPlotProjection.FormatAxisValue(1_000_000_000, GraphMetric.Tokens, culture));
    }

    [Fact]
    public void PlotProjectionOrdersEndpointCandidatesAndReturnsAxisValues()
    {
        var scene = Scene(
            [
                Point(1_000, 100, 1, 2, 3),
                Point(1_100, 75, 2, 4, 6),
            ]);

        var labels = GraphPlotProjection.BuildEndpointLabels(scene, CultureInfo.InvariantCulture);

        Assert.Equal(
            [GraphSeries.Luna, GraphSeries.Remaining, GraphSeries.Terra, GraphSeries.Sol],
            labels.Select(label => label.Series));
        Assert.Equal(["LUNA $6.00", "75%", "TERRA $4.00", "SOL $2.00"], labels.Select(label => label.Text));
        Assert.Equal(0d, labels[0].NormalizedTop);
        Assert.Equal(0.25d, labels[1].NormalizedTop);
        Assert.Equal(1d - 4d / 6d, labels[2].NormalizedTop, precision: 12);
        Assert.Equal(1d - 2d / 6d, labels[3].NormalizedTop, precision: 12);
        Assert.Equal(5.85d, labels[0].AxisValue, precision: 12);
        Assert.Equal(75d, labels[1].AxisValue, precision: 12);
        Assert.Equal(4d, labels[2].AxisValue, precision: 12);
        Assert.Equal(2d, labels[3].AxisValue, precision: 12);
        Assert.True(labels.Zip(labels.Skip(1)).All(pair => pair.Second.ArrangedTop - pair.First.ArrangedTop >= 0.062));
    }

    [Fact]
    public void PlotProjectionHandlesEmptyScenesAndRejectsNullInputs()
    {
        Assert.Empty(GraphPlotProjection.BuildEndpointLabels(GraphScene.Empty(), CultureInfo.InvariantCulture));
        Assert.Throws<ArgumentNullException>(() => GraphPlotProjection.BuildAxes(null!, TimeZoneInfo.Utc, CultureInfo.InvariantCulture));
        Assert.Throws<ArgumentNullException>(() => GraphPlotProjection.BuildEndpointLabels(GraphScene.Empty(), null!));
        Assert.Throws<ArgumentNullException>(() => GraphPlotProjection.FormatAxisValue(1, GraphMetric.Dollars, null!));
    }

    [Fact]
    public void Remaining_with_no_graph_points_stays_empty()
    {
        Assert.Empty(GraphScene.Empty().Remaining);
    }

    [Fact]
    public void Graph_samples_use_minute_bucket_maxima_and_cumulative_values()
    {
        var period = new ApiHistoryPeriod("2000", 1_000, 1_500, false, "history")
        {
            Samples =
            [
                new ApiHistorySample(1_201, 2_000, null, 2, 1, 0, 20, 10, 0),
                new ApiHistorySample(1_229, 2_000, 90, 1, 3, 0, 10, 30, 0),
                new ApiHistorySample(1_281, 2_000, 80, 3, 2, 0, 30, 20, 0),
            ],
        };

        var samples = GraphWindowViewModel.BuildGraphSamples(period, 1_400);

        Assert.Equal([1_000L, 1_200L, 1_260L, 1_500L], samples.Select(sample => sample.Timestamp));
        Assert.Equal(2, samples[1].SolDollars);
        Assert.Equal(3, samples[1].TerraDollars);
        Assert.Equal(30UL, samples[1].TerraTokens);
        Assert.Equal(3, samples[2].SolDollars);
        Assert.Equal(3, samples[2].TerraDollars);
        Assert.Equal(80, samples[^1].RemainingPercent);
    }

    [Fact]
    public void Graph_samples_restore_the_native_100_percent_anchor_for_a_missing_first_quota()
    {
        var period = new ApiHistoryPeriod("2000", 1_000, 1_500, false, "history")
        {
            Samples =
            [
                new ApiHistorySample(1_000, 2_000, null, 1, 0, 0, 10, 0, 0),
                new ApiHistorySample(1_061, 2_000, 90, 2, 0, 0, 20, 0, 0),
            ],
        };

        var samples = GraphWindowViewModel.BuildGraphSamples(period, 1_400);

        Assert.Equal(1_000, samples[0].Timestamp);
        Assert.Equal(100, samples[0].RemainingPercent);
        Assert.Equal(90, samples[1].RemainingPercent);
    }

    [Fact]
    public void Idle_intervals_merge_flat_segments_but_keep_the_synthetic_reset_boundary()
    {
        var points = new[]
        {
            Point(1_000, 100, 0, 0, 0),
            Point(1_060, 100, 0, 0, 0),
            Point(1_120, 90, 1, 0, 0),
            Point(1_180, 90, 1, 0, 0),
            Point(1_300, 90, 1, 0, 0),
        };

        var intervals = Scene(points, 1_000, 1_300).IdleIntervals;

        Assert.Equal(2, intervals.Count);
        Assert.Equal((1_000L, 1_060L, false), (intervals[0].StartAt, intervals[0].EndAt, intervals[0].PreserveBoundary));
        Assert.Equal((1_120L, 1_300L, false), (intervals[1].StartAt, intervals[1].EndAt, intervals[1].PreserveBoundary));

        var sparse = new[]
        {
            Point(1_000, 100, 0, 0, 0),
            Point(1_120, 90, 1, 0, 0),
            Point(1_300, 90, 1, 0, 0),
        };
        var sparseIntervals = Scene(sparse, 1_000, 1_300).IdleIntervals;

        Assert.Equal(2, sparseIntervals.Count);
        Assert.True(sparseIntervals[0].PreserveBoundary);
        Assert.Equal((1_000L, 1_120L), (sparseIntervals[0].StartAt, sparseIntervals[0].EndAt));
        Assert.Equal((1_120L, 1_300L), (sparseIntervals[1].StartAt, sparseIntervals[1].EndAt));
    }

    [Fact]
    public void Remaining_repeated_active_samples_are_interpolated_by_active_time()
    {
        var points = new[]
        {
            Point(1_000, 100, 0, 0, 0),
            Point(1_060, 90, 1, 0, 0),
            Point(1_120, 90, 2, 0, 0),
            Point(1_180, 80, 3, 0, 0),
        };

        var effective = Scene(points).Remaining;

        Assert.Equal(100d, effective[0]);
        Assert.Equal(90d, effective[1]);
        Assert.Equal(85d, effective[2]);
        Assert.Equal(80d, effective[3]);
    }

    [Fact]
    public void Remaining_long_active_plateau_is_distributed_across_every_interval()
    {
        var points = new[]
        {
            Point(1_000, 100, 0, 0, 0),
            Point(1_060, 90, 1, 0, 0),
            Point(1_120, 90, 2, 0, 0),
            Point(1_180, 90, 3, 0, 0),
            Point(1_240, 80, 4, 0, 0),
        };

        var effective = Scene(points).Remaining;
        var line = GraphPlotProjection.BuildRemainingLine(Scene(points));

        Assert.Equal(100d, effective[0]);
        Assert.Equal(90d, effective[1]);
        Assert.Equal(86.66666666666667d, effective[2], precision: 12);
        Assert.Equal(83.33333333333333d, effective[3], precision: 12);
        Assert.Equal(80d, effective[4]);
        Assert.Equal(points.Select(point => (double)point.Timestamp), line.X);
        Assert.DoesNotContain(
            line.X.Zip(line.X.Skip(1)),
            pair => pair.First == pair.Second);
    }

    [Fact]
    public void Remaining_stays_flat_through_idle_and_does_not_fabricate_terminal_consumption()
    {
        var points = new[]
        {
            Point(1_000, 100, 0, 0, 0),
            Point(1_060, 90, 1, 0, 0),
            Point(1_120, 90, 1, 0, 0),
            Point(1_180, 80, 2, 0, 0),
        };

        var effective = Scene(points).Remaining;

        Assert.Equal(90d, effective[2]);
        Assert.Equal(80d, effective[3]);

        var idleReread = new[]
        {
            Point(1_000, 100, 0, 0, 0),
            Point(1_060, 90, 1, 0, 0),
            // A lower quota reread while all model totals are unchanged must
            // not create a diagonal segment in the graph.
            Point(1_120, 70, 1, 0, 0),
            Point(1_180, 60, 2, 0, 0),
        };
        var idleRereadEffective = Scene(idleReread).Remaining;
        Assert.Equal(90d, idleRereadEffective[2]);
        Assert.Equal(60d, idleRereadEffective[3]);

        var terminal = new[]
        {
            Point(1_000, 100, 0, 0, 0),
            Point(1_060, 90, 1, 0, 0),
            Point(1_120, 90, 2, 0, 0),
        };
        var terminalEffective = Scene(terminal).Remaining;

        Assert.Equal(90d, terminalEffective[^1]);
    }

    [Fact]
    public void Remaining_keeps_the_reset_gap_flat_and_places_first_use_at_its_timestamp()
    {
        var points = new[]
        {
            Point(1_000, 100, 0, 0, 0),
            Point(1_120, 90, 1, 0, 0),
            Point(1_300, 90, 1, 0, 0),
        };

        var scene = Scene(points);
        var effective = scene.Remaining;

        Assert.Equal(100d, effective[0]);
        Assert.Equal(90d, effective[1]);
        Assert.Equal(90d, effective[2]);
        Assert.Contains(scene.IdleIntervals, interval => interval.PreserveBoundary && interval.StartAt == 1_000 && interval.EndAt == 1_120);

        var shortGap = new[]
        {
            Point(1_000, 100, 0, 0, 0),
            Point(1_060, 90, 1, 0, 0),
        };
        Assert.DoesNotContain(Scene(shortGap).IdleIntervals, interval => interval.PreserveBoundary);
    }

    private static ApiHistorySample Point(long timestamp, double? remaining, double sol, double terra, double luna) =>
        new(timestamp, 2_000, remaining, sol, terra, luna, (ulong)sol, (ulong)terra, (ulong)luna);

    private static GraphScene Scene(IReadOnlyList<ApiHistorySample> points, long? start = null, long? end = null) =>
        GraphScene.Create(points, GraphMetric.Dollars, start ?? points[0].Timestamp, end ?? points[^1].Timestamp);
}
