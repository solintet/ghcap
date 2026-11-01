//! EN: Process-wide cancellation signals. No Git-specific decisions live here.
//! JA: プロセス全体の停止シグナルを管理し、Git固有の判断は行いません。
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, OnceLock,
};

static SHUTDOWN: OnceLock<Arc<AtomicBool>> = OnceLock::new();

pub fn init_signals() -> std::io::Result<()> {
    let flag = SHUTDOWN
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone();
    for signal in [signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT] {
        signal_hook::flag::register(signal, flag.clone())?;
    }
    Ok(())
}
pub fn shutting_down() -> bool {
    SHUTDOWN.get().is_some_and(|f| f.load(Ordering::SeqCst))
}
