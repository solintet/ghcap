//! EN: Explicit process termination outcomes and captured result fields.
//! JA: 外部プロセスの終了理由と出力を型付きで区別します。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Termination {
    Exited(i32),
    Signal,
    Cancelled,
    TimedOut,
    OutputLimit,
    ReadFailed,
}
#[derive(Debug)]
pub struct Output {
    pub success: bool,
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    pub termination: Termination,
}
