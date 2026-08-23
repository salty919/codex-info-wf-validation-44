// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Buffers;
using System.Globalization;
using System.Net;
using System.Net.Http.Headers;
using System.Security.Cryptography;
using System.Text.Json;

namespace CodexInfo.WindowsClient.Core;

/// <summary>The failure categories exposed by the Windows update boundary.</summary>
public enum WindowsUpdateFailure
{
    Transport,
    Response,
    Integrity,
}

/// <summary>The validated installer metadata advertised by a Windows release.</summary>
public sealed record WindowsUpdateRelease(
    Version Version,
    Uri InstallerUri,
    string Sha256,
    long Size)
{
    // Short compatibility aliases keep the data contract readable to callers
    // that refer to the installer and digest by their common names.
    public Uri Installer => InstallerUri;

    public string SHA256 => Sha256;
}

/// <summary>A release check result. A null release with no failure means no update.</summary>
public sealed record WindowsUpdateCheckResult(
    WindowsUpdateRelease? Release,
    WindowsUpdateFailure? Failure)
{
    public bool IsSuccess => Failure is null;

    public bool HasUpdate => IsSuccess && Release is not null;

    public WindowsUpdateRelease? Update => Release;

    public static WindowsUpdateCheckResult NoUpdate() => new(null, null);

    public static WindowsUpdateCheckResult Success(WindowsUpdateRelease release)
    {
        ArgumentNullException.ThrowIfNull(release);
        return new WindowsUpdateCheckResult(release, null);
    }

    public static WindowsUpdateCheckResult FromFailure(WindowsUpdateFailure failure) =>
        new(null, failure);
}

/// <summary>A download result that never carries response text or exception details.</summary>
public sealed record WindowsUpdateDownloadResult(WindowsUpdateFailure? Failure)
{
    public bool IsSuccess => Failure is null;

    public static WindowsUpdateDownloadResult Success() => new((WindowsUpdateFailure?)null);

    public static WindowsUpdateDownloadResult FromFailure(WindowsUpdateFailure failure) =>
        new(failure);
}

/// <summary>Checks and downloads the release-bound Windows installer.</summary>
public interface IWindowsUpdateClient
{
    Task<WindowsUpdateCheckResult> CheckAsync(
        Version current,
        CancellationToken cancellationToken = default);

    Task<WindowsUpdateDownloadResult> DownloadAsync(
        WindowsUpdateRelease release,
        Stream destination,
        CancellationToken cancellationToken = default);
}

/// <remarks>
/// The endpoint, release shape, and network allow-list are intentionally fixed
/// here. A handler constructor exists only as a transport seam for tests.
/// </remarks>
public sealed class WindowsUpdateClient : IWindowsUpdateClient, IDisposable
{
    private const string RepositoryOwner = "salty919";
    private const string RepositoryName = "codex_info_v2";
    private const string ApiEndpoint =
        "https://api.github.com/repos/salty919/codex_info_v2/releases?per_page=20";
    private const string ManifestName = "CodexInfo.WindowsClient.update.json";
    private const string InstallerName = "CodexInfo.WindowsClient.Setup.exe";
    private const string GithubHost = "github.com";
    private const string ReleaseAssetsHost = "release-assets.githubusercontent.com";
    private const string GithubJsonAccept = "application/vnd.github+json";
    private const string UserAgent = "CodexInfo.WindowsClient/1.0";
    private const int MaxRedirects = 3;
    private const int MaxApiJsonBytes = 1 * 1024 * 1024;
    private const int MaxManifestBytes = 16 * 1024;
    private const long MaxInstallerSize = 256L * 1024 * 1024;
    private const int MaxResponseHeaderBytes = 64 * 1024;
    private static readonly TimeSpan CheckDeadline = TimeSpan.FromSeconds(30);
    private static readonly TimeSpan DownloadDeadline = TimeSpan.FromMinutes(5);

