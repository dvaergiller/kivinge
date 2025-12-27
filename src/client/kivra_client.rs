use bytes::Bytes;
use reqwest::blocking::{RequestBuilder, Response};
use tracing::instrument;

use super::session::{self, Session};
use super::{Client, Error};
use crate::model::{auth::*, content::*, Config};
use crate::tui;

const API_URL: &str = "https://app.api.kivra.com";
const ACCOUNTS_URL: &str = "https://accounts.kivra.com";

#[derive(Default)]
pub struct KivraClient {
    client: reqwest::blocking::Client,
    session: Option<Session>,
}

impl KivraClient {
    pub fn with_session(
        &mut self,
        request: RequestBuilder,
    ) -> Result<Response, Error> {
        match self
            .try_with_session(request.try_clone().ok_or(Error::CloneError)?)
        {
            Ok(response) => Ok(response),
            Err(Error::NoSession) => {
                self.get_session_or_login()?;
                self.try_with_session(request)
            }
            Err(Error::SessionExpired) => {
                self.get_session_or_login()?;
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
        Ok(request
            .bearer_auth(&session.access_token)
            .send()?
            .error_for_status()?)
    }
}

impl Client for KivraClient {
    #[instrument(skip(self))]
    fn get_config(&self) -> Result<Config, Error> {
        Ok(self
            .client
            .get(format!("{ACCOUNTS_URL}/config.json"))
            .send()?
            .error_for_status()?
            .json()?)
    }

    #[instrument(skip(self))]
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

        let resp = self
            .client
            .post(format!("{API_URL}/v2/oauth2/authorize"))
            .json(&auth_request)
            .send()?
            .error_for_status()?
            .json()?;
        Ok((verifier, resp))
    }

    #[instrument(skip(self))]
    fn check_auth(&self, poll_url: &str) -> Result<AuthStatus, Error> {
        Ok(self
            .client
            .get(format!("{API_URL}{poll_url}"))
            .send()?
            .error_for_status()?
            .json()?)
    }

    #[instrument(skip(self))]
    fn abort_auth(&self, poll_url: &str) -> Result<(), Error> {
        self.client
            .delete(format!("{API_URL}{poll_url}"))
            .send()?
            .error_for_status()?;
        Ok(())
    }

    #[instrument(skip(self))]
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

        let resp = self
            .client
            .post(format!("{API_URL}/v2/oauth2/token"))
            .json(&token_request)
            .send()?
            .error_for_status()?;
        Ok(resp.json()?)
    }

    #[instrument(skip(self))]
    fn revoke_auth_token(&mut self) -> Result<(), Error> {
        if let Some(session) = self.get_or_load_session()? {
            self.client
                .post(format!("{API_URL}/v2/oauth2/token/revoke"))
                .json(&RevokeRequest {
                    token: session.access_token.clone(),
                    token_type_hint: "access_token".to_string(),
                })
                .send()?
                .error_for_status()?;
        }
        Ok(())
    }

    #[instrument(skip(self))]
    fn get_inbox_listing(&mut self) -> Result<InboxListing, Error> {
        let session = self.get_session_or_login()?;
        let user_id = &session.user_info.kivra_user_id;
        let listing = self
            .with_session(
                self.client
                    .get(format!("{API_URL}/v3/user/{user_id}/content"))
                    .query(&[("listing", "all")]),
            )?
            .json()?;
        Ok(InboxListing::from_content_specs(listing))
    }

    #[instrument(skip(self))]
    fn get_item_details(
        &mut self,
        item_key: &str,
    ) -> Result<ItemDetails, Error> {
        let session = self.get_session_or_login()?;
        let user_id = &session.user_info.kivra_user_id;
        let url = format!("{API_URL}/v3/user/{user_id}/content/{item_key}");
        let details = self.with_session(self.client.get(url))?.json()?;
        Ok(details)
    }

    #[instrument(skip(self))]
    fn mark_as_read(&mut self, item_key: &str) -> Result<(), Error> {
        let session = self.get_session_or_login()?;
        let user_id = &session.user_info.kivra_user_id;
        self.with_session(
            self.client
                .post(format!(
                    "{API_URL}/v2/user/{user_id}/content/{item_key}/view"
                ))
                .header("content-type", "application/json"),
        )?;
        Ok(())
    }

    #[instrument(skip(self))]
    fn download_attachment(
        &mut self,
        item_key: &str,
        attachment_key: &str,
    ) -> Result<Bytes, Error> {
        let session = self.get_session_or_login()?;
        let user_id = &session.user_info.kivra_user_id;
        let url = format!(
            "{API_URL}/v1/user/{user_id}/content/{item_key}/file/{attachment_key}/raw"
        );
        let attachment = self.with_session(self.client.get(url))?.bytes()?;
        Ok(attachment)
    }

    #[instrument(skip(self))]
    fn get_session(&self) -> Option<Session> {
        self.session.clone()
    }

    #[instrument(skip(self))]
    fn get_or_load_session(&mut self) -> Result<Option<Session>, Error> {
        if let Some(session) = &self.session {
            return Ok(Some(session.clone()));
        }

        let opt_session = session::try_load()?;
        self.session = opt_session.clone();
        Ok(opt_session)
    }

    #[instrument(skip(self))]
    fn get_session_or_login(&mut self) -> Result<Session, Error> {
        if let Some(session) = self.get_or_load_session()? {
            return Ok(session);
        }

        let to_dyn_boxed = |error: tui::Error| -> Box<dyn std::error::Error> {
            Box::new(error)
        };

        let mut terminal = tui::terminal::load().map_err(to_dyn_boxed)?;
        let mut login_view =
            tui::login::LoginView::make(self).map_err(to_dyn_boxed)?;
        match tui::show(&mut login_view, &mut terminal, None)
            .map_err(to_dyn_boxed)?
        {
            Some(auth_response) => {
                let session = session::make(
                    auth_response.access_token,
                    auth_response.id_token,
                )?;
                session::save(&session)?;
                self.session = Some(session.clone());
                Ok(session)
            }
            None => Err(Error::LoginAborted),
        }
    }
}
