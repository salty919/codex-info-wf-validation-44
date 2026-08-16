// Copyright (C) 2026 salty919
// SPDX-License-Identifier: GPL-3.0-only

//! Read-only native sub-agent inventory supplement.
//!
//! A fresh Codex app-server does not expose native sub-agents owned by another
//! in-process collaboration runtime.  This module uses the state database only
//! to recover the bounded parent/child graph and rollout paths.  Liveness is
//! deliberately left to the validated rollout parser.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::limits::Limit;
use rusqlite::{params, params_from_iter, Connection, OpenFlags};

use crate::security;

pub const MAX_NATIVE_THREAD_DEPTH: usize = 64;
pub const MAX_NATIVE_THREAD_DESCENDANTS: usize = 1_024;
const QUERY_PARENT_BATCH: usize = 128;
const MAX_STATE_DB_VALUE_BYTES: i32 = 1024 * 1024;
const MAX_ROLLOUT_PATH_BYTES: usize = 16 * 1024;
const STATE_DB_BUSY_TIMEOUT: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadStateError {
    UnsafePath,
    Database,
    InvalidSchema,
    InvalidRow,
    LimitExceeded,
    Cycle,
    Replaced,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeThreadCandidate {
    pub id: String,
    pub created_at: Option<i64>,
    pub updated_at: i64,
    pub title: String,
    pub rollout_path: PathBuf,
    pub parent_thread_id: String,
    pub depth: i32,
}

#[derive(Clone, Debug)]
struct NativeThreadRow {
    id: String,
    created_at: Option<i64>,
    updated_at: i64,
    title: String,
    rollout_path: PathBuf,
    parent_thread_id: String,
}

#[derive(Clone, Debug)]
struct FrontierEntry {
    id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ColumnContract {
    declared_type: String,
    not_null: bool,
    primary_key: bool,
}

fn valid_thread_id(value: &str) -> bool {
    let count = value.chars().count();
    (1..=128).contains(&count) && !value.chars().any(char::is_control)
}

fn path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value: OsString = path.as_os_str().to_owned();
    value.push(suffix);
    PathBuf::from(value)
}

fn validate_optional_sidecars(codex_root: &Path, database: &Path) -> Result<(), ThreadStateError> {
    for suffix in ["-journal", "-shm", "-wal"] {
        let sidecar = path_with_suffix(database, suffix);
        match fs::symlink_metadata(&sidecar) {
            Ok(_) => {
                security::canonical_regular_file_under(codex_root, &sidecar)
                    .map_err(|_| ThreadStateError::UnsafePath)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(ThreadStateError::UnsafePath),
        }
    }
    Ok(())
}

#[cfg(unix)]
fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(not(unix))]
fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.is_file() && right.is_file() && left.len() == right.len()
}

fn open_state_database(
    codex_root: &Path,
) -> Result<(Connection, PathBuf, fs::Metadata), ThreadStateError> {
    let canonical_root =
        security::validate_absolute_root(codex_root).map_err(|_| ThreadStateError::UnsafePath)?;
    let configured = canonical_root.join("state_5.sqlite");
    let canonical = security::canonical_regular_file_under(&canonical_root, &configured)
        .map_err(|_| ThreadStateError::UnsafePath)?;
    if canonical.parent() != Some(canonical_root.as_path())
        || canonical.file_name().and_then(|name| name.to_str()) != Some("state_5.sqlite")
    {
        return Err(ThreadStateError::UnsafePath);
    }
    validate_optional_sidecars(&canonical_root, &canonical)?;
    let before = fs::symlink_metadata(&canonical).map_err(|_| ThreadStateError::UnsafePath)?;
    if before.file_type().is_symlink() || !before.is_file() {
        return Err(ThreadStateError::UnsafePath);
    }

    let connection = Connection::open_with_flags(
        &canonical,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| ThreadStateError::Database)?;
    connection
        .busy_timeout(STATE_DB_BUSY_TIMEOUT)
        .map_err(|_| ThreadStateError::Database)?;
    connection
        .pragma_update(None, "query_only", true)
        .map_err(|_| ThreadStateError::Database)?;
    connection
        .pragma_update(None, "trusted_schema", false)
        .map_err(|_| ThreadStateError::Database)?;
    connection.set_limit(Limit::SQLITE_LIMIT_LENGTH, MAX_STATE_DB_VALUE_BYTES);
    connection.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0);
    connection.set_limit(Limit::SQLITE_LIMIT_WORKER_THREADS, 0);
    Ok((connection, canonical, before))
}