    private static readonly Uri ApiUri = new(ApiEndpoint, UriKind.Absolute);

    private static readonly HashSet<string> ManifestProperties = new(StringComparer.Ordinal)
    {
        "schema_version",
        "version",
        "installer",
    };

    private static readonly HashSet<string> InstallerProperties = new(StringComparer.Ordinal)
    {
        "name",
        "url",
        "sha256",
        "size",
    };

    private readonly HttpClient _httpClient;

    public WindowsUpdateClient()
        : this(CreateDefaultHandler())
    {
    }

    public WindowsUpdateClient(HttpMessageHandler handler)
    {
        ArgumentNullException.ThrowIfNull(handler);

        if (handler is HttpClientHandler httpClientHandler)
        {
            // Keep the policy true even when an HttpClientHandler is supplied
            // as a test seam.
            httpClientHandler.UseProxy = false;
            httpClientHandler.AllowAutoRedirect = false;
            httpClientHandler.AutomaticDecompression = DecompressionMethods.None;
            httpClientHandler.UseCookies = false;
            httpClientHandler.MaxResponseHeadersLength = MaxResponseHeaderBytes / 1024;
        }

        _httpClient = new HttpClient(handler, disposeHandler: true)
        {
            // Each public operation supplies its own deadline. This must not
            // be a shorter HttpClient-wide timeout because ResponseHeadersRead
            // leaves installer body reads outside HttpClient.Timeout.
            Timeout = System.Threading.Timeout.InfiniteTimeSpan,
        };
    }

    public async Task<WindowsUpdateCheckResult> CheckAsync(
        Version current,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(current);

        try
        {
            using var deadline = new CancellationTokenSource(CheckDeadline);
            using var linkedCancellation = CancellationTokenSource.CreateLinkedTokenSource(
                cancellationToken,
                deadline.Token);
            var operationToken = linkedCancellation.Token;
            operationToken.ThrowIfCancellationRequested();
            using var response = await SendGetFollowingRedirectsAsync(
                    ApiUri,
                    RequestKind.Api,
                    ApiUri,
                    GithubJsonAccept,
                    operationToken)
                .ConfigureAwait(false);

            if (response is null || !IsSuccessful(response))
            {
                return CheckFailure(WindowsUpdateFailure.Response);
            }

            var body = await ReadBoundedBodyAsync(
                    response.Content,
                    MaxApiJsonBytes,
                    operationToken)
                .ConfigureAwait(false);

            if (body.Oversize || body.Bytes is null ||
                !TryParseReleases(body.Bytes, out var releases))
            {
                return CheckFailure(WindowsUpdateFailure.Response);
            }

            var selected = SelectHighestRelease(releases);
            if (selected is null || selected.Version.CompareTo(current) <= 0)
            {
                return WindowsUpdateCheckResult.NoUpdate();
            }

            if (!TryValidateReleaseAssets(
                    selected,
                    selected.Version,
                    out var manifestUri,
                    out var installerUri))
            {
                return CheckFailure(WindowsUpdateFailure.Response);
            }

            using var manifestResponse = await SendGetFollowingRedirectsAsync(
                    manifestUri,
                    RequestKind.ReleaseAsset,
                    manifestUri,
                    GithubJsonAccept,
                    operationToken)
                .ConfigureAwait(false);

            if (manifestResponse is null || !IsSuccessful(manifestResponse))
            {
                return CheckFailure(WindowsUpdateFailure.Response);
            }

            var manifestBody = await ReadBoundedBodyAsync(
                    manifestResponse.Content,
                    MaxManifestBytes,
                    operationToken)
                .ConfigureAwait(false);

            if (manifestBody.Oversize || manifestBody.Bytes is null ||
                !TryParseManifest(
                    manifestBody.Bytes,
                    selected.Version,
                    installerUri,
                    out var sha256,
                    out var size))
            {
                return CheckFailure(WindowsUpdateFailure.Response);
            }

            return WindowsUpdateCheckResult.Success(
                new WindowsUpdateRelease(selected.Version, installerUri, sha256, size));
        }
        catch (OperationCanceledException)
        {
            return CheckFailure(WindowsUpdateFailure.Transport);
        }
        catch (HttpRequestException)
        {
            return CheckFailure(WindowsUpdateFailure.Transport);
        }
        catch (IOException)
        {
            return CheckFailure(WindowsUpdateFailure.Transport);
        }
        catch (Exception)
        {
            // No exception object or response text crosses this boundary,
            // including failures from a custom test handler.
            return CheckFailure(WindowsUpdateFailure.Transport);
        }
    }

