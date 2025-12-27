use std::cell::RefCell;
use std::include_str;

use super::Client;
use crate::error::Error;
use crate::kivra::model::*;
use crate::kivra::session::Session;

pub struct MockClient {
    check_auth_calls: RefCell<u32>
}

impl MockClient {
    pub fn new() -> MockClient {
        MockClient { check_auth_calls: RefCell::new(0) }
    }
}

impl Client for MockClient {
    fn get_config(&self) -> Result<Config, Error> {
        let config = serde_json::from_str(include_str!("test_data/config.json"))?;
        Ok(config)
    }

    fn start_auth(&self, _config: &Config) -> Result<(CodeVerifier, AuthResponse), Error> {
        let verifier = pkce::code_verifier(48);
        let response = serde_json::from_str(include_str!("test_data/auth_response.json"))?;
        Ok((verifier, response))
    }

    fn check_auth(&self, _poll_url: &str) -> Result<AuthStatus, Error> {
        let mut updates = self.check_auth_calls.borrow_mut();
        (*updates) += 1;
        let status = serde_json::from_str(include_str!("test_data/auth_status.json"))?;

        if (*updates) > 3 {
            Ok(AuthStatus {
                ssn: Some("195208152712".to_string()),
                ..status
            })
        } else {
            Ok(status)
        }
    }

    fn abort_auth(&self, _poll_url: &str) -> Result<(), Error> {
        Ok(())
    }

    fn get_auth_token(
        &self,
        _config: &Config,
        _auth_code: String,
        _verifier: CodeVerifier,
    ) -> Result<AuthTokenResponse, Error> {
        let response = serde_json::from_str(include_str!("test_data/auth_token_response.json"))?;
        Ok(response)
    }

    fn revoke_auth_token(&self, _session: &Session) -> Result<(), Error> {
        Ok(())
    }

    fn get_inbox_listing(&self, _session: &Session) -> Result<InboxListing, Error> {
        let listing = serde_json::from_str(include_str!("test_data/inbox.json"))?;
        Ok(InboxListing::from_content_specs(listing))
    }

    fn get_item_details(
        &self,
        _session: &Session,
        _item_key: String
    ) -> Result<ItemDetails, Error> {
        let details = serde_json::from_str(include_str!("test_data/details.json"))?;
        Ok(details)
    }
}
