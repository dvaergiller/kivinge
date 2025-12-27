use super::model::*;
use super::session::Session;
use super::error::Error;

pub type Client = reqwest::blocking::Client;

const API_URL: &str = "https://app.api.kivra.com";
const ACCOUNTS_URL: &str = "https://accounts.kivra.com";

pub fn client() -> Client {
    Client::new()
}

pub fn get_config(client: &Client) -> Result<Config, Error> {
    Ok(client.get(format!("{ACCOUNTS_URL}/config.json")).send()?.json()?)
}

pub fn start_auth(client: &Client, config: &Config) ->
    Result<(String, AuthResponse), Error>
{
    let verifier = pkce::code_verifier(48);
    let challenge = pkce::code_challenge(&verifier);

    let auth_request = AuthRequest {
        client_id: config.oauth_default_client_id.clone(),
        response_type: "bankid_all".to_string(),
        code_challenge: challenge,
        code_challenge_method: "S256".into(),
        scope: "openid profile".into(),
        redirect_uri: config.oauth_default_redirect_uri.clone(),
    };

    let resp = client.post(format!("{API_URL}/v2/oauth2/authorize"))
        .json(&auth_request)
        .send()?
        .json()?;
    Ok((String::from_utf8(verifier).unwrap(), resp))
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

pub fn get_auth_token(client: &Client,
                      config: &Config,
                      auth_response: &AuthResponse,
                      verifier: String) ->
    Result<AuthTokenResponse, Error>
{
    let token_request = AuthTokenRequest {
        client_id: config.oauth_default_client_id.clone(),
        code: auth_response.code.clone(),
        code_verifier: verifier,
        grant_type: "authorization_code".to_string(),
        redirect_uri: config.oauth_default_redirect_uri.clone(),
    };
    let resp = client.post(format!("{API_URL}/v2/oauth2/token"))
        .json(&token_request)
        .send()?;
    Ok(resp.json()?)
}

pub fn revoke_auth_token(client: &Client, session: Session) ->
    Result<(), Error>
{
    client.post(format!("{API_URL}/v2/oauth2/token/revoke"))
        .json(&RevokeRequest {
            token: session.access_token,
            token_type_hint: "access_token".to_string()
        })
        .send()?
        .error_for_status()?;
    Ok(())
}

pub fn get_inbox_listing(client: &Client, session: &Session) ->
    Result<Vec<ContentSpec>, Error>
{
    let user_id = &session.user_info.kivra_user_id;
    Ok(client.get(format!("{API_URL}/v3/user/{user_id}/content"))
       .query(&[("listing", "all")])
       .bearer_auth(&session.access_token)
       .send()?
       .json()?)
}
