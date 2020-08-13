use super::{construct_remote_path, load_config, save_config};
use crate::db::{self, Connection};
use crate::file_hash;
use crate::ignore::parce_ignore;

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
    // remote_path never ends with /.
    let remote_path = construct_remote_path(target, &config, &local_root)?;

    if target.is_file() {
        db::upsert_file(
            conn,
            db::FileData::new(remote_path, file_hash(target).await?.to_vec()),
        )
    } else if target.is_dir() {
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
    remote_path: String,
    conn: &Connection,
) -> Result<(), Box<dyn Error>> {
    db::upsert_file(
        conn,
        db::FileData::new(remote_path, file_hash(target).await?.to_vec()),
    )
}
