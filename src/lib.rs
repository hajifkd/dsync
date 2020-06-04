extern crate reqwest;
extern crate tokio;

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::convert::TryInto;
use std::error::Error;
use tokio::io;
use tokio::prelude::*;

pub mod files;

const BASE_URL: &str = "https://api.dropboxapi.com/2";

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}

pub(crate) async fn request_json(
    api: &str,
    access_token: &str,
    headers: Option<&HashMap<String, String>>,
    body_json: &(impl Serialize + ?Sized),
) -> Result<reqwest::Response, Box<dyn Error>> {
    let mut req = reqwest::Client::new()
        .post(&format!("{}/{}", BASE_URL, api))
        .header("Authorization", format!("Bearer {}", access_token));
    if let Some(headers) = headers {
        req = req.headers(headers.try_into()?);
    }
    Ok(req.json(body_json).send().await?)
}

pub async fn request_json_response_text(
    api: &str,
    access_token: &str,
    headers: Option<&HashMap<String, String>>,
    body_json: &(impl Serialize + ?Sized),
) -> Result<String, Box<dyn Error>> {
    Ok(request_json(api, access_token, headers, body_json)
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
    Ok(request_json(api, access_token, headers, body_json)
        .await?
        .json()
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

fn bytes_to_hex_string(data: &[u8]) -> String {
    data.into_iter().fold(String::new(), |mut acc, x| {
        acc.push_str(&format!("{:02x}", x));
        acc
    })
}

pub fn content_hash(data: &[u8]) -> String {
    const LEN: usize = 4 * 1024 * 1024;
    let mut index = 0;
    let mut hashes = vec![];

    while index < data.len() {
        let hash = Sha256::digest(&data[index..std::cmp::min(data.len(), index + LEN)]);
        hashes.push(hash);
        index += LEN;
    }

    bytes_to_hex_string(&Sha256::digest(&(hashes.join(&[][..])[..])))
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
            content_hash(&data)
        )
    }
}
