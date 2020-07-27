use crate::files::list_folder::Entry;
use crate::files::{download, list_folder};
use crate::ignore::{parce_ignore, IGNORE_FILE};
use tokio::fs;

use std::path::{Path, PathBuf};

pub async fn clone(
    remote_path: &str,
    local_path: &Path,
    token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let curr_dir = std::env::current_dir()?;
    let is_curr = curr_dir == local_path;
    let ignore_filter = parce_ignore().await?;

    fs::create_dir_all(local_path).await?;

    let mut dirs = vec![remote_path.to_owned()];
    let mut files: Vec<String> = vec![];

    // Download process is intentionally non-parallel
    while dirs.len() > 0 || files.len() > 0 {
        for file in files.iter() {
            if !ignore_filter.is_ignored(file) {
                download_file(file, remote_path.len(), local_path, token).await?;
            }
        }

        let mut new_dirs = vec![];
        let mut new_files = vec![];

        for dir in dirs.iter() {
            let entries = read_dir(dir, remote_path.len(), local_path, token).await?;

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

    if !is_curr {
        let mut ignore_file = curr_dir.clone();
        ignore_file.push(IGNORE_FILE);
        let mut ignore_file_dst = local_path.to_owned();
        ignore_file_dst.push(IGNORE_FILE);
        fs::copy(ignore_file, ignore_file_dst).await?;
    }
    Ok(())
}

fn construct_path(remote_path: &str, remote_path_base_n: usize, local_path: &Path) -> PathBuf {
    let mut path_name = &remote_path[remote_path_base_n..];
    if path_name.starts_with('/') {
        path_name = &path_name[1..];
    }
    let mut local_path = local_path.to_owned();
    local_path.push(path_name);

    local_path
}

async fn download_file(
    remote_path: &str,
    remote_path_base_n: usize,
    local_root: &Path,
    token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let local_file = construct_path(remote_path, remote_path_base_n, local_root);
    let (info, data) = download::download(remote_path, token).await?;
    // TODO put info in DB and backups in backup folder
    fs::write(local_file, data).await?;
    Ok(())
}

async fn read_dir(
    remote_path: &str,
    remote_path_base_n: usize,
    local_root: &Path,
    token: &str,
) -> Result<Vec<Entry>, Box<dyn std::error::Error>> {
    let local_dir = construct_path(remote_path, remote_path_base_n, local_root);
    fs::create_dir_all(local_dir).await?;
    let entries = list_folder::list_folder(
        remote_path,
        token,
        #[cfg(test)]
        200,
    )
    .await?;
    Ok(entries)
}
