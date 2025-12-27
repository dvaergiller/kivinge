use chrono::{Local, TimeZone};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Cell, Row, Table, TableState},
    Frame,
};

use super::{keymap::KeyEvent, Command, Event, Error, TuiView};
use crate::{
    client::Client,
    model::content::{InboxEntry, InboxListing, Status},
};

pub struct InboxView {
    inbox: InboxListing,
    table_state: TableState,
}

impl InboxView {
    pub fn make(client: &mut impl Client) -> Result<InboxView, Error> {
        let inbox = client.get_inbox_listing()?;
        let table_state = TableState::new().with_selected(Some(0));
        Ok(InboxView { inbox, table_state })
    }
}

impl TuiView for InboxView {
    type ReturnType = Option<InboxEntry>;

    fn update(
        &mut self,
        event: Event,
    ) -> Result<Command<Self::ReturnType>, Error> {
        match event {
            Event::Key(KeyEvent::Quit) => Ok(Command::Return(None)),

            Event::Key(KeyEvent::Up) => {
                let select = match self.table_state.selected().unwrap_or(0) {
                    0 => 0,
                    n => n - 1,
                };
                self.table_state.select(Some(select));
                Ok(Command::AwaitKey)
            }

            Event::Key(KeyEvent::Down) => {
                let select = match self.table_state.selected().unwrap_or(0) {
                    n if n >= self.inbox.len() - 1 => n,
                    n => n + 1,
                };
                self.table_state.select(Some(select));
                Ok(Command::AwaitKey)
            }

            Event::Key(KeyEvent::Select) => match self.table_state.selected() {
                None => Ok(Command::AwaitKey),
                Some(selected) => {
                    let index = self.inbox.len() - 1 - selected;
                    let entry = self
                        .inbox
                        .get(index)
                        .ok_or(Error::AppError("Selected item out of bounds"))?
                        .clone();
                    Ok(Command::Return(Some(entry)))
                }
            },

            _ => Ok(Command::AwaitKey),
        }
    }

    fn render(&mut self, frame: &mut Frame, rect: Rect) {
        let widget = inbox_widget(&self.inbox);
        frame.render_stateful_widget(widget, rect, &mut self.table_state);
    }
}

fn inbox_widget(inbox: &InboxListing) -> Table<'static> {
    let rows = inbox.iter().rev().map(inbox_row);
    let max_id_len =
        inbox.iter().map(|i| i.id.to_string().len()).max().unwrap_or_default();
    let widths = [
        Constraint::Max(3),
        Constraint::Length(max_id_len as u16),
        Constraint::Max(20),
        Constraint::Fill(1),
        Constraint::Length(16),
    ];

    Table::new(rows, widths)
        .column_spacing(1)
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .block(Block::bordered().fg(Color::Green))
}

fn inbox_row(entry: &InboxEntry) -> Row<'static> {
    let local_datetime =
        Local.from_utc_datetime(&entry.item.created_at.naive_utc());
    let unread_marker =
        if entry.item.status == Status::Unread { "NEW" } else { "   " };
    let cells = [
        Cell::new(unread_marker).bold(),
        Cell::new(entry.id.to_string()),
        Cell::new(entry.item.sender_name.clone()),
        Cell::new(entry.item.subject.clone()),
        Cell::new(local_datetime.format("%Y-%m-%d %H:%M").to_string()),
    ];
    Row::new(cells)
}
