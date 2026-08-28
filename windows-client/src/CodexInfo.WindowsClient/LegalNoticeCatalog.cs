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
        return LoadCore(texts, Read);
    }

    internal static IReadOnlyList<ApiLegalNotice> LoadForTest(
        UiText texts,
        Func<string, string> sourceReader)
    {
        ArgumentNullException.ThrowIfNull(texts);
        ArgumentNullException.ThrowIfNull(sourceReader);
        return LoadCore(
            texts,
            resource => ProjectForDisplay(resource, sourceReader(resource)));
    }

    private static IReadOnlyList<ApiLegalNotice> LoadCore(
        UiText texts,
        Func<string, string> read)
    {
        try
        {
            return
            [
                new(texts.LegalCodeName, read("Legal/LICENSE.ja.md")),
                new(texts.LegalWarrantyName, read("Legal/LICENSE")),
                new(texts.LegalLicenseName, read("Legal/LICENSE")),
                new(texts.LegalFontName, Combine(read,
                    "Legal/THIRD_PARTY_NOTICES.md",
                    "Legal/LICENSES/OFL-1.1.txt")),
                new(texts.LegalProtocolName, Combine(read,
                    "Legal/THIRD_PARTY_NOTICES.md",
                    "Legal/LICENSES/Apache-2.0.txt",
                    "Legal/LICENSES/OPENAI-CODEX-NOTICE.txt")),
                new(texts.LegalSchemaName, Combine(read,
                    "Legal/LICENSES/Apache-2.0.txt",
                    "Legal/LICENSES/OPENAI-CODEX-NOTICE.txt")),
                new(texts.LegalThirdPartyName, Combine(read,
                    "Legal/THIRD_PARTY_NOTICES.md",
                    "Legal/LICENSES/MIT.txt",
                    "Legal/LICENSES/BSD-3-Clause-ANGLE.txt")),
                new(texts.LegalDetailsName, Combine(read,
                    "Legal/THIRD_PARTY_NOTICES.md",
                    "Legal/Windows-THIRD_PARTY_NOTICES.md",
                    "Legal/NOTICE.txt")),
                new(texts.LegalDistributionName, Combine(read,
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

    private static string Combine(Func<string, string> read, params string[] resources)
    {
        var builder = new StringBuilder();
        foreach (var resource in resources)
        {
            if (builder.Length > 0)
            {
                builder.AppendLine();
                builder.AppendLine();
            }

            builder.Append('\uff3b');
            builder.Append(resource.StartsWith("Legal/", StringComparison.Ordinal)
                ? resource["Legal/".Length..]
                : resource);
            builder.AppendLine("\uff3d");
            builder.Append(read(resource));
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
                return ProjectForDisplay(resource, ReadText(manifestStream));
            }
        }

        var assetLoader = new StandardAssetLoader();
        assetLoader.SetDefaultAssembly(assembly);
        using var stream = assetLoader.Open(new Uri(ResourcePrefix + resource));
        return ProjectForDisplay(resource, ReadText(stream));
    }

    internal static string ProjectForDisplay(string resource, string source)
    {
        ArgumentNullException.ThrowIfNull(resource);
        ArgumentNullException.ThrowIfNull(source);
        return resource.EndsWith(".md", StringComparison.OrdinalIgnoreCase)
            ? ProjectMarkdownToPlainText(source)
            : source;
    }

    internal static string ProjectMarkdownToPlainText(string source)
    {
        ArgumentNullException.ThrowIfNull(source);

        var lines = source
            .Replace("\r\n", "\n", StringComparison.Ordinal)
            .Replace('\r', '\n')
            .Split('\n');
        var outputLines = new List<string>(lines.Length);
        string? fence = null;
        var inHtmlComment = false;

        foreach (var line in lines)
        {
            if (fence is not null)
            {
                if (IsFenceEnd(line, fence))
                {
                    fence = null;
                    continue;
                }

                outputLines.Add(line);
                continue;
            }

            if (!inHtmlComment && TryReadFenceStart(line, out fence))
            {
                continue;
            }

            outputLines.Add(ProjectMarkdownLine(RemoveHtmlCommentDelimiters(line, ref inHtmlComment)));
        }

        if (fence is not null)
        {
            throw new InvalidDataException("The legal Markdown document contains an unterminated code fence.");
        }

        if (inHtmlComment)
        {
            throw new InvalidDataException("The legal Markdown document contains an unterminated HTML comment.");
        }

        return string.Join('\n', outputLines);
    }

    private static string RemoveHtmlCommentDelimiters(string line, ref bool inHtmlComment)
    {
        var builder = new StringBuilder(line.Length);
        var position = 0;
        while (position < line.Length)
        {
            if (inHtmlComment)
            {
                var closing = line.IndexOf("-->", position, StringComparison.Ordinal);
                if (closing < 0)
                {
                    builder.Append(line, position, line.Length - position);
                    return builder.ToString();
                }

                builder.Append(line, position, closing - position);
                position = closing + "-->".Length;
                inHtmlComment = false;
                continue;
            }

            var opening = line.IndexOf("<!--", position, StringComparison.Ordinal);
            var closingOutsideComment = line.IndexOf("-->", position, StringComparison.Ordinal);
            if (closingOutsideComment >= 0
                && (opening < 0 || closingOutsideComment < opening))
            {
                throw new InvalidDataException("The legal Markdown document contains an unmatched HTML comment close delimiter.");
            }

            if (opening < 0)
            {
                builder.Append(line, position, line.Length - position);
                break;
            }

            builder.Append(line, position, opening - position);
            position = opening + "<!--".Length;
            inHtmlComment = true;
        }

        return builder.ToString();
    }

    private static string ProjectMarkdownLine(string line)
    {
        if (IsHorizontalRule(line))
        {
            return string.Empty;
        }

        var projected = RemoveAtxHeadingMarker(line);
        if (TryProjectListMarker(projected, out var listLine))
        {
            projected = listLine;
        }

        return ProjectInlineMarkdown(projected);
    }

    private static string RemoveAtxHeadingMarker(string line)
    {
        var markerStart = 0;
        while (markerStart < line.Length
            && markerStart < 3
            && (line[markerStart] == ' ' || line[markerStart] == '\t'))
        {
            markerStart++;
        }

        var markerEnd = markerStart;
        while (markerEnd < line.Length && line[markerEnd] == '#')
        {
            markerEnd++;
        }

        var markerLength = markerEnd - markerStart;
        if (markerLength is < 1 or > 6
            || (markerEnd < line.Length && line[markerEnd] != ' ' && line[markerEnd] != '\t'))
        {
            return line;
        }

        while (markerEnd < line.Length && (line[markerEnd] == ' ' || line[markerEnd] == '\t'))
        {
            markerEnd++;
        }

        var content = line[markerEnd..];
        var closingMarkerEnd = content.Length - 1;
        while (closingMarkerEnd >= 0 && (content[closingMarkerEnd] == ' ' || content[closingMarkerEnd] == '\t'))
        {
            closingMarkerEnd--;
        }

        var closingMarkerStart = closingMarkerEnd;
        while (closingMarkerStart >= 0 && content[closingMarkerStart] == '#')
        {
            closingMarkerStart--;
        }

        if (closingMarkerStart < closingMarkerEnd
            && closingMarkerStart >= 0
            && (content[closingMarkerStart] == ' ' || content[closingMarkerStart] == '\t'))
        {
            content = content[..(closingMarkerStart + 1)].TrimEnd(' ', '\t');
        }

        return line[..markerStart] + content;
    }

    private static bool TryProjectListMarker(string line, out string projected)
    {
        var markerStart = 0;
        while (markerStart < line.Length
            && (line[markerStart] == ' ' || line[markerStart] == '\t'))
        {
            markerStart++;
        }

        if (markerStart + 1 < line.Length
            && (line[markerStart] == '-' || line[markerStart] == '+' || line[markerStart] == '*')
            && (line[markerStart + 1] == ' ' || line[markerStart + 1] == '\t'))
        {
            var contentStart = markerStart + 1;
            while (contentStart < line.Length && (line[contentStart] == ' ' || line[contentStart] == '\t'))
            {
                contentStart++;
            }

            projected = line[..markerStart] + "\u2022 " + line[contentStart..];
            return true;
        }

        var numberEnd = markerStart;
        while (numberEnd < line.Length && line[numberEnd] is >= '0' and <= '9')
        {
            numberEnd++;
        }

        if (numberEnd > markerStart
            && numberEnd + 1 < line.Length
            && (line[numberEnd] == '.' || line[numberEnd] == ')')
            && (line[numberEnd + 1] == ' ' || line[numberEnd + 1] == '\t'))
        {
            var contentStart = numberEnd + 1;
            while (contentStart < line.Length && (line[contentStart] == ' ' || line[contentStart] == '\t'))
            {
                contentStart++;
            }

            projected = line[..markerStart] + line[markerStart..numberEnd] + ". " + line[contentStart..];
            return true;
        }

        projected = line;
        return false;
    }

    private static bool IsHorizontalRule(string line)
    {
        var start = 0;
        while (start < line.Length && (line[start] == ' ' || line[start] == '\t'))
        {
            start++;
        }

        if (start == line.Length || (line[start] != '-' && line[start] != '*' && line[start] != '_'))
        {
            return false;
        }

        var marker = line[start];
        var count = 0;
        for (var index = start; index < line.Length; index++)
        {
            if (line[index] == marker)
            {
                count++;
            }
            else if (line[index] != ' ' && line[index] != '\t')
            {
                return false;
            }
        }

        return count >= 3;
    }

    private static bool TryReadFenceStart(string line, out string? fence)
    {
        var start = 0;
        while (start < line.Length && start < 3 && (line[start] == ' ' || line[start] == '\t'))
        {
            start++;
        }

        var end = start;
        while (end < line.Length && line[end] == '`')
        {
            end++;
        }

        if (end - start < 3)
        {
            fence = null;
            return false;
        }

        fence = new string('`', end - start);
        return true;
    }

    private static bool IsFenceEnd(string line, string fence)
    {
        var start = 0;
        while (start < line.Length && start < 3 && (line[start] == ' ' || line[start] == '\t'))
        {
            start++;
        }

        var end = start;
        while (end < line.Length && line[end] == '`')
        {
            end++;
        }

        if (end - start < fence.Length)
        {
            return false;
        }

        for (var index = end; index < line.Length; index++)
        {
            if (line[index] != ' ' && line[index] != '\t')
            {
                return false;
            }
        }

        return true;
    }

    private static string ProjectInlineMarkdown(string input)
    {
        var builder = new StringBuilder(input.Length);
        var emphasis = new List<(char Marker, int Length)>();
        var index = 0;

        while (index < input.Length)
        {
            if (input[index] == '[' && TryReadLink(input, index, out var linkEnd, out var label, out var url))
            {
                var plainLabel = StripInlineDelimiters(label);
                builder.Append(plainLabel);
                if (!string.Equals(plainLabel, url, StringComparison.Ordinal))
                {
                    builder.Append(" (");
                    builder.Append(url);
                    builder.Append(')');
                }

                index = linkEnd;
                continue;
            }

            if (input[index] == '<' && IsAutolinkStart(input, index))
            {
                var end = input.IndexOf('>', index + 1);
                if (end < 0)
                {
                    throw new InvalidDataException("The legal Markdown document contains an unterminated autolink.");
                }

                builder.Append(input, index + 1, end - index - 1);
                index = end + 1;
                continue;
            }

            if (input[index] == '`')
            {
                var runLength = CountRun(input, index, '`');
                var delimiter = new string('`', runLength);
                var end = input.IndexOf(delimiter, index + runLength, StringComparison.Ordinal);
                if (end < 0)
                {
                    throw new InvalidDataException("The legal Markdown document contains an unterminated inline code span.");
                }

                builder.Append(input, index + runLength, end - index - runLength);
                index = end + runLength;
                continue;
            }

            if (input[index] is '*' or '_' or '~')
            {
                var marker = input[index];
                var runLength = CountRun(input, index, marker);
                var canOpen = CanOpenEmphasis(input, index, runLength);
                var canClose = CanCloseEmphasis(input, index, runLength);
                var matching = FindOpenEmphasis(emphasis, marker, runLength);

                if (canClose && matching >= 0)
                {
                    emphasis.RemoveAt(matching);
                    index += runLength;
                    continue;
                }

                if (canOpen)
                {
                    emphasis.Add((marker, runLength));
                    index += runLength;
                    continue;
                }
            }

            builder.Append(input[index]);
            index++;
        }

        if (emphasis.Count > 0)
        {
            throw new InvalidDataException("The legal Markdown document contains an unterminated emphasis span.");
        }

        return builder.ToString();
    }

    private static string StripInlineDelimiters(string input)
    {
        var builder = new StringBuilder(input.Length);
        var emphasis = new List<(char Marker, int Length)>();
        var index = 0;
        while (index < input.Length)
        {
            if (input[index] == '`')
            {
                var runLength = CountRun(input, index, '`');
                var delimiter = new string('`', runLength);
                var end = input.IndexOf(delimiter, index + runLength, StringComparison.Ordinal);
                if (end < 0)
                {
                    throw new InvalidDataException("The legal Markdown document contains an unterminated inline code span.");
                }

                builder.Append(input, index + runLength, end - index - runLength);
                index = end + runLength;
                continue;
            }

            if (input[index] is '*' or '_' or '~')
            {
                var marker = input[index];
                var runLength = CountRun(input, index, marker);
                var canOpen = CanOpenEmphasis(input, index, runLength);
                var canClose = CanCloseEmphasis(input, index, runLength);
                var matching = FindOpenEmphasis(emphasis, marker, runLength);
                if (canClose && matching >= 0)
                {
                    emphasis.RemoveAt(matching);
                    index += runLength;
                    continue;
                }

                if (canOpen)
                {
                    emphasis.Add((marker, runLength));
                    index += runLength;
                    continue;
                }
            }

            builder.Append(input[index]);
            index++;
        }

        if (emphasis.Count > 0)
        {
            throw new InvalidDataException("The legal Markdown link label contains an unterminated emphasis span.");
        }

        return builder.ToString();
    }

    private static bool TryReadLink(
        string input,
        int start,
        out int end,
        out string label,
        out string url)
    {
        var labelEnd = input.IndexOf(']', start + 1);
        if (labelEnd < 0 || labelEnd + 1 >= input.Length || input[labelEnd + 1] != '(')
        {
            end = 0;
            label = string.Empty;
            url = string.Empty;
            return false;
        }

        var urlStart = labelEnd + 2;
        var depth = 0;
        var urlEnd = -1;
        for (var index = urlStart; index < input.Length; index++)
        {
            if (input[index] == '(')
            {
                depth++;
            }
            else if (input[index] == ')')
            {
                if (depth == 0)
                {
                    urlEnd = index;
                    break;
                }

                depth--;
            }
        }

        if (urlEnd < 0 || depth != 0)
        {
            throw new InvalidDataException("The legal Markdown document contains an unterminated link.");
        }

        label = input[(start + 1)..labelEnd];
        url = input[urlStart..urlEnd];
        if (label.Length == 0 || url.Length == 0)
        {
            throw new InvalidDataException("The legal Markdown document contains an empty link label or URL.");
        }

        end = urlEnd + 1;
        return true;
    }

    private static bool IsAutolinkStart(string input, int start)
    {
        return input.AsSpan(start).StartsWith("<https://", StringComparison.OrdinalIgnoreCase)
            || input.AsSpan(start).StartsWith("<http://", StringComparison.OrdinalIgnoreCase);
    }

    private static int CountRun(string input, int start, char marker)
    {
        var end = start;
        while (end < input.Length && input[end] == marker)
        {
            end++;
        }

        return end - start;
    }

    private static int FindOpenEmphasis(List<(char Marker, int Length)> emphasis, char marker, int length)
    {
        for (var index = emphasis.Count - 1; index >= 0; index--)
        {
            if (emphasis[index].Marker == marker && emphasis[index].Length == length)
            {
                return index;
            }
        }

        return -1;
    }

    private static bool CanOpenEmphasis(string input, int start, int length)
    {
        var after = start + length;
        if (after >= input.Length || char.IsWhiteSpace(input[after]))
        {
            return false;
        }

        if (input[start] != '_')
        {
            return true;
        }

        return start == 0 || !IsWordCharacter(input[start - 1]) || !IsWordCharacter(input[after]);
    }

    private static bool CanCloseEmphasis(string input, int start, int length)
    {
        if (start == 0 || char.IsWhiteSpace(input[start - 1]))
        {
            return false;
        }

        if (input[start] != '_')
        {
            return true;
        }

        var after = start + length;
        return after >= input.Length || !IsWordCharacter(input[start - 1]) || !IsWordCharacter(input[after]);
    }

    private static bool IsWordCharacter(char value)
    {
        return char.IsLetterOrDigit(value) || value == '_';
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
