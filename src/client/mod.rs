use bytes::Bytes;
use thiserror::Error;

use super::model::{auth::*, content::*, Config};

mod kivra_client;
mod mock_client;
pub mod session;
// pub mod session_manager;

pub use kivra_client::KivraClient;
pub use mock_client::MockClient;
use session::Session;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("utf8 decode failed: {0}")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("no session")]
    NoSession,

    #[error("session expired")]
    SessionExpired,

    #[error("login failed")]
    LoginFailed,

    #[error("login aborted")]
    LoginAborted,

    #[error("session error: {0}")]
    SessionError(#[from] session::Error),

    #[error("tui error: {0}")]
    TuiError(#[from] Box<dyn std::error::Error>),

    #[error("request body is not cloneable")]
    CloneError,
}

pub trait Client {
    fn get_config(&self) -> Result<Config, Error>;

    fn start_auth(
        &self,
        config: &Config,
    ) -> Result<(CodeVerifier, AuthResponse), Error>;

    fn check_auth(&self, poll_url: &str) -> Result<AuthStatus, Error>;

    fn abort_auth(&self, poll_url: &str) -> Result<(), Error>;

    fn get_auth_token(
        &self,
        config: &Config,
        auth_code: AuthCode,
        verifier: CodeVerifier,
    ) -> Result<AuthTokenResponse, Error>;

    fn revoke_auth_token(&mut self) -> Result<(), Error>;

    fn get_inbox_listing(&mut self) -> Result<InboxListing, Error>;

    fn get_item_details(
        &mut self,
        item_key: &str,
    ) -> Result<ItemDetails, Error>;

    fn mark_as_read(&mut self, item_key: &str) -> Result<(), Error>;

    fn download_attachment(
        &mut self,
        item_key: &str,
        attachment_key: &str,
    ) -> Result<Bytes, Error>;
}

impl Client for Box<dyn Client> {
    fn get_config(&self) -> Result<Config, Error> {
        (**self).get_config()
    }

    fn start_auth(
        &self,
        config: &Config,
    ) -> Result<(CodeVerifier, AuthResponse), Error> {
        (**self).start_auth(config)
    }

    fn check_auth(&self, poll_url: &str) -> Result<AuthStatus, Error> {
        (**self).check_auth(poll_url)
    }

    fn abort_auth(&self, poll_url: &str) -> Result<(), Error> {
        (**self).abort_auth(poll_url)
    }

    fn get_auth_token(
        &self,
        config: &Config,
        auth_code: AuthCode,
        verifier: CodeVerifier,
    ) -> Result<AuthTokenResponse, Error> {
        (**self).get_auth_token(config, auth_code, verifier)
    }

    fn revoke_auth_token(&mut self) -> Result<(), Error> {
        (**self).revoke_auth_token()
    }

    fn get_inbox_listing(&mut self) -> Result<InboxListing, Error> {
        (**self).get_inbox_listing()
    }

    fn get_item_details(
        &mut self,
        item_key: &str
    ) -> Result<ItemDetails, Error> {
        (**self).get_item_details(item_key)
    }

    fn mark_as_read(&mut self, item_key: &str) -> Result<(), Error> {
        (**self).mark_as_read(item_key)
    }

    fn download_attachment(
        &mut self,
        item_key: &str,
        attachment_key: &str,
    ) -> Result<Bytes, Error> {
        (**self).download_attachment(item_key, attachment_key)
    }
}
