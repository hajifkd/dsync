use super::{
    construct_local_path, construct_meta_path, create_dirs, create_metadir_for_file, download_file,
    file_status, load_config, save_config, visit_all_dirs, Config, FileStatus,
};
use crate::files::{delete, download, get_metadata, FileInfo};
use crate::ignore::parce_ignore;
use crate::{content_hash, db, file_hash};
use diffmerge::merge;
use rusqlite::Connection;
use std::collections::HashMap;

use std::path::Path;

pub async fn push(
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
    let updates = db::list_files_to_update(&conn)?;

    for update in updates.iter() {
        let path = &update.path;
        let local_path = construct_local_path(path, &config, &local_root);
        let meta_path = construct_meta_path(path, &config, &local_root);

        let repo_hash = db::find_file(&conn, path).ok().map(|fd| fd.hash);
        let curr_hash = file_hash(&local_path).await.ok();
        let orig_hash = file_hash(&meta_path).await.ok();
        let remote_hash = get_metadata::get_metadata(path, token)
            .await
            .ok()
            .and_then(|i| i.content_hash.as_ref().and_then(|s| hex::decode(s).ok()));

        if repo_hash != curr_hash {
            println!("The file {} is edited after added. Ignoring...", path);
            continue;
        }

        if orig_hash != remote_hash {
            println!(
                "The file {} is remotely updated. Pull first. Ignoring...",
                path
            );
            continue;
        }

        match update.operation {
            db::FileUpdate::ADD => {
                create_metadir_for_file(path, config, local_root);
                tokio::fs::copy(local_path, meta_path).await?;
            }
            db::FileUpdate::REMOVE => {
                delete::delete(path, token).await?;
                tokio::fs::remove_file(meta_path).await?;
                // TODO remove meta dir?
            }
            db::FileUpdate::UPDATE => {
                create_metadir_for_file(path, config, local_root);
                tokio::fs::copy(local_path, meta_path).await?;
            }
            _ => {
                panic!("The repository is broken");
            }
        }
    }

    db::clear_all_files_to_update(&conn)?;
}
