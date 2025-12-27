use reqwest::blocking::*;
use serde::{Serialize, Deserialize};
use rand::Rng;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use sha256;

use super::model::*;
use super::error::Error;

const API_URL: &str = "https://app.api.kivra.com";
const ACCOUNTS_URL: &str = "https://accounts.kivra.com";

#[derive(Serialize)]
struct AuthRequest {
    response_type: String,
    code_challenge: String,
    code_challenge_method: String,
    scope: String,
    client_id: String,
    redirect_uri: String,
}

#[derive(Deserialize, Debug)]
pub struct AuthResponse {
    pub auto_start_token: String,
    pub qr_data: Vec<String>,
    pub qr_code: String,
    pub code: String,
    pub next_poll_url: String,
}

#[derive(Deserialize, Debug)]
pub struct AuthStatus {
    pub status: String,
    pub progress_status: String,
    pub message_code: String,
    pub qr_code: String,
    pub retry_after: u32,
    pub next_poll_url: String,
}

pub fn client() -> Client {
    Client::new()
}

pub fn get_config(client: &Client) -> Result<Config, Error> {
    Ok(client.get(format!("{ACCOUNTS_URL}/config.json")).send()?.json()?)
}

pub fn start_auth(client: &Client, config: &Config) ->
    Result<AuthResponse, Error>
{
    let mut code_verifier = [0u8; 64];
    rand::thread_rng().try_fill(&mut code_verifier)?;
    let challenge = URL_SAFE.encode(sha256::digest(&code_verifier));
    
    let auth_request = AuthRequest {
	client_id: config.oauth_default_client_id.clone(),
	response_type: "bankid_all".into(),
	code_challenge: challenge,
	code_challenge_method: "S256".into(),
	scope: "openid profile".into(),
	redirect_uri: "https://inbox.kivra.com/auth/kivra/return".into(),
    };

    let resp = client.post(format!("{API_URL}/v2/oauth2/authorize"))
	.json(&auth_request)
	.send()?
	.json()?;
    Ok(resp)
}

pub fn check_auth(client: &Client, poll_url: &String) ->
    Result<AuthStatus, Error>
{
    Ok(client.get(format!("{API_URL}{poll_url}")).send()?.json()?)
}

pub fn abort_auth(client: &Client, poll_url: &String) ->
    Result<(), Error>
{
    client.delete(format!("{API_URL}{poll_url}")).send()?.error_for_status()?;
    Ok(())
}
