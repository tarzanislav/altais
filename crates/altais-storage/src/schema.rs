//! Database schema and migrations (`PRAGMA user_version`).

use rusqlite::Connection;

use crate::StorageError;

/// Migrations in order; migration `i` brings the schema to version `i + 1`.
const MIGRATIONS: &[&str] = &[
    // 1: notebooks, notes (body as ADM JSON until the Loro docstore), tags,
    //    attachments and their content.
    r#"
    CREATE TABLE notebook (
        id      TEXT PRIMARY KEY,
        name    TEXT NOT NULL,
        parent  TEXT REFERENCES notebook(id) ON DELETE SET NULL,
        created TEXT NOT NULL,
        updated TEXT NOT NULL
    ) STRICT;

    CREATE TABLE note (
        id       TEXT PRIMARY KEY,
        notebook TEXT REFERENCES notebook(id) ON DELETE SET NULL,
        title    TEXT NOT NULL,
        created  TEXT NOT NULL,
        updated  TEXT NOT NULL,
        deleted  TEXT,
        adm      TEXT NOT NULL
    ) STRICT;
    CREATE INDEX note_by_notebook ON note(notebook, updated);

    CREATE TABLE tag (
        id   INTEGER PRIMARY KEY,
        name TEXT NOT NULL UNIQUE COLLATE NOCASE
    ) STRICT;

    CREATE TABLE note_tag (
        note TEXT NOT NULL REFERENCES note(id) ON DELETE CASCADE,
        tag  INTEGER NOT NULL REFERENCES tag(id) ON DELETE CASCADE,
        PRIMARY KEY (note, tag)
    ) STRICT, WITHOUT ROWID;

    CREATE TABLE blob (
        hash TEXT PRIMARY KEY,
        size INTEGER NOT NULL,
        data BLOB NOT NULL
    ) STRICT;

    CREATE TABLE attachment (
        note TEXT NOT NULL REFERENCES note(id) ON DELETE CASCADE,
        bid  TEXT NOT NULL,
        blob TEXT NOT NULL REFERENCES blob(hash),
        name TEXT NOT NULL,
        mime TEXT NOT NULL,
        PRIMARY KEY (note, bid)
    ) STRICT, WITHOUT ROWID;
    "#,
];

/// Newest schema version this build knows.
pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

/// Brings the database to the newest schema.
pub fn migrate(conn: &mut Connection) -> Result<(), StorageError> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current > SCHEMA_VERSION {
        return Err(StorageError::NewerSchema {
            found: current,
            known: SCHEMA_VERSION,
        });
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i as i64 + 1)?;
        tx.commit()?;
    }
    Ok(())
}
