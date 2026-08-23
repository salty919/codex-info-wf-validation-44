// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Net;
using System.Security.Cryptography;
using System.Text;
using CodexInfo.WindowsClient.Core;
using Xunit;

namespace CodexInfo.WindowsClient.Core.Tests;

public sealed class WindowsUpdateClientTests
{
    private const string ApiUri =
        "https://api.github.com/repos/salty919/codex_info_v2/releases?per_page=20";
    private const string InstallerName = "CodexInfo.WindowsClient.Setup.exe";
    private const string ManifestName = "CodexInfo.WindowsClient.update.json";

    [Fact]
    public async Task NoUpdateIsReturnedWhenTheHighestReleaseIsNotNewer()
    {
        var handler = new StubHandler(_ => JsonResponse(Releases(
            Release("windows-v1.2.3", draft: false, prerelease: false))));
        using var client = new WindowsUpdateClient(handler);

        var result = await client.CheckAsync(new Version(1, 2, 3));

        Assert.True(result.IsSuccess);
        Assert.False(result.HasUpdate);
        Assert.Null(result.Release);
        Assert.Null(result.Failure);
    }

    [Fact]
    public async Task SelectsHighestStableReleaseAndFetchesItsManifest()
    {
        var expectedHash = new string('a', 64);
        var releases = Releases(
            Release("windows-v1.2.4", draft: false, prerelease: false),
            Release("windows-v2.0.0", draft: false, prerelease: false),
            Release("windows-v9.0.0", draft: true, prerelease: false),
            Release("windows-v8.0.0", draft: false, prerelease: true));
        var handler = new StubHandler(request => request.RequestUri!.AbsoluteUri == ApiUri
            ? JsonResponse(releases)
            : JsonResponse(Manifest("2.0.0", expectedHash, 4)));
        using var client = new WindowsUpdateClient(handler);

        var result = await client.CheckAsync(new Version(1, 0, 0));

        Assert.True(result.IsSuccess);
        var release = Assert.IsType<WindowsUpdateRelease>(result.Release);
        Assert.Equal(new Version(2, 0, 0), release.Version);
        Assert.Equal(
            "https://github.com/salty919/codex_info_v2/releases/download/windows-v2.0.0/CodexInfo.WindowsClient.Setup.exe",
            release.InstallerUri.AbsoluteUri);
        Assert.Equal(expectedHash, release.Sha256);
        Assert.Equal(4, release.Size);
    }

    [Theory]
    [InlineData("unknown")]
    [InlineData("duplicate")]
    [InlineData("version")]
    [InlineData("url")]
    [InlineData("name")]
    [InlineData("hash")]
    [InlineData("size")]
    [InlineData("overlimit")]
    public async Task ManifestShapeAndMetadataMismatchesAreResponseFailures(string kind)
    {
        var hash = new string('b', 64);
        var manifest = kind switch
        {
            "unknown" => AddTopLevelProperty(Manifest("1.2.4", hash, 4), "\"unknown\":true"),
            "duplicate" => AddTopLevelProperty(Manifest("1.2.4", hash, 4), "\"schema_version\":1"),
            "version" => Manifest("1.2.5", hash, 4),
            "url" => Manifest("1.2.4", hash, 4).Replace(
                "https://github.com/salty919/codex_info_v2/releases/download/windows-v1.2.4/CodexInfo.WindowsClient.Setup.exe",
                "https://github.com/elsewhere/setup.exe",
                StringComparison.Ordinal),
            "name" => Manifest("1.2.4", hash, 4).Replace(InstallerName, "other.exe", StringComparison.Ordinal),
            "hash" => Manifest("1.2.4", "B" + new string('b', 63), 4),
            "size" => Manifest("1.2.4", hash, 0),
            "overlimit" => Manifest("1.2.4", hash, 268_435_457),
            _ => throw new ArgumentOutOfRangeException(nameof(kind)),
        };
        var handler = new StubHandler(request => request.RequestUri!.AbsoluteUri == ApiUri
            ? JsonResponse(Releases(Release("windows-v1.2.4", false, false)))
            : JsonResponse(manifest));
        using var client = new WindowsUpdateClient(handler);

        var result = await client.CheckAsync(new Version(1, 0, 0));

        Assert.Equal(WindowsUpdateFailure.Response, result.Failure);
        Assert.Null(result.Release);
    }

