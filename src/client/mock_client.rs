use bytes::Bytes;
use std::cell::RefCell;
use std::include_str;

use super::{Client, Error, Session};
use crate::model::{auth::*, content::*, Config};

#[derive(Default)]
pub struct MockClient {
    check_auth_calls: RefCell<u32>,
}

impl Client for MockClient {
    fn get_config(&self) -> Result<Config, Error> {
        let input = include_str!("test_data/config.json");
        let config = serde_json::from_str(input)?;
        Ok(config)
    }

    fn start_auth(
        &self,
        _config: &Config,
    ) -> Result<(CodeVerifier, AuthResponse), Error> {
        let verifier = pkce::code_verifier(48);
        let input = include_str!("test_data/auth_response.json");
        let response = serde_json::from_str(input)?;
        Ok((verifier, response))
    }

    fn check_auth(&self, _poll_url: &str) -> Result<AuthStatus, Error> {
        let mut updates = self.check_auth_calls.borrow_mut();
        (*updates) += 1;
        let input = include_str!("test_data/auth_status.json");
        let status = serde_json::from_str(input)?;

        if (*updates) > 3 {
            Ok(AuthStatus { ssn: Some("195208152712".to_string()), ..status })
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
        let input = include_str!("test_data/auth_token_response.json");
        let response = serde_json::from_str(input)?;
        Ok(response)
    }

    fn revoke_auth_token(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn get_inbox_listing(&mut self) -> Result<InboxListing, Error> {
        let input = include_str!("test_data/inbox.json");
        let listing = serde_json::from_str(input)?;
        Ok(InboxListing::from_content_specs(listing))
    }

    fn get_item_details(
        &mut self,
        _item_key: &str,
    ) -> Result<ItemDetails, Error> {
        let input = include_str!("test_data/details.json");
        let details = serde_json::from_str(input)?;
        Ok(details)
    }

    fn mark_as_read(&mut self, _item_key: &str) -> Result<(), Error> {
        Ok(())
    }

    fn download_attachment(
        &mut self,
        _item_key: &str,
        _attachment_key: &str,
    ) -> Result<Bytes, Error> {
        Ok(Bytes::from_static(b"tjena"))
    }

    fn get_session(&self) -> Option<Session> {
        None
    }

    fn get_or_load_session(&mut self) -> Result<Option<Session>, Error> {
        Ok(None)
    }

    fn get_session_or_login(&mut self) -> Result<Session, Error> {
        Err(Error::NoSession)
    }
}
