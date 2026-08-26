// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Globalization;
using System.Text.Json;
using CodexInfo.WindowsClient.Core;
using CodexInfo.WindowsClient.Controls;
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
    public void ViewportReductionKeepsBothEdgesOfEveryMonotonicBucket()
    {
        var samples = Enumerable.Range(0, 12)
            .Select(index => new ApiHistorySample(
                index,
                100,
                100 - index,
                index < 5 ? 0 : 10,
                0,
                0,
                0,
                0,
                0))
            .ToArray();

        var reduced = GraphWindowViewModel.ReduceGraphSamples(samples, 6);

        Assert.Equal([0L, 3L, 4L, 7L, 8L, 11L], reduced.Select(sample => sample.Timestamp));
        Assert.Equal(0, reduced[2].SolDollars);
        Assert.Equal(10, reduced[3].SolDollars);
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
    public void PlotProjectionCoalescesAdjacentSegmentsWithTheSameLineStyle()
    {
        var scene = Scene(
            [
                Point(1_000, 100, 0, 0, 0),
                Point(1_060, 99, 0, 0, 1),
                Point(1_120, 98, 0, 0, 2),
                Point(1_180, 97, 0, 0, 3),
            ]);

        var lines = GraphPlotProjection.BuildModelLines(scene, scene.Luna);

        Assert.Empty(lines.Flat.X);
        Assert.Equal([1_000d, 1_060d, 1_120d, 1_180d], lines.Rising.X);
        Assert.Equal([0d, 1d, 2d, 3d], lines.Rising.Y);
    }

    [Fact]
    public void PlotProjectionDoesNotInventSpendDuringAnUnobservedGap()
    {
        var scene = Scene(
            [
                Point(1_000, 100, 0, 0, 0),
                Point(1_060, 99, 0, 0, 1),
                Point(3_600, 98, 0, 0, 2),
                Point(3_660, 97, 0, 0, 2),
            ]);

        var lines = GraphPlotProjection.BuildModelLines(scene, scene.Luna);

        Assert.Equal([1_060d, 3_600d, double.NaN, 3_600d, 3_660d], lines.Flat.X);
        Assert.Equal([1d, 1d, double.NaN, 2d, 2d], lines.Flat.Y);
        Assert.Equal([1_000d, 1_060d, double.NaN, 3_600d, 3_600d], lines.Rising.X);
        Assert.Equal([0d, 1d, double.NaN, 1d, 2d], lines.Rising.Y);
    }

    [Fact]
    public void PlotProjectionStartsAtFirstObservationWithoutSyntheticVerticalJump()
    {
        var scene = Scene(
            [
                Point(1_000, 100, 0, 0, 0),
                Point(1_120, 90, 1, 0, 0),
                Point(1_180, 80, 2, 0, 0),
            ]);

        var model = GraphPlotProjection.BuildModelLines(scene, scene.Sol);
        var remaining = GraphPlotProjection.BuildRemainingLine(scene);

        Assert.Empty(model.Flat.X);
        Assert.Equal([1_120d, 1_180d], model.Rising.X);
        Assert.Equal([1d, 2d], model.Rising.Y);
        Assert.DoesNotContain(
            model.Flat.X.Zip(model.Flat.X.Skip(1)),
            pair => pair.First == pair.Second);
        Assert.DoesNotContain(
            model.Rising.X.Zip(model.Rising.X.Skip(1)),
            pair => pair.First == pair.Second);
        Assert.Equal([1_120d, 1_180d], remaining.X);
        Assert.Equal([90d, 80d], remaining.Y);
        Assert.DoesNotContain(
            remaining.X.Zip(remaining.X.Skip(1)),
            pair => pair.First == pair.Second);
        Assert.True(Assert.Single(scene.IdleIntervals).PreserveBoundary);
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

        Assert.Equal(2, visible.Count);
        Assert.Equal((1_060L, 2_000L), (visible[0].StartAt, visible[0].EndAt));
        Assert.True(visible[0].PreserveBoundary);
        Assert.Equal((2_000L, 7_000L), (visible[1].StartAt, visible[1].EndAt));

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
    public void IdleBandsUseTheDedicatedVisibleNeutralColor()
    {
        Assert.Equal("#3F5D7C", GraphPlotControl.IdleBandColorHex);
        Assert.Equal(0.22, GraphPlotControl.IdleBandOpacity);
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
    public void Graph_samples_do_not_choose_a_conflicting_quota_row_by_order()
    {
        var period = new ApiHistoryPeriod("2000", 1_000, 1_500, false, "history")
        {
            Samples =
            [
                new ApiHistorySample(1_000, 2_000, 88, 1, 0, 0, 10, 0, 0),
                // Same display minute, reset alias, and a contradictory
                // quota value.  The graph must not render 88 -> 14 from row
                // order; model maxima remain useful and deterministic.
                new ApiHistorySample(1_000, 2_050, 14, 2, 0, 0, 20, 0, 0),
                new ApiHistorySample(1_060, 2_050, 87, 3, 0, 0, 30, 0, 0),
            ],
        };

        var samples = GraphWindowViewModel.BuildGraphSamples(period, 1_400);
        var scene = GraphScene.Create(samples, GraphMetric.Dollars, 1_000, 1_500);

        Assert.Equal(100, samples[0].RemainingPercent);
        Assert.DoesNotContain(samples, sample => sample.RemainingPercent == 14);
        Assert.Equal(2, samples[0].SolDollars);
        Assert.Equal(20UL, samples[0].SolTokens);
        Assert.Equal(100d, scene.Remaining[0]);
        Assert.True(scene.Remaining.All(value => !double.IsFinite(value) || value >= 87));
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
    public void Remaining_accepts_a_delayed_lower_quota_after_unobserved_sol_usage()
    {
        var points = new[]
        {
            Point(1_000, 87, 0, 0, 0),
            Point(1_060, null, 140, 0, 0),
            Point(1_120, null, 420, 0, 0),
            Point(1_240, 1, 420, 0, 0),
        };

        var effective = Scene(points).Remaining;

        Assert.Equal(87d, effective[0]);
        Assert.True(effective[1] < 87d);
        Assert.Equal(1d, effective[2]);
        Assert.Equal(1d, effective[3]);
    }

    [Fact]
    public void Shared_graph_fixture_matches_the_native_history_oracle()
    {
        var specRemaining = new[] { 87d, 44d, 1d, 1d, 1d };
        const double specSolMax = 420.40d;
        const int specPeriodCount = 1;
        var fixturePath = Path.Combine(
            AppContext.BaseDirectory,
            "Fixtures",
            "graph_delayed_quota.json");
        using var document = JsonDocument.Parse(File.ReadAllText(fixturePath));
        var root = document.RootElement;
        var periodStart = root.GetProperty("period_start").GetInt64();
        var periodEnd = root.GetProperty("period_end").GetInt64();
        var samples = root.GetProperty("samples")
            .EnumerateArray()
            .Select(sample => new ApiHistorySample(
                sample.GetProperty("timestamp").GetInt64(),
                root.GetProperty("reset_at").GetInt64(),
                sample.GetProperty("remaining_percent").ValueKind == JsonValueKind.Null
                    ? null
                    : sample.GetProperty("remaining_percent").GetDouble(),
                sample.GetProperty("sol_dollars").GetDouble(),
                sample.GetProperty("terra_dollars").GetDouble(),
                sample.GetProperty("luna_dollars").GetDouble(),
                0,
                0,
                0))
            .ToArray();

        var period = new ApiHistoryPeriod("shared", periodStart, periodEnd, false, "shared")
        {
            ResetAt = root.GetProperty("reset_at").GetInt64(),
            Samples = samples,
        };
        var graphSamples = GraphWindowViewModel.BuildGraphSamples(period, periodEnd);
        var scene = GraphScene.Create(graphSamples, GraphMetric.Dollars, periodStart, periodEnd);
        var expected = root.GetProperty("expected_remaining")
            .EnumerateArray()
            .Select(value => value.GetDouble())
            .ToArray();

        // Reviewed literals are the acceptance oracle; do not derive these
        // expected values from GraphScene/BuildGraphSamples.
        Assert.Equal(specPeriodCount, root.GetProperty("expected_period_count").GetInt32());
        Assert.Equal(specRemaining, expected);
        Assert.Equal(specSolMax, root.GetProperty("expected_sol_max").GetDouble(), precision: 6);
        Assert.Equal(periodStart, scene.PeriodStartAt);
        Assert.Equal(periodEnd, scene.PeriodEndAt);
        Assert.Equal(specRemaining.Length, scene.Remaining.Count);
        for (var index = 0; index < specRemaining.Length; index++)
        {
            Assert.Equal(specRemaining[index], scene.Remaining[index], precision: 6);
        }
        Assert.Equal(specSolMax, scene.ModelMaximum, precision: 6);
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

    [Fact]
    public void Idle_band_preserves_every_long_unobserved_spend_gap_not_just_the_first_one()
    {
        var points = new[]
        {
            Point(1_000, 100, 1, 0, 0),
            Point(1_060, 100, 1, 0, 0),
            // No observations for two minutes; the cumulative increase is
            // only known at the endpoint and must not be rendered as usage
            // throughout the unobserved interval.
            Point(1_180, 90, 2, 0, 0),
        };

        var interval = Assert.Single(Scene(points).IdleIntervals, candidate =>
            candidate.StartAt == 1_060 && candidate.EndAt == 1_180);
        Assert.True(interval.PreserveBoundary);
    }

    private static ApiHistorySample Point(long timestamp, double? remaining, double sol, double terra, double luna) =>
        new(timestamp, 2_000, remaining, sol, terra, luna, (ulong)sol, (ulong)terra, (ulong)luna);

    private static GraphScene Scene(IReadOnlyList<ApiHistorySample> points, long? start = null, long? end = null) =>
        GraphScene.Create(points, GraphMetric.Dollars, start ?? points[0].Timestamp, end ?? points[^1].Timestamp);
}
