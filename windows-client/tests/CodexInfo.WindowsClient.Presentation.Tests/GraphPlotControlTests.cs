// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using CodexInfo.WindowsClient.Controls;
using CodexInfo.WindowsClient.Core;
using CodexInfo.WindowsClient.ViewModels;
using Xunit;

namespace CodexInfo.WindowsClient.Presentation.Tests;

public sealed class GraphPlotControlTests
{
    [Fact]
    public void Remaining_with_no_graph_points_stays_empty()
    {
        Assert.Empty(GraphPlotControl.BuildEffectiveRemaining(Array.Empty<GraphPointViewModel>()));
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

        var intervals = GraphPlotControl.BuildIdleIntervals(points, 1_000, 1_300);

        Assert.Equal(2, intervals.Count);
        Assert.Equal((0d, 20d, false), (intervals[0].Start, intervals[0].Width, intervals[0].PreserveBoundary));
        Assert.Equal((40d, 60d, false), (intervals[1].Start, intervals[1].Width, intervals[1].PreserveBoundary));

        var sparse = new[]
        {
            Point(1_000, 100, 0, 0, 0),
            Point(1_120, 90, 1, 0, 0),
            Point(1_300, 90, 1, 0, 0),
        };
        var sparseIntervals = GraphPlotControl.BuildIdleIntervals(sparse, 1_000, 1_300);

        Assert.Equal(2, sparseIntervals.Count);
        Assert.True(sparseIntervals[0].PreserveBoundary);
        Assert.Equal(0d, sparseIntervals[0].Start);
        Assert.Equal(40d, sparseIntervals[0].Width);
        Assert.Equal((40d, 60d), (sparseIntervals[1].Start, sparseIntervals[1].Width));
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

        var effective = GraphPlotControl.BuildEffectiveRemaining(points);

        Assert.Equal(100d, effective[0]!.Value);
        Assert.Equal(90d, effective[1]!.Value);
        Assert.Equal(85d, effective[2]!.Value);
        Assert.Equal(80d, effective[3]!.Value);
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

        var effective = GraphPlotControl.BuildEffectiveRemaining(points);

        Assert.Equal(90d, effective[2]!.Value);
        Assert.Equal(80d, effective[3]!.Value);

        var idleReread = new[]
        {
            Point(1_000, 100, 0, 0, 0),
            Point(1_060, 90, 1, 0, 0),
            // A lower quota reread while all model totals are unchanged must
            // not create a diagonal segment in the graph.
            Point(1_120, 70, 1, 0, 0),
            Point(1_180, 60, 2, 0, 0),
        };
        var idleRereadEffective = GraphPlotControl.BuildEffectiveRemaining(idleReread);
        Assert.Equal(90d, idleRereadEffective[2]!.Value);
        Assert.Equal(60d, idleRereadEffective[3]!.Value);

        var terminal = new[]
        {
            Point(1_000, 100, 0, 0, 0),
            Point(1_060, 90, 1, 0, 0),
            Point(1_120, 90, 2, 0, 0),
        };
        var terminalEffective = GraphPlotControl.BuildEffectiveRemaining(terminal);

        Assert.Equal(90d, terminalEffective[^1]!.Value);
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

        var effective = GraphPlotControl.BuildEffectiveRemaining(points);

        Assert.Equal(100d, effective[0]!.Value);
        Assert.Equal(90d, effective[1]!.Value);
        Assert.Equal(90d, effective[2]!.Value);
        Assert.True(GraphPlotControl.IsSyntheticRemainingGap(points, 1, 1_000));

        var shortGap = new[]
        {
            Point(1_000, 100, 0, 0, 0),
            Point(1_060, 90, 1, 0, 0),
        };
        Assert.False(GraphPlotControl.IsSyntheticRemainingGap(shortGap, 1, 1_000));
    }

    private static GraphPointViewModel Point(long timestamp, double? remaining, double sol, double terra, double luna) =>
        new(new ApiHistorySample(timestamp, 2_000, remaining, sol, terra, luna, (ulong)sol, (ulong)terra, (ulong)luna), GraphMetric.Dollars);
}
