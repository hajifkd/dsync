use super::download::FileInfo;
use crate::request_blob_response_json;
use serde_json::{json, Value};
use std::collections::HashMap;

pub async fn upload(
    data: impl Into<reqwest::Body>,
    dpath: &str,
    rev: Option<&str>,
    token: &str,
) -> Result<FileInfo, Box<dyn std::error::Error>> {
    let mut arg = HashMap::new();
    arg.insert("path".to_owned(), Value::String(dpath.to_owned()));
    if let Some(rev) = rev {
        let mode = json!({
            ".tag": "update",
            "update": rev,
        });
        arg.insert("mode".to_owned(), mode);
    }

    request_blob_response_json(
        "files/upload",
        token,
        None,
        &serde_json::to_string(&arg).unwrap(),
        data,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[tokio::test]
    async fn upload_test() {
        let data = std::fs::read("milky-way-nasa.jpg").unwrap();
        let result = upload(
            data,
            "/dsync_test/milky-way-nasa.jpg",
            None,
            &get_token().await.unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(
            "485291fa0ee50c016982abbfa943957bcd231aae0492ccbaa22c58e3997b35e0",
            result.content_hash.unwrap()
        )
    }
}