    public async Task<WindowsUpdateDownloadResult> DownloadAsync(
        WindowsUpdateRelease release,
        Stream destination,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(release);
        ArgumentNullException.ThrowIfNull(destination);
        if (!destination.CanWrite)
        {
            throw new ArgumentException("The destination stream must be writable.", nameof(destination));
        }

        try
        {
            using var deadline = new CancellationTokenSource(DownloadDeadline);
            using var linkedCancellation = CancellationTokenSource.CreateLinkedTokenSource(
                cancellationToken,
                deadline.Token);
            var operationToken = linkedCancellation.Token;
            operationToken.ThrowIfCancellationRequested();
            if (!TryValidateReleaseForDownload(release, out var expectedGithubUri))
            {
                return DownloadFailure(WindowsUpdateFailure.Integrity);
            }

            using var response = await SendGetFollowingRedirectsAsync(
                    release.InstallerUri,
                    RequestKind.ReleaseAsset,
                    expectedGithubUri,
                    "application/octet-stream",
                    operationToken)
                .ConfigureAwait(false);

            if (response is null || !IsSuccessful(response))
            {
                return DownloadFailure(WindowsUpdateFailure.Response);
            }

            if (response.Content.Headers.ContentLength is { } contentLength &&
                contentLength != release.Size)
            {
                return DownloadFailure(WindowsUpdateFailure.Integrity);
            }

            await using var source = await response.Content
                .ReadAsStreamAsync(operationToken)
                .ConfigureAwait(false);
            using var hash = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
            var buffer = ArrayPool<byte>.Shared.Rent(64 * 1024);
            long total = 0;

            try
            {
                while (true)
                {
                    var read = await source.ReadAsync(
                            buffer.AsMemory(),
                            operationToken)
                        .ConfigureAwait(false);
                    if (read == 0)
                    {
                        break;
                    }

                    if (read > release.Size - total)
                    {
                        return DownloadFailure(WindowsUpdateFailure.Integrity);
                    }

                    hash.AppendData(buffer, 0, read);
                    await destination.WriteAsync(
                            buffer.AsMemory(0, read),
                            operationToken)
                        .ConfigureAwait(false);
                    total += read;
                }
            }
            finally
            {
                ArrayPool<byte>.Shared.Return(buffer);
            }

            if (total != release.Size)
            {
                return DownloadFailure(WindowsUpdateFailure.Integrity);
            }

            var actualHash = hash.GetHashAndReset();
            var expectedHash = Convert.FromHexString(release.Sha256);
            if (!CryptographicOperations.FixedTimeEquals(actualHash, expectedHash))
            {
                return DownloadFailure(WindowsUpdateFailure.Integrity);
            }

            return WindowsUpdateDownloadResult.Success();
        }
        catch (OperationCanceledException)
        {
            return DownloadFailure(WindowsUpdateFailure.Transport);
        }
        catch (HttpRequestException)
        {
            return DownloadFailure(WindowsUpdateFailure.Transport);
        }
        catch (IOException)
        {
            return DownloadFailure(WindowsUpdateFailure.Transport);
        }
        catch (Exception)
        {
            return DownloadFailure(WindowsUpdateFailure.Transport);
        }
    }

    public void Dispose() => _httpClient.Dispose();

