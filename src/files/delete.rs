use crate::request_json_response_json;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = ".tag", rename_all = "snake_case")]
pub enum DeleteResult {
    Metadata(super::FileInfo),
    ErrorSummary(String),
}

pub async fn delete(path: &str, token: &str) -> Result<DeleteResult, Box<dyn std::error::Error>> {
    let mut json = HashMap::new();
    json.insert("path".to_owned(), path.to_owned());
    Ok(request_json_response_json("/files/delete_v2", token, None, &json).await?)
}
