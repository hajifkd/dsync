use crate::files::list_folder::{self, Entry};
use crate::files::FileInfo;
use crate::ignore::Ignore;
use crate::{db, files};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::{Path, PathBuf};
use tokio::fs;

pub mod clone;

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

pub(crate) async fn visit_all_dirs(
    initial: &str,
    ignore_filter: &Ignore,
    token: &str,
) -> Result<(Vec<String>, Vec<FileInfo>), Box<dyn Error>> {
    let mut dirs = vec![initial.to_owned()];
    let mut index = 0;
    let mut files = vec![];

    while index < dirs.len() {
        let new_index = dirs.len();
        for i in index..new_index {
            let dir = &dirs[i];
            if ignore_filter.is_ignored(dir) {
                continue;
            }

            let entries = read_dir(dir, token).await?;

            for entry in entries.into_iter() {
                match entry {
                    Entry::File(file_info) => files.push(file_info),
                    Entry::Folder { path_display, .. } => {
                        if let Some(name) = path_display {
                            dirs.push(name);
                        }
                    }
                    _ => (),
                }
            }
        }
        index = new_index;
    }

    Ok((dirs, files))
}
