use crate::files::download;
use crate::files::list_folder::{self, Entry};
use crate::files::FileInfo;
use crate::ignore::Ignore;
use crate::{db, files};

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::{Path, PathBuf};
use tokio::fs;

pub mod add;
pub mod clone;
pub mod init;
pub mod pull;
// TODO push, rm, auth?

pub const CONF_DIR: &str = ".dsync";
pub const CONF_FILE: &str = ".dsyncconfig";

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
pub(crate) struct Config {
    pub remote_path: String,
    pub sync_dirs: Vec<String>,
}

impl Config {
    fn remote_to_local_path<'a>(&self, path: &'a str) -> &'a str {
        &path[self.remote_path.len()..]
    }
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
    let path_name = config.remote_to_local_path(remote_path);
    let mut local_path = local_root.to_owned();

    for path in path_name.split('/') {
        if path != "" {
            local_path.push(path);
        }
    }

    local_path
}

pub(crate) fn construct_remote_path(
    local_path: impl AsRef<Path>,
    config: &Config,
    local_root: impl AsRef<Path>,
) -> Result<String, Box<dyn Error>> {
    let local_canonical_path = local_path.as_ref().canonicalize()?;
    let canonical_root = local_root.as_ref().canonicalize()?;
    let addition = local_canonical_path.strip_prefix(canonical_root)?;
    let addition = addition
        .strip_prefix(std::path::MAIN_SEPARATOR.to_string()) // remove / if exists
        .unwrap_or(addition)
        .iter() // replace the separator into /
        .map(|s| s.to_str())
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| format!("Invalid file name {}", local_path.as_ref().display()))?
        .join("/");
    let ref remote_path = config.remote_path;

    if remote_path.ends_with("/") {
        Ok(format!("{}{}", remote_path, addition))
    } else {
        Ok(format!("{}/{}", remote_path, addition))
    }
}

pub(crate) fn construct_meta_path(
    remote_path: &str,
    config: &Config,
    local_root: &Path,
) -> PathBuf {
    let mut local_root = local_root.to_owned();
    local_root.push(CONF_DIR);
    construct_local_path(remote_path, config, &local_root)
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
    n_remote_path: usize,
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
            if ignore_filter.is_ignored(&dir[n_remote_path..]) {
                continue;
            }

            let entries = read_dir(dir, token).await?;

            for entry in entries.into_iter() {
                match entry {
                    Entry::File(file_info) => {
                        if let Some(ref name) = file_info.path_display {
                            if !ignore_filter.is_ignored(&name[n_remote_path..]) {
                                files.push(file_info);
                            }
                        }
                    }
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

pub(crate) async fn download_file(
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

pub(crate) async fn create_dirs(
    remote_dir: &str,
    config: &Config,
    local_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let local_dir = construct_local_path(remote_dir, config, local_root);
    println!("Creating directory {} ...", local_dir.to_string_lossy());
    fs::create_dir_all(local_dir).await?;
    create_metadir(remote_dir, config, local_root).await?;
    Ok(())
}

#[derive(Eq, PartialEq, Debug)]
pub(crate) enum FileStatus {
    NotChanged,
    OnlyLocallyChanged,
    ToBeUpdated,
    ToBeCreated,
    NotSaved,
    Conflicted,
    ToBeRemoved,
    IdenticallyChanged,
}

pub(crate) fn file_status(
    remote_hash: Option<&[u8]>,
    curr_hash: Option<&[u8]>,
    repo_hash: Option<&[u8]>,
    orig_hash: Option<&[u8]>,
) -> FileStatus {
    if curr_hash != repo_hash {
        FileStatus::NotSaved
    } else {
        if remote_hash.is_some() {
            if remote_hash == orig_hash {
                if repo_hash == orig_hash {
                    FileStatus::NotChanged
                } else {
                    FileStatus::OnlyLocallyChanged
                }
            } else {
                if repo_hash == orig_hash {
                    if repo_hash.is_some() {
                        FileStatus::ToBeUpdated
                    } else {
                        FileStatus::ToBeCreated
                    }
                } else {
                    if remote_hash == repo_hash {
                        FileStatus::IdenticallyChanged
                    } else {
                        FileStatus::Conflicted
                    }
                }
            }
        } else {
            if repo_hash == orig_hash {
                FileStatus::ToBeRemoved
            } else {
                if repo_hash.is_none() {
                    FileStatus::IdenticallyChanged
                } else {
                    FileStatus::Conflicted
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_status_test() {
        assert_eq!(
            file_status(None, Some(&[1]), Some(&[1]), Some(&[1])),
            FileStatus::ToBeRemoved
        );

        assert_eq!(
            file_status(None, Some(&[2]), Some(&[1]), Some(&[1])),
            FileStatus::NotSaved
        );
        assert_eq!(
            file_status(None, Some(&[2]), Some(&[1]), None),
            FileStatus::NotSaved
        );
        assert_eq!(
            file_status(None, Some(&[2]), None, Some(&[1])),
            FileStatus::NotSaved
        );
        assert_eq!(
            file_status(None, Some(&[2]), None, None),
            FileStatus::NotSaved
        );

        assert_eq!(
            file_status(None, Some(&[2]), Some(&[2]), Some(&[1])),
            FileStatus::Conflicted
        );
        assert_eq!(
            file_status(None, Some(&[2]), Some(&[2]), None),
            FileStatus::Conflicted
        );

        assert_eq!(
            file_status(None, None, None, Some(&[1])),
            FileStatus::IdenticallyChanged
        );

        //////

        assert_eq!(
            file_status(Some(&[1]), Some(&[1]), Some(&[1]), Some(&[1])),
            FileStatus::NotChanged
        );

        assert_eq!(
            file_status(Some(&[1]), Some(&[2]), Some(&[1]), Some(&[1])),
            FileStatus::NotSaved
        );
        assert_eq!(
            file_status(Some(&[1]), Some(&[2]), Some(&[3]), Some(&[1])),
            FileStatus::NotSaved
        );

        assert_eq!(
            file_status(Some(&[1]), Some(&[2]), Some(&[2]), Some(&[1])),
            FileStatus::OnlyLocallyChanged
        );

        assert_eq!(
            file_status(Some(&[2]), Some(&[1]), Some(&[1]), Some(&[1])),
            FileStatus::ToBeUpdated
        );
        assert_eq!(
            file_status(Some(&[2]), None, None, None),
            FileStatus::ToBeCreated
        );

        assert_eq!(
            file_status(Some(&[3]), Some(&[2]), Some(&[2]), Some(&[1])),
            FileStatus::Conflicted
        );
        assert_eq!(
            file_status(Some(&[3]), Some(&[2]), Some(&[2]), None),
            FileStatus::Conflicted
        );
        assert_eq!(
            file_status(Some(&[3]), None, None, Some(&[1])),
            FileStatus::Conflicted
        );

        assert_eq!(
            file_status(Some(&[2]), Some(&[2]), Some(&[2]), None),
            FileStatus::IdenticallyChanged
        );
        assert_eq!(
            file_status(Some(&[2]), Some(&[2]), Some(&[2]), Some(&[1])),
            FileStatus::IdenticallyChanged
        );
    }
}
