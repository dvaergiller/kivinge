use super::model::*;
use super::session::Session;
use crate::error::Error;

mod kivra_client;
pub use kivra_client::KivraClient;

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

    fn get_inbox_listing(&self, session: &Session) -> Result<Vec<ContentSpec>, Error>;
}
