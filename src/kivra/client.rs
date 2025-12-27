use super::model::*;
use super::session::Session;
use crate::error::Error;

mod kivra_client;
mod mock_client;

pub use kivra_client::KivraClient;
pub use mock_client::MockClient;

pub trait Client {
    fn get_config(&self) -> Result<Config, Error>;

    fn start_auth(&self, config: &Config) -> Result<(CodeVerifier, AuthResponse), Error>;

    fn check_auth(&self, poll_url: &str) -> Result<AuthStatus, Error>;

    fn abort_auth(&self, poll_url: &str) -> Result<(), Error>;

    fn get_auth_token(
        &self,
        config: &Config,
        auth_code: AuthCode,
        verifier: CodeVerifier,
    ) -> Result<AuthTokenResponse, Error>;

    fn revoke_auth_token(&self, session: &Session) -> Result<(), Error>;

    fn get_inbox_listing(&self, session: &Session) -> Result<InboxListing, Error>;

    fn get_item_details(&self, session: &Session, item_key: &String) -> Result<ItemDetails, Error>;

    fn download_attachment(
        &self,
        session: &Session,
        item_key: &String,
        attachment_key: &String,
    ) -> Result<Vec<u8>, Error>;
}

impl Client for Box<dyn Client> {
    fn get_config(&self) -> Result<Config, Error> {
        (**self).get_config()
    }

    fn start_auth(&self, config: &Config) -> Result<(CodeVerifier, AuthResponse), Error> {
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

    fn revoke_auth_token(&self, session: &Session) -> Result<(), Error> {
        (**self).revoke_auth_token(session)
    }

    fn get_inbox_listing(&self, session: &Session) -> Result<InboxListing, Error> {
        (**self).get_inbox_listing(session)
    }

    fn get_item_details(&self, session: &Session, item_key: &String) -> Result<ItemDetails, Error> {
        (**self).get_item_details(session, item_key)
    }

    fn download_attachment(
        &self,
        session: &Session,
        item_key: &String,
        attachment_key: &String,
    ) -> Result<Vec<u8>, Error> {
        (**self).download_attachment(session, item_key, attachment_key)
    }
}
