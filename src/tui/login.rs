use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude,
    style::Color,
    widgets::Paragraph,
};
use std::time::Duration;

const QR_BRANDING: &str =
    concat!(
        " ▄▄  ▄▄ \n",
        " ██▄█▀  \n",
        " ██▀█▄  \n",
        " ▀▀  ▀▀ \n",
    );

use super::{keymap::KeyEvent, qr, Command, Error, Event, TuiView};
use crate::{
    client::{self, Client},
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

    fn check_auth(
        &mut self,
    ) -> Result<Option<AuthTokenResponse>, client::Error> {
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
        let qr_height = qr.lines().count() as u16;
        let margin_top = (rect.height - qr_height) / 2;

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(margin_top - 2),
                Constraint::Length(2),
                Constraint::Length(qr_height),
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .split(rect);

        frame.render_widget(
            Paragraph::new("Authenticate with BankID")
                .alignment(Alignment::Center),
            layout[1],
        );
        let qr_style = Style::default().fg(Color::White).bg(Color::Black);
        let qr_rect = Rect {
            x: layout[2].x + (layout[2].width.saturating_sub(qr_width)) / 2,
            y: layout[2].y,
            width: qr_width,
            height: qr_height,
        };
        frame.render_widget(Paragraph::new(qr).style(qr_style), qr_rect);

        let branding_height = QR_BRANDING.lines().count() as u16;
        let branding_width =
            QR_BRANDING.lines().next().unwrap_or_default().chars().count()
                as u16;
        let branding_rect = Rect {
            x: layout[2].x + layout[2].width / 2 - branding_width / 2,
            y: layout[2].y + layout[2].height / 2 - branding_height / 2,
            width: branding_width,
            height: branding_height,
        };

        frame.render_widget(
            Paragraph::new(QR_BRANDING)
                .alignment(Alignment::Center)
                .style(Color::Green),
            branding_rect,
        );

        frame.render_widget(
            Paragraph::new("Press 'q' to abort login")
                .alignment(Alignment::Center),
            layout[3],
        );
    }
}
