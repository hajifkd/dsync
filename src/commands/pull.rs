use super::{
    construct_local_path, construct_meta_path, create_dirs, download_file, file_status,
    load_config, visit_all_dirs, Config, FileStatus,
};
use crate::files::FileInfo;
use crate::ignore::parce_ignore;
use crate::{db, file_hash};
use rusqlite::Connection;
use std::collections::HashMap;

use std::path::Path;

pub async fn clone(
    local_root: impl AsRef<Path>,
    token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let ignore_filter = parce_ignore().await?;
    let local_root = local_root.as_ref();
    let config = load_config(local_root).await.map_err(|_| {
        format!(
            "Directory {} is not a correct dsync repo.",
            local_root.display()
        )
    })?;
    let conn = db::connect(local_root)?;

    println!("Checking updates in {}", &config.remote_path);
    let (remote_dirs, remote_files) = {
        let (mut dirs, files) = visit_all_dirs(
            &config.remote_path,
            config.remote_path.len(),
            &ignore_filter,
            token,
        )
        .await?;
        dirs.sort_unstable_by(|a, b| b.cmp(a)); // desc sorted
        (dirs, files)
    };
    let local_dirs = {
        let mut dirs: Vec<&str> = config.sync_dirs.iter().map(|s| &**s).collect();
        dirs.sort_unstable_by(|a, b| b.cmp(a));
        dirs
    };
    let local_files = db::list_files(&conn)?;

    let mut i_remote_dir = 0;
    let mut i_local_dir = 0;
    let mut dir_to_remove = vec![];

    while i_remote_dir < remote_dirs.len() && i_local_dir < local_dirs.len() {
        if i_local_dir == local_dirs.len() {
            for remote_dir in remote_dirs[i_remote_dir..].into_iter() {
                create_dirs(remote_dir, &config, local_root).await?;
            }
        }

        if i_remote_dir == remote_dirs.len() {
            dir_to_remove.extend(local_dirs[i_local_dir..].into_iter());
        }

        if remote_dirs[i_remote_dir] == local_dirs[i_local_dir] {
            i_remote_dir += 1;
            i_local_dir += 1;
        } else if &remote_dirs[i_remote_dir][..] > local_dirs[i_local_dir] {
            create_dirs(&remote_dirs[i_remote_dir], &config, local_root).await?;
            i_remote_dir += 1;
        } else if &remote_dirs[i_remote_dir][..] < local_dirs[i_local_dir] {
            dir_to_remove.push(local_dirs[i_local_dir]);
            i_local_dir += 1;
        }
    }

    let files_to_unlink = update_files(
        &remote_files,
        local_files,
        local_root,
        &config,
        &conn,
        token,
    )
    .await?;

    unlink_files(&files_to_unlink, local_root, &config, &conn).await?;

    // TODO remove directories

    Ok(())
}

async fn update_files(
    remote_files: &[FileInfo],
    local_files: HashMap<String, db::FileData>,
    local_root: &Path,
    config: &Config,
    conn: &Connection,
    token: &str,
) -> Result<HashMap<String, db::FileData>, Box<dyn std::error::Error>> {
    let mut local_files = local_files;
    for remote_file in remote_files.into_iter() {
        let remote_hash = remote_file
            .content_hash
            .as_ref()
            .and_then(|s| hex::decode(s).ok());

        if remote_file.path_display.is_none() || remote_hash.is_none() {
            continue;
        }

        let path = remote_file.path_display.as_ref().unwrap(); // ensured

        let local_path = construct_local_path(path, &config, &local_root);
        let meta_path = construct_meta_path(path, &config, &local_root);

        let curr_hash = file_hash(&local_path).await.ok();
        let orig_hash = file_hash(&meta_path).await.ok();
        let repo_hash = local_files.remove(path).map(|f| f.hash);

        match file_status(
            remote_hash.as_deref(),
            curr_hash.as_ref().map(|h| &h[..]),
            repo_hash.as_deref(),
            orig_hash.as_ref().map(|h| &h[..]),
        ) {
            FileStatus::NotSaved => {
                println!(
                    "File {} is updated but being edited. Use add command to merge. Ignoring...",
                    local_path.display()
                );
            }
            FileStatus::ToBeUpdated => {
                println!("Updating file {} ...", local_path.display());
                download_file(&path, &config, &local_root, &conn, &token).await?;
            }
            FileStatus::ToBeCreated => {
                println!("Creating file {} ...", local_path.display());
                download_file(&path, &config, &local_root, &conn, &token).await?;
            }
            FileStatus::Conflicted => {
                // merge
            }
            FileStatus::IdenticallyChanged => {
                println!(
                    "Both remote and local repo adopt the same change for file {}.",
                    local_path.display()
                );
                // copy data to meta_path
                tokio::fs::copy(local_path, meta_path).await?;
            }
            FileStatus::NotChanged => {
                // Do nothing. logging?
            }
            FileStatus::OnlyLocallyChanged => {
                // Do nothing. logging?
            }
            FileStatus::ToBeRemoved => {
                panic!("Never reach here");
            }
        }
    }

    Ok(local_files)
}

async fn unlink_files(
    files_to_unlink: &HashMap<String, db::FileData>,
    local_root: &Path,
    config: &Config,
    conn: &Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    for (path, local_file) in files_to_unlink.into_iter() {
        let local_path = construct_local_path(path, &config, &local_root);
        let meta_path = construct_meta_path(path, &config, &local_root);

        let curr_hash = file_hash(&local_path).await.ok();
        let orig_hash = file_hash(&meta_path).await.ok();
        let repo_hash = &local_file.hash;

        match file_status(
            None,
            curr_hash.as_ref().map(|h| &h[..]),
            Some(repo_hash),
            orig_hash.as_ref().map(|h| &h[..]),
        ) {
            FileStatus::NotSaved => {
                println!(
                    "File {} is remotely removed but being edited. Ignoring...",
                    local_path.display()
                );
            }
            FileStatus::Conflicted => {
                println!(
                    "CONFLICT: File {} is remotely removed but locally modified.",
                    local_path.display()
                );
                // TODO add data in a conflict controlling table
            }
            FileStatus::IdenticallyChanged => {
                println!(
                    "Both remote and local repo remove file {}.",
                    local_path.display()
                );
                tokio::fs::remove_file(meta_path).await?;
            }
            FileStatus::ToBeRemoved => {
                tokio::fs::remove_file(local_path).await?;
                tokio::fs::remove_file(meta_path).await?;
                // TODO delete entry from DB.
            }
            FileStatus::ToBeUpdated
            | FileStatus::ToBeCreated
            | FileStatus::NotChanged
            | FileStatus::OnlyLocallyChanged => {
                panic!("Never reach here");
            }
        }
    }
    Ok(())
}
