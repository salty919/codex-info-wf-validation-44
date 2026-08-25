// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

using System.Text;
using Avalonia.Platform;
using CodexInfo.WindowsClient.Core;
using CodexInfo.WindowsClient.Localization;

namespace CodexInfo.WindowsClient;

/// <summary>
/// The legal window renders the same complete, packaged source documents that
/// are distributed with the client.  Summaries are not used as a substitute
/// for license text; a missing resource produces an explicit fail-closed page.
/// </summary>
internal static class LegalNoticeCatalog
{
    private const string ResourcePrefix = "avares://CodexInfo.WindowsClient/";

    public static IReadOnlyList<ApiLegalNotice> Load(UiText texts)
    {
        ArgumentNullException.ThrowIfNull(texts);

        try
        {
            return
            [
                new(texts.LegalCodeName, Read("Legal/LICENSE.ja.md")),
                new(texts.LegalWarrantyName, Read("Legal/LICENSE")),
                new(texts.LegalLicenseName, Read("Legal/LICENSE")),
                new(texts.LegalFontName, Combine(
                    "Legal/THIRD_PARTY_NOTICES.md",
                    "Legal/LICENSES/OFL-1.1.txt")),
                new(texts.LegalProtocolName, Combine(
                    "Legal/THIRD_PARTY_NOTICES.md",
                    "Legal/LICENSES/Apache-2.0.txt",
                    "Legal/LICENSES/OPENAI-CODEX-NOTICE.txt")),
                new(texts.LegalSchemaName, Combine(
                    "Legal/LICENSES/Apache-2.0.txt",
                    "Legal/LICENSES/OPENAI-CODEX-NOTICE.txt")),
                new(texts.LegalThirdPartyName, Combine(
                    "Legal/THIRD_PARTY_NOTICES.md",
                    "Legal/LICENSES/MIT.txt",
                    "Legal/LICENSES/BSD-3-Clause-ANGLE.txt")),
                new(texts.LegalDetailsName, Combine(
                    "Legal/THIRD_PARTY_NOTICES.md",
                    "Legal/Windows-THIRD_PARTY_NOTICES.md",
                    "Legal/NOTICE.txt")),
                new(texts.LegalDistributionName, Combine(
                    "Legal/LICENSE.ja.md",
                    "Legal/LICENSES/Inno-Setup.txt",
                    "Legal/Windows-THIRD_PARTY_NOTICES.md")),
            ];
        }
        catch (Exception exception) when (exception is IOException or UriFormatException or InvalidDataException)
        {
            var text = texts.LanguageCode == "ja"
                ? "法的文書を読み込めません。再インストールするまで配布・再配布の判断にこの画面を使用しないでください。"
                : "Legal documents are unavailable. Do not use this screen to make distribution or redistribution decisions until the client is reinstalled.";
            return [new ApiLegalNotice(texts.LegalDetailsName, text)];
        }
    }

    private static string Combine(params string[] resources)
    {
        var builder = new StringBuilder();
        foreach (var resource in resources)
        {
            if (builder.Length > 0)
            {
                builder.AppendLine();
                builder.AppendLine();
            }

            builder.Append("----- ");
            builder.Append(resource["Legal/".Length..]);
            builder.AppendLine(" -----");
            builder.Append(Read(resource));
        }

        return builder.ToString();
    }

    private static string Read(string resource)
    {
        var assembly = typeof(LegalNoticeCatalog).Assembly;
        var manifestCandidates = new[]
        {
            resource,
            resource.Replace('/', '.'),
            $"{assembly.GetName().Name}.{resource.Replace('/', '.')}",
        };
        foreach (var candidate in manifestCandidates)
        {
            var manifestStream = assembly.GetManifestResourceStream(candidate);
            if (manifestStream is not null)
            {
                return ReadText(manifestStream);
            }
        }

        var assetLoader = new StandardAssetLoader();
        assetLoader.SetDefaultAssembly(assembly);
        using var stream = assetLoader.Open(new Uri(ResourcePrefix + resource));
        return ReadText(stream);
    }

    private static string ReadText(Stream stream)
    {
        using var reader = new StreamReader(
            stream,
            Encoding.UTF8,
            detectEncodingFromByteOrderMarks: true);
        return reader.ReadToEnd();
    }
}
