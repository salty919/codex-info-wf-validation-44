// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

//! The independent local-session recorder.
//!
//! The recorder intentionally owns no UI or app-server state.  It reads the
//! same bounded JSONL collector used by the native client and commits only
//! through [`UsageStore`].  A short-lived process lock prevents multiple
//! recorders from continuously scanning the same input, while SQLite's own
//! transaction/upsert contract remains the authority for concurrent writers.

use crate::security;
use crate::usage_store::UsageStore;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(crate) const RESET_HINT_FILE_NAME: &str = "usage_reset_hint.json";
pub(crate) const DAEMON_LOCK_FILE_NAME: &str = "usage_record_daemon.lock";
pub(crate) const DEFAULT_INTERVAL: Duration = Duration::from_secs(60);
pub(crate) const MIN_INTERVAL: Duration = Duration::from_secs(5);
pub(crate) const MAX_INTERVAL: Duration = Duration::from_secs(60 * 60);

const MAX_HINT_BYTES: u64 = 4 * 1024;
const MAX_LOCK_BYTES: u64 = 4 * 1024;
const STALE_LOCK_AGE: Duration = Duration::from_secs(24 * 60 * 60);
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ResetHint {
    pub(crate) reset_at: i64,
    pub(crate) window_seconds: i64,
}

impl ResetHint {
    fn new(reset_at: i64, window_seconds: i64) -> Option<Self> {
        (reset_at > 0 && (1..=366 * 86_400).contains(&window_seconds)).then_some(Self {
            reset_at,
            window_seconds,
        })
    }

    fn is_valid(self) -> bool {
        self.reset_at > 0 && (1..=366 * 86_400).contains(&self.window_seconds)
    }
}

/// Resolve the metadata location from the same protected data root as the
/// history database.  The path is intentionally not configurable separately:
/// a daemon must never read a hint from a different account/data directory.
pub(crate) fn reset_hint_path() -> Option<PathBuf> {
    crate::usage_data_root().map(|root| root.join("history").join(RESET_HINT_FILE_NAME))
}

pub(crate) fn daemon_lock_path() -> Option<PathBuf> {
    crate::usage_data_root().map(|root| root.join("history").join(DAEMON_LOCK_FILE_NAME))
}

/// Read a bounded, private reset hint.  Any malformed, replaced, symlinked,
/// or oversized metadata is ignored; the next authenticated quota response
/// can safely replace it.
pub(crate) fn load_reset_hint() -> Option<(i64, i64)> {
    let path = reset_hint_path()?;
    let metadata = fs::symlink_metadata(&path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > MAX_HINT_BYTES {
        return None;
    }
    let mut file = File::open(path).ok()?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut bytes).ok()?;
    let hint = serde_json::from_slice::<ResetHint>(&bytes).ok()?;
    hint.is_valid()
        .then_some((hint.reset_at, hint.window_seconds))
}

/// Atomically replace the reset hint after a successful quota response.
/// Existing metadata is not opened for writing, and a failed temporary write
/// leaves the previous hint untouched.
pub(crate) fn persist_reset_hint(reset_at: i64, window_seconds: i64) -> Result<(), ()> {
    let hint = ResetHint::new(reset_at, window_seconds).ok_or(())?;
    let path = reset_hint_path().ok_or(())?;
    let parent = path.parent().ok_or(())?;
    fs::create_dir_all(parent).map_err(|_| ())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).map_err(|_| ())?;
    }

    // Refuse to replace a symlink.  A regular target is replaced with rename,
    // which is atomic within this directory and never follows the old path.
    if let Ok(metadata) = fs::symlink_metadata(&path) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(());
        }
    }

    let bytes = serde_json::to_vec(&hint).map_err(|_| ())?;
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".{RESET_HINT_FILE_NAME}.tmp-{}-{counter}",
        std::process::id()
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary).map_err(|_| ())?;
    let result = (|| {
        file.write_all(&bytes).map_err(|_| ())?;
        file.sync_all().map_err(|_| ())?;
        drop(file);
        fs::rename(&temporary, &path).map_err(|_| ())?;
        #[cfg(unix)]
        if let Ok(directory) = File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub(crate) fn daemon_interval_from_environment() -> Duration {
    let raw = std::env::var("CODEX_INFO_DAEMON_INTERVAL_SECS")
        .ok()
        .or_else(|| std::env::var("CODEX_INFO_RECORD_INTERVAL_SECS").ok());
    let seconds = raw
        .as_deref()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_INTERVAL);
    seconds.clamp(MIN_INTERVAL, MAX_INTERVAL)
}

#[derive(Debug)]
enum DaemonError {
    DataRoot,
    Lock(std::io::Error),
    Input,
    Store,
    Runtime,
}

impl std::fmt::Display for DaemonError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::DataRoot => "daemon data directory is unavailable",
            Self::Lock(error) => {
                let _ = error.kind();
                "daemon lock operation failed"
            }
            Self::Input => "daemon input scan failed",
            Self::Store => "daemon history commit failed",
            Self::Runtime => "daemon runtime could not start",
        })
    }
}

