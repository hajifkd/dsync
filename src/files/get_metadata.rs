use super::FileInfo;
use crate::request_json_response_json;
use std::collections::HashMap;

pub async fn get_metadata(path: &str, token: &str) -> Result<FileInfo, Box<dyn std::error::Error>> {
    let mut json = HashMap::new();
    json.insert("path".to_owned(), path.to_owned());
    Ok(request_json_response_json("/files/get_metadata", token, None, &json).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hash_test() {
        let token = crate::get_token().await.unwrap();
        let info = get_metadata("/dsync_test/milky-way-nasa.jpg", &token)
            .await
            .unwrap();
        assert_eq!(
            "485291fa0ee50c016982abbfa943957bcd231aae0492ccbaa22c58e3997b35e0",
            info.content_hash.unwrap()
        );
    }
}
