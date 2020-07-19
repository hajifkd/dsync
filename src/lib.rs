extern crate bytes;
extern crate reqwest;
extern crate rusqlite;
extern crate tokio;

use bytes::Bytes;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::convert::TryInto;
use std::error::Error;
use tokio::io;
use tokio::prelude::*;

pub mod db;
pub mod files;

const BASE_URL: &str = "https://api.dropboxapi.com/2";
const CONTENT_BASE_URL: &str = "https://content.dropboxapi.com/2";
const RESULT_HEADER: &str = "Dropbox-API-Result";

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}

pub(crate) async fn request_json(
    url: &str,
    access_token: &str,
    headers: Option<&HashMap<String, String>>,
    body_json: &(impl Serialize + ?Sized),
) -> Result<reqwest::Response, Box<dyn Error>> {
    let mut req = reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", access_token));
    if let Some(headers) = headers {
        req = req.headers(headers.try_into()?);
    }
    Ok(req.json(body_json).send().await?)
}

pub(crate) async fn request(
    url: &str,
    access_token: &str,
    headers: Option<&HashMap<String, String>>,
    arg: &str,
) -> Result<reqwest::Response, Box<dyn Error>> {
    let mut req = reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", access_token));
    if let Some(headers) = headers {
        req = req.headers(headers.try_into()?);
    }
    Ok(req.query(&[("arg", arg)]).send().await?)
}

pub(crate) async fn request_blob(
    url: &str,
    access_token: &str,
    headers: Option<&HashMap<String, String>>,
    arg: &str,
    body: impl Into<reqwest::Body>,
) -> Result<reqwest::Response, Box<dyn Error>> {
    let mut req = reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/octet-stream");
    if let Some(headers) = headers {
        req = req.headers(headers.try_into()?);
    }
    Ok(req.query(&[("arg", arg)]).body(body).send().await?)
}

pub(crate) async fn request_response_blob<T>(
    api: &str,
    access_token: &str,
    headers: Option<&HashMap<String, String>>,
    arg: &str,
) -> Result<(T, Bytes), Box<dyn Error>>
where
    T: serde::de::DeserializeOwned,
{
    let response = request(
        &format!("{}/{}", CONTENT_BASE_URL, api),
        access_token,
        headers,
        arg,
    )
    .await?;
    let result = response
        .headers()
        .get(RESULT_HEADER)
        .ok_or_else(|| "Result header not found".to_owned())?
        .to_str()?;
    Ok((serde_json::from_str(&result)?, response.bytes().await?))
}

pub async fn request_json_response_text(
    api: &str,
    access_token: &str,
    headers: Option<&HashMap<String, String>>,
    body_json: &(impl Serialize + ?Sized),
) -> Result<String, Box<dyn Error>> {
    Ok(request_json(
        &format!("{}/{}", BASE_URL, api),
        access_token,
        headers,
        body_json,
    )
    .await?
    .text()
    .await?)
}

pub async fn request_json_response_json<T>(
    api: &str,
    access_token: &str,
    headers: Option<&HashMap<String, String>>,
    body_json: &(impl Serialize + ?Sized),
) -> Result<T, Box<dyn Error>>
where
    T: serde::de::DeserializeOwned,
{
    Ok(request_json(
        &format!("{}/{}", BASE_URL, api),
        access_token,
        headers,
        body_json,
    )
    .await?
    .json()
    .await?)
}

pub async fn request_blob_response_json<T>(
    api: &str,
    access_token: &str,
    headers: Option<&HashMap<String, String>>,
    arg: &str,
    body: impl Into<reqwest::Body>,
) -> Result<T, Box<dyn Error>>
where
    T: serde::de::DeserializeOwned,
{
    Ok(request_blob(
        &format!("{}/{}", CONTENT_BASE_URL, api),
        access_token,
        headers,
        arg,
        body,
    )
    .await?
    .json()
    .await?)
}

pub async fn request_blob_response_text(
    api: &str,
    access_token: &str,
    headers: Option<&HashMap<String, String>>,
    arg: &str,
    body: impl Into<reqwest::Body>,
) -> Result<String, Box<dyn Error>> {
    Ok(request_blob(
        &format!("{}/{}", CONTENT_BASE_URL, api),
        access_token,
        headers,
        arg,
        body,
    )
    .await?
    .text()
    .await?)
}

pub(crate) fn conf_path() -> Result<String, std::env::VarError> {
    Ok(format!("{}/.dsync_config", std::env::var("HOME")?))
}

pub async fn get_token() -> Result<String, Box<dyn Error>> {
    let conf_path = conf_path()?;
    let mut result = if let Ok(d) = tokio::fs::read(&conf_path).await {
        String::from_utf8(d)?
    } else {
        let mut stdout = io::stdout();
        stdout.write_all(b"Paste the token Here: ").await?;
        stdout.flush().await?;
        let mut buf = String::new();
        io::BufReader::new(io::stdin()).read_line(&mut buf).await?;
        tokio::fs::File::create(&conf_path)
            .await?
            .write_all(buf.as_bytes())
            .await?;
        buf
    };

    trim_newline(&mut result);

    Ok(result)
}

pub fn bytes_to_hex_string(data: &[u8]) -> String {
    data.into_iter().fold(String::new(), |mut acc, x| {
        acc.push_str(&format!("{:02x}", x));
        acc
    })
}

pub fn content_hash(data: &[u8]) -> [u8; 32] {
    const LEN: usize = 4 * 1024 * 1024;
    let mut index = 0;
    let mut hashes = vec![];

    while index < data.len() {
        let hash = Sha256::digest(&data[index..std::cmp::min(data.len(), index + LEN)]);
        hashes.push(hash);
        index += LEN;
    }

    Sha256::digest(&(hashes.join(&[][..])[..])).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hash_test() {
        let file = "milky-way-nasa.jpg";
        let data = if let Ok(d) = tokio::fs::read(file).await {
            d
        } else {
            let buf =
                reqwest::get("https://www.dropbox.com/static/images/developers/milky-way-nasa.jpg")
                    .await
                    .unwrap()
                    .bytes()
                    .await
                    .unwrap();
            tokio::fs::File::create(file)
                .await
                .unwrap()
                .write_all(&buf)
                .await
                .unwrap();
            (&buf[..]).to_owned()
        };
        assert_eq!(
            "485291fa0ee50c016982abbfa943957bcd231aae0492ccbaa22c58e3997b35e0",
            bytes_to_hex_string(&content_hash(&data))
        )
    }
}
