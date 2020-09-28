use rusqlite::NO_PARAMS;
use rusqlite::{params, Connection, Result};
use std::collections::HashMap;

const DB_NAME: &str = ".dsync.db";

pub struct FileData {
    pub path: String,
    pub hash: Vec<u8>,
}

impl FileData {
    pub fn new(path: String, hash: Vec<u8>) -> Self {
        FileData { path, hash }
    }
}

pub struct FileUpdate {
    pub path: String,
    pub operation: u8,
}

impl FileUpdate {
    pub const ADD: u8 = 1;
    pub const REMOVE: u8 = 2;
    pub const UPDATE: u8 = 3;

    pub fn new(path: String, operation: u8) -> Self {
        FileUpdate { path, operation }
    }
}

pub fn connect(root: impl AsRef<std::path::Path>) -> Result<Connection> {
    let mut path = root.as_ref().to_owned();
    path.push(DB_NAME);
    let conn = Connection::open(&path)?;

    conn.execute(
        "create table if not exists files (
             id integer primary key autoincrement,
             path text not null unique,
             hash blob not null
         )",
        NO_PARAMS,
    )?;

    conn.execute(
        "create table if not exists updates (
             path text not null unique,
             operation int1 not null
         )",
        NO_PARAMS,
    )?;

    Ok(conn)
}

pub fn find_file(conn: &Connection, path: &str) -> Result<FileData> {
    conn.prepare("select path, hash from files where path = ?1")?
        .query_row(params!(path), |row| {
            Ok(FileData::new(row.get(0)?, row.get(1)?))
        })
}

pub fn list_files(conn: &Connection) -> Result<HashMap<String, FileData>> {
    conn.prepare("select path, hash from files")?
        .query_map(NO_PARAMS, |row| {
            Ok((row.get(0)?, FileData::new(row.get(0)?, row.get(1)?)))
        })?
        .collect()
}

pub fn upsert_file(conn: &Connection, file: &FileData) -> Result<()> {
    conn.execute(
        "insert into files (path, hash) values (?1, ?2) on conflict (path) do update set hash=excluded.hash",
        params!(
            &file.path,
            &file.hash,
        ),
    )?;

    Ok(())
}

pub fn delete_file_entry(conn: &Connection, path: &str) -> Result<()> {
    conn.execute("delete from files where path = ?", params!(path))?;
    conn.execute("delete from updates where path = ?", params!(path))?;
    Ok(())
}

pub fn upsert_files(conn: &mut Connection, files: &[FileData]) -> Result<()> {
    let tx = conn.transaction()?;

    for file in files.into_iter() {
        tx.execute(
            "insert into files (path, hash) values (?1, ?2) on conflict (path) do update set hash=excluded.hash",
            params!(
                &file.path,
                &file.hash,
            ),
        )?;
    }

    tx.commit()?;
    Ok(())
}

pub fn add_update(conn: &Connection, fileupdate: &FileUpdate) -> Result<()> {
    conn.execute(
        "insert or replace into updates (path, operation) values (?1, ?2)",
        params!(&fileupdate.path, &fileupdate.operation),
    )?;
    Ok(())
}

pub fn add_update_list(conn: &mut Connection, fileupdates: &[FileUpdate]) -> Result<()> {
    let tx = conn.transaction()?;

    for fileupdate in fileupdates.into_iter() {
        tx.execute(
            "insert or replace into updates (path, operation) values (?1, ?2)",
            params!(&fileupdate.path, &fileupdate.operation),
        )?;
    }

    tx.commit()?;
    Ok(())
}

pub fn list_files_to_update(conn: &Connection) -> Result<Vec<FileUpdate>> {
    conn.prepare("select path, operation from updates order")?
        .query_map(NO_PARAMS, |row| {
            Ok(FileUpdate::new(row.get(0)?, row.get(1)?))
        })?
        .collect()
}

pub fn clear_all_files_to_update(conn: &Connection) -> Result<()> {
    conn.execute("delete from updates", NO_PARAMS)?;
    Ok(())
}

pub fn clear_files_to_update(conn: &mut Connection, paths: &[&str]) -> Result<()> {
    let tx = conn.transaction()?;

    for path in paths.into_iter() {
        tx.execute(
            "delete from updates where updates.path == ?1",
            params!(&path),
        )?;
    }

    tx.commit()?;
    Ok(())
}
