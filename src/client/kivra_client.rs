use bytes::Bytes;
use reqwest::blocking::{RequestBuilder, Response};
use tracing::instrument;

use super::session::{self, Session};
use super::{Client, Error};
use crate::model::{auth::*, content::*, Config};
use crate::tui;

const API_URL: &str = "https://app.api.kivra.com";
const ACCOUNTS_URL: &str = "https://accounts.kivra.com";

macro_rules! get {
    ($self:ident, $pattern:literal) => {
        $self.client.get(format!($pattern))
    };
}

macro_rules! post {
    ($self:ident, $pattern:literal) => {
        $self.client.post(format!($pattern))
    };
}

macro_rules! delete {
    ($self:ident, $pattern:literal) => {
        $self.client.post(format!($pattern))
    };
}

trait Request {
    fn try_send(self) -> reqwest::Result<Response>;
}

impl Request for reqwest::blocking::RequestBuilder {
    #[instrument(level = "DEBUG")]
    fn try_send(self) -> reqwest::Result<Response> {
        self.send()?.error_for_status()
    }
}

pub struct KivraClient {
    client: reqwest::blocking::Client,
    session: Option<Session>,
}

impl KivraClient {
    pub fn new() -> Result<KivraClient, Error> {
        let client = reqwest::blocking::Client::builder()
            .use_native_tls()
            .build()?;
        Ok(KivraClient {
            client,
            session: None,
        })
    }

    pub fn auth_request(
        &mut self,
        request: RequestBuilder,
    ) -> Result<Response, Error> {
        let req_clone = request.try_clone().ok_or(Error::CloneError)?;
        match self.try_with_session(req_clone) {
            Ok(response) => Ok(response),
            Err(Error::NoSession) => {
                self.get_session_or_login()?;
                self.try_with_session(request)
            }
            Err(Error::SessionExpired) => {
                self.login()?;
                self.try_with_session(request)
            }
            Err(error) => Err(error),
        }
    }

    fn try_with_session(
        &self,
        request: RequestBuilder,
    ) -> Result<Response, Error> {
        let session = self.session.as_ref().ok_or(Error::NoSession)?;
        request
            .bearer_auth(&session.access_token)
            .try_send()
            .map_err(|err| {
                if err.status() == Some(reqwest::StatusCode::UNAUTHORIZED) {
                    Error::SessionExpired
                } else {
                    err.into()
                }
            })
    }
}

impl Client for KivraClient {
    fn get_config(&self) -> Result<Config, Error> {
        Ok(get!(self, "{ACCOUNTS_URL}/config.json").try_send()?.json()?)
    }

    fn start_auth(
        &self,
        config: &Config,
    ) -> Result<(CodeVerifier, AuthResponse), Error> {
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
        let response = get!(self, "{API_URL}/v2/oauth2/authorize")
            .query(&auth_request)
            .try_send()?
            .json()?;
        Ok((verifier, response))
    }

    fn check_auth(&self, poll_url: &str) -> Result<AuthStatus, Error> {
        Ok(get!(self, "{API_URL}{poll_url}").try_send()?.json()?)
    }

    fn abort_auth(&self, poll_url: &str) -> Result<(), Error> {
        delete!(self, "{API_URL}{poll_url}").try_send()?;
        Ok(())
    }

    fn get_auth_token(
        &self,
        config: &Config,
        auth_code: String,
        verifier: CodeVerifier,
    ) -> Result<AuthTokenResponse, Error> {
        let verifier_string = String::from_utf8(verifier)?;
        let token_request = AuthTokenRequest {
            client_id: config.oauth_default_client_id.clone(),
            code: auth_code,
            code_verifier: verifier_string,
            grant_type: "authorization_code".to_string(),
            redirect_uri: config.oauth_default_redirect_uri.clone(),
        };

        Ok(post!(self, "{API_URL}/v2/oauth2/token")
            .json(&token_request)
            .try_send()?
            .json()?)
    }

    fn revoke_auth_token(&mut self) -> Result<(), Error> {
        if let Some(session) = self.get_or_load_session()? {
            let body = RevokeRequest {
                token: session.access_token.clone(),
                token_type_hint: "access_token".to_string(),
            };
            post!(self, "{API_URL}/v2/oauth2/token/revoke")
                .json(&body)
                .try_send()?;
        }
        Ok(())
    }

    fn get_inbox_listing(&mut self) -> Result<InboxListing, Error> {
        let session = self.get_session_or_login()?;
        let user_id = &session.user_info.kivra_user_id;
        let request = get!(self, "{API_URL}/v3/user/{user_id}/content")
            .query(&[("listing", "all")]);
        let listing = self.auth_request(request)?.json()?;
        Ok(InboxListing::from_content_specs(listing))
    }

    fn get_item_details(
        &mut self,
        item_key: &str,
    ) -> Result<ItemDetails, Error> {
        let session = self.get_session_or_login()?;
        let user_id = &session.user_info.kivra_user_id;
        let response = self.auth_request(
            get!(self, "{API_URL}/v3/user/{user_id}/content/{item_key}")
        )?;
        Ok(response.json()?)
    }

    fn mark_as_read(&mut self, item_key: &str) -> Result<(), Error> {
        let session = self.get_session_or_login()?;
        let user_id = &session.user_info.kivra_user_id;
        self.auth_request(
            post!(self, "{API_URL}/v2/user/{user_id}/content/{item_key}/view")
                .header("content-type", "application/json")
        )?;
        Ok(())
    }

    fn download_attachment(
        &mut self,
        item_key: &str,
        attachment_key: &str,
    ) -> Result<Bytes, Error> {
        let session = self.get_session_or_login()?;
        let user_id = &session.user_info.kivra_user_id;
        let req = get!(
            self,
            "{API_URL}/v1/user/{user_id}/content/{item_key}/file/{attachment_key}/raw"
        );
        Ok(self.auth_request(req)?.bytes()?)
    }

    fn get_session(&self) -> Option<Session> {
        self.session.clone()
    }

    fn set_session(&mut self, session: Session) {
        self.session = Some(session);
    }

    fn login(&mut self) -> Result<Session, Error> {
        let to_dyn_boxed = |error: tui::Error| -> Box<dyn std::error::Error> {
            Box::new(error)
        };

        let mut terminal = tui::terminal::load().map_err(to_dyn_boxed)?;
        let mut view =
            tui::login::LoginView::make(self).map_err(to_dyn_boxed)?;

        match tui::show(&mut view, &mut terminal, None).map_err(to_dyn_boxed)? {
            Some(auth_response) => {
                let session = session::make(
                    auth_response.access_token,
                    auth_response.id_token,
                )?;
                session::save(&session)?;
                self.set_session(session.clone());
                Ok(session)
            }
            None => Err(Error::LoginAborted),
        }
    }
}
