//! EN: Dependency acquisition policy; deliberately never execute unsigned downloads.
//! JA: 依存ツール取得方針。検証できないバイナリの自動実行はしない。

pub fn installation_hint() -> &'static str {
    "GitHub CLI (gh): https://cli.github.com/ | GitHub CLI の公式導入案内を参照してください。\n\
     Managed binaries may be selected with GHCAP_GH_PATH after version checks. /\
     GHCAP_GH_PATH で指定した実行ファイルも検証されます。"
}
