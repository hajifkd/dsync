use super::{construct_remote_path, load_config, Config};
use crate::db;
use crate::file_hash;
use crate::ignore::{parce_ignore, Ignore};
use futures::prelude::*;
use rusqlite::Connection;

use std::error::Error;
use std::path::Path;

pub async fn add(
    target: impl AsRef<Path>,
    local_root: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ignore_filter = parce_ignore().await?;
    let local_root = local_root.as_ref();
    let target = target.as_ref();
    let config = load_config(local_root).await.map_err(|_| {
        format!(
            "Directory {} is not a correct dsync repo.",
            local_root.display()
        )
    })?;
    let conn = db::connect(local_root)?;

    if ignore_filter.is_ignored(&target.to_string_lossy()) {
        return Ok(());
    }

    if target.is_file() {
        // remote_path never ends with /.
        let remote_path = construct_remote_path(target, &config, &local_root)?;
        db::upsert_file(
            &conn,
            &db::FileData::new(remote_path, file_hash(target).await?.to_vec()),
        )?;
    } else if target.is_dir() {
        add_dir(target, &config, &local_root, &conn, &ignore_filter).await?;
    } else {
        return Err(format!(
            "File {} does not exist or is not either a file or directory.",
            target.display()
        )
        .into());
    }

    Ok(())
}

async fn add_dir(
    target: &Path,
    config: &Config,
    local_root: &Path,
    conn: &Connection,
    ignore_filter: &Ignore,
) -> Result<(), Box<dyn Error>> {
    // List all files under target and upsert.
    let mut dirs = vec![target.to_owned()];

    while dirs.len() != 0 {
        let mut new_dirs = vec![];
        for dir in dirs.iter() {
            let mut reads = tokio::fs::read_dir(dir).await?;
            while let Some(entry) = reads.next().await {
                let path = entry?.path();
                if ignore_filter.is_ignored(&path.to_string_lossy()) {
                    continue;
                }
                if path.is_file() {
                    // remote_path never ends with /.
                    let remote_path = construct_remote_path(&path, &config, &local_root)?;
                    db::upsert_file(
                        conn,
                        &db::FileData::new(remote_path, file_hash(&path).await?.to_vec()),
                    )?;
                } else if path.is_dir() {
                    new_dirs.push(path);
                }
            }
        }

        dirs = new_dirs;
    }

    Ok(())
}
