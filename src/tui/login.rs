use ratatui::{layout::Rect, prelude, widgets};
use std::time::Duration;

use super::{keymap::KeyEvent, qr, Command, Event, Error, TuiView};
use crate::{
    client::Client,
    model::{
        auth::{AuthCode, AuthTokenResponse},
        Config,
    },
};

pub struct LoginView<'a, C: Client> {
    client: &'a C,
    config: Config,
    auth_code: AuthCode,
    code_verifier: Vec<u8>,
    qr_code: String,
    next_poll_url: String,
    retry_after: u32,
}

impl<'a, C: Client> LoginView<'a, C> {
    pub fn make(client: &'a C) -> Result<LoginView<'a, C>, Error> {
        let config = client.get_config()?;
        let (verifier, auth_resp) = client.start_auth(&config)?;

        Ok(LoginView {
            client,
            config,
            auth_code: auth_resp.code,
            code_verifier: verifier,
            qr_code: auth_resp.qr_code,
            next_poll_url: auth_resp.next_poll_url,
            retry_after: 1,
        })
    }

    fn check_auth(&mut self) -> Result<Option<AuthTokenResponse>, Error> {
        let check = self.client.check_auth(&self.next_poll_url)?;
        match check.ssn {
            None => {
                self.qr_code = check.qr_code;
                self.next_poll_url =
                    check.next_poll_url.unwrap_or(self.next_poll_url.clone());
                self.retry_after =
                    check.retry_after.unwrap_or(self.retry_after);
                Ok(None)
            }
            Some(_) => {
                let auth_token = self.client.get_auth_token(
                    &self.config,
                    self.auth_code.clone(),
                    self.code_verifier.clone(),
                )?;
                Ok(Some(auth_token))
            }
        }
    }
}

impl<'a, C: Client> TuiView for LoginView<'a, C> {
    type ReturnType = Option<AuthTokenResponse>;
    fn update(
        &mut self,
        event: Event,
    ) -> Result<Command<Self::ReturnType>, Error> {
        match event {
            Event::Init => {
                let duration = Duration::from_secs(self.retry_after.into());
                Ok(Command::AwaitTimeout(duration))
            }

            Event::Key(KeyEvent::Quit) => {
                self.client.abort_auth(&self.next_poll_url)?;
                Ok(Command::Return(None))
            }

            Event::Timeout => match self.check_auth()? {
                None => {
                    let timeout = Duration::from_secs(self.retry_after.into());
                    Ok(Command::AwaitTimeout(timeout))
                }
                Some(auth_token) => Ok(Command::Return(Some(auth_token))),
            },

            _ => {
                let timeout = Duration::from_secs(self.retry_after.into());
                Ok(Command::AwaitTimeout(timeout))
            }
        }
    }

    fn render(&mut self, frame: &mut prelude::Frame, rect: Rect) {
        let qr = qr::encode(&self.qr_code).unwrap();

        let title = "Authenticate with BankID";
        let block = widgets::Block::default()
            .title(title)
            .title_alignment(prelude::Alignment::Center)
            .title_bottom("Press 'q' to abort login")
            .borders(widgets::Borders::NONE);
        let paragraph = widgets::Paragraph::new(qr).block(block).centered();

        frame.render_widget(paragraph, rect);
    }
}
