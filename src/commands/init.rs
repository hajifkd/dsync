use super::{save_config, Config};
use crate::db;

use std::path::Path;

pub async fn init(
    remote_path: &str,
    local_root: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    db::connect(&local_root)?;
    save_config(
        &Config {
            remote_path: remote_path.to_owned(),
            sync_dirs: vec![],
        },
        &local_root,
    )
    .await?;
    Ok(())
}
