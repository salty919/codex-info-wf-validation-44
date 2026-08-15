use chrono::{DateTime, Months, Utc};
use rusqlite::types::Value;
use rusqlite::{params, Connection, OptionalExtension};
use std::fmt;
use std::fs;
use std::fs::OpenOptions;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS usage_history (
    timestamp INTEGER NOT NULL CHECK (timestamp > 0),
    reset_at INTEGER NOT NULL CHECK (reset_at > 0),
    remaining_percent REAL,
    sol_dollars REAL NOT NULL,
    terra_dollars REAL NOT NULL,
    luna_dollars REAL NOT NULL,
    sol_tokens INTEGER NOT NULL DEFAULT 0,
    terra_tokens INTEGER NOT NULL DEFAULT 0,
    luna_tokens INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (reset_at, timestamp)
);
CREATE INDEX IF NOT EXISTS usage_history_timestamp_idx
    ON usage_history (timestamp);

CREATE TABLE IF NOT EXISTS durable_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    data_generation INTEGER NOT NULL CHECK (data_generation >= 0),
    data_hash TEXT NOT NULL,
    snapshot_json TEXT NOT NULL
);
"#;

const RESET_GROUP_TOLERANCE_SECONDS: i128 = 60;

const UPSERT_SAMPLE: &str = r#"
INSERT INTO usage_history (
    timestamp,
    reset_at,
    remaining_percent,
    sol_dollars,
    terra_dollars,
    luna_dollars,
    sol_tokens,
    terra_tokens,
    luna_tokens
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
ON CONFLICT (reset_at, timestamp) DO UPDATE SET
    remaining_percent = COALESCE(excluded.remaining_percent, usage_history.remaining_percent),
    sol_dollars = MAX(usage_history.sol_dollars, excluded.sol_dollars),
    terra_dollars = MAX(usage_history.terra_dollars, excluded.terra_dollars),
    luna_dollars = MAX(usage_history.luna_dollars, excluded.luna_dollars),
    sol_tokens = MAX(usage_history.sol_tokens, excluded.sol_tokens),
    terra_tokens = MAX(usage_history.terra_tokens, excluded.terra_tokens),
    luna_tokens = MAX(usage_history.luna_tokens, excluded.luna_tokens)
"#;

/// Returns the UTC instant three calendar months before `now`.
///
/// Chrono clamps an end-of-month date to the last valid day in the target
/// month, so May 31 minus three months is February 29 in a leap year (and
/// February 28 otherwise), rather than an arbitrary 90-day duration.
fn three_months_before(now: DateTime<Utc>) -> DateTime<Utc> {
    now.checked_sub_months(Months::new(3))
        .expect("subtracting three calendar months from UTC now must be representable")
}

/// Upper bound for the serialized durable snapshot kept in SQLite.
pub const MAX_SNAPSHOT_JSON_BYTES: usize = 1024 * 1024;

/// One minute of usage history for a particular reset window.
#[derive(Clone, Debug, PartialEq)]
pub struct UsageHistorySample {
    pub timestamp: i64,
    pub reset_at: i64,
    pub remaining_percent: Option<f64>,
    pub sol_dollars: f64,
    pub terra_dollars: f64,
    pub luna_dollars: f64,
    pub sol_tokens: u64,
    pub terra_tokens: u64,
    pub luna_tokens: u64,
}

/// A reset period identified only by the canonical reset timestamp.
///
/// The identifier is intentionally opaque to storage consumers. In
/// particular, it is not a formatted local-time label.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResetPeriod {
    pub canonical_id: i64,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
}

/// Backwards-compatible descriptive alias for callers that use the history
/// terminology rather than the reset-period terminology.
pub type UsageHistoryPeriod = ResetPeriod;

/// The singleton durable snapshot associated with a committed history batch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DurableRecord {
    pub data_generation: u64,
    pub data_hash: String,
    pub snapshot_json: String,
}

/// Errors returned while opening or using a usage history database.
#[derive(Debug)]
pub enum UsageStoreError {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    InvalidImport(String),
    InvalidDurableRecord(String),
    InvalidTimestamp { field: &'static str, value: i64 },
    NonFiniteValue { field: &'static str },
    GenerationConflict { expected: u64, actual: u64 },
    GenerationOverflow,
}

pub type Result<T> = std::result::Result<T, UsageStoreError>;

impl fmt::Display for UsageStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "database directory error: {error}"),
            Self::Sqlite(error) => write!(formatter, "SQLite error: {error}"),
            Self::InvalidImport(error) => write!(formatter, "invalid usage import: {error}"),
            Self::InvalidDurableRecord(error) => {
                write!(formatter, "invalid durable record: {error}")
            }
            Self::InvalidTimestamp { field, value } => write!(
                formatter,
                "invalid {field} timestamp {value}; expected a positive Unix timestamp"
            ),
            Self::NonFiniteValue { field } => {
                write!(formatter, "{field} must be finite")
            }
            Self::GenerationConflict { expected, actual } => write!(
                formatter,
                "durable generation conflict: expected {expected}, found {actual}"
            ),
            Self::GenerationOverflow => write!(formatter, "durable generation overflow"),
        }
    }
}

impl std::error::Error for UsageStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Sqlite(error) => Some(error),
            Self::InvalidImport(_)
            | Self::InvalidDurableRecord(_)
            | Self::InvalidTimestamp { .. }
            | Self::NonFiniteValue { .. }
            | Self::GenerationConflict { .. }
            | Self::GenerationOverflow => None,
        }
    }
}

impl From<std::io::Error> for UsageStoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rusqlite::Error> for UsageStoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl UsageHistorySample {
    fn validate(&self) -> Result<()> {
        for (field, value) in [
            ("sol_dollars", self.sol_dollars),
            ("terra_dollars", self.terra_dollars),
            ("luna_dollars", self.luna_dollars),
        ] {
            if !value.is_finite() {
                return Err(UsageStoreError::NonFiniteValue { field });
            }
            if value < 0.0 {
                return Err(UsageStoreError::InvalidImport(format!(
                    "{field} must be finite and non-negative"
                )));
            }
        }

        if let Some(value) = self.remaining_percent {
            if !value.is_finite() {
                return Err(UsageStoreError::NonFiniteValue {
                    field: "remaining_percent",
                });
            }
            if !(0.0..=100.0).contains(&value) {
                return Err(UsageStoreError::InvalidImport(
                    "remaining_percent must be finite and between 0 and 100".into(),
                ));
            }
        }

        if self.timestamp <= 0 {
            return Err(UsageStoreError::InvalidTimestamp {
                field: "timestamp",
                value: self.timestamp,
            });
        }
        if self.reset_at <= 0 {
            return Err(UsageStoreError::InvalidTimestamp {
                field: "reset_at",
                value: self.reset_at,
            });
        }
        if [self.sol_tokens, self.terra_tokens, self.luna_tokens]
            .into_iter()
            .any(|tokens| tokens > i64::MAX as u64)
        {
            return Err(UsageStoreError::InvalidImport(
                "token count exceeds SQLite INTEGER range".into(),
            ));
        }

        Ok(())
    }
}

fn numeric_sqlite_value(value: Value) -> Option<f64> {
    match value {
        Value::Integer(value) => {
            let value_as_f64 = value as f64;
            (value_as_f64 as i128 == i128::from(value)).then_some(value_as_f64)
        }
        Value::Real(value) => Some(value),
        _ => None,
    }
}

fn valid_sample_from_row(row: &rusqlite::Row<'_>) -> Result<Option<UsageHistorySample>> {
    let timestamp = match row.get::<_, Value>(0)? {
        Value::Integer(value) => value,
        _ => return Ok(None),
    };
    let reset_at = match row.get::<_, Value>(1)? {
        Value::Integer(value) => value,
        _ => return Ok(None),
    };
    let remaining_percent = match row.get::<_, Value>(2)? {
        Value::Null => None,
        value => {
            let Some(value) = numeric_sqlite_value(value) else {
                return Ok(None);
            };
            Some(value)
        }
    };
    let sol_dollars = match numeric_sqlite_value(row.get(3)?) {
        Some(value) => value,
        _ => return Ok(None),
    };
    let terra_dollars = match numeric_sqlite_value(row.get(4)?) {
        Some(value) => value,
        _ => return Ok(None),
    };
    let luna_dollars = match numeric_sqlite_value(row.get(5)?) {
        Some(value) => value,
        _ => return Ok(None),
    };
    let sol_tokens = match row.get::<_, Value>(6)? {
        Value::Integer(value) if value >= 0 => match u64::try_from(value) {
            Ok(value) => value,
            Err(_) => return Ok(None),
        },
        _ => return Ok(None),
    };
    let terra_tokens = match row.get::<_, Value>(7)? {
        Value::Integer(value) if value >= 0 => match u64::try_from(value) {
            Ok(value) => value,
            Err(_) => return Ok(None),
        },
        _ => return Ok(None),
    };
    let luna_tokens = match row.get::<_, Value>(8)? {
        Value::Integer(value) if value >= 0 => match u64::try_from(value) {
            Ok(value) => value,
            Err(_) => return Ok(None),
        },
        _ => return Ok(None),
    };

    let sample = UsageHistorySample {
        timestamp,
        reset_at,
        remaining_percent,
        sol_dollars,
        terra_dollars,
        luna_dollars,
        sol_tokens,
        terra_tokens,
        luna_tokens,
    };
    if sample.validate().is_err() {
        return Ok(None);
    }
    Ok(Some(sample))
}

fn validate_data_hash(data_hash: &str) -> Result<()> {
    if data_hash.len() != 64
        || !data_hash
            .bytes()
            .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
    {
        return Err(UsageStoreError::InvalidDurableRecord(
            "data_hash must be exactly 64 lowercase hexadecimal characters".into(),
        ));
    }
    Ok(())
}

fn validate_snapshot_json(snapshot_json: &str) -> Result<()> {
    if snapshot_json.len() <= MAX_SNAPSHOT_JSON_BYTES {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(snapshot_json) {
            if !value.is_object() {
                return Err(UsageStoreError::InvalidImport(
                    "snapshot_json must be a JSON object".into(),
                ));
            }
        }
    }
    if snapshot_json.len() > MAX_SNAPSHOT_JSON_BYTES {
        return Err(UsageStoreError::InvalidDurableRecord(format!(
            "snapshot_json exceeds {MAX_SNAPSHOT_JSON_BYTES} bytes"
        )));
    }
    serde_json::from_str::<serde_json::Value>(snapshot_json)
        .map_err(|error| UsageStoreError::InvalidDurableRecord(error.to_string()))?;
    Ok(())
}

impl DurableRecord {
    fn validate(&self) -> Result<()> {
        validate_data_hash(&self.data_hash)?;
        validate_snapshot_json(&self.snapshot_json)
    }
}

fn durable_record_from_sql(
    data_generation: i64,
    data_hash: String,
    snapshot_json: String,
) -> Result<DurableRecord> {
    if data_generation < 0 {
        return Err(UsageStoreError::InvalidDurableRecord(
            "data_generation must not be negative".into(),
        ));
    }
    let record = DurableRecord {
        data_generation: data_generation as u64,
        data_hash,
        snapshot_json,
    };
    record.validate()?;
    Ok(record)
}

struct ResetPeriodAccumulator {
    min_reset_at: i64,
    canonical_id: i64,
    start_timestamp: i64,
}

fn build_reset_periods(samples: &[UsageHistorySample]) -> Vec<ResetPeriod> {
    let mut ordered = samples.to_vec();
    ordered.sort_by(|left, right| {
        left.reset_at
            .cmp(&right.reset_at)
            .then_with(|| left.timestamp.cmp(&right.timestamp))
    });

    let mut groups = Vec::<ResetPeriodAccumulator>::new();
    for sample in ordered {
        let Some(current) = groups.last_mut() else {
            groups.push(ResetPeriodAccumulator {
                min_reset_at: sample.reset_at,
                canonical_id: sample.reset_at,
                start_timestamp: sample.timestamp,
            });
            continue;
        };

        let reset_distance = i128::from(sample.reset_at) - i128::from(current.min_reset_at);
        if reset_distance <= RESET_GROUP_TOLERANCE_SECONDS {
            current.canonical_id = current.canonical_id.max(sample.reset_at);
            current.start_timestamp = current.start_timestamp.min(sample.timestamp);
        } else {
            groups.push(ResetPeriodAccumulator {
                min_reset_at: sample.reset_at,
                canonical_id: sample.reset_at,
                start_timestamp: sample.timestamp,
            });
        }
    }

    let mut periods = groups
        .iter()
        .enumerate()
        .map(|(index, group)| {
            let end_timestamp = groups
                .get(index + 1)
                .map(|next| group.canonical_id.min(next.start_timestamp))
                .unwrap_or(group.canonical_id);
            ResetPeriod {
                canonical_id: group.canonical_id,
                start_timestamp: group.start_timestamp,
                end_timestamp,
            }
        })
        .collect::<Vec<_>>();
    periods.sort_by(|left, right| {
        right
            .start_timestamp
            .cmp(&left.start_timestamp)
            .then_with(|| right.canonical_id.cmp(&left.canonical_id))
    });
    periods
}

