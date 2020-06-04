use crate::request_json_response_json;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;

#[derive(Deserialize)]
struct ListFolder {
    entries: Vec<Entry>,
    cursor: String,
    has_more: bool,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = ".tag", rename_all = "lowercase")]
pub enum Entry {
    File {
        name: String,
        id: String,
        client_modified: String,
        server_modified: String,
        rev: String,
        size: u64,
        path_lower: Option<String>,
        path_display: Option<String>,
        content_hash: Option<String>,
    },
    Folder {
        name: String,
        id: String,
        path_lower: Option<String>,
        path_display: Option<String>,
    },
    Deleted {
        name: String,
        path_lower: Option<String>,
        path_display: Option<String>,
    },
}

pub async fn list_folder(
    path: &str,
    token: &str,
    #[cfg(test)] limit: u32,
) -> Result<Vec<Entry>, Box<dyn Error>> {
    let mut json = HashMap::new();
    json.insert("path".to_owned(), Value::String(path.to_owned()));
    #[cfg(test)]
    {
        json.insert("limit".to_owned(), limit.into());
    }
    let ListFolder {
        mut entries,
        mut cursor,
        mut has_more,
    } = request_json_response_json("/files/list_folder", token, None, &json).await?;
    while has_more {
        let mut json = HashMap::new();
        json.insert("cursor".to_owned(), cursor.clone());

        let ListFolder {
            entries: e2,
            cursor: c2,
            has_more: h2,
        } = request_json_response_json("/files/list_folder/continue", token, None, &json).await?;
        cursor = c2;
        has_more = h2;
        entries.extend(e2.into_iter());
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_folder_test() {
        let token = crate::get_token().await.unwrap();
        let mut result1 = list_folder("", &token, 2000).await.unwrap();
        let mut result2 = list_folder("", &token, 100).await.unwrap();
        result1.sort_unstable();
        result2.sort_unstable();
        assert_eq!(result1, result2);
    }
}
