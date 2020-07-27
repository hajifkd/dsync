pub mod download;
pub mod list_folder;
pub mod upload;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct FileInfo {
    pub name: String,
    pub id: String,
    pub client_modified: String,
    pub server_modified: String,
    pub rev: String,
    pub size: u64,
    pub path_lower: Option<String>,
    pub path_display: Option<String>,
    pub content_hash: Option<String>,
}