    [Fact]
    public async Task DuplicateRequiredAssetsAreRejected()
    {
        var release = Release("windows-v1.2.4", false, false)
            .Replace(
                $"{{\"name\":\"{InstallerName}\",\"browser_download_url\":\"https://github.com/salty919/codex_info_v2/releases/download/windows-v1.2.4/{InstallerName}\"}}]}}",
                $"{{\"name\":\"{InstallerName}\",\"browser_download_url\":\"https://github.com/salty919/codex_info_v2/releases/download/windows-v1.2.4/{InstallerName}\"}},{{\"name\":\"{ManifestName}\",\"browser_download_url\":\"https://github.com/salty919/codex_info_v2/releases/download/windows-v1.2.4/{ManifestName}\"}}]}}",
                StringComparison.Ordinal);
        using var client = new WindowsUpdateClient(new StubHandler(request =>
            request.RequestUri!.AbsoluteUri == ApiUri
                ? JsonResponse(Releases(release))
                : JsonResponse(Manifest("1.2.4", new string('c', 64), 1))));

        var result = await client.CheckAsync(new Version(1, 0, 0));

        Assert.Equal(WindowsUpdateFailure.Response, result.Failure);
    }

    [Fact]
    public async Task ReleaseAssetRedirectToReleaseAssetsHostIsAllowed()
    {
        var handler = new StubHandler(request =>
        {
            if (request.RequestUri!.AbsoluteUri == ApiUri)
            {
                return JsonResponse(Releases(Release("windows-v1.2.4", false, false)));
            }

            if (request.RequestUri.Host.Equals("github.com", StringComparison.OrdinalIgnoreCase))
            {
                return new HttpResponseMessage(HttpStatusCode.Found)
                {
                    Headers =
                    {
                        Location = new Uri(
                            "https://release-assets.githubusercontent.com/assets/manifest?sig=test"),
                    },
                };
            }

            return JsonResponse(Manifest("1.2.4", new string('c', 64), 1));
        });
        using var client = new WindowsUpdateClient(handler);

        var result = await client.CheckAsync(new Version(1, 0, 0));

        Assert.True(result.IsSuccess);
    }

    [Theory]
    [InlineData("https://github.com/salty919/codex_info_v2/releases/download/windows-v1.2.4/CodexInfo.WindowsClient.update.json", "https://github.com/salty919/codex_info_v2/releases/download/windows-v1.2.4/CodexInfo.WindowsClient.update.json")]
    [InlineData("https://github.com/salty919/codex_info_v2/releases/download/windows-v1.2.4/CodexInfo.WindowsClient.update.json", "https://github.com/other/release/CodexInfo.WindowsClient.update.json")]
    public async Task RedirectsAreBoundedAndHostRestricted(string firstLocation, string secondLocation)
    {
        var calls = 0;
        var handler = new StubHandler(request =>
        {
            calls++;
            if (request.RequestUri!.AbsoluteUri == ApiUri)
            {
                return JsonResponse(Releases(Release("windows-v1.2.4", false, false)));
            }

            return new HttpResponseMessage(HttpStatusCode.Found)
            {
                Headers = { Location = new Uri(calls % 2 == 0 ? firstLocation : secondLocation) },
            };
        });
        using var client = new WindowsUpdateClient(handler);

        var result = await client.CheckAsync(new Version(1, 0, 0));

        Assert.Equal(WindowsUpdateFailure.Response, result.Failure);
        Assert.True(calls <= 5);
    }

    [Fact]
    public async Task ApiAndManifestBodyLimitsAreEnforced()
    {
        var apiBody = new string('x', 1_048_577);
        using var apiClient = new WindowsUpdateClient(new StubHandler(_ => JsonResponse(apiBody)));
        var apiResult = await apiClient.CheckAsync(new Version(1, 0, 0));
        Assert.Equal(WindowsUpdateFailure.Response, apiResult.Failure);

        var largeManifest = AddTopLevelProperty(
            Manifest("1.2.4", new string('d', 64), 1),
            "\"padding\":\"" + new string('x', 16_000) + "\"");
        using var manifestClient = new WindowsUpdateClient(new StubHandler(request =>
            request.RequestUri!.AbsoluteUri == ApiUri
                ? JsonResponse(Releases(Release("windows-v1.2.4", false, false)))
                : JsonResponse(largeManifest)));
        var manifestResult = await manifestClient.CheckAsync(new Version(1, 0, 0));
        Assert.Equal(WindowsUpdateFailure.Response, manifestResult.Failure);
    }

    [Theory]
    [InlineData("{")]
    [InlineData("null")]
    [InlineData("{}")]
    public async Task MalformedApiResponsesAreResponseFailures(string body)
    {
        using var client = new WindowsUpdateClient(new StubHandler(_ => JsonResponse(body)));

        var result = await client.CheckAsync(new Version(1, 0, 0));

        Assert.Equal(WindowsUpdateFailure.Response, result.Failure);
    }