fn table_columns(
    connection: &Connection,
    table: &str,
) -> Result<BTreeMap<String, ColumnContract>, ThreadStateError> {
    let object_type: String = connection
        .query_row(
            "SELECT type FROM sqlite_schema WHERE name = ?1",
            params![table],
            |row| row.get(0),
        )
        .map_err(|_| ThreadStateError::InvalidSchema)?;
    if object_type != "table" {
        return Err(ThreadStateError::InvalidSchema);
    }
    let sql = match table {
        "threads" => "PRAGMA table_info('threads')",
        "thread_spawn_edges" => "PRAGMA table_info('thread_spawn_edges')",
        _ => return Err(ThreadStateError::InvalidSchema),
    };
    let mut statement = connection
        .prepare(sql)
        .map_err(|_| ThreadStateError::InvalidSchema)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                ColumnContract {
                    declared_type: row.get::<_, String>(2)?.to_ascii_uppercase(),
                    not_null: row.get::<_, i64>(3)? == 1,
                    primary_key: row.get::<_, i64>(5)? > 0,
                },
            ))
        })
        .map_err(|_| ThreadStateError::InvalidSchema)?;
    let mut columns = BTreeMap::new();
    for row in rows {
        let (name, contract) = row.map_err(|_| ThreadStateError::InvalidSchema)?;
        if columns.insert(name, contract).is_some() {
            return Err(ThreadStateError::InvalidSchema);
        }
    }
    Ok(columns)
}

fn require_column(
    columns: &BTreeMap<String, ColumnContract>,
    name: &str,
    declared_type: &str,
    not_null: bool,
    primary_key: bool,
) -> Result<(), ThreadStateError> {
    let Some(column) = columns.get(name) else {
        return Err(ThreadStateError::InvalidSchema);
    };
    if column.declared_type != declared_type
        || column.not_null != not_null
        || column.primary_key != primary_key
    {
        return Err(ThreadStateError::InvalidSchema);
    }
    Ok(())
}

fn optional_text_column(
    columns: &BTreeMap<String, ColumnContract>,
    name: &str,
) -> Result<bool, ThreadStateError> {
    let Some(column) = columns.get(name) else {
        return Ok(false);
    };
    if column.declared_type != "TEXT" || column.not_null || column.primary_key {
        return Err(ThreadStateError::InvalidSchema);
    }
    Ok(true)
}

fn optional_integer_column(
    columns: &BTreeMap<String, ColumnContract>,
    name: &str,
) -> Result<bool, ThreadStateError> {
    let Some(column) = columns.get(name) else {
        return Ok(false);
    };
    if column.declared_type != "INTEGER" || column.primary_key {
        return Err(ThreadStateError::InvalidSchema);
    }
    Ok(true)
}

fn task_title_from_agent_path(path: &str) -> Option<String> {
    let component = Path::new(path).file_name()?.to_str()?.trim();
    if component.is_empty() {
        return None;
    }
    let readable = component.replace('_', " ");
    let mut characters = readable.chars();
    let first = characters.next()?.to_uppercase().collect::<String>();
    let title = format!("{first}{}", characters.collect::<String>());
    security::bounded_thread_title(&title)
        .ok()
        .filter(|title| !title.is_empty())
}

