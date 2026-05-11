//! Schema migration system for RocksDB.
//!
//! On startup, reads `SCHEMA_VERSION` from the `meta` Column Family.
//! If the stored version is behind `LATEST_VERSION`, pending migrations
//! run in order. Each migration is a function that receives the DB handle.
//!
//! ## Adding a new migration
//!
//! 1. Increment `LATEST_VERSION`.
//! 2. Add a function `fn migrate_vN(db: &RocksDB) -> Result<()>`.
//! 3. Register it in `MIGRATIONS` array.
//!
//! Migrations MUST be idempotent — a crash mid-migration means the same
//! migration runs again on next startup (version only advances after success).

use super::adapters::RocksDbBlockStore;
use super::errors::{StorageError, StorageResult};

const META_SCHEMA_VERSION: &[u8] = b"schema_version";
const CF_META: &str = "meta";

/// Current schema version. Increment when adding a new migration.
pub const LATEST_VERSION: u32 = 2;

/// A single migration step.
struct Migration {
    /// Target version after this migration runs.
    version: u32,
    /// Human-readable description.
    description: &'static str,
    /// The migration function.
    apply: fn(&RocksDbBlockStore) -> StorageResult<()>,
}

/// Registry of all migrations. Must be sorted by version ascending.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "initial schema — all Column Families created by RocksDbBlockStore::new()",
        apply: migrate_v1,
    },
    Migration {
        version: 2,
        description: "add audit_log, sandbox_reports, oracle_records CFs",
        apply: migrate_v2,
    },
];

/// Read the current schema version from the DB. Returns 0 if not set.
pub fn current_version(store: &RocksDbBlockStore) -> StorageResult<u32> {
    let cf = store
        .db
        .cf_handle(CF_META)
        .ok_or_else(|| StorageError::RocksDbError("missing meta CF".to_string()))?;
    match store.db.get_cf(&cf, META_SCHEMA_VERSION) {
        Ok(Some(bytes)) => {
            if bytes.len() == 4 {
                Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
            } else {
                // Legacy: might be stored as UTF-8 string
                let s = String::from_utf8_lossy(&bytes);
                s.trim()
                    .parse::<u32>()
                    .map_err(|e| StorageError::RocksDbError(format!("bad schema version: {e}")))
            }
        }
        Ok(None) => Ok(0),
        Err(e) => Err(StorageError::RocksDbError(e.to_string())),
    }
}

/// Write the schema version to the DB.
fn set_version(store: &RocksDbBlockStore, version: u32) -> StorageResult<()> {
    let cf = store
        .db
        .cf_handle(CF_META)
        .ok_or_else(|| StorageError::RocksDbError("missing meta CF".to_string()))?;
    store
        .db
        .put_cf(&cf, META_SCHEMA_VERSION, version.to_le_bytes())
        .map_err(|e| StorageError::RocksDbError(e.to_string()))
}

/// Run all pending migrations from `current` to `LATEST_VERSION`.
///
/// Returns the number of migrations applied.
pub fn run_pending(store: &RocksDbBlockStore) -> StorageResult<u32> {
    let current = current_version(store)?;

    if current >= LATEST_VERSION {
        log::info!("Schema version {current} is up to date (latest: {LATEST_VERSION})");
        return Ok(0);
    }

    if current == 0 {
        // Fresh database — stamp with latest version, no migrations needed
        // (all CFs are already created by RocksDbBlockStore::new).
        log::info!("Fresh database — setting schema version to {LATEST_VERSION}");
        set_version(store, LATEST_VERSION)?;
        return Ok(0);
    }

    let mut applied = 0u32;
    for migration in MIGRATIONS {
        if migration.version <= current {
            continue;
        }
        log::info!(
            "Running migration v{}: {}",
            migration.version,
            migration.description
        );
        (migration.apply)(store)?;
        set_version(store, migration.version)?;
        applied += 1;
        log::info!("Migration v{} complete", migration.version);
    }

    log::info!(
        "Schema migrated from v{current} to v{LATEST_VERSION} ({applied} migrations applied)"
    );
    Ok(applied)
}

// ── Migration implementations ────────────────────────────────────────────────

/// v1: Initial schema. All CFs already created by `RocksDbBlockStore::new()`.
/// This migration is a no-op — it just marks the baseline.
fn migrate_v1(_store: &RocksDbBlockStore) -> StorageResult<()> {
    Ok(())
}

/// v2: Add new Column Families for audit, sandbox, and oracle.
/// `create_missing_column_families(true)` handles this at open time,
/// so this migration just verifies they exist.
fn migrate_v2(store: &RocksDbBlockStore) -> StorageResult<()> {
    for cf_name in &["audit_log", "sandbox_reports", "oracle_records"] {
        if store.db.cf_handle(cf_name).is_none() {
            return Err(StorageError::RocksDbError(format!(
                "migration v2: CF '{cf_name}' not found — database may need re-creation"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp_store() -> (RocksDbBlockStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = RocksDbBlockStore::new(dir.path()).unwrap();
        (store, dir)
    }

    #[test]
    fn fresh_db_returns_version_zero() {
        let (store, _dir) = tmp_store();
        assert_eq!(current_version(&store).unwrap(), 0);
    }

    #[test]
    fn run_pending_on_fresh_db_stamps_latest() {
        let (store, _dir) = tmp_store();
        let applied = run_pending(&store).unwrap();
        assert_eq!(applied, 0); // No migrations needed, just stamped
        assert_eq!(current_version(&store).unwrap(), LATEST_VERSION);
    }

    #[test]
    fn run_pending_is_idempotent() {
        let (store, _dir) = tmp_store();
        run_pending(&store).unwrap();
        let applied = run_pending(&store).unwrap();
        assert_eq!(applied, 0); // Already up to date
        assert_eq!(current_version(&store).unwrap(), LATEST_VERSION);
    }

    #[test]
    fn run_pending_from_v1_runs_v2() {
        let (store, _dir) = tmp_store();
        // Simulate a v1 database
        set_version(&store, 1).unwrap();
        assert_eq!(current_version(&store).unwrap(), 1);

        let applied = run_pending(&store).unwrap();
        assert_eq!(applied, 1); // v2 migration ran
        assert_eq!(current_version(&store).unwrap(), LATEST_VERSION);
    }

    #[test]
    fn version_survives_reopen() {
        let dir = TempDir::new().unwrap();
        {
            let store = RocksDbBlockStore::new(dir.path()).unwrap();
            run_pending(&store).unwrap();
        }
        // Reopen
        let store = RocksDbBlockStore::new(dir.path()).unwrap();
        assert_eq!(current_version(&store).unwrap(), LATEST_VERSION);
    }

    #[test]
    fn migrations_are_sorted_by_version() {
        for window in MIGRATIONS.windows(2) {
            assert!(
                window[0].version < window[1].version,
                "migrations must be sorted: v{} should be before v{}",
                window[0].version,
                window[1].version,
            );
        }
    }
}
