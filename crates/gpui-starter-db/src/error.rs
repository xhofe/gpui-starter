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

use snafu::Snafu;

/// Errors of the local storage layer.
#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Invalid: {message}"))]
    Invalid { message: String },
    #[snafu(display("IO error: {source}"))]
    Io { source: std::io::Error },
    #[snafu(display("Serde json error: {source}"))]
    SerdeJson { source: serde_json::Error },
    #[snafu(display("Redb error: {source}"))]
    Redb { source: redb::Error },
    #[snafu(display("Redb database error: {source}"))]
    RedbDatabase { source: redb::DatabaseError },
    #[snafu(display("Redb transaction error: {source}"))]
    RedbTransaction { source: redb::TransactionError },
    #[snafu(display("Redb table error: {source}"))]
    RedbTable { source: redb::TableError },
    #[snafu(display("Redb commit error: {source}"))]
    RedbCommit { source: redb::CommitError },
    #[snafu(display("Redb storage error: {source}"))]
    RedbStorage { source: redb::StorageError },
    /// Written by a newer build. Refusing is the safe move.
    #[snafu(display("local database schema v{found} is newer than this app supports (v{supported})"))]
    SchemaTooNew { found: u32, supported: u32 },
}

macro_rules! direct {
    ($($ty:ty => $variant:ident),+ $(,)?) => {$(
        impl From<$ty> for Error {
            fn from(source: $ty) -> Self {
                Error::$variant { source }
            }
        }
    )+};
}
direct!(
    std::io::Error => Io,
    serde_json::Error => SerdeJson,
    redb::Error => Redb,
    redb::DatabaseError => RedbDatabase,
    redb::TransactionError => RedbTransaction,
    redb::TableError => RedbTable,
    redb::CommitError => RedbCommit,
    redb::StorageError => RedbStorage,
);
