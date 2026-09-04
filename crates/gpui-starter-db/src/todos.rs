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
use crate::{TODOS_TABLE, get_database};
use redb::{ReadableDatabase, ReadableTable};
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

type Result<T, E = Error> = std::result::Result<T, E>;

/// One todo row. Container-level `#[serde(default)]` so adding a field never
/// makes an existing row unreadable.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub done: bool,
    pub created_at: i64,
}

/// Lists every readable row, newest first. A row that cannot be parsed is
/// skipped (never deleted).
pub fn list_todos() -> Result<Vec<Todo>> {
    let db = get_database()?;
    let txn = db.begin_read()?;
    let table = txn.open_table(TODOS_TABLE)?;
    let mut items = Vec::new();
    for entry in table.iter()? {
        let (key, value) = entry?;
        match serde_json::from_slice::<Todo>(value.value()) {
            Ok(todo) => items.push(todo),
            Err(e) => {
                warn!(id = %String::from_utf8_lossy(key.value()), error = %e, "skipping unreadable todo row");
            }
        }
    }
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
    Ok(items)
}

pub fn add_todo(title: String) -> Result<Todo> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(Error::Invalid {
            message: "todo title is empty".to_string(),
        });
    }
    let todo = Todo {
        id: Uuid::now_v7().to_string(),
        title,
        done: false,
        created_at: now_unix(),
    };
    upsert(&todo)?;
    Ok(todo)
}

pub fn set_todo_done(id: &str, done: bool) -> Result<Option<Todo>> {
    let Some(mut todo) = get_todo(id)? else {
        return Ok(None);
    };
    todo.done = done;
    upsert(&todo)?;
    Ok(Some(todo))
}

pub fn set_todo_title(id: &str, title: String) -> Result<Option<Todo>> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(Error::Invalid {
            message: "todo title is empty".to_string(),
        });
    }
    let Some(mut todo) = get_todo(id)? else {
        return Ok(None);
    };
    todo.title = title;
    upsert(&todo)?;
    Ok(Some(todo))
}

pub fn delete_todo(id: &str) -> Result<bool> {
    let db = get_database()?;
    let write_txn = db.begin_write()?;
    let existed = {
        let mut table = write_txn.open_table(TODOS_TABLE)?;
        table.remove(id.as_bytes())?.is_some()
    };
    write_txn.commit()?;
    Ok(existed)
}

fn get_todo(id: &str) -> Result<Option<Todo>> {
    let db = get_database()?;
    let txn = db.begin_read()?;
    let table = txn.open_table(TODOS_TABLE)?;
    let Some(value) = table.get(id.as_bytes())? else {
        return Ok(None);
    };
    match serde_json::from_slice::<Todo>(value.value()) {
        Ok(todo) => Ok(Some(todo)),
        Err(e) => {
            warn!(id, error = %e, "skipping unreadable todo row");
            Ok(None)
        }
    }
}

fn upsert(todo: &Todo) -> Result<()> {
    let bytes = serde_json::to_vec(todo)?;
    let db = get_database()?;
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(TODOS_TABLE)?;
        table.insert(todo.id.as_bytes(), bytes.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