    private static HttpMessageHandler CreateDefaultHandler() => new HttpClientHandler
    {
        UseProxy = false,
        AllowAutoRedirect = false,
        AutomaticDecompression = DecompressionMethods.None,
        UseCookies = false,
        MaxResponseHeadersLength = MaxResponseHeaderBytes / 1024,
    };

    private async Task<HttpResponseMessage?> SendGetFollowingRedirectsAsync(
        Uri initialUri,
        RequestKind requestKind,
        Uri expectedGithubUri,
        string accept,
        CancellationToken cancellationToken)
    {
        var visited = new HashSet<string>(StringComparer.Ordinal)
        {
            initialUri.AbsoluteUri,
        };
        var currentUri = initialUri;

        for (var redirectCount = 0; ; redirectCount++)
        {
            if (!IsAllowedUri(currentUri, requestKind, expectedGithubUri))
            {
                return null;
            }

            using var request = new HttpRequestMessage(HttpMethod.Get, currentUri);
            request.Headers.UserAgent.ParseAdd(UserAgent);
            request.Headers.Accept.Clear();
            request.Headers.Accept.Add(new MediaTypeWithQualityHeaderValue(accept));

            var response = await _httpClient.SendAsync(
                    request,
                    HttpCompletionOption.ResponseHeadersRead,
                    cancellationToken)
                .ConfigureAwait(false);

            if (!IsRedirect(response.StatusCode))
            {
                return response;
            }

            Uri? location;
            try
            {
                location = response.Headers.Location;
            }
            catch (Exception)
            {
                response.Dispose();
                return null;
            }

            response.Dispose();

            if (location is null || redirectCount >= MaxRedirects)
            {
                return null;
            }

            Uri nextUri;
            try
            {
                nextUri = new Uri(currentUri, location);
            }
            catch (UriFormatException)
            {
                return null;
            }

            if (!visited.Add(nextUri.AbsoluteUri))
            {
                return null;
            }

            currentUri = nextUri;
        }
    }

    private static bool IsAllowedUri(Uri uri, RequestKind requestKind, Uri expectedGithubUri)
    {
        if (!uri.IsAbsoluteUri || uri.Scheme != Uri.UriSchemeHttps ||
            uri.Port != 443 || uri.UserInfo.Length != 0 || uri.Fragment.Length != 0)
        {
            return false;
        }

        if (requestKind is RequestKind.Api)
        {
            return string.Equals(uri.AbsoluteUri, ApiEndpoint, StringComparison.Ordinal);
        }

        if (string.Equals(uri.Host, GithubHost, StringComparison.OrdinalIgnoreCase))
        {
            return string.Equals(uri.AbsolutePath, expectedGithubUri.AbsolutePath, StringComparison.Ordinal) &&
                string.Equals(uri.Query, expectedGithubUri.Query, StringComparison.Ordinal);
        }

        return string.Equals(uri.Host, ReleaseAssetsHost, StringComparison.OrdinalIgnoreCase) &&
            uri.AbsolutePath.Length > 1;
    }

    private static bool IsRedirect(HttpStatusCode statusCode) => statusCode is
        HttpStatusCode.Moved or
        HttpStatusCode.Found or
        HttpStatusCode.RedirectMethod or
        HttpStatusCode.TemporaryRedirect or
        HttpStatusCode.PermanentRedirect;

    private static bool IsSuccessful(HttpResponseMessage response) =>
        (int)response.StatusCode is >= 200 and <= 299;

