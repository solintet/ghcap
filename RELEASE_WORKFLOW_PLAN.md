# Release workflow / リリースの流れ

## English

CI builds on four native runners; PR builds never publish. Pushes to `master` publish an automatically versioned Pre-release only after all platform tests and asset verification succeed. The exact tag `v0.0.31.0` triggers Stable publication after the same gate. Publication is performed as Draft upload → integrity check → publish; neither existing tags nor published Assets are overwritten.

## 日本語

CIは4つのネイティブRunnerで実行します。PRからの公開は行いません。`master`へのPushは全プラットフォームの検査と配布物検証に成功した場合だけ開発版Pre-releaseを公開します。正確なタグ`v0.0.31.0`で同じ検査後にStableを公開します。Draftへアップロード→整合性検証→公開の順で行い、既存タグや公開済みAssetsは上書きしません。
