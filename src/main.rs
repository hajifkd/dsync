use dsync::get_token;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = get_token().await?;
    let mut json = HashMap::new();
    json.insert("path".to_owned(), "".to_owned());
    dbg!(dsync::request_json_response_text("/files/list_folder", &token, None, &json).await?);

    Ok(())
}
