use crate::files::list_folder::{self, Entry};
use crate::{db, files};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::{Path, PathBuf};
use tokio::fs;

pub const CONF_DIR: &str = ".dsync";
pub const CONF_FILE: &str = ".dsyncconfig";

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
pub(crate) struct Config {
    pub remote_path: String,
}

pub(crate) async fn save_config(
    config: &Config,
    root: impl AsRef<Path>,
) -> Result<(), Box<dyn Error>> {
    let mut path = root.as_ref().to_owned();
    path.push(CONF_DIR);
    fs::create_dir_all(&path).await?;
    path.push(CONF_FILE);
    fs::write(&path, serde_json::to_string_pretty(config)?).await?;
    Ok(())
}

pub(crate) async fn load_config(root: impl AsRef<Path>) -> Result<Config, Box<dyn Error>> {
    let mut path = root.as_ref().to_owned();
    path.push(CONF_DIR);
    path.push(CONF_FILE);
    let data = fs::read(&path).await?;
    Ok(serde_json::from_str(std::str::from_utf8(&data)?)?)
}

pub(crate) fn construct_local_path(
    remote_path: &str,
    config: &Config,
    local_root: &Path,
) -> PathBuf {
    let path_name = &remote_path[config.remote_path.len()..];
    let mut local_path = local_root.to_owned();

    for path in path_name.split('/') {
        if path != "" {
            local_path.push(path);
        }
    }

    local_path
}

pub(crate) async fn create_metadir(
    remote_dir: &str,
    config: &Config,
    local_root: impl AsRef<Path>,
) -> Result<(), Box<dyn Error>> {
    let mut path = local_root.as_ref().to_owned();
    path.push(CONF_DIR);
    let path = construct_local_path(remote_dir, config, &path);
    fs::create_dir_all(&path).await?;
    Ok(())
}

pub(crate) async fn upsert_metadata(
    root: impl AsRef<Path>,
    connection: &rusqlite::Connection,
    config: &Config,
    info: files::FileInfo,
    data: &[u8],
) -> Result<(), Box<dyn Error>> {
    let mut path = root.as_ref().to_owned();
    path.push(CONF_DIR);
    if let Some(remote_path) = info.path_display {
        let file_path = construct_local_path(&remote_path, config, &path);
        fs::write(&file_path, data).await?;
        let metadata = db::FileData {
            path: remote_path,
            hash: info
                .content_hash
                .and_then(|s| hex::decode(s).ok())
                .unwrap_or_else(|| crate::content_hash(&data).to_vec()),
        };

        db::upsert_file(connection, &metadata)?;
    }
    Ok(())
}

async fn read_dir(
    remote_path: &str,
    token: &str,
) -> Result<Vec<Entry>, Box<dyn std::error::Error>> {
    let entries = list_folder::list_folder(
        remote_path,
        token,
        #[cfg(test)]
        200,
    )
    .await?;
    Ok(entries)
}

// TODO make it a function to return dirs and files
macro_rules! visit_all_files_and_dirs {
    ($dirs: expr, $files: expr, $ignore_filter: expr, $config: expr, $local_root: expr, $conn: expr, $token: expr, $process_file: expr, $process_dir:expr) => {{
        // This is intentionally non-parallel
        let mut dirs = $dirs;
        let mut files = $files;
        while dirs.len() > 0 || files.len() > 0 {
            for file in files.iter() {
                if !$ignore_filter.is_ignored(file) {
                    $process_file(file, $config, $local_root, $conn, $token).await?;
                }
            }

            let mut new_dirs = vec![];
            let mut new_files = vec![];

            for dir in dirs.iter() {
                if $ignore_filter.is_ignored(dir) {
                    continue;
                }
                $process_dir(dir, $config, $local_root, $conn).await?;
                let entries = super::read_dir(dir, $token).await?;

                for entry in entries.into_iter() {
                    match entry {
                        Entry::File(file_info) => {
                            if let Some(name) = file_info.path_display {
                                new_files.push(name);
                            }
                        }
                        Entry::Folder { path_display, .. } => {
                            if let Some(name) = path_display {
                                new_dirs.push(name);
                            }
                        }
                        _ => (),
                    }
                }
            }

            dirs = new_dirs;
            files = new_files;
        }
    }};
}

pub mod clone;
