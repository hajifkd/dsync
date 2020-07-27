pub mod clone;

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

pub(crate) fn construct_path(remote_path: &str, config: &Config, local_path: &Path) -> PathBuf {
    let path_name = &remote_path[config.remote_path.len()..];
    let mut local_path = local_path.to_owned();

    for path in path_name.split('/') {
        if path != "" {
            local_path.push(path);
        }
    }

    local_path
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
    fs::create_dir_all(&path).await?;
    if let Some(remote_path) = info.path_display {
        let file_path = construct_path(&remote_path, config, &path);
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