impl std::error::Error for DaemonError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileFingerprint {
    path: PathBuf,
    length: u64,
    modified_nanos: u128,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct InputFingerprint {
    hint: Option<ResetHint>,
    files: Vec<FileFingerprint>,
    recovery: Option<FileFingerprint>,
}

fn modified_nanos(metadata: &fs::Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |value| value.as_nanos())
}

fn fingerprint_file(path: &Path) -> Result<FileFingerprint, DaemonError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| DaemonError::Input)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > security::MAX_SESSION_FILE_BYTES
    {
        return Err(DaemonError::Input);
    }
    #[cfg(unix)]
    let (device, inode) = {
        use std::os::unix::fs::MetadataExt;
        (metadata.dev(), metadata.ino())
    };
    Ok(FileFingerprint {
        path: path.to_owned(),
        length: metadata.len(),
        modified_nanos: modified_nanos(&metadata),
        #[cfg(unix)]
        device,
        #[cfg(unix)]
        inode,
    })
}

fn input_fingerprint(hint: Option<ResetHint>) -> Result<InputFingerprint, DaemonError> {
    let mut files = Vec::new();
    if let Some(root) = crate::local_sessions_root() {
        match fs::symlink_metadata(&root) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(DaemonError::Input)
            }
            Ok(_) => {
                let paths = crate::session_jsonl_files(&root).map_err(|_| DaemonError::Input)?;
                for path in paths {
                    files.push(fingerprint_file(&path)?);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(DaemonError::Input),
        }
    }

    let recovery = if let Some(path) = crate::delegation_usage_recovery_path() {
        match fs::symlink_metadata(&path) {
            Ok(metadata)
                if metadata.file_type().is_symlink()
                    || !metadata.is_file()
                    || metadata.len() > security::MAX_SESSION_FILE_BYTES =>
            {
                // The collector treats an absent recovery file as empty, but
                // an unsafe replacement must remain fail-closed.
                return Err(DaemonError::Input);
            }
            Ok(_) => Some(fingerprint_file(&path)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err(DaemonError::Input),
        }
    } else {
        None
    };

    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(InputFingerprint {
        hint,
        files,
        recovery,
    })
}

#[derive(Deserialize)]
struct LockRecord {
    pid: u32,
    started_at: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LockIdentity {
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(not(unix))]
    length: u64,
    #[cfg(not(unix))]
    modified_nanos: u128,
}

fn lock_identity_from_metadata(metadata: &fs::Metadata) -> LockIdentity {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        LockIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }
    #[cfg(not(unix))]
    {
        LockIdentity {
            length: metadata.len(),
            modified_nanos: modified_nanos(metadata),
        }
    }
}

fn lock_is_same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    lock_identity_from_metadata(left) == lock_identity_from_metadata(right)
}

#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    // Unit tests acquire a lock in-process without the daemon command-line
    // marker. Keep that owner live, while rejecting a stale lock whose PID
    // has been reused by an unrelated process. A systemd restart can then
    // reclaim the old file without ever deleting another process's lock.
    if pid == std::process::id() {
        return true;
    }
    let process_root = Path::new("/proc").join(pid.to_string());
    if !process_root.is_dir() {
        return false;
    }
    let Ok(command_line) = fs::read(process_root.join("cmdline")) else {
        return false;
    };
    command_line
        .split(|byte| *byte == 0)
        .filter(|argument| !argument.is_empty())
        .any(|argument| argument == b"--record-daemon")
}

#[cfg(not(unix))]
fn process_is_alive(pid: u32) -> bool {
    pid == std::process::id()
}

