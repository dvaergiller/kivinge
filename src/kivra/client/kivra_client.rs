use crate::kivra::model::*;
use crate::kivra::session::Session;
use crate::error::Error;
use super::Client;

const API_URL: &str = "https://app.api.kivra.com";
const ACCOUNTS_URL: &str = "https://accounts.kivra.com";

pub struct KivraClient {
    client: reqwest::blocking::Client,
}

impl KivraClient {
    pub fn new() -> KivraClient {
        KivraClient { client: reqwest::blocking::Client::new() }
    }
}

impl Client for KivraClient {
    fn get_config(&self) -> Result<Config, Error> {
        Ok(self.client
           .get(format!("{ACCOUNTS_URL}/config.json"))
           .send()?
           .error_for_status()?
           .json()?)
    }

    fn start_auth(&self, config: &Config) -> Result<(CodeVerifier, AuthResponse), Error> {
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

        let resp = self.client
            .post(format!("{API_URL}/v2/oauth2/authorize"))
            .json(&auth_request)
            .send()?
            .error_for_status()?
            .json()?;
        Ok((verifier, resp))
    }

    fn check_auth(&self, poll_url: &String) -> Result<AuthStatus, Error> {
        Ok(self.client
           .get(format!("{API_URL}{poll_url}"))
           .send()?
           .error_for_status()?
           .json()?)
    }

    fn abort_auth(&self, poll_url: &String) -> Result<(), Error> {
        self.client
            .delete(format!("{API_URL}{poll_url}"))
            .send()?
            .error_for_status()?;
        Ok(())
    }

    fn get_auth_token(&self,
                      config: &Config,
                      auth_code: String,
                      verifier: CodeVerifier,
    ) -> Result<AuthTokenResponse, Error> {
        let verifier_string = String::from_utf8(verifier)
            .map_err(|e| Error::AppError(e.to_string()))?;

        let token_request = AuthTokenRequest {
            client_id: config.oauth_default_client_id.clone(),
            code: auth_code,
            code_verifier: verifier_string,
            grant_type: "authorization_code".to_string(),
            redirect_uri: config.oauth_default_redirect_uri.clone(),
        };

        let resp = self.client
            .post(format!("{API_URL}/v2/oauth2/token"))
            .json(&token_request)
            .send()?
            .error_for_status()?;
        Ok(resp.json()?)
    }

    fn revoke_auth_token(&self, session: &Session) -> Result<(), Error> {
        self.client
            .post(format!("{API_URL}/v2/oauth2/token/revoke"))
            .json(&RevokeRequest {
                token: session.access_token.clone(),
                token_type_hint: "access_token".to_string(),
            })
            .send()?
            .error_for_status()?;
        Ok(())
    }

    fn get_inbox_listing(&self, session: &Session) -> Result<Vec<ContentSpec>, Error> {
        let user_id = &session.user_info.kivra_user_id;
        Ok(self.client
           .get(format!("{API_URL}/v3/user/{user_id}/content"))
           .query(&[("listing", "all")])
           .bearer_auth(&session.access_token)
           .send()?
           .error_for_status()?
           .json()?)
    }
}