    [Fact]
    public async Task DownloadStreamsAndVerifiesExactBytesAndSha256()
    {
        var payload = Encoding.UTF8.GetBytes("setup");
        var hash = Convert.ToHexString(SHA256.HashData(payload)).ToLowerInvariant();
        var release = new WindowsUpdateRelease(
            new Version(1, 2, 4),
            new Uri("https://github.com/salty919/codex_info_v2/releases/download/windows-v1.2.4/" + InstallerName),
            hash,
            payload.Length);
        using var destination = new MemoryStream();
        using var client = new WindowsUpdateClient(new StubHandler(_ => BinaryResponse(payload)));

        var result = await client.DownloadAsync(release, destination);

        Assert.True(result.IsSuccess);
        Assert.Equal(payload, destination.ToArray());
    }

    [Theory]
    [InlineData("hash")]
    [InlineData("size")]
    [InlineData("short")]
    [InlineData("long")]
    public async Task DownloadIntegrityMismatchesAreTyped(string kind)
    {
        var payload = Encoding.UTF8.GetBytes("setup");
        var hash = Convert.ToHexString(SHA256.HashData(payload)).ToLowerInvariant();
        var release = new WindowsUpdateRelease(
            new Version(1, 2, 4),
            new Uri("https://github.com/salty919/codex_info_v2/releases/download/windows-v1.2.4/" + InstallerName),
            kind == "hash" ? new string('e', 64) : hash,
            kind is "size" or "short" ? payload.Length + 1 :
            kind == "long" ? payload.Length - 1 : payload.Length);
        using var destination = new MemoryStream();
        using var client = new WindowsUpdateClient(new StubHandler(_ => BinaryResponse(payload)));

        var result = await client.DownloadAsync(release, destination);

        Assert.Equal(WindowsUpdateFailure.Integrity, result.Failure);
    }

    [Fact]
    public async Task DownloadRejectsCallerSuppliedReleaseAssetsUrlBeforeNetwork()
    {
        var calls = 0;
        var release = new WindowsUpdateRelease(
            new Version(1, 2, 4),
            new Uri("https://release-assets.githubusercontent.com/assets/setup?sig=unbound"),
            new string('a', 64),
            1);
        using var client = new WindowsUpdateClient(new StubHandler(_ =>
        {
            calls++;
            return BinaryResponse([0]);
        }));

        var result = await client.DownloadAsync(release, new MemoryStream());

        Assert.Equal(WindowsUpdateFailure.Integrity, result.Failure);
        Assert.Equal(0, calls);
    }

    [Fact]
    public async Task TransportAndCancellationFailuresDoNotEscape()
    {
        using var throwingClient = new WindowsUpdateClient(new StubHandler(_ =>
            throw new HttpRequestException("private")));
        var transport = await throwingClient.CheckAsync(new Version(1, 0, 0));
        Assert.Equal(WindowsUpdateFailure.Transport, transport.Failure);

        using var cancelledClient = new WindowsUpdateClient(new StubHandler(_ =>
            throw new OperationCanceledException()));
        using var cancellation = new CancellationTokenSource();
        var cancelled = await cancelledClient.CheckAsync(new Version(1, 0, 0), cancellation.Token);
        Assert.Equal(WindowsUpdateFailure.Transport, cancelled.Failure);
    }

    private static string Releases(params string[] releases) => "[" + string.Join(',', releases) + "]";

    private static string Release(string tag, bool draft, bool prerelease) =>
        $"{{\"tag_name\":\"{tag}\",\"draft\":{draft.ToString().ToLowerInvariant()},\"prerelease\":{prerelease.ToString().ToLowerInvariant()},\"assets\":[" +
        $"{{\"name\":\"{ManifestName}\",\"browser_download_url\":\"https://github.com/salty919/codex_info_v2/releases/download/{tag}/{ManifestName}\"}}," +
        $"{{\"name\":\"{InstallerName}\",\"browser_download_url\":\"https://github.com/salty919/codex_info_v2/releases/download/{tag}/{InstallerName}\"}}]}}";

    private static string Manifest(string version, string hash, long size) =>
        $"{{\"schema_version\":1,\"version\":\"{version}\",\"installer\":{{\"name\":\"{InstallerName}\",\"url\":\"https://github.com/salty919/codex_info_v2/releases/download/windows-v{version}/{InstallerName}\",\"sha256\":\"{hash}\",\"size\":{size}}}}}";

    private static string AddTopLevelProperty(string json, string property) =>
        json[..^1] + "," + property + "}";

    private static HttpResponseMessage JsonResponse(string body) => new(HttpStatusCode.OK)
    {
        Content = new StringContent(body, Encoding.UTF8, "application/json"),
    };

    private static HttpResponseMessage BinaryResponse(byte[] payload) => new(HttpStatusCode.OK)
    {
        Content = new ByteArrayContent(payload),
    };

    private sealed class StubHandler(Func<HttpRequestMessage, HttpResponseMessage> responder) : HttpMessageHandler
    {
        protected override Task<HttpResponseMessage> SendAsync(
            HttpRequestMessage request,
            CancellationToken cancellationToken) =>
            Task.FromResult(responder(request));
    }
}
