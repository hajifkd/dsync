use super::{construct_local_path, create_metadir, save_config, upsert_metadata, Config};
use crate::db;
use crate::files::download;
use crate::files::list_folder::Entry;
use crate::ignore::{parce_ignore, IGNORE_FILE};
use tokio::fs;

use std::path::Path;

pub async fn clone(
    remote_path: &str,
    local_root: &Path,
    token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let ignore_filter = parce_ignore().await?;

    fs::create_dir_all(local_root).await?;
    let conn = db::connect(local_root)?;

    let config = Config {
        remote_path: remote_path.to_owned(),
    };

    println!(
        "Cloning {} into {} ...",
        remote_path,
        local_root.to_string_lossy()
    );

    visit_all_files_and_dirs!(
        vec![remote_path.to_owned()],
        Vec::<String>::new(),
        &ignore_filter,
        &config,
        local_root,
        &conn,
        token,
        download_file,
        create_dirs
    );

    let curr_dir = std::env::current_dir()?;
    let is_curr = std::fs::canonicalize(&curr_dir)? == std::fs::canonicalize(local_root)?;
    if !is_curr {
        let mut ignore_file = curr_dir.clone();
        ignore_file.push(IGNORE_FILE);
        let mut ignore_file_dst = local_root.to_owned();
        ignore_file_dst.push(IGNORE_FILE);
        if ignore_file.exists() {
            fs::copy(ignore_file, ignore_file_dst).await?;
        }
    }

    save_config(&config, local_root).await?;

    println!("done.");
    Ok(())
}

async fn download_file(
    remote_path: &str,
    config: &Config,
    local_root: &Path,
    conn: &rusqlite::Connection,
    token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let local_file = construct_local_path(remote_path, config, local_root);
    println!(
        "Found file {}. Downloading to {} ...",
        remote_path,
        local_file.to_string_lossy()
    );
    let (info, data) = download::download(remote_path, token).await?;
    upsert_metadata(local_root, conn, config, info, &data).await?;
    fs::write(local_file, data).await?;
    Ok(())
}

async fn create_dirs(
    remote_dir: &str,
    config: &Config,
    local_root: &Path,
    _conn: &rusqlite::Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    let local_dir = construct_local_path(remote_dir, config, local_root);
    println!("Creating directory {} ...", local_dir.to_string_lossy());
    fs::create_dir_all(local_dir).await?;
    create_metadir(remote_dir, config, local_root).await?;
    Ok(())
}