    private static async Task<BodyReadResult> ReadBoundedBodyAsync(
        HttpContent content,
        int maximumBytes,
        CancellationToken cancellationToken)
    {
        if (content.Headers.ContentLength is { } contentLength &&
            (contentLength < 0 || contentLength > maximumBytes))
        {
            return BodyReadResult.Oversized();
        }

        await using var source = await content
            .ReadAsStreamAsync(cancellationToken)
            .ConfigureAwait(false);
        using var memory = new MemoryStream(Math.Min(maximumBytes, 64 * 1024));
        var buffer = ArrayPool<byte>.Shared.Rent(64 * 1024);
        var total = 0;

        try
        {
            while (true)
            {
                var read = await source.ReadAsync(
                        buffer.AsMemory(),
                        cancellationToken)
                    .ConfigureAwait(false);
                if (read == 0)
                {
                    break;
                }

                if (read > maximumBytes - total)
                {
                    return BodyReadResult.Oversized();
                }

                memory.Write(buffer, 0, read);
                total += read;
            }
        }
        finally
        {
            ArrayPool<byte>.Shared.Return(buffer);
        }

        return BodyReadResult.Success(memory.ToArray());
    }

    private static bool TryParseReleases(byte[] body, out IReadOnlyList<ApiRelease> releases)
    {
        releases = Array.Empty<ApiRelease>();

        try
        {
            using var document = JsonDocument.Parse(body, new JsonDocumentOptions
            {
                AllowTrailingCommas = false,
                CommentHandling = JsonCommentHandling.Disallow,
            });

            if (document.RootElement.ValueKind is not JsonValueKind.Array)
            {
                return false;
            }

            var parsed = new List<ApiRelease>();
            foreach (var releaseElement in document.RootElement.EnumerateArray())
            {
                if (!TryReadObject(releaseElement, allowedProperties: null, out var releaseProperties) ||
                    !TryGetBoolean(releaseProperties, "draft", out var draft) ||
                    !TryGetBoolean(releaseProperties, "prerelease", out var prerelease) ||
                    !TryGetString(releaseProperties, "tag_name", out var tag))
                {
                    return false;
                }

                if (!TryParseWindowsVersion(tag, out var version) || draft || prerelease)
                {
                    continue;
                }

                if (!releaseProperties.TryGetValue("assets", out var assetsElement) ||
                    assetsElement.ValueKind is not JsonValueKind.Array)
                {
                    return false;
                }

                var assets = new List<ApiAsset>();
                foreach (var assetElement in assetsElement.EnumerateArray())
                {
                    if (!TryReadObject(assetElement, allowedProperties: null, out var assetProperties) ||
                        !TryGetString(assetProperties, "name", out var name) ||
                        !TryGetString(assetProperties, "browser_download_url", out var url))
                    {
                        return false;
                    }

                    assets.Add(new ApiAsset(name, url));
                }

                parsed.Add(new ApiRelease(version, assets));
            }

            releases = parsed;
            return true;
        }
        catch (Exception)
        {
            return false;
        }
    }

    private static ApiRelease? SelectHighestRelease(IReadOnlyList<ApiRelease> releases)
    {
        ApiRelease? selected = null;
        foreach (var release in releases)
        {
            if (selected is null || release.Version.CompareTo(selected.Version) > 0)
            {
                selected = release;
            }
        }

        return selected;
    }

    private static bool TryValidateReleaseAssets(
        ApiRelease release,
        Version version,
        out Uri manifestUri,
        out Uri installerUri)
    {
        manifestUri = null!;
        installerUri = null!;

        var expectedManifest = BuildAssetUri(version, ManifestName);
        var expectedInstaller = BuildAssetUri(version, InstallerName);
        var manifestAssets = release.Assets
            .Where(asset => string.Equals(asset.Name, ManifestName, StringComparison.Ordinal))
            .ToArray();
        var installerAssets = release.Assets
            .Where(asset => string.Equals(asset.Name, InstallerName, StringComparison.Ordinal))
            .ToArray();

        if (manifestAssets.Length != 1 || installerAssets.Length != 1 ||
            !string.Equals(manifestAssets[0].Url, expectedManifest.AbsoluteUri, StringComparison.Ordinal) ||
            !string.Equals(installerAssets[0].Url, expectedInstaller.AbsoluteUri, StringComparison.Ordinal))
        {
            return false;
        }

        manifestUri = expectedManifest;
        installerUri = expectedInstaller;
        return true;
    }

