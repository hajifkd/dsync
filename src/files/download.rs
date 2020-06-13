use crate::request_response_blob;
use bytes::Bytes;
use serde::Deserialize;
use serde_json;
use std::error::Error;

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

pub async fn download(path: &str, token: &str) -> Result<(FileInfo, Bytes), Box<dyn Error>> {
    request_response_blob(
        "files/download",
        token,
        None,
        &serde_json::to_string(&{
            let mut arg = std::collections::HashMap::new();
            arg.insert("path".to_owned(), path.to_owned());
            arg
        })
        .unwrap(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[tokio::test]
    async fn download_test() {
        assert_eq!(
            "485291fa0ee50c016982abbfa943957bcd231aae0492ccbaa22c58e3997b35e0",
            bytes_to_hex_string(&content_hash(
                &download("/milky-way-nasa.jpg", &get_token().await.unwrap())
                    .await
                    .unwrap()
                    .1
            ))
        )
    }
}
