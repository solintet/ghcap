//! EN: Process subsystem. User output and Git decisions are kept separate.
//! JA: 外部プロセス基盤。表示処理とGit固有の判断は区別します。
mod display;
mod executor;
mod interactive;
mod shell;
mod signal;
mod stream;
#[cfg(test)]
mod tests;
mod types;

pub use display::{failure, safe_text};
pub use executor::{capture, capture_with_wait, live};
pub use interactive::interactive;
pub use shell::run_shell;
pub use signal::{init_signals, shutting_down};
pub use stream::stream;
pub use types::{Output, Termination};
