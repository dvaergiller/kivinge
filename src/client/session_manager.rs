use reqwest::blocking::RequestBuilder;
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::tui;
use super::{Client, session::{self, Session}};

#[derive(Debug, Error)]
pub enum Error {
    #[error("no session")]
    NoSession,

    #[error("session expired")]
    SessionExpired,

    #[error("login failed")]
    LoginFailed,

    #[error("login aborted")]
    LoginAborted,

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("session error: {0}")]
    SessionError(#[from] session::Error),

    #[error("tui error: {0}")]
    TuiError(#[from] Box<dyn std::error::Error>),

    #[error("request body is not cloneable")]
    CloneError,
}

#[derive(Default)]
pub struct SessionManager {
    session: Option<Session>,
}

impl SessionManager {
    pub fn new() -> SessionManager { SessionManager { session: None } }

    pub fn request<Resp: DeserializeOwned>(
        &mut self,
        client: &impl Client,
        request: RequestBuilder
    ) -> Result<Resp, Error> {
        match self.try_request(request.try_clone().ok_or(Error::CloneError)?) {
            Ok(response) => {
                Ok(response)
            }
            Err(Error::NoSession) => {
                self.get_session_or_login(client)?;
                self.try_request(request)
            }
            Err(error) => {
                Err(error)
            }
        }
    }

    fn try_request<Resp: DeserializeOwned>(
        &self,
        request: RequestBuilder,
    ) -> Result<Resp, Error> {
        let session = self.session.as_ref().ok_or(Error::NoSession)?;
        Ok(request
           .bearer_auth(&session.access_token)
           .send()?
           .error_for_status()?
           .json()?)
    }

    pub fn get_or_load_session(&mut self) -> Result<Option<Session>, Error> {
        if let Some(session) = &self.session {
            return Ok(Some(session.clone()));
        }

        let opt_session = session::try_load()?;
        self.session = opt_session.clone();
        Ok(opt_session)
    }

    pub fn get_session_or_login(
        &mut self,
        client: &mut impl Client,
    ) -> Result<Session, Error> {
        if let Some(session) = self.get_or_load_session()? {
            return Ok(session);
        }

        let to_dyn_boxed = |error: tui::Error| -> Box<dyn std::error::Error> {
            Box::new(error)
        };

        let mut terminal = tui::terminal::load().map_err(to_dyn_boxed)?;
        let mut login_view = tui::login::LoginView::make(client).map_err(to_dyn_boxed)?;
        match tui::show(&mut login_view, &mut terminal, None).map_err(to_dyn_boxed)? {
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
