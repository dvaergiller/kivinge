use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::widgets::{
    Block, Borders, List, ListDirection, ListState, Paragraph,
};
use ratatui::{symbols, Frame};
use std::fmt::Display;

use crate::client::session::Session;
use crate::client::Client;
use crate::model::content::Status;
use crate::{
    error::Error,
    model::content::{InboxItem, ItemDetails},
};

use super::keymap::KeyEvent;
use super::{Command, Event, TuiView};

pub struct ItemView {
    item: InboxItem,
    details: ItemDetails,
    list_state: ListState,
}

pub enum ItemViewResult {
    Open(u32),
    MarkRead,
    Close,
}

impl ItemView {
    pub fn make(
        client: &impl Client,
        session: &Session,
        item: InboxItem,
    ) -> Result<ItemView, Error> {
        let details = client.get_item_details(session, &item.key)?;
        let list_state = match details.parts.len() {
            0 => ListState::default(),
            _ => ListState::default().with_selected(Some(0)),
        };
        Ok(ItemView { item, details, list_state })
    }
}

impl TuiView for ItemView {
    type ReturnType = ItemViewResult;

    fn update(
        &mut self,
        event: Event,
    ) -> Result<Command<Self::ReturnType>, Error> {
        match event {
            Event::Key(KeyEvent::Up) => {
                let select = match self.list_state.selected().unwrap_or(0) {
                    0 => 0,
                    n => n - 1,
                };
                self.list_state.select(Some(select));
                Ok(Command::AwaitKey)
            }

            Event::Key(KeyEvent::Down) => {
                let select = match self.list_state.selected().unwrap_or(0) {
                    n if n >= self.details.parts.len() - 1 => n,
                    n => n + 1,
                };
                self.list_state.select(Some(select));
                Ok(Command::AwaitKey)
            }

            Event::Key(KeyEvent::Select) => {
                let selected =
                    self.list_state.selected().ok_or(Error::AppError(
                        "No attachment selected \
                         (this should not be possible and is a bug)",
                    ))?;
                Ok(Command::Return(ItemViewResult::Open(selected as u32)))
            }

            Event::Key(KeyEvent::Quit) | Event::Key(KeyEvent::Back) => {
                Ok(Command::Return(ItemViewResult::Close))
            }

            Event::Key(KeyEvent::Key(KeyCode::Char('r'))) => {
                self.item.status = Status::Read;
                Ok(Command::Return(ItemViewResult::MarkRead))
            }

            _ => Ok(Command::AwaitKey),
        }
    }

    fn render(&mut self, frame: &mut Frame, rect: Rect) {
        render_widget(
            &self.item,
            &self.details,
            &mut self.list_state,
            frame,
            rect,
        );
    }
}

fn indent(n: usize, s: impl Display) -> String {
    format!("\n{:indent$}{}", "", s, indent = n)
}

fn render_widget(
    item: &InboxItem,
    details: &ItemDetails,
    list_state: &mut ListState,
    frame: &mut Frame,
    rect: Rect,
) {
    let main_layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(vec![
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Min(5),
        ])
        .split(rect);

    let top_layout = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints(vec![
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .split(main_layout[0]);

    let sender_block = Block::new()
        .border_set(symbols::border::Set {
            top_right: symbols::line::HORIZONTAL_DOWN,
            bottom_right: symbols::line::HORIZONTAL_UP,
            ..symbols::border::PLAIN
        })
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .title("Sender:")
        .title_style(Style::new().bold());
    let sender_text = indent(2, &item.sender_name);
    let sender_widget = Paragraph::new(sender_text).block(sender_block);
    frame.render_widget(sender_widget, top_layout[0]);

    let status_block = Block::new()
        .borders(Borders::TOP | Borders::RIGHT)
        .title("Status:")
        .title_style(Style::new().bold());
    let status_text =
        if item.status == Status::Read { "Read" } else { "Unread" };
    let status_widget =
        Paragraph::new(indent(2, status_text)).block(status_block);
    frame.render_widget(status_widget, top_layout[1]);

    let created_block = Block::new()
        .borders(Borders::TOP | Borders::RIGHT)
        .title("Created at:")
        .title_style(Style::new().bold());
    let created_text = indent(2, item.created_at.format("%Y-%m-%d %H:%M"));
    let created_widget = Paragraph::new(created_text).block(created_block);
    frame.render_widget(created_widget, top_layout[2]);

    let subject_block = Block::new()
        .border_set(symbols::border::Set {
            top_left: symbols::line::VERTICAL_RIGHT,
            top_right: symbols::line::VERTICAL_LEFT,
            ..symbols::border::PLAIN
        })
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .title("Subject:")
        .title_style(Style::new().bold());
    let subject_text = indent(2, &item.subject);
    let subject_widget = Paragraph::new(subject_text).block(subject_block);
    frame.render_widget(subject_widget, main_layout[1]);

    let attachments_block = Block::new()
        .border_set(symbols::border::Set {
            top_left: symbols::line::VERTICAL_RIGHT,
            top_right: symbols::line::VERTICAL_LEFT,
            ..symbols::border::PLAIN
        })
        .borders(Borders::ALL)
        .title("Attachments:")
        .title_style(Style::new().bold());
    let attachments: Vec<String> = (0..(details.parts.len()))
        .map(|i| details.attachment_name(i).unwrap())
        .collect();
    let attachments_widget = List::new(attachments)
        .block(attachments_block)
        .direction(ListDirection::TopToBottom)
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(
        attachments_widget,
        main_layout[2],
        list_state,
    );
}
