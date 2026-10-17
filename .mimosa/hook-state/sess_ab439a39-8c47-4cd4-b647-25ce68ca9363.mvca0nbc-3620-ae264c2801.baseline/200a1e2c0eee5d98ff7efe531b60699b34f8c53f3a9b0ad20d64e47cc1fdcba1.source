//! FTP wire transcript logger.
//!
//! suppaftp and the protocol adapters emit `log::debug` lines describing
//! every command round trip (connect, login, CWD/PWD, listings). This module
//! routes those lines to `~/Library/Logs/com.flowftp.app/ftp.log` so a
//! misbehaving server can be diagnosed from a real transcript instead of
//! guesswork. suppaftp never logs the password itself — it logs "Password is
//! required" — so the file stays credential-free.

use std::fs::{create_dir_all, File, OpenOptions};
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use log::{Level, LevelFilter, Log, Metadata, Record};

struct TranscriptLogger {
    start: Instant,
    file: Mutex<Option<File>>,
}

impl Log for TranscriptLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        matches!(metadata.level(), Level::Debug | Level::Trace)
            && (metadata.target().starts_with("suppaftp")
                || metadata.target().starts_with("flow_protocols")
                || metadata.target().starts_with("flow_ftp"))
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let Ok(mut guard) = self.file.lock() else {
            return;
        };
        let Some(file) = guard.as_mut() else {
            return;
        };
        let elapsed = self.start.elapsed().as_secs_f32();
        let _ = writeln!(
            file,
            "[{:8.3}s] {}: {}",
            elapsed,
            record.target(),
            record.args()
        );
        let _ = file.flush();
    }

    fn flush(&self) {
        if let Ok(mut guard) = self.file.lock() {
            if let Some(file) = guard.as_mut() {
                let _ = file.flush();
            }
        }
    }
}

/// Install the transcript logger. The file starts empty on every launch, so
/// it always reflects the latest session. Failures are silent: logging must
/// never keep the app from starting.
pub fn install() {
    let file = log_path().and_then(|path| {
        create_dir_all(path.parent()?).ok()?;
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .ok()
    });
    let logger = TranscriptLogger {
        start: Instant::now(),
        file: Mutex::new(file),
    };
    let unix_now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if log::set_boxed_logger(Box::new(logger)).is_ok() {
        log::set_max_level(LevelFilter::Debug);
        log::debug!("session start (unix {unix_now})");
    }
}

fn log_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library")
            .join("Logs")
            .join("com.flowftp.app")
            .join("ftp.log"),
    )
}
