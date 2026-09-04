// Copyright 2026 Andy Hsu.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::error::Error;
use redb::{Database, DatabaseError, ReadableTable, StorageError, TableDefinition, WriteTransaction};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

pub mod error;
mod todos;

pub use todos::*;

const TODOS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("todos");
const META_TABLE: TableDefinition<&str, u32> = TableDefinition::new("meta");
const SCHEMA_VERSION_KEY: &str = "schema_version";
/// Version of the on-disk layout this build writes. Bump it together with a
/// matching step in [`migrate_step`] whenever a table's encoding changes.
pub const SCHEMA_VERSION: u32 = 1;

type Result<T, E = Error> = std::result::Result<T, E>;

static DATABASE: OnceLock<Database> = OnceLock::new();

fn get_database() -> Result<&'static Database> {
    DATABASE.get().ok_or(Error::Invalid {
        message: "database not initialized".to_string(),
    })
}

pub fn init_database(path: impl AsRef<Path>) -> Result<()> {
    let db_path = path.as_ref();
    debug!(path = %db_path.display(), "create database");
    let db = Database::create(db_path)?;
    ensure_schema(&db)?;
    DATABASE.set(db).map_err(|_| Error::Invalid {
        message: "database initialized failed".to_string(),
    })?;
    Ok(())
}

fn ensure_schema(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    let stored = {
        let table = write_txn.open_table(META_TABLE)?;
        table.get(SCHEMA_VERSION_KEY)?.map(|v| v.value())
    };
    match stored {
        Some(found) if found > SCHEMA_VERSION => {
            return Err(Error::SchemaTooNew {
                found,
                supported: SCHEMA_VERSION,
            });
        }
        Some(found) => {
            for from in found..SCHEMA_VERSION {
                debug!(from, to = from + 1, "migrating local database schema");
                migrate_step(&write_txn, from)?;
            }
        }
        None => {}
    }
    {
        write_txn.open_table(TODOS_TABLE)?;
        write_txn
            .open_table(META_TABLE)?
            .insert(SCHEMA_VERSION_KEY, SCHEMA_VERSION)?;
    }
    write_txn.commit()?;
    Ok(())
}

fn migrate_step(_txn: &WriteTransaction, from: u32) -> Result<()> {
    Err(Error::Invalid {
        message: format!("no migration from local database schema v{from}"),
    })
}

/// Why [`init_database`] failed, reduced to what the UI can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbOpenFailure {
    Locked,
    SchemaTooNew { found: u32, supported: u32 },
    Damaged(String),
    Inaccessible(String),
}

pub fn open_failure_kind(error: &Error) -> DbOpenFailure {
    match error {
        Error::SchemaTooNew { found, supported } => DbOpenFailure::SchemaTooNew {
            found: *found,
            supported: *supported,
        },
        Error::RedbDatabase {
            source: DatabaseError::DatabaseAlreadyOpen,
        } => DbOpenFailure::Locked,
        Error::RedbDatabase {
            source: DatabaseError::Storage(StorageError::Io(io)),
        }
        | Error::Io { source: io }
            if !matches!(io.kind(), ErrorKind::InvalidData | ErrorKind::UnexpectedEof) =>
        {
            DbOpenFailure::Inaccessible(io.to_string())
        }
        other => DbOpenFailure::Damaged(other.to_string()),
    }
}

/// Moves the database file aside so a following [`init_database`] starts
/// fresh. Nothing is deleted.
pub fn quarantine_database(path: impl AsRef<Path>) -> Result<PathBuf> {
    if DATABASE.get().is_some() {
        return Err(Error::Invalid {
            message: "database is open; cannot quarantine it".to_string(),
        });
    }
    let path = path.as_ref();
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let target = path.with_file_name(format!("{name}.corrupt-{secs}"));
    std::fs::rename(path, &target)?;
    Ok(target.to_path_buf())
}

#[cfg(test)]
mod schema_tests {
    use super::*;
    use redb::{ReadableDatabase, TableHandle};

    struct ScratchDb(PathBuf);

    impl ScratchDb {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("gpui-starter-db-schema-{}-{name}.redb", std::process::id()));
            let _ = std::fs::remove_file(&path);
            Self(path)
        }
    }

    impl Drop for ScratchDb {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn stored_version(db: &Database) -> Option<u32> {
        let txn = db.begin_read().expect("begin read");
        let Ok(table) = txn.open_table(META_TABLE) else {
            return None;
        };
        table.get(SCHEMA_VERSION_KEY).expect("get").map(|v| v.value())
    }

    #[test]
    fn every_table_this_crate_defines_exists_after_an_open() {
        let scratch = ScratchDb::new("tables");
        let db = Database::create(&scratch.0).expect("create");
        ensure_schema(&db).expect("ensure schema");

        let txn = db.begin_read().expect("begin read");
        let mut found: Vec<String> = txn
            .list_tables()
            .expect("list tables")
            .map(|t| t.name().to_string())
            .collect();
        found.sort();
        assert_eq!(found, vec!["meta", "todos"]);
    }

    #[test]
    fn a_newer_schema_is_refused_without_touching_the_file() {
        let scratch = ScratchDb::new("newer");
        let db = Database::create(&scratch.0).expect("create");
        {
            let txn = db.begin_write().expect("begin write");
            txn.open_table(META_TABLE)
                .expect("open meta")
                .insert(SCHEMA_VERSION_KEY, SCHEMA_VERSION + 5)
                .expect("insert");
            txn.commit().expect("commit");
        }
        let err = ensure_schema(&db).expect_err("must refuse");
        assert!(
            matches!(err, Error::SchemaTooNew { found, supported } if found == SCHEMA_VERSION + 5 && supported == SCHEMA_VERSION)
        );
        assert_eq!(stored_version(&db), Some(SCHEMA_VERSION + 5));
    }
}