fn native_thread_title(
    name: Option<&str>,
    preview: &str,
    agent_path: Option<&str>,
    agent_nickname: Option<&str>,
) -> Result<String, ThreadStateError> {
    let normalized_name = name
        .map(security::bounded_thread_title)
        .transpose()
        .map_err(|_| ThreadStateError::InvalidRow)?
        .unwrap_or_default();
    let normalized_preview =
        security::bounded_thread_title(preview).map_err(|_| ThreadStateError::InvalidRow)?;
    if !normalized_name.is_empty() {
        return Ok(normalized_name);
    }
    if !normalized_preview.is_empty() {
        return Ok(normalized_preview);
    }
    if let Some(title) = agent_path.and_then(task_title_from_agent_path) {
        return Ok(title);
    }
    if let Some(nickname) = agent_nickname
        .map(security::bounded_thread_title)
        .transpose()
        .map_err(|_| ThreadStateError::InvalidRow)?
        .filter(|nickname| !nickname.is_empty())
    {
        return Ok(nickname);
    }
    Ok("アクティブなスレッド".to_owned())
}

fn validate_state_schema(connection: &Connection) -> Result<(), ThreadStateError> {
    let edges = table_columns(connection, "thread_spawn_edges")?;
    require_column(&edges, "parent_thread_id", "TEXT", true, false)?;
    require_column(&edges, "child_thread_id", "TEXT", true, true)?;
    require_column(&edges, "status", "TEXT", true, false)?;

    let threads = table_columns(connection, "threads")?;
    require_column(&threads, "id", "TEXT", false, true)?;
    require_column(&threads, "rollout_path", "TEXT", true, false)?;
    require_column(&threads, "updated_at", "INTEGER", true, false)?;
    require_column(&threads, "archived", "INTEGER", true, false)?;
    require_column(&threads, "name", "TEXT", false, false)?;
    require_column(&threads, "preview", "TEXT", true, false)?;
    require_column(&threads, "thread_source", "TEXT", false, false)?;
    Ok(())
}

fn creates_cycle(parent: &str, child: &str, parents: &HashMap<String, String>) -> bool {
    if parent == child {
        return true;
    }
    let mut cursor = parent;
    let mut steps = 0usize;
    while let Some(ancestor) = parents.get(cursor) {
        if ancestor == child {
            return true;
        }
        cursor = ancestor;
        steps += 1;
        if steps > MAX_NATIVE_THREAD_DESCENDANTS {
            return true;
        }
    }
    false
}

fn relation_depth(
    thread_id: &str,
    parents: &HashMap<String, String>,
) -> Result<usize, ThreadStateError> {
    let mut cursor = thread_id;
    let mut visited = HashSet::new();
    let mut depth = 0usize;
    while let Some(parent) = parents.get(cursor) {
        if !visited.insert(cursor.to_owned()) {
            return Err(ThreadStateError::Cycle);
        }
        depth = depth
            .checked_add(1)
            .ok_or(ThreadStateError::LimitExceeded)?;
        if depth > MAX_NATIVE_THREAD_DEPTH {
            return Err(ThreadStateError::LimitExceeded);
        }
        cursor = parent;
    }
    Ok(depth)
}