/// Groups samples by reset timestamps using only deterministic UTC epoch
/// values. Reset timestamps within sixty seconds of a group's first reset are
/// one period; sixty-one seconds starts a distinct period.
pub fn group_reset_periods(samples: &[UsageHistorySample]) -> Vec<ResetPeriod> {
    build_reset_periods(samples)
}

fn json_sample(value: &serde_json::Value) -> Result<UsageHistorySample> {
    let object = value
        .as_object()
        .ok_or_else(|| UsageStoreError::InvalidImport("each sample must be an object".into()))?;
    let required_i64 = |name: &str| {
        object
            .get(name)
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(|| UsageStoreError::InvalidImport(format!("missing integer field {name}")))
    };
    let required_f64 = |name: &str| {
        object
            .get(name)
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| UsageStoreError::InvalidImport(format!("missing number field {name}")))
    };
    let optional_u64 = |name: &str| match object.get(name) {
        None => Ok(0),
        Some(value) => value.as_u64().ok_or_else(|| {
            UsageStoreError::InvalidImport(format!("{name} must be a non-negative integer"))
        }),
    };
    let remaining_percent = match object.get("remaining_percent") {
        None | Some(serde_json::Value::Null) => None,
        Some(value) => Some(value.as_f64().ok_or_else(|| {
            UsageStoreError::InvalidImport("remaining_percent must be a number or null".into())
        })?),
    };

    Ok(UsageHistorySample {
        timestamp: required_i64("timestamp")?,
        reset_at: required_i64("reset_at")?,
        remaining_percent,
        sol_dollars: required_f64("sol_dollars")?,
        terra_dollars: required_f64("terra_dollars")?,
        luna_dollars: required_f64("luna_dollars")?,
        sol_tokens: optional_u64("sol_tokens")?,
        terra_tokens: optional_u64("terra_tokens")?,
        luna_tokens: optional_u64("luna_tokens")?,
    })
}

/// Persistent SQLite storage for minute-level usage samples.
pub struct UsageStore {
    connection: Connection,
}