    private static bool TryParseManifest(
        byte[] body,
        Version expectedVersion,
        Uri expectedInstallerUri,
        out string sha256,
        out long size)
    {
        sha256 = string.Empty;
        size = 0;

        try
        {
            using var document = JsonDocument.Parse(body, new JsonDocumentOptions
            {
                AllowTrailingCommas = false,
                CommentHandling = JsonCommentHandling.Disallow,
            });

            if (!TryReadObject(document.RootElement, ManifestProperties, out var properties) ||
                !properties.TryGetValue("schema_version", out var schemaVersion) ||
                schemaVersion.GetRawText() != "1" ||
                !TryGetString(properties, "version", out var versionText) ||
                !TryParseVersion(versionText, out var manifestVersion) ||
                manifestVersion != expectedVersion ||
                !properties.TryGetValue("installer", out var installerElement) ||
                !TryReadObject(installerElement, InstallerProperties, out var installer))
            {
                return false;
            }

            if (!TryGetString(installer, "name", out var name) ||
                !string.Equals(name, InstallerName, StringComparison.Ordinal) ||
                !TryGetString(installer, "url", out var url) ||
                !string.Equals(url, expectedInstallerUri.AbsoluteUri, StringComparison.Ordinal) ||
                !TryGetString(installer, "sha256", out sha256) ||
                !IsLowerHexSha256(sha256) ||
                !TryGetPositiveSize(installer, out size))
            {
                return false;
            }

            return true;
        }
        catch (Exception)
        {
            sha256 = string.Empty;
            size = 0;
            return false;
        }
    }

    private static bool TryValidateReleaseForDownload(
        WindowsUpdateRelease release,
        out Uri expectedGithubUri)
    {
        expectedGithubUri = null!;

        if (release.Version is null || release.InstallerUri is null ||
            !TryParseVersion(FormatVersion(release.Version), out var normalizedVersion) ||
            normalizedVersion != release.Version ||
            !IsLowerHexSha256(release.Sha256) ||
            release.Size <= 0 || release.Size > MaxInstallerSize)
        {
            return false;
        }

        expectedGithubUri = BuildAssetUri(release.Version, InstallerName);
        // The validated release always begins at the canonical repository URL.
        // release-assets.githubusercontent.com is accepted only as a redirect
        // hop received from that exact URL, never as caller-supplied metadata.
        return string.Equals(
            release.InstallerUri.AbsoluteUri,
            expectedGithubUri.AbsoluteUri,
            StringComparison.Ordinal);
    }

    private static Uri BuildAssetUri(Version version, string assetName) =>
        new(
            $"https://github.com/{RepositoryOwner}/{RepositoryName}/releases/download/windows-v{FormatVersion(version)}/{assetName}",
            UriKind.Absolute);

    private static string FormatVersion(Version version) =>
        $"{version.Major}.{version.Minor}.{version.Build}";

    private static bool TryParseWindowsVersion(string? text, out Version version)
    {
        version = null!;
        const string prefix = "windows-v";
        if (text is null || !text.StartsWith(prefix, StringComparison.Ordinal))
        {
            return false;
        }

        return TryParseVersion(text[prefix.Length..], out version);
    }

    private static bool TryParseVersion(string? text, out Version version)
    {
        version = null!;
        if (text is null)
        {
            return false;
        }

        var numeric = text.AsSpan();
        var firstDot = numeric.IndexOf('.');
        if (firstDot <= 0)
        {
            return false;
        }

        var secondPart = numeric[(firstDot + 1)..];
        var secondDot = secondPart.IndexOf('.');
        if (secondDot <= 0 || secondDot == secondPart.Length - 1)
        {
            return false;
        }

        var majorText = numeric[..firstDot];
        var minorText = secondPart[..secondDot];
        var buildText = secondPart[(secondDot + 1)..];
        if (!TryParseCanonicalInt(majorText, out var major) ||
            !TryParseCanonicalInt(minorText, out var minor) ||
            !TryParseCanonicalInt(buildText, out var build))
        {
            return false;
        }

        try
        {
            version = new Version(major, minor, build);
            return true;
        }
        catch (ArgumentOutOfRangeException)
        {
            return false;
        }
    }

