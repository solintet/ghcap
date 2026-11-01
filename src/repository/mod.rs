//! EN: Repository state is inspected before an operation plan is created.
//! JA: コマンド実行より前にリポジトリの状態を型付きで調べる。
pub mod inspect;
pub mod planner;
pub mod safety;
pub mod state;
