use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;

const APP_KEY: &str = "64s4zj0hgs5kpfu";
const APP_SECRET: &str = "ibqbwy4rid535dy";
const TOKEN_URL: &str = "https://api.dropbox.com/oauth2/token";

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
struct AuthToken {
    pub access_token: String,
}

pub(crate) async fn code_to_token(code: &str) -> Result<String, Box<dyn Error>> {
    let mut data = HashMap::new();
    data.insert("code", code);
    data.insert("grant_type", "authorization_code");
    let auth: AuthToken = reqwest::Client::new()
        .post(TOKEN_URL)
        .basic_auth(APP_KEY, Some(APP_SECRET))
        .form(&data)
        .send()
        .await?
        .json()
        .await?;

    Ok(auth.access_token)
}
