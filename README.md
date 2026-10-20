# ghcap v0.0.29.0 — safety candidate

GitHub用のRust製TUIです。Git操作は `git`、GitHub操作は `gh`、署名鍵操作は `gpg` を利用します。この版は安全性改善の候補版です。両macOS実機での操作・配布確認を終えるまでは、正式なStableとは扱いません。

## 実行環境

- Git、GitHub CLI、GnuPGをPATHへ導入してください。
- `gh auth status --json hosts` に対応し、セキュリティ更新が適用されたGitHub CLIが必要です。古いDebian標準パッケージでは、このJSON機能が不足する場合があります。
- GitHubアカウントの認証は `gh auth login` で管理します。ghcapにトークンを直接設定しません。
- Rustからビルドする場合はRust 1.89以上とCargo.lockが必要です。最低Rust版そのもののコンパイルは別途検証が必要です。

## 既存機能

アカウントとリポジトリの管理、Clone、Git関連付け、Commit、Push、Commit & Push、Commitプリセット、任意の事前/事後シェル、GPG鍵管理、フォルダからのGitHub Release作成・資産追加を提供します。

Commitは確認した対象を `git add -A` で全件stageします。未追跡・削除も含み、既存のstage範囲を広げます。一覧を必ず確認してください。競合、merge/rebase等の進行中操作、別のGit root、確認後の変更を検出した場合は停止します。stage後の停止・Commit失敗ではindexを自動的に戻しません。Gitで確認してください。

Pushは確認したSHAをoriginの単一upstreamブランチへ、forceなしで送信します。upstreamがない場合はGitで明示設定してください。mirror、matching、force/delete/wildcard refspec、複数送信先、異なるpushurlは通常経路で拒否します。通信の中断はリモートへの未反映を保証しません。再試行前にリモートを確認してください。

任意シェルとGit hookはユーザー権限のコードです。信頼できるリポジトリ・コマンドのみ使用してください。Skipはシェルを実行しません。ghcapはGitや外部コマンドを隔離するサンドボックスではありません。

## Releaseフォルダ

`release-notes.md`、`RELEASE_NOTES.md`、`Release-Notes.md` のうち1個と、配布する通常ファイルを配置します。ファイル名の重複（ASCIIの大文字小文字を区別しない）、制御文字、`#`、外部ノートへのシンボリックリンクを拒否します。通常の資産シンボリックリンクは除外します。

Unixではno-followで開いた内容を非公開の一時フォルダへコピーし、確認後はそのスナップショットを使用します。ノート1 MiB、資産1個512 MiB、合計1 GiB、資産10,000個までです。これらを超えた場合は明示エラーになります。Windowsのフォルダ公開は安全なno-follow実装が検証されるまで停止します。これは既存機能の制限です。

Release先は選択した `owner/repo` を明示指定します。新規タグの対象は確認したローカルHEADのSHAです。既存タグとの不一致を拒否します。手動で用意したバイナリとソースの一致をghcapが自動認定するわけではありません。既存資産は上書きせず、途中失敗時は作成済みReleaseと資産を手動確認してください。

## 設定と停止

設定はOSのユーザー設定領域に保存します。解決不能時の作業ディレクトリへのfallbackはありません。不正JSON、未知schema、外部からの同時変更を検出した場合は保存を止めます。原子的置換・ファイルと親ディレクトリのfsyncを維持し、同一内容の保存は省略します。Unixの新規ファイルは0600、設定ディレクトリは0700です。旧tokenを含む設定の移行では非公開の原本バックアップを作成します。バックアップにも機密値があるためユーザーが管理してください。

`ghcap commitstop`、`ghcap pushstop`、`ghcap commitpushstop` は起動中インスタンスへ停止を要求します。「要求受付」は「処理が巻き戻った」という意味ではありません。Unixでは処理専用のプロセスグループを停止し、実際の終了結果を表示します。通常捕捉コマンドは60秒、ライブ処理は15分で期限切れになります。大量出力も上限を超えると失敗として扱います。

## ビルドと検査

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
```

GitHub ActionsはmacOS Intel・Apple Silicon、Windows x64、Linux x86_64のネイティブビルドとテストを行います。全ジョブが成功したmasterへのPushでは、同一Commitの成果物とSHA-256をGitHub Pre-releaseへ公開します。既存タグの移動・資産の上書き・Stableの変更は拒否します。同じ版で別Commitを公開する場合は `-pre.<Commitの先頭12桁>` 付きタグを使用し、過去の配布物を保持します。`.github/workflows/build-macos.yml` はmasterへのPush、PR、手動操作で実行します。各Macで `./packaging/build-macos.sh intel` または `./packaging/build-macos.sh apple-silicon` を実行します。生成済み同名アーカイブの上書きは拒否します。アーカイブにはソースCommit、ツールチェーン情報、Cargo.lockのSHA-256と第三者ライセンス資料を含めます。署名・公証はしていません。最低macOS版とGatekeeper/quarantineの実機検査は別途必要です。

旧build-all.sh/build-linux.shはActionsから呼びません。新しいCI専用Python処理でWindows exe/zip、Linux tar.gz/deb/rpm/pacman/AppImageを生成します。Linuxはglibc 2.35以上が必要です。AppImageの公式ツールとruntimeは版とSHA-256を固定しています。

公開版表示は `0.0.29.0`、Cargoの3要素SemVerは `0.29.0` です。過去の `0.0.29.8` とは別のユーザー指定番号であり、過去のタグ・成果物を置き換えません。

BSD-2-Clause。依存ライセンスは `THIRD_PARTY_NOTICES.md` と `THIRD_PARTY_LICENSES/` を参照してください。未収集の本文があるため、正式再配布前の確認事項が残っています。