#[allow(dead_code)]
impl UsageStore {
    /// Opens `path`, creating its parent directories and schema as needed.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.is_absolute() {
            return Err(UsageStoreError::InvalidImport(
                "database path must be absolute".into(),
            ));
        }
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
                }
            }
        }

        if let Ok(metadata) = fs::symlink_metadata(path) {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(UsageStoreError::InvalidImport(
                    "database path must be a regular file".into(),
                ));
            }
        } else {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            options.open(path)?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }

        let mut connection = Connection::open(path)?;
        let transaction = connection.transaction()?;
        transaction.execute_batch(SCHEMA)?;
        // Migrate v1 tables additively; existing rows and dollar values remain.
        for column in ["sol_tokens", "terra_tokens", "luna_tokens"] {
            let present: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('usage_history') WHERE name = ?1)",
                [column],
                |row| row.get(0),
            )?;
            if !present {
                transaction.execute(
                    &format!(
                        "ALTER TABLE usage_history ADD COLUMN {column} INTEGER NOT NULL DEFAULT 0"
                    ),
                    [],
                )?;
            }
        }
        transaction.commit()?;
        Ok(Self { connection })
    }

    /// Alias for callers that prefer constructor-style naming.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::open(path)
    }

    /// Loads all samples in reset-window and timestamp order.
    pub fn load_all(&self) -> Result<Vec<UsageHistorySample>> {
        let mut statement = self.connection.prepare(
            "SELECT timestamp, reset_at, remaining_percent, sol_dollars, \
                    terra_dollars, luna_dollars, sol_tokens, terra_tokens, luna_tokens \
             FROM usage_history \
             ORDER BY reset_at ASC, timestamp ASC",
        )?;
        let mut rows = statement.query([])?;
        let mut samples = Vec::new();

        while let Some(row) = rows.next()? {
            if let Some(sample) = valid_sample_from_row(row)? {
                samples.push(sample);
            }
        }

        Ok(samples)
    }

    fn load_recent_history_impl(&self, now: DateTime<Utc>) -> Result<Vec<UsageHistorySample>> {
        let cutoff = three_months_before(now).timestamp();
        let now_timestamp = now.timestamp();
        let mut statement = self.connection.prepare(
            "SELECT timestamp, reset_at, remaining_percent, sol_dollars, \
                    terra_dollars, luna_dollars, sol_tokens, terra_tokens, luna_tokens \
             FROM usage_history \
             WHERE timestamp >= ?1 AND timestamp <= ?2 \
             ORDER BY reset_at ASC, timestamp ASC",
        )?;
        let mut rows = statement.query(params![cutoff, now_timestamp])?;
        let mut samples = Vec::new();
        while let Some(row) = rows.next()? {
            if let Some(sample) = valid_sample_from_row(row)? {
                samples.push(sample);
            }
        }
        Ok(samples)
    }

    /// Loads valid, non-pruned samples in the inclusive three-calendar-month
    /// UTC interval ending at the explicit `now` instant.
    pub fn load_recent_three_months(&self, now: DateTime<Utc>) -> Result<Vec<UsageHistorySample>> {
        self.load_recent_history_impl(now)
    }

    /// Alias for the same bounded read, retaining the history terminology.
    pub fn load_recent_history(&self, now: DateTime<Utc>) -> Result<Vec<UsageHistorySample>> {
        self.load_recent_history_impl(now)
    }

    /// Alias for callers that name the interval by its calendar length.
    pub fn load_three_month_history(&self, now: DateTime<Utc>) -> Result<Vec<UsageHistorySample>> {
        self.load_recent_history_impl(now)
    }

    /// Pure grouping helper exposed beside the store API for callers that
    /// already have a bounded sample slice.
    pub fn group_reset_periods(samples: &[UsageHistorySample]) -> Vec<ResetPeriod> {
        build_reset_periods(samples)
    }

    /// Inserts a sample or updates the sample with the same reset window and minute.
    ///
    /// A missing remaining-quota value never erases an already stored value.
    pub fn upsert_sample(&self, sample: &UsageHistorySample) -> Result<()> {
        sample.validate()?;
        self.connection.execute(
            UPSERT_SAMPLE,
            params![
                sample.timestamp,
                sample.reset_at,
                sample.remaining_percent,
                sample.sol_dollars,
                sample.terra_dollars,
                sample.luna_dollars,
                sample.sol_tokens as i64,
                sample.terra_tokens as i64,
                sample.luna_tokens as i64,
            ],
        )?;
        Ok(())
    }

    /// Atomically upserts several samples after validating the complete batch.
    pub fn upsert_samples(&mut self, samples: &[UsageHistorySample]) -> Result<()> {
        for sample in samples {
            sample.validate()?;
        }

        let transaction = self.connection.transaction()?;
        {
            let mut statement = transaction.prepare(UPSERT_SAMPLE)?;
            for sample in samples {
                statement.execute(params![
                    sample.timestamp,
                    sample.reset_at,
                    sample.remaining_percent,
                    sample.sol_dollars,
                    sample.terra_dollars,
                    sample.luna_dollars,
                    sample.sol_tokens as i64,
                    sample.terra_tokens as i64,
                    sample.luna_tokens as i64,
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// Imports already-decoded v1 samples without replacing existing data.
    ///
    /// This is the common migration boundary: validation happens before the
    /// transaction starts, and the same composite key is merged idempotently.
    pub fn import_samples(&mut self, samples: &[UsageHistorySample]) -> Result<usize> {
        self.upsert_samples(samples)?;
        Ok(samples.len())
    }

    /// Imports a v1 SQLite database containing the legacy `usage_history`
    /// table. The source is read completely before writing, so migration is
    /// additive even when it is run repeatedly.
    pub fn import_v1_sqlite<P: AsRef<Path>>(&mut self, path: P) -> Result<usize> {
        let source = Connection::open(path)?;
        let mut statement = source.prepare(
            "SELECT timestamp, reset_at, remaining_percent, sol_dollars, \
                    terra_dollars, luna_dollars, \
                    0 AS sol_tokens, 0 AS terra_tokens, 0 AS luna_tokens \
             FROM usage_history ORDER BY reset_at ASC, timestamp ASC",
        )?;
        let mut rows = statement.query([])?;
        let mut samples = Vec::new();
        while let Some(row) = rows.next()? {
            let Some(sample) = valid_sample_from_row(row)? else {
                return Err(UsageStoreError::InvalidImport(
                    "invalid v1 SQLite sample".into(),
                ));
            };
            samples.push(sample);
        }
        self.import_samples(&samples)
    }

    /// Imports a v1 JSON array, or an object with a `samples` array.
    pub fn import_v1_json<P: AsRef<Path>>(&mut self, path: P) -> Result<usize> {
        let contents = fs::read_to_string(path)?;
        let value: serde_json::Value = serde_json::from_str(&contents)
            .map_err(|error| UsageStoreError::InvalidImport(error.to_string()))?;
        let values = value
            .as_array()
            .or_else(|| value.get("samples").and_then(serde_json::Value::as_array))
            .ok_or_else(|| UsageStoreError::InvalidImport("expected a samples array".into()))?;
        let samples = values.iter().map(json_sample).collect::<Result<Vec<_>>>()?;
        self.import_samples(&samples)
    }

    fn commit_durable_state_inner(
        &mut self,
        expected_generation: Option<u64>,
        samples: &[UsageHistorySample],
        data_hash: &str,
        snapshot_json: &str,
    ) -> Result<DurableRecord> {
        for sample in samples {
            sample.validate()?;
        }
        validate_data_hash(data_hash)?;
        validate_snapshot_json(snapshot_json)?;

        let transaction = self.connection.transaction()?;
        let current_raw: Option<(i64, String, String)> = transaction
            .query_row(
                "SELECT data_generation, data_hash, snapshot_json \
                 FROM durable_state WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let current = current_raw
            .map(|(data_generation, data_hash, snapshot_json)| {
                durable_record_from_sql(data_generation, data_hash, snapshot_json)
            })
            .transpose()?;
        let current_generation = current
            .as_ref()
            .map(|record| record.data_generation)
            .unwrap_or(0);
        if let Some(expected_generation) = expected_generation {
            if expected_generation != current_generation {
                return Err(UsageStoreError::GenerationConflict {
                    expected: expected_generation,
                    actual: current_generation,
                });
            }
        }
        let next_generation = current_generation
            .checked_add(1)
            .ok_or(UsageStoreError::GenerationOverflow)?;
        let sqlite_generation =
            i64::try_from(next_generation).map_err(|_| UsageStoreError::GenerationOverflow)?;

        {
            let mut statement = transaction.prepare(UPSERT_SAMPLE)?;
            for sample in samples {
                statement.execute(params![
                    sample.timestamp,
                    sample.reset_at,
                    sample.remaining_percent,
                    sample.sol_dollars,
                    sample.terra_dollars,
                    sample.luna_dollars,
                    sample.sol_tokens as i64,
                    sample.terra_tokens as i64,
                    sample.luna_tokens as i64,
                ])?;
            }
        }
        transaction.execute(
            "INSERT INTO durable_state (singleton, data_generation, data_hash, snapshot_json) \
             VALUES (1, ?1, ?2, ?3) \
             ON CONFLICT (singleton) DO UPDATE SET \
                 data_generation = excluded.data_generation, \
                 data_hash = excluded.data_hash, \
                 snapshot_json = excluded.snapshot_json",
            params![sqlite_generation, data_hash, snapshot_json],
        )?;
        transaction.commit()?;

        Ok(DurableRecord {
            data_generation: next_generation,
            data_hash: data_hash.to_owned(),
            snapshot_json: snapshot_json.to_owned(),
        })
    }

    /// Atomically upserts `samples` and commits the next durable snapshot.
    /// The first committed generation is one; all validation occurs before
    /// the transaction can change either history or durable state.
    pub fn commit_durable_state<H: AsRef<str>, J: AsRef<str>>(
        &mut self,
        samples: &[UsageHistorySample],
        data_hash: H,
        snapshot_json: J,
    ) -> Result<DurableRecord> {
        self.commit_durable_state_inner(None, samples, data_hash.as_ref(), snapshot_json.as_ref())
    }

    /// Atomically commits only when the currently stored generation matches
    /// `expected_generation`; zero denotes an empty durable-state table.
    pub fn commit_durable_state_if_generation<H: AsRef<str>, J: AsRef<str>>(
        &mut self,
        expected_generation: u64,
        samples: &[UsageHistorySample],
        data_hash: H,
        snapshot_json: J,
    ) -> Result<DurableRecord> {
        self.commit_durable_state_inner(
            Some(expected_generation),
            samples,
            data_hash.as_ref(),
            snapshot_json.as_ref(),
        )
    }

    /// Descriptive alias for the optimistic-generation commit operation.
    pub fn commit_durable_state_with_expected_generation<H: AsRef<str>, J: AsRef<str>>(
        &mut self,
        expected_generation: u64,
        samples: &[UsageHistorySample],
        data_hash: H,
        snapshot_json: J,
    ) -> Result<DurableRecord> {
        self.commit_durable_state_if_generation(
            expected_generation,
            samples,
            data_hash,
            snapshot_json,
        )
    }

    /// Reads and validates the singleton durable snapshot, if one exists.
    pub fn load_durable_record(&self) -> Result<Option<DurableRecord>> {
        let raw: Option<(i64, String, String)> = self
            .connection
            .query_row(
                "SELECT data_generation, data_hash, snapshot_json \
                 FROM durable_state WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        raw.map(|(data_generation, data_hash, snapshot_json)| {
            durable_record_from_sql(data_generation, data_hash, snapshot_json)
        })
        .transpose()
    }

    /// Alias for callers that refer to the table as durable state.
    pub fn load_durable_state(&self) -> Result<Option<DurableRecord>> {
        self.load_durable_record()
    }

    /// Removes observations older than the exclusive UTC calendar-month cutoff.
    ///
    /// This is the only destructive operation in the store. The cutoff is
    /// strictly exclusive, so observations at the cutoff or in the future
    /// remain stored regardless of reset period.
    pub fn prune_older_than_three_months(&mut self, now: DateTime<Utc>) -> Result<usize> {
        let cutoff = three_months_before(now).timestamp();
        let transaction = self.connection.transaction()?;
        let deleted = transaction.execute(
            "DELETE FROM usage_history WHERE timestamp < ?1",
            params![cutoff],
        )?;
        transaction.commit()?;
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn database_path(test_name: &str) -> PathBuf {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir()
            .join(format!(
                "codex-info-usage-store-{test_name}-{}-{id}",
                std::process::id()
            ))
            .join("nested")
            .join("usage.sqlite3")
    }

    fn remove_database(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::remove_dir_all(parent.parent().unwrap_or(parent))
                .expect("failed to remove test database directory");
        }
    }

    fn sample(
        timestamp: i64,
        reset_at: i64,
        remaining_percent: Option<f64>,
        sol_dollars: f64,
    ) -> UsageHistorySample {
        UsageHistorySample {
            timestamp,
            reset_at,
            remaining_percent,
            sol_dollars,
            terra_dollars: 2.0,
            luna_dollars: 3.0,
            sol_tokens: 11,
            terra_tokens: 22,
            luna_tokens: 33,
        }
    }

    #[test]
    fn usage_store_reopen_persists_samples() {
        let path = database_path("reopen");
        let expected = sample(1_700_000_060, 1_700_604_800, Some(75.0), 1.25);

        {
            let store = UsageStore::open(&path).unwrap();
            store.upsert_sample(&expected).unwrap();
        }

        let actual = UsageStore::open(&path).unwrap().load_all().unwrap();
        assert_eq!(actual, vec![expected]);
        assert_eq!(actual[0].sol_tokens, 11);
        assert_eq!(actual[0].terra_tokens, 22);
        assert_eq!(actual[0].luna_tokens, 33);
        remove_database(&path);
    }

    #[test]
    fn opening_v1_database_adds_zero_token_columns_without_losing_dollars() {
        let path = database_path("v1-migration");
        {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let connection = Connection::open(&path).unwrap();
            connection
                .execute_batch(
                    "CREATE TABLE usage_history (timestamp INTEGER NOT NULL, reset_at INTEGER NOT NULL, remaining_percent REAL, sol_dollars REAL NOT NULL, terra_dollars REAL NOT NULL, luna_dollars REAL NOT NULL, PRIMARY KEY(reset_at, timestamp)); INSERT INTO usage_history VALUES (1700000060, 1700604800, 75.0, 1.25, 2.0, 3.0);",
                )
                .unwrap();
        }
        let rows = UsageStore::open(&path).unwrap().load_all().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].sol_dollars, 1.25);
        assert_eq!(rows[0].sol_tokens, 0);
        remove_database(&path);
    }

    #[test]
    fn usage_store_same_key_replaces_value() {
        let path = database_path("replacement");
        let first = sample(1_700_000_060, 1_700_604_800, Some(75.0), 1.25);
        let replacement = sample(1_700_000_060, 1_700_604_800, Some(60.0), 9.5);

        let store = UsageStore::open(&path).unwrap();
        store.upsert_sample(&first).unwrap();
        store.upsert_sample(&replacement).unwrap();

        assert_eq!(store.load_all().unwrap(), vec![replacement]);
        drop(store);
        remove_database(&path);
    }

    #[test]
    fn usage_store_upsert_keeps_existing_rows() {
        let path = database_path("append-only");
        let first_period = sample(1_700_000_060, 1_700_604_800, Some(75.0), 1.25);
        let second_period = sample(1_700_000_060, 1_701_209_600, Some(95.0), 8.0);

        let mut store = UsageStore::open(&path).unwrap();
        store.upsert_sample(&first_period).unwrap();
        store
            .upsert_samples(std::slice::from_ref(&second_period))
            .unwrap();

        assert_eq!(store.load_all().unwrap(), vec![first_period, second_period]);
        drop(store);
        remove_database(&path);
    }

    #[test]
    fn usage_store_import_is_additive_and_idempotent() {
        let path = database_path("import-idempotent");
        let first = sample(1_700_000_060, 1_700_604_800, Some(75.0), 1.25);
        let second = sample(1_700_000_120, 1_700_604_800, Some(70.0), 2.5);

        let mut store = UsageStore::open(&path).unwrap();
        assert_eq!(
            store
                .import_samples(&[first.clone(), second.clone()])
                .unwrap(),
            2
        );
        assert_eq!(
            store
                .import_samples(&[first.clone(), second.clone()])
                .unwrap(),
            2
        );
        assert_eq!(
            store.load_all().unwrap(),
            vec![first.clone(), second.clone()]
        );
        drop(store);

        assert_eq!(
            UsageStore::open(&path).unwrap().load_all().unwrap(),
            vec![first, second]
        );
        remove_database(&path);
    }

    #[test]
    fn usage_store_missing_remaining_does_not_erase_existing_value() {
        let path = database_path("nullable-update");
        let observed = sample(1_700_000_060, 1_700_604_800, Some(75.0), 1.25);
        let missing = sample(1_700_000_060, 1_700_604_800, None, 9.5);

        let store = UsageStore::open(&path).unwrap();
        store.upsert_sample(&observed).unwrap();
        store.upsert_sample(&missing).unwrap();

        let actual = store.load_all().unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].remaining_percent, Some(75.0));
        assert_eq!(actual[0].sol_dollars, 9.5);
        drop(store);
        remove_database(&path);
    }

    #[test]
    fn usage_store_smaller_cumulative_cost_does_not_erase_existing_value() {
        let path = database_path("cumulative-cost");
        let larger = sample(1_700_000_060, 1_700_604_800, Some(75.0), 9.5);
        let smaller = sample(1_700_000_060, 1_700_604_800, Some(74.0), 1.25);

        let store = UsageStore::open(&path).unwrap();
        store.upsert_sample(&larger).unwrap();
        store.upsert_sample(&smaller).unwrap();

        let actual = store.load_all().unwrap();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].remaining_percent, Some(74.0));
        assert_eq!(actual[0].sol_dollars, 9.5);
        drop(store);
        remove_database(&path);
    }

    #[test]
    fn usage_store_distinct_reset_periods_keep_same_timestamp() {
        let path = database_path("reset-periods");
        let first_period = sample(1_700_000_060, 1_700_604_800, Some(75.0), 1.25);
        let second_period = sample(1_700_000_060, 1_701_209_600, Some(95.0), 8.0);

        let store = UsageStore::open(&path).unwrap();
        store.upsert_sample(&first_period).unwrap();
        store.upsert_sample(&second_period).unwrap();

        assert_eq!(store.load_all().unwrap(), vec![first_period, second_period]);
        drop(store);
        remove_database(&path);
    }

    #[test]
    fn usage_store_nullable_remaining_quota_round_trips_as_sql_null() {
        let path = database_path("nullable");
        let expected = sample(1_700_000_060, 1_700_604_800, None, 1.25);

        {
            let store = UsageStore::open(&path).unwrap();
            store.upsert_sample(&expected).unwrap();
            let stored: Option<f64> = store
                .connection
                .query_row(
                    "SELECT remaining_percent FROM usage_history \
                     WHERE reset_at = ?1 AND timestamp = ?2",
                    params![expected.reset_at, expected.timestamp],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(stored, None);
        }

        assert_eq!(
            UsageStore::open(&path).unwrap().load_all().unwrap(),
            vec![expected]
        );
        remove_database(&path);
    }

    #[test]
    fn three_month_cutoff_clamps_end_of_month_by_calendar_rule() {
        let now = Utc.with_ymd_and_hms(2024, 5, 31, 12, 34, 56).unwrap();
        let expected = Utc.with_ymd_and_hms(2024, 2, 29, 12, 34, 56).unwrap();

        assert_eq!(three_months_before(now), expected);
    }

    #[test]
    fn pruning_removes_only_old_rows_and_preserves_boundary_across_reset_periods() {
        let path = database_path("prune");
        let now = Utc.with_ymd_and_hms(2024, 5, 31, 12, 34, 56).unwrap();
        let cutoff = 1_709_210_096_i64;
        let old = sample(cutoff - 1, 1_700_604_800, Some(10.0), 1.0);
        let old_other_period = sample(cutoff - 1, 1_701_209_600, Some(11.0), 1.1);
        let boundary = sample(cutoff, 1_700_604_800, Some(20.0), 2.0);
        let boundary_other_period = sample(cutoff, 1_701_209_600, Some(21.0), 2.1);
        let newer = sample(cutoff + 1, 1_701_814_400, Some(30.0), 3.0);
        let future = sample(now.timestamp() + 1, 1_701_814_400, Some(40.0), 4.0);

        let mut store = UsageStore::open(&path).unwrap();
        store
            .upsert_samples(&[
                old,
                old_other_period,
                boundary.clone(),
                boundary_other_period.clone(),
                newer.clone(),
                future.clone(),
            ])
            .unwrap();
        assert_eq!(store.prune_older_than_three_months(now).unwrap(), 2);
        assert_eq!(
            store.load_all().unwrap(),
            vec![
                boundary.clone(),
                boundary_other_period.clone(),
                newer.clone(),
                future.clone()
            ]
        );

        // Reopening must not perform another implicit destructive operation.
        drop(store);
        let mut reopened = UsageStore::open(&path).unwrap();
        assert_eq!(
            reopened.load_all().unwrap(),
            vec![boundary, boundary_other_period, newer, future]
        );
        assert_eq!(reopened.prune_older_than_three_months(now).unwrap(), 0);
        drop(reopened);
        remove_database(&path);
    }

    #[test]
    fn ordinary_upsert_never_prunes_old_rows() {
        let path = database_path("upsert-no-prune");
        let now = Utc.with_ymd_and_hms(2024, 5, 31, 12, 34, 56).unwrap();
        let cutoff = three_months_before(now).timestamp();
        let old = sample(cutoff - 1, 1_700_604_800, Some(10.0), 1.0);

        let store = UsageStore::open(&path).unwrap();
        store.upsert_sample(&old).unwrap();
        assert_eq!(store.load_all().unwrap(), vec![old]);
        drop(store);
        remove_database(&path);
    }

    #[cfg(unix)]
    #[test]
    fn storage_directory_and_database_modes_are_private() {
        use std::os::unix::fs::PermissionsExt;

        let path = database_path("private-modes");
        let store = UsageStore::open(&path).unwrap();
        drop(store);
        assert_eq!(
            fs::metadata(path.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        remove_database(&path);
    }

    #[cfg(unix)]
    #[test]
    fn database_symlink_relative_path_and_token_overflow_are_rejected() {
        use std::fs::File;
        use std::os::unix::fs::symlink;

        assert!(UsageStore::open(Path::new("relative.sqlite3")).is_err());
        let path = database_path("unsafe-paths");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let target = path.with_file_name("target.sqlite3");
        File::create(&target).unwrap();
        symlink(&target, &path).unwrap();
        assert!(UsageStore::open(&path).is_err());
        fs::remove_file(&path).unwrap();

        let store = UsageStore::open(&path).unwrap();
        let mut oversized = sample(100, 200, Some(50.0), 1.0);
        oversized.sol_tokens = i64::MAX as u64 + 1;
        assert!(store.upsert_sample(&oversized).is_err());
        drop(store);
        remove_database(&path);
    }
}
#[cfg(test)]
mod wave_b_correction_tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use rusqlite::{params, Connection, OptionalExtension};
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

    const VALID_HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn database_path(label: &str) -> PathBuf {
        let serial = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "codex-info-wave-b-{label}-{}-{serial}",
            std::process::id()
        ));
        assert!(!directory.exists(), "fixture directory unexpectedly exists");
        fs::create_dir(&directory).expect("create private fixture directory");
        #[cfg(unix)]
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
            .expect("make fixture directory private");
        directory.join("usage.sqlite")
    }

    fn cleanup(path: &Path) {
        if path.exists() {
            fs::remove_file(path).expect("remove fixture database");
        }
        for suffix in ["-wal", "-shm"] {
            let sidecar = PathBuf::from(format!("{}{}", path.display(), suffix));
            if sidecar.exists() {
                fs::remove_file(&sidecar).expect("remove fixture database sidecar");
            }
        }
        if let Some(parent) = path.parent() {
            fs::remove_dir(parent).expect("remove private fixture directory");
        }
    }

    fn sample_value(
        timestamp: i64,
        reset_at: i64,
        remaining_percent: Option<f64>,
        sol_dollars: f64,
        terra_dollars: f64,
        luna_dollars: f64,
    ) -> serde_json::Value {
        serde_json::json!({
            "timestamp": timestamp,
            "reset_at": reset_at,
            "remaining_percent": remaining_percent,
            "sol_dollars": sol_dollars,
            "terra_dollars": terra_dollars,
            "luna_dollars": luna_dollars,
            "input_tokens": 1,
            "cached_input_tokens": 1,
            "output_tokens": 1,
            "prompt_tokens": 1,
            "cache_read_tokens": 1,
            "completion_tokens": 1,
            "sol_input_tokens": 1,
            "sol_cached_input_tokens": 1,
            "sol_output_tokens": 1,
            "terra_input_tokens": 1,
            "terra_cached_input_tokens": 1,
            "terra_output_tokens": 1,
            "luna_input_tokens": 1,
            "luna_cached_input_tokens": 1,
            "luna_output_tokens": 1,
            "sol_tokens": 1,
            "terra_tokens": 1,
            "luna_tokens": 1,
            "tokens": 1
        })
    }

    fn sample(
        timestamp: i64,
        reset_at: i64,
        remaining_percent: Option<f64>,
        sol_dollars: f64,
    ) -> UsageHistorySample {
        json_sample(&sample_value(
            timestamp,
            reset_at,
            remaining_percent,
            sol_dollars,
            sol_dollars + 1.0,
            sol_dollars + 2.0,
        ))
        .expect("fixture must satisfy the legacy input contract")
    }

    fn overflowing_token_sample() -> UsageHistorySample {
        let mut value = sample_value(1_700_000_123, 1_700_000_000, Some(50.0), 1.0, 2.0, 3.0);
        let object = value
            .as_object_mut()
            .expect("sample fixture must be an object");
        for key in [
            "input_tokens",
            "cached_input_tokens",
            "output_tokens",
            "prompt_tokens",
            "cache_read_tokens",
            "completion_tokens",
            "sol_input_tokens",
            "sol_cached_input_tokens",
            "sol_output_tokens",
            "terra_input_tokens",
            "terra_cached_input_tokens",
            "terra_output_tokens",
            "luna_input_tokens",
            "luna_cached_input_tokens",
            "luna_output_tokens",
            "sol_tokens",
            "terra_tokens",
            "luna_tokens",
            "tokens",
        ] {
            object.insert(key.to_owned(), serde_json::json!(u64::MAX));
        }
        json_sample(&value).expect("token-overflow fixture must parse at the input boundary")
    }

    fn history_rows(path: &Path) -> Vec<(i64, i64, Option<f64>, f64, f64, f64)> {
        let connection = Connection::open(path).expect("history inspection connection");
        let mut statement = connection
            .prepare(
                "SELECT timestamp, reset_at, remaining_percent, sol_dollars, \
                        terra_dollars, luna_dollars \
                 FROM usage_history ORDER BY reset_at ASC, timestamp ASC",
            )
            .expect("history inspection query");
        statement
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            })
            .expect("history inspection rows")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("history inspection values")
    }

    fn durable_row(path: &Path) -> Option<(i64, String, String)> {
        let connection = Connection::open(path).expect("durable inspection connection");
        connection
            .query_row(
                "SELECT data_generation, data_hash, snapshot_json \
                 FROM durable_state WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .expect("durable inspection query")
    }

    fn singleton_count(path: &Path) -> i64 {
        let connection = Connection::open(path).expect("singleton inspection connection");
        connection
            .query_row("SELECT COUNT(*) FROM durable_state", [], |row| row.get(0))
            .expect("singleton inspection count")
    }

    fn reset_period_values(periods: &[ResetPeriod]) -> Vec<(i64, i64, i64)> {
        periods
            .iter()
            .map(|period| {
                (
                    period.canonical_id,
                    period.start_timestamp,
                    period.end_timestamp,
                )
            })
            .collect()
    }

    fn create_v1_database(path: &Path) {
        let connection = Connection::open(path).expect("v1 database connection");
        connection
            .execute_batch(
                "PRAGMA user_version = 1;
                CREATE TABLE usage_history (
                    timestamp INTEGER NOT NULL,
                    reset_at INTEGER NOT NULL,
                    remaining_percent REAL,
                    sol_dollars REAL NOT NULL,
                    terra_dollars REAL NOT NULL,
                    luna_dollars REAL NOT NULL,
                    PRIMARY KEY(reset_at, timestamp)
                );
                INSERT INTO usage_history
                    (timestamp, reset_at, remaining_percent, sol_dollars, terra_dollars, luna_dollars)
                VALUES (1700000060, 1700000000, 75.0, 1.25, 2.50, 3.75);",
            )
            .expect("create v1 fixture");
    }

    #[test]
    fn legacy_inputs_and_method_signatures_remain_usable() {
        let path = database_path("compatibility");
        let now = Utc.timestamp_opt(1_715_156_800, 0).single().unwrap();
        let legacy_input: UsageHistorySample = json_sample(&sample_value(
            1_715_156_700,
            1_715_156_000,
            Some(75.0),
            1.0,
            2.0,
            3.0,
        ))
        .unwrap();
        let mut store = UsageStore::open(&path).unwrap();
        store.upsert_sample(&legacy_input).unwrap();
        store
            .upsert_samples(std::slice::from_ref(&legacy_input))
            .unwrap();
        assert_eq!(store.load_recent_history(now).unwrap().len(), 1);
        assert_eq!(store.load_three_month_history(now).unwrap().len(), 1);
        assert_eq!(UsageStore::group_reset_periods(&[legacy_input]).len(), 1);
        drop(store);
        cleanup(&path);
    }

    #[test]
    fn recent_read_uses_independent_closed_interval_epochs_for_leap_and_non_leap_month_ends() {
        let cases = [
            // 2024-05-31T12:00:00Z -> 2024-02-29T12:00:00Z.
            (1_717_156_800_i64, 1_709_208_000_i64),
            // 2023-05-31T12:00:00Z -> 2023-02-28T12:00:00Z.
            (1_685_534_400_i64, 1_677_585_600_i64),
        ];
        for (case_number, (now_epoch, cutoff_epoch)) in cases.into_iter().enumerate() {
            let path = database_path(&format!("recent-{case_number}"));
            let now = Utc.timestamp_opt(now_epoch, 0).single().unwrap();
            let reset_at = 1_700_000_000 + case_number as i64;
            let mut store = UsageStore::open(&path).unwrap();
            store
                .upsert_samples(&[
                    sample(cutoff_epoch - 1, reset_at, Some(10.0), 1.0),
                    sample(cutoff_epoch, reset_at, Some(20.0), 2.0),
                    sample(cutoff_epoch + 1, reset_at, Some(30.0), 3.0),
                    sample(now_epoch - 1, reset_at, Some(40.0), 4.0),
                    sample(now_epoch, reset_at, Some(50.0), 5.0),
                    sample(now_epoch + 1, reset_at, Some(60.0), 6.0),
                ])
                .unwrap();
            let timestamps = store
                .load_recent_three_months(now)
                .unwrap()
                .into_iter()
                .map(|row| row.timestamp)
                .collect::<Vec<_>>();
            assert_eq!(
                timestamps,
                vec![cutoff_epoch, cutoff_epoch + 1, now_epoch - 1, now_epoch]
            );
            assert_eq!(history_rows(&path).len(), 6);
            drop(store);
            cleanup(&path);
        }
    }

    #[test]
    fn recent_read_filters_invalid_values_without_deleting_rows() {
        let path = database_path("recent-invalid");
        let now_epoch = 1_717_156_800_i64;
        let cutoff_epoch = 1_709_208_000_i64;
        let now = Utc.timestamp_opt(now_epoch, 0).single().unwrap();
        let store = UsageStore::open(&path).unwrap();
        store
            .upsert_sample(&sample(cutoff_epoch, 1_700_000_000, Some(50.0), 1.0))
            .unwrap();
        let connection = Connection::open(&path).unwrap();
        connection
            .execute(
                "INSERT INTO usage_history
                    (timestamp, reset_at, remaining_percent, sol_dollars, terra_dollars, luna_dollars)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![cutoff_epoch + 10, 1_700_000_010_i64, -1.0, 1.0, 2.0, 3.0],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO usage_history
                    (timestamp, reset_at, remaining_percent, sol_dollars, terra_dollars, luna_dollars)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![cutoff_epoch + 11, 1_700_000_011_i64, 101.0, 1.0, 2.0, 3.0],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO usage_history
                    (timestamp, reset_at, remaining_percent, sol_dollars, terra_dollars, luna_dollars)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![cutoff_epoch + 12, 1_700_000_012_i64, 50.0, -1.0, 2.0, 3.0],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO usage_history
                    (timestamp, reset_at, remaining_percent, sol_dollars, terra_dollars, luna_dollars)
                 VALUES (?1, ?2, 1e999, 1e999, 2.0, 3.0)",
                params![cutoff_epoch + 13, 1_700_000_013_i64],
            )
            .unwrap();
        drop(connection);
        assert_eq!(
            store
                .load_recent_three_months(now)
                .unwrap()
                .into_iter()
                .map(|row| row.timestamp)
                .collect::<Vec<_>>(),
            vec![cutoff_epoch]
        );
        assert_eq!(history_rows(&path).len(), 5);
        drop(store);
        cleanup(&path);
    }

    #[test]
    fn load_all_filters_negative_token_rows_without_coercion_or_deletion() {
        let path = database_path("load-all-negative-tokens");
        let valid_timestamp = 1_700_000_000_i64;
        let valid_reset_at = 1_700_000_100_i64;
        let store = UsageStore::open(&path).unwrap();
        store
            .upsert_sample(&sample(valid_timestamp, valid_reset_at, Some(50.0), 1.0))
            .unwrap();
        drop(store);

        let token_columns = ["sol_tokens", "terra_tokens", "luna_tokens"];
        let connection = Connection::open(&path).unwrap();
        for (offset, token_column) in token_columns.iter().enumerate() {
            let timestamp = valid_timestamp + offset as i64 + 1;
            let reset_at = valid_reset_at + offset as i64 + 1;
            let statement = format!(
                "INSERT INTO usage_history
                    (timestamp, reset_at, remaining_percent, sol_dollars, terra_dollars, luna_dollars, {token_column})
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
            );
            connection
                .execute(
                    &statement,
                    params![timestamp, reset_at, 50.0_f64, 1.0_f64, 2.0_f64, 3.0_f64, -1_i64],
                )
                .unwrap();
        }
        drop(connection);

        let store = UsageStore::open(&path).unwrap();
        let samples = store.load_all().unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].timestamp, valid_timestamp);
        drop(store);

        let connection = Connection::open(&path).unwrap();
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM usage_history", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 4);
        for (offset, token_column) in token_columns.iter().enumerate() {
            let timestamp = valid_timestamp + offset as i64 + 1;
            let statement =
                format!("SELECT {token_column} FROM usage_history WHERE timestamp = ?1");
            let value: i64 = connection
                .query_row(&statement, params![timestamp], |row| row.get(0))
                .unwrap();
            assert_eq!(value, -1);
        }
        drop(connection);
        cleanup(&path);
    }

    #[test]
    fn recent_read_filters_invalid_values_from_unchecked_v1_rows_without_deleting_rows() {
        let path = database_path("recent-invalid-v1");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "PRAGMA user_version = 1;
                CREATE TABLE usage_history (
                    timestamp INTEGER NOT NULL,
                    reset_at INTEGER NOT NULL,
                    remaining_percent REAL,
                    sol_dollars REAL NOT NULL,
                    terra_dollars REAL NOT NULL,
                    luna_dollars REAL NOT NULL,
                    PRIMARY KEY(reset_at, timestamp)
                );
                INSERT INTO usage_history
                    (timestamp, reset_at, remaining_percent, sol_dollars, terra_dollars, luna_dollars)
                VALUES (1700000060, 1700000000, 75.0, 1.25, 2.50, 3.75);",
            )
            .unwrap();
        drop(connection);
        let store = UsageStore::open(&path).unwrap();
        assert_eq!(store.load_all().unwrap().len(), 1);
        drop(store);

        let connection = Connection::open(&path).unwrap();
        connection.execute("DELETE FROM usage_history", []).unwrap();
        connection
            .execute(
                "INSERT INTO usage_history
                    (timestamp, reset_at, remaining_percent, sol_dollars, terra_dollars, luna_dollars)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![0_i64, 100_i64, 50.0, 1.0, 2.0, 3.0],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO usage_history
                    (timestamp, reset_at, remaining_percent, sol_dollars, terra_dollars, luna_dollars)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![900_i64, 0_i64, 50.0, 1.0, 2.0, 3.0],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO usage_history
                    (timestamp, reset_at, remaining_percent, sol_dollars, terra_dollars, luna_dollars, sol_tokens)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![800_i64, 100_i64, 50.0, 1.0, 2.0, 3.0, -1_i64],
            )
            .unwrap();
        let before_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM usage_history", [], |row| row.get(0))
            .unwrap();
        assert_eq!(before_count, 3);
        drop(connection);

        let store = UsageStore::open(&path).unwrap();
        let now = Utc.timestamp_opt(1_000, 0).single().unwrap();
        assert!(store.load_recent_three_months(now).unwrap().is_empty());
        drop(store);

        let connection = Connection::open(&path).unwrap();
        let after_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM usage_history", [], |row| row.get(0))
            .unwrap();
        assert_eq!(after_count, 3);
        drop(connection);
        cleanup(&path);
    }

    #[test]
    fn grouping_has_sixty_second_boundary_canonical_ids_and_explicit_order() {
        let samples = vec![
            sample(100, 1_000, Some(1.0), 1.0),
            sample(200, 1_060, Some(2.0), 2.0),
            sample(300, 1_061, Some(3.0), 3.0),
        ];
        assert_eq!(
            reset_period_values(&group_reset_periods(&samples)),
            vec![(1_061, 300, 1_061), (1_060, 100, 300)]
        );
    }

    #[test]
    fn grouping_handles_same_timestamp_periods_mid_week_and_permutation_invariance() {
        let samples = vec![
            sample(604_700, 604_800, Some(1.0), 1.0),
            sample(604_750, 604_805, Some(2.0), 2.0),
            sample(604_900, 605_000, Some(3.0), 3.0),
            sample(605_100, 605_000, Some(4.0), 4.0),
            sample(605_200, 605_100, Some(5.0), 5.0),
            sample(605_200, 605_300, Some(6.0), 6.0),
        ];
        let expected = vec![
            (605_300, 605_200, 605_300),
            (605_100, 605_200, 605_100),
            (605_000, 604_900, 605_000),
            (604_805, 604_700, 604_805),
        ];
        assert_eq!(
            reset_period_values(&group_reset_periods(&samples)),
            expected
        );
        let mut permutation = samples.clone();
        permutation.reverse();
        assert_eq!(
            group_reset_periods(&samples),
            group_reset_periods(&permutation)
        );
    }

    #[test]
    fn grouping_orders_equal_starts_by_canonical_id_descending() {
        let samples = vec![
            sample(100, 1_000, Some(1.0), 1.0),
            sample(300, 2_000, Some(2.0), 2.0),
            sample(300, 2_061, Some(3.0), 3.0),
            sample(300, 2_122, Some(4.0), 4.0),
        ];
        assert_eq!(
            reset_period_values(&group_reset_periods(&samples)),
            vec![
                (2_122, 300, 2_122),
                (2_061, 300, 300),
                (2_000, 300, 300),
                (1_000, 100, 300)
            ]
        );
    }

    #[test]
    fn json_import_rejects_present_invalid_tokens_without_default_or_partial_write() {
        let mut missing_token_value =
            sample_value(1_700_000_010, 1_700_000_000, Some(80.0), 1.0, 2.0, 3.0);
        let missing_token_object = missing_token_value
            .as_object_mut()
            .expect("sample fixture must be an object");
        for name in ["sol_tokens", "terra_tokens", "luna_tokens"] {
            missing_token_object.remove(name);
        }
        let missing_tokens =
            json_sample(&missing_token_value).expect("missing token fields remain accepted");
        assert_eq!(missing_tokens.sol_tokens, 0);
        assert_eq!(missing_tokens.terra_tokens, 0);
        assert_eq!(missing_tokens.luna_tokens, 0);

        let path = database_path("json-invalid-token-import");
        let import_path = database_path("json-invalid-token-import-json");
        let mut seed = UsageStore::open(&path).unwrap();
        seed.import_samples(&[sample(1_700_000_000, 1_700_000_000, Some(90.0), 1.0)])
            .unwrap();
        drop(seed);
        let baseline = history_rows(&path);
        let invalid_cases = vec![
            ("sol_tokens", serde_json::json!(-1)),
            ("terra_tokens", serde_json::json!(-1)),
            ("luna_tokens", serde_json::json!(-1)),
            ("sol_tokens", serde_json::Value::Null),
            ("sol_tokens", serde_json::json!("not-a-number")),
            ("sol_tokens", serde_json::json!(false)),
            ("sol_tokens", serde_json::json!(1.5)),
        ];

        for (index, (name, invalid_value)) in invalid_cases.into_iter().enumerate() {
            let valid_timestamp = 1_700_001_000 + (index as i64 * 2);
            let invalid_timestamp = valid_timestamp + 1;
            let valid = sample_value(valid_timestamp, 1_700_001_000, Some(80.0), 9.0, 10.0, 11.0);
            let mut invalid = sample_value(
                invalid_timestamp,
                1_700_001_000,
                Some(70.0),
                12.0,
                13.0,
                14.0,
            );
            invalid
                .as_object_mut()
                .expect("sample fixture must be an object")
                .insert(name.to_owned(), invalid_value);
            fs::write(
                &import_path,
                serde_json::to_vec(&serde_json::json!([valid, invalid])).unwrap(),
            )
            .unwrap();

            let mut store = UsageStore::open(&path).unwrap();
            let error = store
                .import_v1_json(&import_path)
                .expect_err("present invalid token must reject the whole import");
            assert!(matches!(
                error,
                UsageStoreError::InvalidImport(message)
                    if message == format!("{name} must be a non-negative integer")
            ));
            drop(store);

            let after = history_rows(&path);
            assert_eq!(after, baseline);
            assert!(!after
                .iter()
                .any(|row| row.0 == valid_timestamp || row.0 == invalid_timestamp));
        }

        cleanup(&path);
        cleanup(&import_path);
    }

    #[test]
    fn sqlite_import_rejects_lossy_or_invalid_rows_without_partial_write() {
        assert_eq!(
            numeric_sqlite_value(Value::Integer(9_007_199_254_740_992)),
            Some(9_007_199_254_740_992.0)
        );
        assert_eq!(
            numeric_sqlite_value(Value::Integer(9_007_199_254_740_993)),
            None
        );
        assert_eq!(numeric_sqlite_value(Value::Integer(i64::MAX)), None);
        assert!(matches!(
            numeric_sqlite_value(Value::Real(f64::NAN)),
            Some(value) if value.is_nan()
        ));

        let target_path = database_path("sqlite-invalid-import-target");
        let mut target = UsageStore::open(&target_path).unwrap();
        target
            .import_samples(&[sample(1_700_000_000, 1_700_000_000, Some(90.0), 1.0)])
            .unwrap();
        drop(target);
        let baseline = history_rows(&target_path);
        let invalid_cases = [
            "lossy integer dollar",
            "negative timestamp",
            "remaining above 100",
            "negative dollar",
        ];
        let mut source_paths = Vec::new();

        for (index, invalid_case) in invalid_cases.iter().enumerate() {
            let source_path = database_path(&format!("sqlite-invalid-import-source-{index}"));
            let source = Connection::open(&source_path).unwrap();
            source
                .execute_batch(
                    "CREATE TABLE usage_history (
                        timestamp,
                        reset_at,
                        remaining_percent,
                        sol_dollars,
                        terra_dollars,
                        luna_dollars
                    )",
                )
                .unwrap();
            let valid_timestamp = 1_700_001_000 + (index as i64 * 10);
            let valid_reset_at = 1_700_000_000;
            source
                .execute(
                    "INSERT INTO usage_history
                        (timestamp, reset_at, remaining_percent, sol_dollars,
                         terra_dollars, luna_dollars)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        valid_timestamp,
                        valid_reset_at,
                        80.0_f64,
                        9.0_f64,
                        10.0_f64,
                        11.0_f64
                    ],
                )
                .unwrap();
            match *invalid_case {
                "lossy integer dollar" => {
                    source
                        .execute(
                            "INSERT INTO usage_history
                                (timestamp, reset_at, remaining_percent, sol_dollars,
                                 terra_dollars, luna_dollars)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                            rusqlite::params![
                                valid_timestamp + 1,
                                valid_reset_at + 1,
                                70.0_f64,
                                9_007_199_254_740_993_i64,
                                13.0_f64,
                                14.0_f64
                            ],
                        )
                        .unwrap();
                }
                "negative timestamp" => {
                    source
                        .execute(
                            "INSERT INTO usage_history
                                (timestamp, reset_at, remaining_percent, sol_dollars,
                                 terra_dollars, luna_dollars)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                            rusqlite::params![
                                -1_i64,
                                valid_reset_at + 1,
                                70.0_f64,
                                12.0_f64,
                                13.0_f64,
                                14.0_f64
                            ],
                        )
                        .unwrap();
                }
                "remaining above 100" => {
                    source
                        .execute(
                            "INSERT INTO usage_history
                                (timestamp, reset_at, remaining_percent, sol_dollars,
                                 terra_dollars, luna_dollars)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                            rusqlite::params![
                                valid_timestamp + 1,
                                valid_reset_at + 1,
                                101.0_f64,
                                12.0_f64,
                                13.0_f64,
                                14.0_f64
                            ],
                        )
                        .unwrap();
                }
                "negative dollar" => {
                    source
                        .execute(
                            "INSERT INTO usage_history
                                (timestamp, reset_at, remaining_percent, sol_dollars,
                                 terra_dollars, luna_dollars)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                            rusqlite::params![
                                valid_timestamp + 1,
                                valid_reset_at + 1,
                                70.0_f64,
                                -1.0_f64,
                                13.0_f64,
                                14.0_f64
                            ],
                        )
                        .unwrap();
                }
                _ => unreachable!("fixed invalid case"),
            }
            drop(source);
            source_paths.push(source_path.clone());

            let mut store = UsageStore::open(&target_path).unwrap();
            let error = store
                .import_v1_sqlite(&source_path)
                .expect_err("invalid v1 SQLite sample must reject the whole import");
            assert!(matches!(
                error,
                UsageStoreError::InvalidImport(message)
                    if message == "invalid v1 SQLite sample"
            ));
            drop(store);

            let after = history_rows(&target_path);
            assert_eq!(after, baseline);
            assert!(!after
                .iter()
                .any(|row| row.0 == valid_timestamp || row.0 == valid_timestamp + 1));
        }

        for source_path in source_paths {
            cleanup(&source_path);
        }
        cleanup(&target_path);
    }

    #[test]
    fn v1_open_import_reopen_and_singleton_are_non_destructive_and_idempotent() {
        let path = database_path("migration");
        let import_path = database_path("migration-json");
        create_v1_database(&path);
        fs::write(
            &import_path,
            serde_json::to_vec(&serde_json::json!({
                "samples": [sample_value(1_700_000_120, 1_700_000_000, Some(80.0), 9.0, 10.0, 11.0)]
            }))
            .unwrap(),
        )
        .unwrap();
        let store = UsageStore::open(&path).unwrap();
        let original = history_rows(&path);
        assert_eq!(original.len(), 1);
        let mut store = store;
        assert_eq!(store.import_v1_json(&import_path).unwrap(), 1);
        let after_first_import = history_rows(&path);
        assert_eq!(after_first_import.len(), 2);
        assert_eq!(after_first_import[0].3, original[0].3);
        assert_eq!(after_first_import[0].4, original[0].4);
        assert_eq!(after_first_import[0].5, original[0].5);
        assert_eq!(store.import_v1_json(&import_path).unwrap(), 1);
        assert_eq!(history_rows(&path), after_first_import);
        drop(store);
        for _ in 0..3 {
            let reopened = UsageStore::open(&path).unwrap();
            assert_eq!(history_rows(&path), after_first_import);
            drop(reopened);
        }
        let mut reopened = UsageStore::open(&path).unwrap();
        let record = reopened
            .commit_durable_state(&[], VALID_HASH, r#"{"kind":"migration"}"#)
            .unwrap();
        assert_eq!(record.data_generation, 1);
        assert_eq!(singleton_count(&path), 1);
        drop(reopened);
        let reopened_again = UsageStore::open(&path).unwrap();
        assert_eq!(history_rows(&path), after_first_import);
        assert_eq!(
            durable_row(&path),
            Some((
                1,
                VALID_HASH.to_owned(),
                r#"{"kind":"migration"}"#.to_owned()
            ))
        );
        drop(reopened_again);
        cleanup(&path);
        cleanup(&import_path);
    }

    #[test]
    fn corrupt_migration_error_preserves_the_original_file() {
        let path = database_path("corrupt");
        let bytes = b"this is not a sqlite database".to_vec();
        fs::write(&path, &bytes).unwrap();
        assert!(UsageStore::open(&path).is_err());
        assert!(path.exists());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        cleanup(&path);
    }

    #[test]
    fn durable_commit_is_one_transaction_and_is_visible_to_a_separate_connection() {
        let path = database_path("commit");
        let committed = sample(1_700_000_123, 1_700_000_000, Some(64.0), 1.5);
        let mut store = UsageStore::open(&path).unwrap();
        let record = store
            .commit_durable_state(
                std::slice::from_ref(&committed),
                VALID_HASH,
                r#"{"ok":true}"#,
            )
            .unwrap();
        assert_eq!(record.data_generation, 1);
        assert_eq!(
            history_rows(&path),
            vec![(
                committed.timestamp,
                committed.reset_at,
                committed.remaining_percent,
                committed.sol_dollars,
                committed.terra_dollars,
                committed.luna_dollars
            )]
        );
        assert_eq!(
            durable_row(&path),
            Some((1, VALID_HASH.to_owned(), r#"{"ok":true}"#.to_owned()))
        );
        assert_eq!(singleton_count(&path), 1);
        drop(store);
        let reopened = UsageStore::open(&path).unwrap();
        assert_eq!(reopened.load_durable_state().unwrap().unwrap(), record);
        drop(reopened);
        cleanup(&path);
    }

    #[test]
    fn validation_conflict_overflow_and_sql_failures_leave_prior_state_unchanged() {
        let path = database_path("rollback");
        let baseline = sample(1_700_000_100, 1_700_000_000, Some(70.0), 7.0);
        let mut store = UsageStore::open(&path).unwrap();
        store
            .commit_durable_state(
                std::slice::from_ref(&baseline),
                VALID_HASH,
                r#"{"generation":1}"#,
            )
            .unwrap();
        let prior_history = history_rows(&path);
        let prior_durable = durable_row(&path);

        let invalid_row = sample(1_700_000_101, 0, Some(60.0), 6.0);
        assert!(store
            .commit_durable_state(
                &[baseline.clone(), invalid_row],
                "f".repeat(64),
                r#"{"generation":2}"#,
            )
            .is_err());
        assert_eq!(history_rows(&path), prior_history);
        assert_eq!(durable_row(&path), prior_durable);

        for invalid_hash in ["A".repeat(64), "a".repeat(63), "g".repeat(64)] {
            assert!(store
                .commit_durable_state(
                    std::slice::from_ref(&baseline),
                    invalid_hash,
                    r#"{"generation":2}"#,
                )
                .is_err());
            assert_eq!(history_rows(&path), prior_history);
            assert_eq!(durable_row(&path), prior_durable);
        }

        let oversized_json = "x".repeat(MAX_SNAPSHOT_JSON_BYTES + 1);
        for invalid_json in ["{", "[]"] {
            assert!(store
                .commit_durable_state(std::slice::from_ref(&baseline), VALID_HASH, invalid_json,)
                .is_err());
            assert_eq!(history_rows(&path), prior_history);
            assert_eq!(durable_row(&path), prior_durable);
        }
        assert!(store
            .commit_durable_state(std::slice::from_ref(&baseline), VALID_HASH, &oversized_json,)
            .is_err());
        assert_eq!(history_rows(&path), prior_history);
        assert_eq!(durable_row(&path), prior_durable);

        assert!(store
            .commit_durable_state_if_generation(0, &[], VALID_HASH, r#"{"generation":2}"#,)
            .is_err());
        assert_eq!(history_rows(&path), prior_history);
        assert_eq!(durable_row(&path), prior_durable);

        let connection = Connection::open(&path).unwrap();
        connection
            .execute(
                "UPDATE durable_state SET data_generation = ?1 WHERE singleton = 1",
                params![i64::MAX],
            )
            .unwrap();
        drop(connection);
        let overflow_history = history_rows(&path);
        let overflow_durable = durable_row(&path);
        assert!(store
            .commit_durable_state_if_generation(
                u64::try_from(i64::MAX).unwrap(),
                std::slice::from_ref(&baseline),
                VALID_HASH,
                r#"{"generation":"overflow"}"#,
            )
            .is_err());
        assert_eq!(history_rows(&path), overflow_history);
        assert_eq!(durable_row(&path), overflow_durable);

        drop(store);
        cleanup(&path);
    }

    #[test]
    fn durable_update_trigger_rolls_back_history_and_durable_state() {
        let path = database_path("durable-update-trigger");
        let mut store = UsageStore::open(&path).unwrap();
        let baseline = sample(1_700_000_100, 1_700_000_000, Some(50.0), 1.0);
        store
            .commit_durable_state(
                std::slice::from_ref(&baseline),
                VALID_HASH,
                r#"{"generation":1}"#,
            )
            .unwrap();
        let captured_history = history_rows(&path);
        let captured_durable = durable_row(&path);

        let trigger_connection = Connection::open(&path).unwrap();
        trigger_connection
            .execute_batch(
                "CREATE TRIGGER wave_b_fail_durable_update
                 BEFORE UPDATE ON durable_state
                 BEGIN SELECT RAISE(ABORT, 'wave-b fault'); END;",
            )
            .unwrap();
        drop(trigger_connection);

        assert!(store
            .commit_durable_state(
                &[sample(1_700_000_200, 1_700_000_000, Some(55.0), 5.5)],
                VALID_HASH,
                r#"{"generation":2}"#,
            )
            .is_err());
        assert_eq!(history_rows(&path), captured_history);
        assert_eq!(durable_row(&path), captured_durable);

        let trigger_connection = Connection::open(&path).unwrap();
        trigger_connection
            .execute_batch("DROP TRIGGER wave_b_fail_durable_update")
            .unwrap();
        drop(trigger_connection);
        drop(store);

        let reopened = UsageStore::open(&path).unwrap();
        assert_eq!(history_rows(&path), captured_history);
        assert_eq!(durable_row(&path), captured_durable);
        drop(reopened);
        cleanup(&path);
    }

    #[test]
    fn storage_focus11_v1_migration_failure_rolls_back_all_schema_and_rows() {
        let path = database_path("storage-focus11-v1-migration-failure");
        let fixture = Connection::open(&path).unwrap();
        fixture
            .execute_batch(
                "CREATE TABLE usage_history (
                    timestamp INTEGER NOT NULL,
                    reset_at INTEGER NOT NULL,
                    remaining_percent REAL,
                    sol_dollars REAL NOT NULL,
                    terra_dollars REAL NOT NULL,
                    luna_dollars REAL NOT NULL,
                    TERRA_TOKENS INTEGER NOT NULL DEFAULT 0,
                    PRIMARY KEY (reset_at, timestamp)
                );
                INSERT INTO usage_history (
                    timestamp, reset_at, remaining_percent,
                    sol_dollars, terra_dollars, luna_dollars, TERRA_TOKENS
                ) VALUES (1700000060, 1700000000, 75.0, 1.25, 2.50, 3.75, 11);",
            )
            .unwrap();
        drop(fixture);

        assert!(UsageStore::open(&path).is_err());

        let raw = Connection::open(&path).unwrap();
        let mut column_statement = raw
            .prepare("SELECT name FROM pragma_table_info('usage_history') ORDER BY cid")
            .unwrap();
        let columns = column_statement
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .map(|row| row.unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            columns,
            vec![
                "timestamp".to_owned(),
                "reset_at".to_owned(),
                "remaining_percent".to_owned(),
                "sol_dollars".to_owned(),
                "terra_dollars".to_owned(),
                "luna_dollars".to_owned(),
                "TERRA_TOKENS".to_owned(),
            ]
        );

        let lowercase_terra_probe: bool = raw
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM pragma_table_info('usage_history')
                    WHERE name = 'terra_tokens'
                )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!lowercase_terra_probe);
        let sol_probe: bool = raw
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM pragma_table_info('usage_history')
                    WHERE name = 'sol_tokens'
                )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!sol_probe);

        let original_row: (i64, i64, Option<f64>, f64, f64, f64, i64) = raw
            .query_row(
                "SELECT timestamp, reset_at, remaining_percent,
                        sol_dollars, terra_dollars, luna_dollars, TERRA_TOKENS
                 FROM usage_history",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(
            original_row,
            (1700000060, 1700000000, Some(75.0), 1.25, 2.50, 3.75, 11)
        );

        let mut schema_statement = raw
            .prepare(
                "SELECT type, name FROM sqlite_master
                 WHERE name IN ('usage_history_timestamp_idx', 'durable_state')
                 ORDER BY name",
            )
            .unwrap();
        let schema_objects = schema_statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
            .map(|row| row.unwrap())
            .collect::<Vec<_>>();
        assert!(schema_objects.is_empty());
        drop(column_statement);
        drop(schema_statement);
        drop(raw);
        cleanup(&path);
    }

    #[test]
    fn storage_focus11_durable_absence_and_malformed_presence_are_distinct() {
        let empty_path = database_path("storage-focus11-durable-empty");
        let mut empty_store = UsageStore::open(&empty_path).unwrap();
        assert_eq!(singleton_count(&empty_path), 0);
        assert_eq!(empty_store.load_durable_state().unwrap(), None);
        let empty_record = empty_store
            .commit_durable_state_if_generation(0, &[], VALID_HASH, r#"{"kind":"empty"}"#)
            .unwrap();
        assert_eq!(empty_record.data_generation, 1);
        assert_eq!(
            durable_row(&empty_path),
            Some((1, VALID_HASH.to_owned(), r#"{"kind":"empty"}"#.to_owned()))
        );
        drop(empty_store);
        let reopened_empty = UsageStore::open(&empty_path).unwrap();
        assert_eq!(
            reopened_empty.load_durable_state().unwrap(),
            Some(empty_record)
        );
        drop(reopened_empty);
        cleanup(&empty_path);

        for (label, generation, data_hash, snapshot_json, ignore_check_constraints) in [
            (
                "negative-generation",
                -1_i64,
                VALID_HASH,
                r#"{"kind":"negative"}"#,
                true,
            ),
            (
                "invalid-hash",
                1_i64,
                "not-a-valid-hash",
                r#"{"kind":"invalid-hash"}"#,
                false,
            ),
            ("non-object-json", 1_i64, VALID_HASH, "[]", false),
        ] {
            let path = database_path(&format!("storage-focus11-durable-{label}"));
            let fixture = Connection::open(&path).unwrap();
            fixture
                .execute_batch(
                    "CREATE TABLE usage_history (
                        timestamp INTEGER NOT NULL CHECK (timestamp > 0),
                        reset_at INTEGER NOT NULL CHECK (reset_at > 0),
                        remaining_percent REAL,
                        sol_dollars REAL NOT NULL,
                        terra_dollars REAL NOT NULL,
                        luna_dollars REAL NOT NULL,
                        sol_tokens INTEGER NOT NULL DEFAULT 0,
                        terra_tokens INTEGER NOT NULL DEFAULT 0,
                        luna_tokens INTEGER NOT NULL DEFAULT 0,
                        PRIMARY KEY (reset_at, timestamp)
                    );
                    CREATE TABLE durable_state (
                        singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                        data_generation INTEGER NOT NULL CHECK (data_generation >= 0),
                        data_hash TEXT NOT NULL,
                        snapshot_json TEXT NOT NULL
                    );
                    INSERT INTO usage_history (
                        timestamp, reset_at, remaining_percent,
                        sol_dollars, terra_dollars, luna_dollars,
                        sol_tokens, terra_tokens, luna_tokens
                    ) VALUES (1700000010, 1700000000, 77.0, 1.25, 2.50, 3.75, 10, 20, 30);",
                )
                .unwrap();
            if ignore_check_constraints {
                fixture
                    .execute_batch("PRAGMA ignore_check_constraints = ON;")
                    .unwrap();
            }
            fixture
                .execute(
                    "INSERT INTO durable_state
                        (singleton, data_generation, data_hash, snapshot_json)
                     VALUES (1, ?1, ?2, ?3)",
                    params![generation, data_hash, snapshot_json],
                )
                .unwrap();
            if ignore_check_constraints {
                fixture
                    .execute_batch("PRAGMA ignore_check_constraints = OFF;")
                    .unwrap();
            }
            drop(fixture);

            let store = UsageStore::open(&path).unwrap();
            assert!(store.load_durable_state().is_err());
            drop(store);

            assert_eq!(
                history_rows(&path),
                vec![(1700000010, 1700000000, Some(77.0), 1.25, 2.50, 3.75,)]
            );
            assert_eq!(
                durable_row(&path),
                Some((generation, data_hash.to_owned(), snapshot_json.to_owned()))
            );
            cleanup(&path);
        }
    }

    #[test]
    fn storage_focus11_first_insert_failure_rolls_back_history_and_durable() {
        let path = database_path("storage-focus11-first-insert-failure");
        let mut store = UsageStore::open(&path).unwrap();
        assert!(history_rows(&path).is_empty());
        assert_eq!(singleton_count(&path), 0);

        let trigger_connection = Connection::open(&path).unwrap();
        trigger_connection
            .execute_batch(
                "CREATE TRIGGER wave_b_fail_durable_insert
                 BEFORE INSERT ON durable_state
                 BEGIN SELECT RAISE(ABORT, 'wave-b first insert fault'); END;",
            )
            .unwrap();
        drop(trigger_connection);

        let candidate = sample(1_700_000_200, 1_700_000_000, Some(55.0), 5.5);
        assert!(store
            .commit_durable_state(
                std::slice::from_ref(&candidate),
                VALID_HASH,
                r#"{"generation":1}"#,
            )
            .is_err());
        assert!(history_rows(&path).is_empty());
        assert_eq!(singleton_count(&path), 0);

        drop(store);
        let reopened = UsageStore::open(&path).unwrap();
        assert!(history_rows(&path).is_empty());
        assert_eq!(singleton_count(&path), 0);
        drop(reopened);
        cleanup(&path);
    }

    #[test]
    fn invalid_input_boundaries_cover_remaining_dollars_and_token_sql_limits() {
        let path = database_path("input-boundaries");
        let mut store = UsageStore::open(&path).unwrap();
        for (index, invalid) in [
            sample(1_700_000_001, 0, Some(50.0), 1.0),
            sample(1_700_000_002, -1, Some(50.0), 1.0),
        ]
        .into_iter()
        .enumerate()
        {
            assert!(
                store
                    .upsert_samples(std::slice::from_ref(&invalid))
                    .is_err(),
                "invalid fixture {index}"
            );
        }
        for (index, invalid) in [
            sample(1_700_000_003, 1_700_000_000, Some(-1.0), 1.0),
            sample(1_700_000_004, 1_700_000_000, Some(101.0), 1.0),
        ]
        .into_iter()
        .enumerate()
        {
            assert!(
                store.upsert_sample(&invalid).is_err(),
                "single-row remaining_percent fixture {index}"
            );
            assert!(
                store
                    .upsert_samples(std::slice::from_ref(&invalid))
                    .is_err(),
                "batch remaining_percent fixture {index}"
            );
        }
        for (index, field) in ["sol_dollars", "terra_dollars", "luna_dollars"]
            .into_iter()
            .enumerate()
        {
            let mut invalid = sample(1_700_000_005 + index as i64, 1_700_000_000, Some(50.0), 1.0);
            match field {
                "sol_dollars" => invalid.sol_dollars = -1.0,
                "terra_dollars" => invalid.terra_dollars = -1.0,
                "luna_dollars" => invalid.luna_dollars = -1.0,
                _ => unreachable!(),
            }
            assert!(
                store.upsert_sample(&invalid).is_err(),
                "single-row negative {field} fixture"
            );
            assert!(
                store
                    .upsert_samples(std::slice::from_ref(&invalid))
                    .is_err(),
                "batch negative {field} fixture"
            );
        }
        let valid = sample(1_700_000_010, 1_700_000_000, Some(50.0), 1.0);
        let mut invalid = valid.clone();
        invalid.timestamp += 1;
        invalid.sol_dollars = -1.0;
        assert!(store.upsert_samples(&[valid, invalid]).is_err());
        assert!(history_rows(&path).is_empty());
        let overflowing = overflowing_token_sample();
        assert!(store
            .upsert_samples(std::slice::from_ref(&overflowing))
            .is_err());
        assert!(history_rows(&path).is_empty());
        drop(store);
        cleanup(&path);
    }

    #[test]
    fn ordinary_upsert_and_range_read_are_non_pruning() {
        let path = database_path("non-pruning");
        let now = Utc.timestamp_opt(1_715_156_800, 0).single().unwrap();
        let store = UsageStore::open(&path).unwrap();
        let old = sample(1_600_000_000, 1_600_000_100, Some(10.0), 1.0);
        store.upsert_sample(&old).unwrap();
        let before = history_rows(&path);
        assert!(store.load_recent_three_months(now).unwrap().is_empty());
        store
            .upsert_sample(&sample(1_715_156_700, 1_715_156_000, Some(20.0), 2.0))
            .unwrap();
        assert_eq!(history_rows(&path).len(), before.len() + 1);
        drop(store);
        cleanup(&path);
    }

    #[test]
    fn storage_focus11_sql_rows_use_one_semantic_validator() {
        use chrono::TimeZone;

        let target_path = database_path("storage-focus11-sql-rows-one-validator");
        let mut store = UsageStore::open(&target_path).unwrap();
        let fixture = Connection::open(&target_path).unwrap();
        fixture
            .execute_batch(
                "PRAGMA ignore_check_constraints = ON;
                 INSERT INTO usage_history (
                    timestamp, reset_at, remaining_percent,
                    sol_dollars, terra_dollars, luna_dollars,
                    sol_tokens, terra_tokens, luna_tokens
                 ) VALUES
                    (1715156700, 1715156800, 50.0, 1.0, 2.0, 3.0, 7, 8, 9),
                    (1715156701, 1715156800, -1.0, 1.0, 2.0, 3.0, 0, 0, 0),
                    (1715156702, 1715156800, 101.0, 1.0, 2.0, 3.0, 0, 0, 0),
                    (1715156703, 1715156800, 50.0, -1.0, 2.0, 3.0, 0, 0, 0),
                    (1715156704, 1715156800, 50.0, 1.0, -2.0, 3.0, 0, 0, 0),
                    (1715156705, 1715156800, 50.0, 1.0, 2.0, -3.0, 0, 0, 0),
                    (0, 1715156800, 50.0, 1.0, 2.0, 3.0, 0, 0, 0),
                    (-1, 1715156800, 50.0, 1.0, 2.0, 3.0, 0, 0, 0),
                    (1715156706, 0, 50.0, 1.0, 2.0, 3.0, 0, 0, 0),
                    (1715156707, -1, 50.0, 1.0, 2.0, 3.0, 0, 0, 0);",
            )
            .unwrap();
        drop(fixture);

        let expected = UsageHistorySample {
            timestamp: 1715156700,
            reset_at: 1715156800,
            remaining_percent: Some(50.0),
            sol_dollars: 1.0,
            terra_dollars: 2.0,
            luna_dollars: 3.0,
            sol_tokens: 7,
            terra_tokens: 8,
            luna_tokens: 9,
        };
        assert_eq!(store.load_all().unwrap(), vec![expected.clone()]);
        let now = Utc.timestamp_opt(1715156800, 0).single().unwrap();
        assert_eq!(store.load_recent_three_months(now).unwrap(), vec![expected]);

        let converter = include_str!("usage_store.rs")
            .split("fn valid_sample_from_row")
            .nth(1)
            .unwrap()
            .split("fn validate_data_hash")
            .next()
            .unwrap();
        let validate_at = converter.find("sample.validate()").unwrap();
        let before_validate = &converter[..validate_at];
        let before_validate_compact: String = before_validate
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect();
        assert_eq!(converter.matches("sample.validate()").count(), 1);
        assert!(!before_validate_compact.contains("timestamp<"));
        assert!(!before_validate_compact.contains("timestamp>"));
        assert!(!before_validate_compact.contains("reset_at<"));
        assert!(!before_validate_compact.contains("reset_at>"));
        assert!(!before_validate.contains("if value > 0"));
        assert!(!before_validate.contains("if value < 0"));
        assert!(!before_validate.contains("value >= 0.0"));
        assert!(!before_validate.contains("value < 0.0"));
        assert!(!before_validate.contains("0.0..=100.0"));
        assert!(!before_validate.contains("is_finite()"));

        let sql_rows = |path: &std::path::Path| {
            let connection = Connection::open(path).unwrap();
            let mut statement = connection
                .prepare(
                    "SELECT timestamp, reset_at, remaining_percent,
                            sol_dollars, terra_dollars, luna_dollars,
                            sol_tokens, terra_tokens, luna_tokens
                     FROM usage_history ORDER BY reset_at, timestamp",
                )
                .unwrap();
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<f64>>(2)?,
                        row.get::<_, f64>(3)?,
                        row.get::<_, f64>(4)?,
                        row.get::<_, f64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, i64>(8)?,
                    ))
                })
                .unwrap()
                .map(|row| row.unwrap())
                .collect::<Vec<_>>()
        };
        let before_import = sql_rows(&target_path);

        let source_path = database_path("storage-focus11-sql-rows-one-validator-source");
        let source = Connection::open(&source_path).unwrap();
        source
            .execute_batch(
                "CREATE TABLE usage_history (
                    timestamp INTEGER,
                    reset_at INTEGER,
                    remaining_percent,
                    sol_dollars,
                    terra_dollars,
                    luna_dollars,
                    sol_tokens INTEGER,
                    terra_tokens INTEGER,
                    luna_tokens INTEGER
                );
                INSERT INTO usage_history (
                    timestamp, reset_at, remaining_percent,
                    sol_dollars, terra_dollars, luna_dollars,
                    sol_tokens, terra_tokens, luna_tokens
                ) VALUES
                    (1715156808, 1715156800, 60.0, 4.0, 5.0, 6.0, 1, 2, 3),
                    (1715156809, 1715156800, -1.0, 4.0, 5.0, 6.0, 1, 2, 3);",
            )
            .unwrap();
        drop(source);

        assert!(store.import_v1_sqlite(&source_path).is_err());
        assert_eq!(sql_rows(&target_path), before_import);
        drop(store);
        cleanup(&target_path);
        cleanup(&source_path);
    }

    #[test]
    fn storage_focus11_public_sqlite_import_accepts_exact_integer_boundary() {
        let target_path = database_path("storage-focus11-integer-boundary-target");
        let mut store = UsageStore::open(&target_path).unwrap();

        let source_path = database_path("storage-focus11-integer-boundary-source");
        let source = Connection::open(&source_path).unwrap();
        source
            .execute_batch(
                "CREATE TABLE usage_history (
                    timestamp INTEGER,
                    reset_at INTEGER,
                    remaining_percent,
                    sol_dollars,
                    terra_dollars,
                    luna_dollars,
                    sol_tokens INTEGER,
                    terra_tokens INTEGER,
                    luna_tokens INTEGER
                );
                INSERT INTO usage_history (
                    timestamp, reset_at, remaining_percent,
                    sol_dollars, terra_dollars, luna_dollars,
                    sol_tokens, terra_tokens, luna_tokens
                ) VALUES
                    (1715156000, 1715155000, 100.0,
                     9007199254740992, 0, 0, 11, 22, 33);",
            )
            .unwrap();
        let source_type: String = source
            .query_row("SELECT typeof(sol_dollars) FROM usage_history", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(source_type, "integer");
        drop(source);

        store.import_v1_sqlite(&source_path).unwrap();
        let target = Connection::open(&target_path).unwrap();
        let imported = target
            .query_row(
                "SELECT timestamp, reset_at, remaining_percent,
                        sol_dollars, terra_dollars, luna_dollars,
                        sol_tokens, terra_tokens, luna_tokens,
                        typeof(sol_dollars)
                 FROM usage_history",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<f64>>(2)?,
                        row.get::<_, f64>(3)?,
                        row.get::<_, f64>(4)?,
                        row.get::<_, f64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, i64>(8)?,
                        row.get::<_, String>(9)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(imported.0, 1715156000);
        assert_eq!(imported.1, 1715155000);
        assert_eq!(imported.2, Some(100.0));
        assert_eq!(imported.3, 9007199254740992.0_f64);
        assert_eq!(imported.4, 0.0);
        assert_eq!(imported.5, 0.0);
        assert_eq!(imported.6, 0);
        assert_eq!(imported.7, 0);
        assert_eq!(imported.8, 0);
        assert_eq!(imported.9, "real");
        drop(target);

        let sql_rows = |path: &std::path::Path| {
            let connection = Connection::open(path).unwrap();
            let mut statement = connection
                .prepare(
                    "SELECT timestamp, reset_at, remaining_percent,
                            sol_dollars, terra_dollars, luna_dollars,
                            sol_tokens, terra_tokens, luna_tokens
                     FROM usage_history ORDER BY reset_at, timestamp",
                )
                .unwrap();
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<f64>>(2)?,
                        row.get::<_, f64>(3)?,
                        row.get::<_, f64>(4)?,
                        row.get::<_, f64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, i64>(8)?,
                    ))
                })
                .unwrap()
                .map(|row| row.unwrap())
                .collect::<Vec<_>>()
        };
        let baseline = sql_rows(&target_path);

        let invalid_source_path = database_path("storage-focus11-integer-boundary-invalid-source");
        let invalid_source = Connection::open(&invalid_source_path).unwrap();
        invalid_source
            .execute_batch(
                "CREATE TABLE usage_history (
                    timestamp INTEGER,
                    reset_at INTEGER,
                    remaining_percent,
                    sol_dollars,
                    terra_dollars,
                    luna_dollars,
                    sol_tokens INTEGER,
                    terra_tokens INTEGER,
                    luna_tokens INTEGER
                );
                INSERT INTO usage_history (
                    timestamp, reset_at, remaining_percent,
                    sol_dollars, terra_dollars, luna_dollars,
                    sol_tokens, terra_tokens, luna_tokens
                ) VALUES
                    (1715156001, 1715155001, 99.0, 0, 0, 0, 1, 2, 3),
                    (1715156002, 1715155002, 98.0,
                     9007199254740993, 0, 0, 1, 2, 3);",
            )
            .unwrap();
        let invalid_source_type: String = invalid_source
            .query_row(
                "SELECT typeof(sol_dollars) FROM usage_history WHERE timestamp = 1715156002",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(invalid_source_type, "integer");
        drop(invalid_source);

        assert!(store.import_v1_sqlite(&invalid_source_path).is_err());
        assert_eq!(sql_rows(&target_path), baseline);
        drop(store);
        cleanup(&target_path);
        cleanup(&source_path);
        cleanup(&invalid_source_path);
    }

    #[test]
    fn storage_focus11_public_write_numeric_partition_table() {
        let path = database_path("storage-focus11-public-write-numeric-partitions");
        let mut store = UsageStore::open(&path).unwrap();

        let sql_rows = |path: &std::path::Path| {
            let connection = Connection::open(path).unwrap();
            let mut statement = connection
                .prepare(
                    "SELECT timestamp, reset_at, remaining_percent,
                            sol_dollars, terra_dollars, luna_dollars,
                            sol_tokens, terra_tokens, luna_tokens
                     FROM usage_history ORDER BY reset_at, timestamp",
                )
                .unwrap();
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<f64>>(2)?,
                        row.get::<_, f64>(3)?,
                        row.get::<_, f64>(4)?,
                        row.get::<_, f64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, i64>(8)?,
                    ))
                })
                .unwrap()
                .map(|row| row.unwrap())
                .collect::<Vec<_>>()
        };
        let durable_sql = |path: &std::path::Path| {
            let connection = Connection::open(path).unwrap();
            match connection.query_row(
                "SELECT data_generation, data_hash, snapshot_json
                 FROM durable_state WHERE singleton = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            ) {
                Ok(value) => Some(value),
                Err(rusqlite::Error::QueryReturnedNoRows) => None,
                Err(error) => panic!("durable query failed: {error}"),
            }
        };

        let valid_none = UsageHistorySample {
            timestamp: 1,
            reset_at: 1,
            remaining_percent: None,
            sol_dollars: 0.0,
            terra_dollars: 0.0,
            luna_dollars: 0.0,
            sol_tokens: 0,
            terra_tokens: 0,
            luna_tokens: 0,
        };
        store.upsert_sample(&valid_none).unwrap();

        let valid_zero = UsageHistorySample {
            timestamp: 2,
            reset_at: 1,
            remaining_percent: Some(0.0),
            sol_dollars: 0.0,
            terra_dollars: 0.0,
            luna_dollars: 0.0,
            sol_tokens: i64::MAX as u64,
            terra_tokens: i64::MAX as u64,
            luna_tokens: i64::MAX as u64,
        };
        store
            .upsert_samples(std::slice::from_ref(&valid_zero))
            .unwrap();

        let valid_full = UsageHistorySample {
            timestamp: 3,
            reset_at: 1,
            remaining_percent: Some(100.0),
            sol_dollars: 0.0,
            terra_dollars: 0.0,
            luna_dollars: 0.0,
            sol_tokens: 0,
            terra_tokens: 0,
            luna_tokens: 0,
        };
        store
            .commit_durable_state(
                std::slice::from_ref(&valid_full),
                VALID_HASH,
                r#"{"kind":"focus11b"}"#,
            )
            .unwrap();

        assert_eq!(
            store.load_all().unwrap(),
            vec![valid_none.clone(), valid_zero.clone(), valid_full.clone()]
        );
        assert_eq!(
            sql_rows(&path),
            vec![
                (1, 1, None, 0.0, 0.0, 0.0, 0, 0, 0),
                (2, 1, Some(0.0), 0.0, 0.0, 0.0, i64::MAX, i64::MAX, i64::MAX,),
                (3, 1, Some(100.0), 0.0, 0.0, 0.0, 0, 0, 0),
            ]
        );
        assert_eq!(
            durable_sql(&path),
            Some((
                1,
                VALID_HASH.to_owned(),
                r#"{"kind":"focus11b"}"#.to_owned()
            ))
        );

        let baseline_history = sql_rows(&path);
        let baseline_durable = durable_sql(&path);
        let base_invalid = UsageHistorySample {
            timestamp: 10,
            reset_at: 10,
            remaining_percent: Some(50.0),
            sol_dollars: 1.0,
            terra_dollars: 2.0,
            luna_dollars: 3.0,
            sol_tokens: 4,
            terra_tokens: 5,
            luna_tokens: 6,
        };
        let invalids = vec![
            (
                "timestamp-zero",
                UsageHistorySample {
                    timestamp: 0,
                    ..base_invalid.clone()
                },
            ),
            (
                "timestamp-negative",
                UsageHistorySample {
                    timestamp: -1,
                    ..base_invalid.clone()
                },
            ),
            (
                "reset-zero",
                UsageHistorySample {
                    reset_at: 0,
                    ..base_invalid.clone()
                },
            ),
            (
                "reset-negative",
                UsageHistorySample {
                    reset_at: -1,
                    ..base_invalid.clone()
                },
            ),
            (
                "remaining-negative",
                UsageHistorySample {
                    remaining_percent: Some(-1.0),
                    ..base_invalid.clone()
                },
            ),
            (
                "remaining-101",
                UsageHistorySample {
                    remaining_percent: Some(101.0),
                    ..base_invalid.clone()
                },
            ),
            (
                "remaining-nan",
                UsageHistorySample {
                    remaining_percent: Some(f64::NAN),
                    ..base_invalid.clone()
                },
            ),
            (
                "remaining-positive-infinity",
                UsageHistorySample {
                    remaining_percent: Some(f64::INFINITY),
                    ..base_invalid.clone()
                },
            ),
            (
                "remaining-negative-infinity",
                UsageHistorySample {
                    remaining_percent: Some(f64::NEG_INFINITY),
                    ..base_invalid.clone()
                },
            ),
            (
                "sol-negative",
                UsageHistorySample {
                    sol_dollars: -1.0,
                    ..base_invalid.clone()
                },
            ),
            (
                "sol-nan",
                UsageHistorySample {
                    sol_dollars: f64::NAN,
                    ..base_invalid.clone()
                },
            ),
            (
                "sol-positive-infinity",
                UsageHistorySample {
                    sol_dollars: f64::INFINITY,
                    ..base_invalid.clone()
                },
            ),
            (
                "sol-negative-infinity",
                UsageHistorySample {
                    sol_dollars: f64::NEG_INFINITY,
                    ..base_invalid.clone()
                },
            ),
            (
                "terra-negative",
                UsageHistorySample {
                    terra_dollars: -1.0,
                    ..base_invalid.clone()
                },
            ),
            (
                "terra-nan",
                UsageHistorySample {
                    terra_dollars: f64::NAN,
                    ..base_invalid.clone()
                },
            ),
            (
                "terra-positive-infinity",
                UsageHistorySample {
                    terra_dollars: f64::INFINITY,
                    ..base_invalid.clone()
                },
            ),
            (
                "terra-negative-infinity",
                UsageHistorySample {
                    terra_dollars: f64::NEG_INFINITY,
                    ..base_invalid.clone()
                },
            ),
            (
                "luna-negative",
                UsageHistorySample {
                    luna_dollars: -1.0,
                    ..base_invalid.clone()
                },
            ),
            (
                "luna-nan",
                UsageHistorySample {
                    luna_dollars: f64::NAN,
                    ..base_invalid.clone()
                },
            ),
            (
                "luna-positive-infinity",
                UsageHistorySample {
                    luna_dollars: f64::INFINITY,
                    ..base_invalid.clone()
                },
            ),
            (
                "luna-negative-infinity",
                UsageHistorySample {
                    luna_dollars: f64::NEG_INFINITY,
                    ..base_invalid.clone()
                },
            ),
            (
                "token-overflow",
                UsageHistorySample {
                    sol_tokens: i64::MAX as u64 + 1,
                    ..base_invalid.clone()
                },
            ),
        ];
        for (label, invalid) in invalids {
            assert!(store.upsert_sample(&invalid).is_err(), "single {label}");
            assert_eq!(sql_rows(&path), baseline_history);
            assert_eq!(durable_sql(&path), baseline_durable);
            assert!(
                store
                    .upsert_samples(std::slice::from_ref(&invalid))
                    .is_err(),
                "batch {label}"
            );
            assert_eq!(sql_rows(&path), baseline_history);
            assert_eq!(durable_sql(&path), baseline_durable);
            assert!(
                store
                    .commit_durable_state(
                        std::slice::from_ref(&invalid),
                        VALID_HASH,
                        r#"{"kind":"invalid"}"#,
                    )
                    .is_err(),
                "durable {label}"
            );
            assert_eq!(sql_rows(&path), baseline_history);
            assert_eq!(durable_sql(&path), baseline_durable);
        }

        let mixed_valid = UsageHistorySample {
            timestamp: 100,
            reset_at: 100,
            remaining_percent: Some(75.0),
            sol_dollars: 0.0,
            terra_dollars: 0.0,
            luna_dollars: 0.0,
            sol_tokens: 0,
            terra_tokens: 0,
            luna_tokens: 0,
        };
        let mixed_invalid = UsageHistorySample {
            timestamp: 101,
            reset_at: 100,
            sol_dollars: -1.0,
            ..mixed_valid.clone()
        };
        assert!(store
            .upsert_samples(&[mixed_valid.clone(), mixed_invalid.clone()])
            .is_err());
        assert_eq!(sql_rows(&path), baseline_history);
        assert_eq!(durable_sql(&path), baseline_durable);
        assert!(store
            .commit_durable_state(
                &[mixed_valid, mixed_invalid],
                VALID_HASH,
                r#"{"kind":"mixed-invalid"}"#,
            )
            .is_err());
        assert_eq!(sql_rows(&path), baseline_history);
        assert_eq!(durable_sql(&path), baseline_durable);

        drop(store);
        cleanup(&path);
    }

    #[test]
    fn storage_focus11_nonpruning_uses_fixed_utc_epoch_oracle() {
        use chrono::TimeZone;

        let now = Utc.timestamp_opt(1715156800, 0).single().unwrap();
        let old = UsageHistorySample {
            timestamp: 1600000000,
            reset_at: 1600000000,
            remaining_percent: Some(10.0),
            sol_dollars: 1.0,
            terra_dollars: 2.0,
            luna_dollars: 3.0,
            sol_tokens: 4,
            terra_tokens: 5,
            luna_tokens: 6,
        };
        let recent = UsageHistorySample {
            timestamp: 1715156700,
            reset_at: 1715156800,
            remaining_percent: Some(20.0),
            sol_dollars: 7.0,
            terra_dollars: 8.0,
            luna_dollars: 9.0,
            sol_tokens: 10,
            terra_tokens: 11,
            luna_tokens: 12,
        };
        let path = database_path("storage-focus11-nonpruning-fixed-utc");
        let store = UsageStore::open(&path).unwrap();
        store.upsert_sample(&old).unwrap();
        store.upsert_sample(&recent).unwrap();

        let count_rows = |path: &std::path::Path| -> i64 {
            let connection = Connection::open(path).unwrap();
            connection
                .query_row("SELECT COUNT(*) FROM usage_history", [], |row| row.get(0))
                .unwrap()
        };
        let old_is_present = |path: &std::path::Path| -> bool {
            let connection = Connection::open(path).unwrap();
            connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM usage_history WHERE timestamp = 1600000000
                    )",
                    [],
                    |row| row.get(0),
                )
                .unwrap()
        };

        assert_eq!(count_rows(&path), 2);
        assert!(old_is_present(&path));
        assert_eq!(
            store.load_recent_three_months(now).unwrap(),
            vec![recent.clone()]
        );
        assert_eq!(count_rows(&path), 2);
        assert!(old_is_present(&path));

        store.upsert_sample(&recent).unwrap();
        assert_eq!(count_rows(&path), 2);
        assert!(old_is_present(&path));
        drop(store);
        cleanup(&path);
    }
}
