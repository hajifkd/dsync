use super::{construct_local_path, create_metadir, load_config, upsert_metadata, visit_all_dirs};
use crate::db;
use crate::files::download;
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
    let config = load_config(local_root).await.map_err(|_| {
        format!(
            "Directory {} is not a correct dsync repo.",
            local_root.display()
        )
    })?;
    let conn = db::connect(local_root)?;

    println!("Checking updates in {}", &config.remote_path);
    let (dirs, files) = visit_all_dirs(remote_path, &ignore_filter, token).await?;
    let synced_files = db::list_files(&conn);

    Ok(())
}
