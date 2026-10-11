//! Captures `log` records per thread. `log` allows one global logger per
//! process and tests run on parallel threads, so each test reads only the
//! lines its own thread logged.

use std::sync::{Mutex, Once};
use std::thread::ThreadId;

struct Capture(Mutex<Vec<(ThreadId, log::Level, String)>>);

impl log::Log for Capture {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Warn
    }
    fn log(&self, r: &log::Record) {
        if self.enabled(r.metadata()) {
            let mut v = self.0.lock().unwrap_or_else(|e| e.into_inner());
            v.push((std::thread::current().id(), r.level(), r.args().to_string()));
        }
    }
    fn flush(&self) {}
}

static CAPTURE: Capture = Capture(Mutex::new(Vec::new()));
static INSTALL: Once = Once::new();

/// Run `f` and return its result with the `Warn` lines this thread logged
/// meanwhile (earlier lines of this thread are dropped first).
pub fn warnings<R>(f: impl FnOnce() -> R) -> (R, Vec<String>) {
    INSTALL.call_once(|| {
        log::set_logger(&CAPTURE).expect("logcap is the only logger in this test binary");
        log::set_max_level(log::LevelFilter::Warn);
    });
    let me = std::thread::current().id();
    CAPTURE
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .retain(|(t, _, _)| *t != me);
    let r = f();
    let lines = CAPTURE
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .filter(|(t, l, _)| *t == me && *l == log::Level::Warn)
        .map(|(_, _, s)| s.clone())
        .collect();
    (r, lines)
}
