extern crate reqwest;
extern crate tokio;

use std::collections::HashMap;
use std::convert::TryInto;
use std::error::Error;
use tokio::prelude::*;

const BASE_URL: &str = "https://api.dropboxapi.com/2";

pub(crate) async fn request(
    api: &str,
    headers: &HashMap<String, String>,
    body_json: &HashMap<String, String>,
) -> Result<HashMap<String, String>, Box<dyn Error>> {
    Ok(reqwest::Client::new()
        .post(&format!("{}/{}", BASE_URL, api))
        .headers(headers.try_into()?)
        .json(body_json)
        .send()
        .await?
        .json()
        .await?)
}

pub(crate) fn conf_path() -> Result<String, std::env::VarError> {
    Ok(format!("{}/.dsync_config", std::env::var("HOME")?))
}

pub(crate) async fn get_token() -> Result<String, Box<dyn Error>> {
    let conf_path = conf_path()?;
    if let Ok(d) = tokio::fs::read(&conf_path).await {
        Ok(String::from_utf8(d)?)
    } else {
        tokio::io::stdout()
            .write_all(b"Paste the token Here: ")
            .await?;
        let mut buf = String::new();
        tokio::io::stdin().read_to_string(&mut buf).await?;
        tokio::fs::File::create(&conf_path)
            .await?
            .write_all(buf.as_bytes())
            .await?;
        Ok(buf)
    }
}
