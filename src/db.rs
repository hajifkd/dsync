use rusqlite::NO_PARAMS;
use rusqlite::{params, Connection, Result};

const DB_NAME: &str = ".dsync.db";

pub struct FileData {
    pub path: String,
    pub hash: Option<Vec<u8>>,
}

impl FileData {
    pub fn new(path: String, hash: Vec<u8>) -> Self {
        FileData {
            path,
            hash: Some(hash),
        }
    }

    pub fn new_empty(path: String) -> Self {
        FileData { path, hash: None }
    }
}

pub fn connect() -> Result<Connection> {
    let conn = Connection::open(DB_NAME)?;

    conn.execute(
        "create table if not exists files (
             id integer primary key autoincrement,
             path text not null unique,
             hash blob
         )",
        NO_PARAMS,
    )?;

    conn.execute(
        "create table if not exists updates (
             path text not null unique
         )",
        NO_PARAMS,
    )?;

    Ok(conn)
}

pub fn list_files_sorted(conn: &Connection) -> Result<Vec<FileData>> {
    conn.prepare("select path, hash from files order by path asc")?
        .query_map(NO_PARAMS, |row| Ok(FileData::new(row.get(0)?, row.get(1)?)))?
        .collect()
}

pub fn upsert_files(conn: &mut Connection, files: &[FileData]) -> Result<()> {
    let tx = conn.transaction()?;

    for file in files.into_iter() {
        tx.execute(
            "insert into files (path, hash) values (?1, ?2) on conflict (path) do update set hash=excluded.hash",
            params!(
                &file.path,
                file.hash.as_ref().map(|v| &v[..]).unwrap_or(&[])
            ),
        )?;
    }

    tx.commit()?;
    Ok(())
}

pub fn add_update_list(conn: &mut Connection, files_to_updates: &[&str]) -> Result<()> {
    let tx = conn.transaction()?;

    for file in files_to_updates.into_iter() {
        tx.execute(
            "insert or replace into updates (path) values (?1)",
            params!(file),
        )?;
    }

    tx.commit()?;
    Ok(())
}
