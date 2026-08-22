// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using CodexInfo.WindowsClient.Core;
using Xunit;

namespace CodexInfo.WindowsClient.Presentation.Tests;

public sealed class PreviewEnvironmentTests
{
    [Theory]
    [InlineData("700x480", 700, 480)]
    [InlineData(" 1200x900 ", 1200, 900)]
    public void PreviewSizeParserAcceptsBoundedIntegerSizes(string text, double width, double height)
    {
        Assert.True(CodexInfo.WindowsClient.PreviewEnvironment.TryParseSize(text, out var actualWidth, out var actualHeight));
        Assert.Equal(width, actualWidth);
        Assert.Equal(height, actualHeight);
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("700")]
    [InlineData("319x480")]
    [InlineData("700x239")]
    [InlineData("700X480")]
    public void PreviewSizeParserRejectsMalformedOrUnsafeSizes(string? text)
    {
        Assert.False(CodexInfo.WindowsClient.PreviewEnvironment.TryParseSize(text, out _, out _));
    }

    [Fact]
    public async Task GraphThreadsAndLegalPreviewFixtureHasCompleteAuthenticatedData()
    {
        using var client = new CodexInfo.WindowsClient.PreviewLoopbackClient();
        var status = await client.FetchAsync();
        var details = await client.FetchDetailsAsync();

        Assert.True(status.IsSuccess);
        Assert.True(status.Snapshot!.Authenticated);
        Assert.Equal(ApiState.Ready, status.Snapshot.State);
        Assert.Equal(3UL, status.Snapshot.ActiveThreadCount);
        Assert.Equal(3, status.Snapshot.Models.Count);

        Assert.True(details.IsSuccess);
        Assert.Equal(3, details.Snapshot!.HistoryPeriods.Single().Samples.Count);
        Assert.Equal(3, details.Snapshot.Threads.Count);
        Assert.Contains(details.Snapshot.Threads, thread => thread.IsOrphan);
        Assert.Contains(details.Snapshot.Threads, thread => thread.ParentId == "preview-root");
        Assert.True(details.Snapshot.EstimatedCostLabel.Length > 0);
    }
}
