// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Xml.Linq;
using Xunit;

namespace CodexInfo.WindowsClient.Presentation.Tests;

public sealed class ThreadsWindowLayoutTests
{
    private static readonly XNamespace Avalonia = "https://github.com/avaloniaui";

    [Fact]
    public void ThreadsRowsReserveIndependentColumnsForWorstCaseFallbackRows()
    {
        var document = XDocument.Parse(LoadRepositoryFile(
            "windows-client",
            "src",
            "CodexInfo.WindowsClient",
            "ThreadsWindow.axaml"));
        var window = document.Root;
        Assert.NotNull(window);

        Assert.Equal("900", window!.Attribute("Width")?.Value);
        Assert.Equal("480", window.Attribute("Height")?.Value);

        var metadataGrid = Assert.Single(document.Descendants(Avalonia + "Grid"),
            element => string.Equals(
                element.Attribute("ColumnDefinitions")?.Value,
                "*,190,180,130",
                StringComparison.Ordinal));
        Assert.Equal("10", metadataGrid.Attribute("ColumnSpacing")?.Value);

        var roleDepthGrid = Assert.Single(metadataGrid.Elements(Avalonia + "Grid"),
            element => CellIndex(element, "Grid.Row") == 1 && CellIndex(element, "Grid.Column") == 1);
        Assert.Equal("*,Auto", roleDepthGrid.Attribute("ColumnDefinitions")?.Value);
        Assert.Equal("8", roleDepthGrid.Attribute("ColumnSpacing")?.Value);

        var roleText = BindingText(roleDepthGrid, "{Binding RoleText}");
        var depthText = BindingText(roleDepthGrid, "{Binding DepthText}");
        Assert.Equal(0, CellIndex(roleText, "Grid.Column"));
        Assert.Equal(1, CellIndex(depthText, "Grid.Column"));
        Assert.Null(roleText.Attribute("TextTrimming"));
        Assert.Null(depthText.Attribute("TextTrimming"));

        Assert.DoesNotContain(document.Descendants(Avalonia + "StackPanel"), element =>
            CellIndex(element, "Grid.Row") == 1 && CellIndex(element, "Grid.Column") == 1);

        var treeControl = Assert.Single(document.Descendants(
            XName.Get("ThreadTreeControl", "using:CodexInfo.WindowsClient.Controls")));
        Assert.Equal("64", treeControl.Attribute("Width")?.Value);
        var card = Assert.Single(document.Descendants(Avalonia + "Border"),
            element => string.Equals(element.Attribute("Classes")?.Value, "thread-card", StringComparison.Ordinal));
        Assert.Equal("{Binding Title}", card.Attribute("AutomationProperties.Name")?.Value);

        // At 900px, the fixed window/frame consumes 20px margins, a 1px border,
        // 8px padding, and the 72px tree reservation before this row grid.
        const double surfaceWidth = 900;
        const double frameMargin = 20;
        const double borderThickness = 1;
        const double cardPadding = 8;
        const double treeReservation = 72;
        const double columnSpacing = 10;
        var metadataWidth = surfaceWidth
            - (2 * frameMargin)
            - (2 * borderThickness)
            - (2 * cardPadding)
            - treeReservation;
        Assert.Equal(770, metadataWidth);

        var columns = new[] { 190d, 180d, 130d };
        var titleWidth = metadataWidth - columns.Sum() - (3 * columnSpacing);
        Assert.Equal(240, titleWidth);
        Assert.True(titleWidth >= 220, "title/parent column must retain bounded readable space");
        Assert.True(columns[0] >= 180, "model/role/depth column must fit the longest fallback role and depth pair");
        Assert.True(columns[1] >= 170, "context/token column must remain independently bounded");
        Assert.True(columns[2] >= 120, "elapsed/instruction column must remain independently bounded");

        var worstCaseRows = new[]
        {
            new RowFixture("root", "Main (Unavailable)", "Depth —", "Context —", "Tokens 600"),
            new RowFixture("child", "Sub", "Depth 1", "Context 100.0% / 200,000", "Tokens 600"),
            new RowFixture("orphan", "Sub (Unavailable)", "Depth —", "Context —", "Tokens 600"),
        };
        Assert.Equal(3, worstCaseRows.Length);
        Assert.Contains(worstCaseRows, row => row.Id == "orphan" && row.Role == "Sub (Unavailable)");
        Assert.All(worstCaseRows, row =>
        {
            Assert.False(string.IsNullOrWhiteSpace(row.Role));
            Assert.False(string.IsNullOrWhiteSpace(row.Depth));
            Assert.False(string.IsNullOrWhiteSpace(row.Context));
            Assert.False(string.IsNullOrWhiteSpace(row.Tokens));
        });
    }

    private static XElement BindingText(XElement scope, string binding) =>
        Assert.Single(scope.Elements(Avalonia + "TextBlock"),
            element => string.Equals(element.Attribute("Text")?.Value, binding, StringComparison.Ordinal));

    private static int CellIndex(XElement element, string attributeName) =>
        int.TryParse(element.Attribute(attributeName)?.Value, out var value) ? value : 0;

    private static string LoadRepositoryFile(params string[] segments)
    {
        for (var directory = new DirectoryInfo(AppContext.BaseDirectory); directory is not null; directory = directory.Parent)
        {
            var candidate = Path.Combine([directory.FullName, .. segments]);
            if (File.Exists(candidate))
            {
                return File.ReadAllText(candidate);
            }
        }

        throw new FileNotFoundException($"Could not locate repository file: {Path.Combine(segments)}");
    }

    private sealed record RowFixture(string Id, string Role, string Depth, string Context, string Tokens);
}