fn read_descendants(
    connection: &Connection,
    sessions_root: &Path,
    owner_root_ids: &BTreeSet<String>,
) -> Result<Vec<NativeThreadCandidate>, ThreadStateError> {
    if owner_root_ids.len() > MAX_NATIVE_THREAD_DESCENDANTS
        || owner_root_ids.iter().any(|id| !valid_thread_id(id))
    {
        return Err(ThreadStateError::LimitExceeded);
    }

    let mut frontier = owner_root_ids
        .iter()
        .cloned()
        .map(|id| FrontierEntry { id })
        .collect::<Vec<_>>();
    let mut expanded = HashSet::new();
    let mut discovered_rows = BTreeMap::<String, NativeThreadRow>::new();
    let mut parents = HashMap::<String, String>::new();
    let thread_columns = table_columns(connection, "threads")?;
    let has_agent_path = optional_text_column(&thread_columns, "agent_path")?;
    let has_agent_nickname = optional_text_column(&thread_columns, "agent_nickname")?;
    let has_created_at = optional_integer_column(&thread_columns, "created_at")?;
    let agent_path_column = if has_agent_path {
        "t.agent_path"
    } else {
        "NULL"
    };
    let agent_nickname_column = if has_agent_nickname {
        "t.agent_nickname"
    } else {
        "NULL"
    };
    let created_at_column = if has_created_at {
        "t.created_at"
    } else {
        "NULL"
    };

    while !frontier.is_empty() {
        let current = frontier
            .into_iter()
            .filter(|entry| expanded.insert(entry.id.clone()))
            .collect::<Vec<_>>();
        if current.is_empty() {
            break;
        }
        let mut next = Vec::new();
        for batch in current.chunks(QUERY_PARENT_BATCH) {
            let placeholders = std::iter::repeat_n("?", batch.len())
                .collect::<Vec<_>>()
                .join(",");
            let remaining = MAX_NATIVE_THREAD_DESCENDANTS
                .checked_sub(discovered_rows.len())
                .ok_or(ThreadStateError::LimitExceeded)?;
            let row_limit = remaining
                .checked_add(1)
                .ok_or(ThreadStateError::LimitExceeded)?;
            let sql = format!(
                "SELECT e.parent_thread_id, e.child_thread_id, t.rollout_path, \
                 {created_at_column}, t.updated_at, t.archived, t.name, t.preview, t.thread_source, \
                 {agent_path_column}, {agent_nickname_column} \
                 FROM thread_spawn_edges e \
                 LEFT JOIN threads t ON t.id = e.child_thread_id \
                 WHERE e.parent_thread_id IN ({placeholders}) \
                 ORDER BY e.parent_thread_id, e.child_thread_id LIMIT {row_limit}"
            );
            let mut statement = connection
                .prepare(&sql)
                .map_err(|_| ThreadStateError::Database)?;
            let rows = statement
                .query_map(
                    params_from_iter(batch.iter().map(|entry| &entry.id)),
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<i64>>(3)?,
                            row.get::<_, Option<i64>>(4)?,
                            row.get::<_, Option<i64>>(5)?,
                            row.get::<_, Option<String>>(6)?,
                            row.get::<_, Option<String>>(7)?,
                            row.get::<_, Option<String>>(8)?,
                            row.get::<_, Option<String>>(9)?,
                            row.get::<_, Option<String>>(10)?,
                        ))
                    },
                )
                .map_err(|_| ThreadStateError::Database)?;
            for row in rows {
                let (
                    parent,
                    id,
                    rollout_path,
                    created_at,
                    updated_at,
                    archived,
                    name,
                    preview,
                    source,
                    agent_path,
                    agent_nickname,
                ) = row.map_err(|_| ThreadStateError::InvalidRow)?;
                if !valid_thread_id(&parent)
                    || !valid_thread_id(&id)
                    || updated_at.is_none_or(|value| value <= 0)
                    || archived != Some(0)
                    || source.as_deref() != Some("subagent")
                {
                    return Err(ThreadStateError::InvalidRow);
                }
                if created_at.is_some_and(|value| value <= 0) {
                    return Err(ThreadStateError::InvalidRow);
                }
                if creates_cycle(&parent, &id, &parents) {
                    return Err(ThreadStateError::Cycle);
                }
                if let Some(existing) = parents.insert(id.clone(), parent.clone()) {
                    if existing != parent {
                        return Err(ThreadStateError::InvalidRow);
                    }
                }
                let rollout_path = rollout_path.ok_or(ThreadStateError::InvalidRow)?;
                if rollout_path.len() > MAX_ROLLOUT_PATH_BYTES {
                    return Err(ThreadStateError::LimitExceeded);
                }
                let canonical_rollout =
                    security::canonical_regular_file_under(sessions_root, Path::new(&rollout_path))
                        .map_err(|_| ThreadStateError::UnsafePath)?;
                if canonical_rollout
                    .extension()
                    .and_then(|value| value.to_str())
                    != Some("jsonl")
                {
                    return Err(ThreadStateError::UnsafePath);
                }
                let title = native_thread_title(
                    name.as_deref(),
                    preview.as_deref().ok_or(ThreadStateError::InvalidRow)?,
                    agent_path.as_deref(),
                    agent_nickname.as_deref(),
                )?;
                let candidate = NativeThreadRow {
                    id: id.clone(),
                    created_at,
                    updated_at: updated_at.ok_or(ThreadStateError::InvalidRow)?,
                    title,
                    rollout_path: canonical_rollout,
                    parent_thread_id: parent,
                };
                if let Some(existing) = discovered_rows.get(&id) {
                    if existing.id != candidate.id
                        || existing.created_at != candidate.created_at
                        || existing.updated_at != candidate.updated_at
                        || existing.title != candidate.title
                        || existing.rollout_path != candidate.rollout_path
                        || existing.parent_thread_id != candidate.parent_thread_id
                    {
                        return Err(ThreadStateError::InvalidRow);
                    }
                } else {
                    if discovered_rows.len() >= MAX_NATIVE_THREAD_DESCENDANTS {
                        return Err(ThreadStateError::LimitExceeded);
                    }
                    discovered_rows.insert(id.clone(), candidate);
                    next.push(FrontierEntry { id });
                }
            }
        }
        frontier = next;
    }

    discovered_rows
        .into_values()
        .map(|row| {
            let depth = relation_depth(&row.id, &parents)?;
            let depth = i32::try_from(depth).map_err(|_| ThreadStateError::LimitExceeded)?;
            Ok(NativeThreadCandidate {
                id: row.id,
                created_at: row.created_at,
                updated_at: row.updated_at,
                title: row.title,
                rollout_path: row.rollout_path,
                parent_thread_id: row.parent_thread_id,
                depth,
            })
        })
        .collect()
}