fn lock_is_stale(path: &Path) -> Result<bool, DaemonError> {
    let metadata = fs::symlink_metadata(path).map_err(DaemonError::Lock)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Ok(false);
    }
    if metadata.len() > MAX_LOCK_BYTES {
        return Ok(false);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)
        .map_err(DaemonError::Lock)?
        .read_to_end(&mut bytes)
        .map_err(DaemonError::Lock)?;
    if let Ok(record) = serde_json::from_slice::<LockRecord>(&bytes) {
        let _ = record.started_at;
        return Ok(!process_is_alive(record.pid));
    }
    let old_enough = metadata
        .modified()
        .ok()
        .and_then(|value| SystemTime::now().duration_since(value).ok())
        .is_some_and(|age| age >= STALE_LOCK_AGE);
    Ok(old_enough)
}

struct DaemonLock {
    path: PathBuf,
    file: File,
    identity: LockIdentity,
}

impl DaemonLock {
    fn acquire(path: PathBuf) -> Result<Option<Self>, DaemonError> {
        let parent = path.parent().ok_or(DaemonError::DataRoot)?;
        fs::create_dir_all(parent).map_err(DaemonError::Lock)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
                .map_err(DaemonError::Lock)?;
        }

        for attempt in 0..2 {
            let mut options = OpenOptions::new();
            options.write(true).read(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&path) {
                Ok(mut file) => {
                    let record = format!(
                        "{{\"pid\":{},\"started_at\":{}}}\n",
                        std::process::id(),
                        unix_now()
                    );
                    let result = (|| -> Result<fs::Metadata, DaemonError> {
                        file.write_all(record.as_bytes())
                            .map_err(DaemonError::Lock)?;
                        file.sync_all().map_err(DaemonError::Lock)?;
                        file.metadata().map_err(DaemonError::Lock)
                    })();
                    match result {
                        Ok(metadata) => {
                            return Ok(Some(Self {
                                path,
                                file,
                                identity: lock_identity_from_metadata(&metadata),
                            }));
                        }
                        Err(error) => {
                            // The lock was newly created by this attempt. If
                            // writing it fails, clean up only when the path
                            // still names this exact inode; never remove a
                            // racing replacement.
                            if let (Ok(path_metadata), Ok(file_metadata)) =
                                (fs::symlink_metadata(&path), file.metadata())
                            {
                                if lock_is_same_file(&path_metadata, &file_metadata) {
                                    let _ = fs::remove_file(&path);
                                }
                            }
                            return Err(error);
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if attempt == 1 {
                        return Ok(None);
                    }
                    if !lock_is_stale(&path)? {
                        return Ok(None);
                    }
                    let before = fs::symlink_metadata(&path).map_err(DaemonError::Lock)?;
                    if before.file_type().is_symlink() {
                        return Ok(None);
                    }
                    // Only remove the exact stale file we inspected.  A
                    // racing new owner leaves a different inode and is not
                    // disturbed.
                    let current = fs::symlink_metadata(&path).map_err(DaemonError::Lock)?;
                    if !lock_is_same_file(&before, &current) {
                        return Ok(None);
                    }
                    match fs::remove_file(&path) {
                        Ok(()) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => return Err(DaemonError::Lock(error)),
                    }
                }
                Err(error) => return Err(DaemonError::Lock(error)),
            }
        }
        Ok(None)
    }
}

impl Drop for DaemonLock {
    fn drop(&mut self) {
        let Ok(path_metadata) = fs::symlink_metadata(&self.path) else {
            return;
        };
        let Ok(file_metadata) = self.file.metadata() else {
            return;
        };
        if path_metadata.file_type().is_symlink()
            || !path_metadata.is_file()
            || lock_identity_from_metadata(&path_metadata) != self.identity
            || lock_identity_from_metadata(&file_metadata) != self.identity
        {
            return;
        }
        let _ = fs::remove_file(&self.path);
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|value| i64::try_from(value.as_secs()).ok())
        .unwrap_or(0)
}

fn data_paths() -> Result<(PathBuf, PathBuf, PathBuf), DaemonError> {
    let root = crate::usage_data_root().ok_or(DaemonError::DataRoot)?;
    let history = root.join("history");
    let database = history.join("usage_history.sqlite3");
    let lock = daemon_lock_path().ok_or(DaemonError::DataRoot)?;
    Ok((history, database, lock))
}

fn scan_and_store(database: &Path, hint: ResetHint) -> Result<usize, DaemonError> {
    let samples = crate::collect_local_model_usage_timeline(hint.reset_at, hint.window_seconds)
        .map_err(|_| DaemonError::Input)?;
    if samples.is_empty() {
        return Ok(0);
    }
    let rows = samples
        .iter()
        .map(crate::UsageHistorySample::to_store)
        .collect::<Vec<_>>();
    let mut store = UsageStore::open(database).map_err(|_| DaemonError::Store)?;
    store
        .upsert_samples(&rows)
        .map_err(|_| DaemonError::Store)?;
    Ok(rows.len())
}

fn run_cycle(
    database: &Path,
    previous: &mut Option<InputFingerprint>,
) -> Result<usize, DaemonError> {
    let hint = load_reset_hint().map(|(reset_at, window_seconds)| ResetHint {
        reset_at,
        window_seconds,
    });
    let fingerprint = input_fingerprint(hint)?;
    if previous.as_ref() == Some(&fingerprint) {
        return Ok(0);
    }
    if fingerprint.files.is_empty() && fingerprint.recovery.is_none() {
        *previous = Some(fingerprint);
        return Ok(0);
    }
    let Some(hint) = hint else {
        // No quota boundary exists yet.  Remember the empty-input snapshot so
        // the daemon does not repeatedly traverse the directory before the
        // first successful account response writes its reset hint.
        *previous = Some(fingerprint);
        return Ok(0);
    };
    let rows = scan_and_store(database, hint)?;
    *previous = Some(fingerprint);
    Ok(rows)
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let ctrl_c = tokio::signal::ctrl_c();
        match signal(SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! {
                    _ = ctrl_c => {}
                    _ = terminate.recv() => {}
                }
            }
            Err(_) => {
                let _ = ctrl_c.await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

async fn run_daemon(once: bool) -> Result<(), DaemonError> {
    let (_history, database, lock_path) = data_paths()?;
    let Some(_lock) = DaemonLock::acquire(lock_path)? else {
        eprintln!("codex-info: recorder daemon is already running");
        return Ok(());
    };
    let mut previous = None;
    match run_cycle(&database, &mut previous) {
        Ok(rows) => {
            if rows > 0 {
                eprintln!("codex-info: recorder daemon committed {rows} samples");
            }
        }
        Err(_) => eprintln!("codex-info: recorder daemon skipped an unsafe input cycle"),
    }
    if once {
        return Ok(());
    }

    let interval = daemon_interval_from_environment();
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    // `interval` emits an immediate first tick.  The startup cycle above is
    // that first tick, so consume it before entering the steady-state loop.
    ticker.tick().await;
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            _ = ticker.tick() => {
                match run_cycle(&database, &mut previous) {
                    Ok(rows) if rows > 0 => {
                        eprintln!("codex-info: recorder daemon committed {rows} samples");
                    }
                    Ok(_) => {}
                    Err(_) => {
                        // Do not retain a failed fingerprint.  A later bounded
                        // interval can retry after a transient replacement or
                        // SQLite busy/IO failure without spinning.
                        previous = None;
                        eprintln!("codex-info: recorder daemon skipped an unsafe input cycle");
                    }
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn run_record_daemon(once: bool) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .map_err(|_| DaemonError::Runtime.to_string())?;
    runtime
        .block_on(run_daemon(once))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn temp_root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("codex-info-daemon-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn interval_is_bounded_even_for_invalid_environment_values() {
        let _guard = ENV_LOCK.lock().unwrap();
        let old = std::env::var_os("CODEX_INFO_DAEMON_INTERVAL_SECS");
        std::env::set_var("CODEX_INFO_DAEMON_INTERVAL_SECS", "1");
        assert_eq!(daemon_interval_from_environment(), MIN_INTERVAL);
        std::env::set_var("CODEX_INFO_DAEMON_INTERVAL_SECS", "999999");
        assert_eq!(daemon_interval_from_environment(), MAX_INTERVAL);
        std::env::set_var("CODEX_INFO_DAEMON_INTERVAL_SECS", "not-a-number");
        assert_eq!(daemon_interval_from_environment(), DEFAULT_INTERVAL);
        match old {
            Some(value) => std::env::set_var("CODEX_INFO_DAEMON_INTERVAL_SECS", value),
            None => std::env::remove_var("CODEX_INFO_DAEMON_INTERVAL_SECS"),
        }
    }

    #[test]
    fn reset_hint_round_trip_is_atomic_and_private() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = temp_root("hint");
        let old = std::env::var_os("CODEX_INFO_DATA_DIR");
        std::env::set_var("CODEX_INFO_DATA_DIR", &root);
        assert_eq!(load_reset_hint(), None);
        persist_reset_hint(1_800_000_000, 604_800).unwrap();
        assert_eq!(load_reset_hint(), Some((1_800_000_000, 604_800)));
        #[cfg(unix)]
        {
            let metadata = fs::metadata(root.join("history").join(RESET_HINT_FILE_NAME)).unwrap();
            assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        }
        match old {
            Some(value) => std::env::set_var("CODEX_INFO_DATA_DIR", value),
            None => std::env::remove_var("CODEX_INFO_DATA_DIR"),
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn stale_pid_lock_is_reclaimed_and_live_lock_is_singleton() {
        let root = temp_root("lock");
        let path = root.join(DAEMON_LOCK_FILE_NAME);
        fs::write(&path, b"{\"pid\":4294967294,\"started_at\":1}\n").unwrap();
        let first = DaemonLock::acquire(path.clone()).unwrap().unwrap();
        assert!(DaemonLock::acquire(path.clone()).unwrap().is_none());
        drop(first);
        assert!(!path.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn daemon_cycle_persists_changed_jsonl_into_history_store() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = temp_root("cycle");
        let codex_home = root.join("codex");
        let sessions = codex_home
            .join("sessions")
            .join("2026")
            .join("08")
            .join("22");
        fs::create_dir_all(&sessions).unwrap();
        let now = unix_now();
        let reset_at = now + 3_600;
        let session = sessions.join("daemon-cycle.jsonl");
        let context = serde_json::json!({
            "timestamp": chrono::DateTime::<chrono::Utc>::from_timestamp(now, 0).unwrap().to_rfc3339(),
            "type": "turn_context",
            "model": "gpt-5.6-luna"
        });
        let tokens = serde_json::json!({
            "timestamp": chrono::DateTime::<chrono::Utc>::from_timestamp(now, 0).unwrap().to_rfc3339(),
            "type": "token_count",
            "payload": {"info": {"total_token_usage": {
                "total_tokens": 120, "input_tokens": 100,
                "cached_input_tokens": 80, "output_tokens": 20
            }}}
        });
        fs::write(&session, format!("{}\n{}\n", context, tokens)).unwrap();
        let data_dir = root.join("data");
        let old_home = std::env::var_os("CODEX_HOME");
        let old_data = std::env::var_os("CODEX_INFO_DATA_DIR");
        std::env::set_var("CODEX_HOME", &codex_home);
        std::env::set_var("CODEX_INFO_DATA_DIR", &data_dir);
        assert_eq!(
            crate::session_jsonl_files(&codex_home.join("sessions"))
                .unwrap()
                .len(),
            1
        );
        let values = fs::read_to_string(&session).unwrap();
        let parsed = values
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            crate::session_event_model(&parsed[0]).as_deref(),
            Some("gpt-5.6-luna")
        );
        assert_eq!(
            crate::session_token_snapshot(&parsed[1]).unwrap().total,
            120
        );
        persist_reset_hint(reset_at, 604_800).unwrap();
        let direct = crate::collect_local_model_usage_timeline(reset_at, 604_800).unwrap();
        assert_eq!(direct.len(), 1, "direct timeline should admit the fixture");
        let (_history, database, _lock) = data_paths().unwrap();
        let mut previous = None;
        let committed = run_cycle(&database, &mut previous).unwrap();
        assert_eq!(committed, 1);
        let samples = UsageStore::open(database).unwrap().load_all().unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].luna_tokens, 120);
        match old_home {
            Some(value) => std::env::set_var("CODEX_HOME", value),
            None => std::env::remove_var("CODEX_HOME"),
        }
        match old_data {
            Some(value) => std::env::set_var("CODEX_INFO_DATA_DIR", value),
            None => std::env::remove_var("CODEX_INFO_DATA_DIR"),
        }
        let _ = fs::remove_dir_all(root);
    }
}
