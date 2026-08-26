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
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread::{self, JoinHandle};
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

#[derive(Debug, Deserialize, Serialize)]
struct LockRecord {
    pid: u32,
    started_at: i64,
    #[serde(default)]
    starttime_ticks: u64,
    #[serde(default)]
    executable_device: u64,
    #[serde(default)]
    executable_inode: u64,
    #[serde(default)]
    owner_nonce: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProcessIdentity {
    pid: u32,
    starttime_ticks: u64,
    executable_device: u64,
    executable_inode: u64,
}

#[cfg(unix)]
fn process_identity(pid: u32) -> Option<ProcessIdentity> {
    use std::os::unix::fs::MetadataExt;

    if pid == 0 {
        return None;
    }
    let process_root = Path::new("/proc").join(pid.to_string());
    // `/proc/<pid>/stat` encloses comm in parentheses and comm may contain
    // spaces. Split only after the final `) `; starttime is field 22, hence
    // index 19 in the remaining field-3-based slice.
    let stat = fs::read_to_string(process_root.join("stat")).ok()?;
    let (_, fields) = stat.rsplit_once(") ")?;
    let starttime_ticks = fields.split_whitespace().nth(19)?.parse().ok()?;
    let executable = fs::metadata(process_root.join("exe")).ok()?;
    Some(ProcessIdentity {
        pid,
        starttime_ticks,
        executable_device: executable.dev(),
        executable_inode: executable.ino(),
    })
}

#[cfg(not(unix))]
fn process_identity(pid: u32) -> Option<ProcessIdentity> {
    (pid == std::process::id()).then_some(ProcessIdentity {
        pid,
        starttime_ticks: 0,
        executable_device: 0,
        executable_inode: 0,
    })
}

#[cfg(unix)]
fn owner_nonce() -> Result<String, DaemonError> {
    let mut bytes = [0_u8; 16];
    File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(DaemonError::Lock)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(not(unix))]
fn owner_nonce() -> Result<String, DaemonError> {
    // The recorder is a Linux/WSL service. Keep non-Unix builds compilable,
    // while never treating this fallback as a cross-process authority.
    Ok(format!(
        "{:016x}{:016x}",
        std::process::id(),
        unix_now().unsigned_abs()
    ))
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

fn lock_owner_is_current(record: &LockRecord) -> bool {
    let Some(identity) = process_identity(record.pid) else {
        return false;
    };
    // Legacy lock records do not carry enough identity to prove ownership.
    // They may be reclaimed only when their PID is dead; a live legacy PID is
    // retained fail-closed so an unrelated process is never disturbed.
    if record.starttime_ticks == 0
        || record.executable_device == 0
        || record.executable_inode == 0
        || record.owner_nonce.len() != 32
    {
        return true;
    }
    identity.starttime_ticks == record.starttime_ticks
        && identity.executable_device == record.executable_device
        && identity.executable_inode == record.executable_inode
}

fn current_lock_owner_pid_at(path: &Path) -> Option<u32> {
    let path_metadata = fs::symlink_metadata(path).ok()?;
    if path_metadata.file_type().is_symlink()
        || !path_metadata.is_file()
        || path_metadata.len() > MAX_LOCK_BYTES
    {
        return None;
    }
    let mut file = File::open(path).ok()?;
    let file_metadata = file.metadata().ok()?;
    if !lock_is_same_file(&path_metadata, &file_metadata) {
        return None;
    }
    let mut bytes = Vec::with_capacity(path_metadata.len() as usize);
    file.read_to_end(&mut bytes).ok()?;
    if bytes.len() > MAX_LOCK_BYTES as usize {
        return None;
    }
    let current_path_metadata = fs::symlink_metadata(path).ok()?;
    if current_path_metadata.file_type().is_symlink()
        || !lock_is_same_file(&current_path_metadata, &file_metadata)
    {
        return None;
    }
    let record = serde_json::from_slice::<LockRecord>(&bytes).ok()?;
    lock_owner_is_current(&record).then_some(record.pid)
}

/// Return only the PID from a complete, current recorder lock identity.
/// Callers use this to distinguish the service child they own from a
/// concurrently-started winner; malformed, stale, or replaced locks are not
/// treated as authority.
pub(crate) fn current_daemon_owner_pid() -> Option<u32> {
    current_lock_owner_pid_at(&daemon_lock_path()?)
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
        return Ok(!lock_owner_is_current(&record));
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
                    let process = process_identity(std::process::id()).ok_or_else(|| {
                        DaemonError::Lock(std::io::Error::other(
                            "current process identity is unavailable",
                        ))
                    })?;
                    let record = LockRecord {
                        pid: process.pid,
                        started_at: unix_now(),
                        starttime_ticks: process.starttime_ticks,
                        executable_device: process.executable_device,
                        executable_inode: process.executable_inode,
                        owner_nonce: owner_nonce()?,
                    };
                    let record = serde_json::to_vec(&record).map_err(|_| {
                        DaemonError::Lock(std::io::Error::other(
                            "lock identity serialization failed",
                        ))
                    })?;
                    let result = (|| -> Result<fs::Metadata, DaemonError> {
                        file.write_all(&record).map_err(DaemonError::Lock)?;
                        file.write_all(b"\n").map_err(DaemonError::Lock)?;
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
    let mut store = UsageStore::open(database).map_err(|_| {
        crate::debug_runtime("recorder history store open failed");
        DaemonError::Store
    })?;
    store.upsert_samples(&rows).map_err(|error| {
        crate::debug_runtime(format!("recorder history batch commit failed: {error}"));
        DaemonError::Store
    })?;
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

/// Recorder ownership embedded in the combined daemon+REST service.
///
/// Unlike `run_record_daemon`, this worker does not install a second signal
/// handler. The service process owns SIGINT/SIGTERM and stops this bounded
/// worker before releasing the REST listener.
pub(crate) struct RecorderWorker {
    shutdown: Option<mpsc::Sender<()>>,
    worker: Option<JoinHandle<()>>,
    active: bool,
}

impl RecorderWorker {
    pub(crate) fn start() -> Result<Self, String> {
        let (shutdown, shutdown_receiver) = mpsc::channel();
        let (started, started_receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("codex-info-recorder".into())
            .spawn(move || {
                let result = (|| -> Result<
                    (PathBuf, Option<DaemonLock>, Option<InputFingerprint>),
                    DaemonError,
                > {
                    let (_history, database, lock_path) = data_paths()?;
                    let lock = DaemonLock::acquire(lock_path)?;
                    Ok((database, lock, None::<InputFingerprint>))
                })();
                let (database, lock, mut previous) = match result {
                    Ok(values) => values,
                    Err(error) => {
                        let _ = started.send(Err(error.to_string()));
                        return;
                    }
                };
                let Some(_lock) = lock else {
                    // An already-running owner is not an error and must never
                    // be killed or replaced by an X-only/combined launch.
                    let _ = started.send(Ok(false));
                    return;
                };

                // Signal ownership before the first bounded scan.  The scan
                // can legitimately take longer than the service readiness
                // window when a large session history is present (especially
                // immediately after WSL boot).  Readiness means the lock and
                // worker are alive; it must not depend on the first backfill
                // finishing within two seconds.
                if started.send(Ok(true)).is_err() {
                    return;
                }

                match run_cycle(&database, &mut previous) {
                    Ok(rows) if rows > 0 => {
                        eprintln!("codex-info: recorder committed {rows} samples")
                    }
                    Ok(_) => {}
                    Err(_) => {
                        previous = None;
                        eprintln!("codex-info: recorder skipped an unsafe input cycle");
                    }
                }
                let interval = daemon_interval_from_environment();
                loop {
                    match shutdown_receiver.recv_timeout(interval) {
                        Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                        Err(RecvTimeoutError::Timeout) => {
                            match run_cycle(&database, &mut previous) {
                                Ok(rows) if rows > 0 => {
                                    eprintln!("codex-info: recorder committed {rows} samples")
                                }
                                Ok(_) => {}
                                Err(_) => {
                                    previous = None;
                                    eprintln!("codex-info: recorder skipped an unsafe input cycle");
                                }
                            }
                        }
                    }
                }
            })
            .map_err(|_| DaemonError::Runtime.to_string())?;

        match started_receiver.recv_timeout(Duration::from_secs(2)) {
            Ok(Ok(active)) => Ok(Self {
                shutdown: Some(shutdown),
                worker: Some(worker),
                active,
            }),
            Ok(Err(error)) => {
                let _ = shutdown.send(());
                let _ = worker.join();
                Err(error)
            }
            Err(_) => {
                let _ = shutdown.send(());
                let _ = worker.join();
                Err(DaemonError::Runtime.to_string())
            }
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) fn shutdown(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for RecorderWorker {
    fn drop(&mut self) {
        self.shutdown();
    }
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
        assert_eq!(current_lock_owner_pid_at(&path), Some(std::process::id()));
        assert!(DaemonLock::acquire(path.clone()).unwrap().is_none());
        drop(first);
        assert_eq!(current_lock_owner_pid_at(&path), None);
        assert!(!path.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn pid_reuse_identity_mismatch_is_reclaimed_without_touching_live_owner() {
        let root = temp_root("pid-reuse");
        let path = root.join(DAEMON_LOCK_FILE_NAME);
        let current = process_identity(std::process::id()).unwrap();
        let reused = LockRecord {
            pid: current.pid,
            started_at: unix_now(),
            starttime_ticks: current.starttime_ticks.saturating_add(1),
            executable_device: current.executable_device,
            executable_inode: current.executable_inode,
            owner_nonce: "00".repeat(16),
        };
        fs::write(&path, serde_json::to_vec(&reused).unwrap()).unwrap();

        let acquired = DaemonLock::acquire(path.clone()).unwrap().unwrap();
        let stored: LockRecord = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(stored.pid, current.pid);
        assert_eq!(stored.starttime_ticks, current.starttime_ticks);
        assert_eq!(stored.executable_device, current.executable_device);
        assert_eq!(stored.executable_inode, current.executable_inode);
        assert_eq!(stored.owner_nonce.len(), 32);
        drop(acquired);
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