    private static bool TryParseCanonicalInt(ReadOnlySpan<char> text, out int value)
    {
        value = 0;
        if (text.Length == 0 || (text.Length > 1 && text[0] == '0') ||
            !int.TryParse(text, NumberStyles.None, CultureInfo.InvariantCulture, out value))
        {
            return false;
        }

        return true;
    }

    private static bool IsLowerHexSha256(string? value)
    {
        if (value is null || value.Length != 64)
        {
            return false;
        }

        foreach (var character in value)
        {
            if (!((character >= '0' && character <= '9') ||
                  (character >= 'a' && character <= 'f')))
            {
                return false;
            }
        }

        return true;
    }

    private static bool TryGetPositiveSize(
        IReadOnlyDictionary<string, JsonElement> properties,
        out long size)
    {
        size = 0;
        if (!properties.TryGetValue("size", out var element) ||
            element.ValueKind is not JsonValueKind.Number)
        {
            return false;
        }

        var raw = element.GetRawText();
        if (raw.Length == 0 || (raw.Length > 1 && raw[0] == '0'))
        {
            return false;
        }

        foreach (var character in raw)
        {
            if (character < '0' || character > '9')
            {
                return false;
            }
        }

        return long.TryParse(raw, NumberStyles.None, CultureInfo.InvariantCulture, out size) &&
            size > 0 && size <= MaxInstallerSize;
    }

    private static bool TryReadObject(
        JsonElement element,
        IReadOnlySet<string>? allowedProperties,
        out Dictionary<string, JsonElement> properties)
    {
        properties = new Dictionary<string, JsonElement>(StringComparer.Ordinal);
        if (element.ValueKind is not JsonValueKind.Object)
        {
            return false;
        }

        foreach (var property in element.EnumerateObject())
        {
            if (!properties.TryAdd(property.Name, property.Value) ||
                (allowedProperties is not null && !allowedProperties.Contains(property.Name)))
            {
                properties.Clear();
                return false;
            }
        }

        return allowedProperties is null || properties.Count == allowedProperties.Count;
    }

    private static bool TryGetString(
        IReadOnlyDictionary<string, JsonElement> properties,
        string name,
        out string value)
    {
        value = string.Empty;
        return properties.TryGetValue(name, out var element) &&
            element.ValueKind is JsonValueKind.String &&
            (value = element.GetString()!) is not null;
    }

    private static bool TryGetBoolean(
        IReadOnlyDictionary<string, JsonElement> properties,
        string name,
        out bool value)
    {
        value = false;
        if (!properties.TryGetValue(name, out var element) ||
            element.ValueKind is not (JsonValueKind.True or JsonValueKind.False))
        {
            return false;
        }

        value = element.GetBoolean();
        return true;
    }

    private static WindowsUpdateCheckResult CheckFailure(WindowsUpdateFailure failure) =>
        WindowsUpdateCheckResult.FromFailure(failure);

    private static WindowsUpdateDownloadResult DownloadFailure(WindowsUpdateFailure failure) =>
        WindowsUpdateDownloadResult.FromFailure(failure);

    private enum RequestKind
    {
        Api,
        ReleaseAsset,
    }

    private sealed record ApiRelease(Version Version, IReadOnlyList<ApiAsset> Assets);

    private sealed record ApiAsset(string Name, string Url);

    private readonly record struct BodyReadResult(byte[]? Bytes, bool Oversize)
    {
        public static BodyReadResult Success(byte[] bytes) => new(bytes, false);

        public static BodyReadResult Oversized() => new(null, true);
    }
}
