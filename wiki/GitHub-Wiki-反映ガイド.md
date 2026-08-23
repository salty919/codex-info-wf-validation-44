# GitHub Wiki 反映ガイド

## 事前確認

- 対象リポジトリは公開 or アクセス権限付きで利用可能であること
- `has_wiki` が有効であること（本リポジトリは有効）
- GitHub CLI (`gh`) の認証情報が有効であること

```bash
gh auth status
gh api repos/salty919/codex_info_v2 --jq '.has_wiki'
```

## 反映手順

> 注意: GitHub Wiki は別リポジトリとして管理されます。  
> まず本体側にWikiページを追加し、`/tmp`配下に反映用リポジトリを作ります。

```bash
WIKI_REPO="salty919/codex_info_v2.wiki"
TMP_DIR="/tmp/codex_info_v2.wiki"

mkdir -p "$TMP_DIR"
gh repo clone "$WIKI_REPO" "$TMP_DIR"
cd "$TMP_DIR"

# 本体リポジトリ側のwikiディレクトリからコピー
cp -f /home/salty/code/codex_info_v2/wiki/*.md .

git add .
git commit -m "docs: add wiki pages for Codex Info"
git push
```

## 補足

上のcloneが `Repository not found` で失敗する場合は、以下のいずれかを確認してください。

- Wikiが未作成（初回未使用）で、リモートGitリポジトリが未生成の可能性
- GitHub上で対象Wikiページを1件でも作成してから再試行
- 権限不足または認証トークンのスコープ不足

必要なら最初にGitHub Web上で `https://github.com/salty919/codex_info_v2/wiki` を開き、任意の1ページを作成してから再実行すると進みます。