/// Load descendants of externally proven current owner roots.
///
/// This never decides whether a returned thread is running.  Callers must use
/// the bounded rollout parser and publish the root/descendant union atomically.
pub fn load_native_descendants(
    codex_root: &Path,
    sessions_root: &Path,
    owner_root_ids: &BTreeSet<String>,
) -> Result<Vec<NativeThreadCandidate>, ThreadStateError> {
    if owner_root_ids.is_empty() {
        return Ok(Vec::new());
    }
    let canonical_root =
        security::validate_absolute_root(codex_root).map_err(|_| ThreadStateError::UnsafePath)?;
    let canonical_sessions = security::validate_absolute_root(sessions_root)
        .map_err(|_| ThreadStateError::UnsafePath)?;
    if canonical_sessions != canonical_root.join("sessions") {
        return Err(ThreadStateError::UnsafePath);
    }

    let (mut connection, database_path, before) = open_state_database(&canonical_root)?;
    let transaction = connection
        .transaction()
        .map_err(|_| ThreadStateError::Database)?;
    validate_state_schema(&transaction)?;
    let descendants = read_descendants(&transaction, &canonical_sessions, owner_root_ids)?;
    transaction
        .commit()
        .map_err(|_| ThreadStateError::Database)?;
    drop(connection);

    validate_optional_sidecars(&canonical_root, &database_path)?;
    let after_path = security::canonical_regular_file_under(&canonical_root, &database_path)
        .map_err(|_| ThreadStateError::Replaced)?;
    let after = fs::symlink_metadata(after_path).map_err(|_| ThreadStateError::Replaced)?;
    if !same_file(&before, &after) || after.len() < before.len() {
        return Err(ThreadStateError::Replaced);
    }
    Ok(descendants)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};
    use std::sync::atomic::{AtomicU64, Ordering};

    struct StateFixture {
        root: PathBuf,
        sessions: PathBuf,
    }

    impl StateFixture {
        fn new() -> Self {
            static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "codex-info-native-state-{}-{}",
                std::process::id(),
                NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
            ));
            let sessions = root.join("sessions");
            fs::create_dir_all(&sessions).expect("state fixture sessions");
            let connection =
                Connection::open(root.join("state_5.sqlite")).expect("state fixture database");
            connection
                .execute_batch(
                    "CREATE TABLE threads (
                        id TEXT PRIMARY KEY,
                        rollout_path TEXT NOT NULL,
                        created_at INTEGER NOT NULL,
                        updated_at INTEGER NOT NULL,
                        archived INTEGER NOT NULL,
                        name TEXT,
                        preview TEXT NOT NULL,
                        thread_source TEXT,
                        agent_path TEXT,
                        agent_nickname TEXT
                    );
                    CREATE TABLE thread_spawn_edges (
                        parent_thread_id TEXT NOT NULL,
                        child_thread_id TEXT NOT NULL PRIMARY KEY,
                        status TEXT NOT NULL
                    );",
                )
                .expect("state fixture schema");
            drop(connection);
            Self { root, sessions }
        }

        fn database(&self) -> Connection {
            Connection::open(self.root.join("state_5.sqlite")).expect("state fixture connection")
        }

        fn add_thread(&self, id: &str, rollout_path: &Path) {
            self.database()
                .execute(
                    "INSERT INTO threads
                     (id, rollout_path, created_at, updated_at, archived, name, preview, thread_source)
                     VALUES (?1, ?2, ?3, ?3, 0, ?4, ?5, 'subagent')",
                    params![
                        id,
                        rollout_path.to_string_lossy().as_ref(),
                        1_i64,
                        format!("title-{id}"),
                        format!("preview-{id}"),
                    ],
                )
                .expect("state fixture thread");
        }

        fn add_edge(&self, parent: &str, child: &str) {
            self.database()
                .execute(
                    "INSERT INTO thread_spawn_edges
                     (parent_thread_id, child_thread_id, status) VALUES (?1, ?2, 'active')",
                    params![parent, child],
                )
                .expect("state fixture edge");
        }

        fn set_agent_metadata(&self, id: &str, path: Option<&str>, nickname: Option<&str>) {
            self.database()
                .execute(
                    "UPDATE threads SET agent_path = ?2, agent_nickname = ?3 WHERE id = ?1",
                    params![id, path, nickname],
                )
                .expect("state fixture agent metadata");
        }

        fn rollout(&self, id: &str) -> PathBuf {
            let path = self.sessions.join(format!("{id}.jsonl"));
            fs::write(&path, "{}\n").expect("state fixture rollout");
            path
        }

        fn roots(&self, ids: &[&str]) -> BTreeSet<String> {
            ids.iter().map(|id| (*id).to_owned()).collect()
        }
    }

    impl Drop for StateFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn descendants_follow_parent_child_grandchild_and_exclude_other_roots() {
        let fixture = StateFixture::new();
        let child_path = fixture.rollout("child");
        let grandchild_path = fixture.rollout("grandchild");
        let other_path = fixture.rollout("other");
        fixture.add_thread("child", &child_path);
        fixture.add_thread("grandchild", &grandchild_path);
        fixture.add_thread("other", &other_path);
        fixture.add_edge("root", "child");
        fixture.add_edge("child", "grandchild");
        fixture.add_edge("other-root", "other");

        let descendants =
            load_native_descendants(&fixture.root, &fixture.sessions, &fixture.roots(&["root"]))
                .expect("descendant graph");

        assert_eq!(
            descendants
                .iter()
                .map(|candidate| (
                    candidate.id.as_str(),
                    candidate.parent_thread_id.as_str(),
                    candidate.depth,
                ))
                .collect::<Vec<_>>(),
            vec![("child", "root", 1), ("grandchild", "child", 2)]
        );
    }

    #[test]
    fn native_descendant_title_uses_agent_task_name_when_database_title_is_empty() {
        let fixture = StateFixture::new();
        let child_path = fixture.rollout("child");
        fixture.add_thread("child", &child_path);
        fixture
            .database()
            .execute(
                "UPDATE threads SET name = '', preview = '' WHERE id = 'child'",
                [],
            )
            .expect("clear state fixture title");
        fixture.set_agent_metadata(
            "child",
            Some("/root/graph_period_final_audit_v2"),
            Some("Einstein"),
        );
        fixture.add_edge("root", "child");

        let descendants =
            load_native_descendants(&fixture.root, &fixture.sessions, &fixture.roots(&["root"]))
                .expect("descendant graph");
        assert_eq!(descendants[0].title, "Graph period final audit v2");

        fixture.set_agent_metadata("child", None, Some("Einstein"));
        let descendants =
            load_native_descendants(&fixture.root, &fixture.sessions, &fixture.roots(&["root"]))
                .expect("descendant graph");
        assert_eq!(descendants[0].title, "Einstein");
    }

    #[test]
    fn state_database_is_read_only_and_rejects_invalid_schema() {
        let fixture = StateFixture::new();
        let (connection, _, _) = open_state_database(&fixture.root).expect("read-only database");
        assert_eq!(
            connection
                .query_row("PRAGMA query_only", [], |row| row.get::<_, i64>(0))
                .expect("query_only pragma"),
            1
        );
        assert!(connection
            .execute("CREATE TABLE should_not_write (id INTEGER)", [])
            .is_err());
        drop(connection);

        let malformed = fixture.root.join("state_5.sqlite");
        let connection = Connection::open(&malformed).expect("replace schema");
        connection
            .execute("DROP TABLE threads", [])
            .expect("drop threads");
        drop(connection);
        assert_eq!(
            load_native_descendants(&fixture.root, &fixture.sessions, &fixture.roots(&["root"]),),
            Err(ThreadStateError::InvalidSchema)
        );
    }

    #[test]
    fn graph_cycle_dangling_row_depth_and_unsafe_path_fail_closed() {
        let cycle = StateFixture::new();
        let a_path = cycle.rollout("a");
        let b_path = cycle.rollout("b");
        let c_path = cycle.rollout("c");
        cycle.add_thread("a", &a_path);
        cycle.add_thread("b", &b_path);
        cycle.add_thread("c", &c_path);
        cycle.add_edge("a", "b");
        cycle.add_edge("b", "c");
        cycle.add_edge("c", "a");
        assert_eq!(
            load_native_descendants(&cycle.root, &cycle.sessions, &cycle.roots(&["a"])),
            Err(ThreadStateError::Cycle)
        );

        let dangling = StateFixture::new();
        dangling.add_edge("root", "missing");
        assert_eq!(
            load_native_descendants(
                &dangling.root,
                &dangling.sessions,
                &dangling.roots(&["root"]),
            ),
            Err(ThreadStateError::InvalidRow)
        );

        let too_deep = StateFixture::new();
        let mut parent = "root".to_owned();
        for index in 0..=MAX_NATIVE_THREAD_DEPTH {
            let child = format!("depth-{index}");
            let path = too_deep.rollout(&child);
            too_deep.add_thread(&child, &path);
            too_deep.add_edge(&parent, &child);
            parent = child;
        }
        assert_eq!(
            load_native_descendants(
                &too_deep.root,
                &too_deep.sessions,
                &too_deep.roots(&["root"]),
            ),
            Err(ThreadStateError::LimitExceeded)
        );

        let unsafe_path = StateFixture::new();
        let outside = unsafe_path.root.join("outside.jsonl");
        fs::write(&outside, "{}\n").expect("outside rollout");
        unsafe_path.add_thread("child", &outside);
        unsafe_path.add_edge("root", "child");
        assert_eq!(
            load_native_descendants(
                &unsafe_path.root,
                &unsafe_path.sessions,
                &unsafe_path.roots(&["root"]),
            ),
            Err(ThreadStateError::UnsafePath)
        );
    }

    #[test]
    fn invalid_descendant_does_not_return_a_partial_snapshot() {
        let fixture = StateFixture::new();
        let valid_path = fixture.rollout("valid");
        fixture.add_thread("valid", &valid_path);
        fixture.add_edge("root", "valid");
        fixture.add_edge("root", "missing");

        let result =
            load_native_descendants(&fixture.root, &fixture.sessions, &fixture.roots(&["root"]));
        assert_eq!(result, Err(ThreadStateError::InvalidRow));
    }
}
