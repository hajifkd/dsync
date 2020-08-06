use super::{create_dirs, download_file, save_config, visit_all_dirs, Config};
use crate::db;
use crate::ignore::{parce_ignore, IGNORE_FILE};
use tokio::fs;

use std::path::Path;

pub async fn clone(
    remote_path: &str,
    local_root: impl AsRef<Path>,
    token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let ignore_filter = parce_ignore().await?;
    let local_root = local_root.as_ref();

    fs::create_dir_all(local_root).await?;
    let conn = db::connect(local_root)?;

    println!(
        "Cloning {} into {} ...",
        remote_path,
        local_root.to_string_lossy()
    );

    let (dirs, files) =
        visit_all_dirs(remote_path, remote_path.len(), &ignore_filter, token).await?;

    let config = Config {
        remote_path: remote_path.to_owned(),
        sync_dirs: dirs.clone(),
    };

    for dir in dirs.iter() {
        create_dirs(dir, &config, local_root).await?;
    }

    for file in files.iter() {
        if let Some(ref name) = file.path_display {
            download_file(name, &config, &local_root, &conn, token).await?;
        }
    }

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
