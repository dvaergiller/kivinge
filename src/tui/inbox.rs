use chrono::{Local, TimeZone};
use crossterm::event::{read, Event, KeyCode};
use ratatui::{
    layout::Constraint,
    style::{Modifier, Style},
    widgets::{Block, BorderType, Row, Table, TableState},
    Frame,
};

use crate::{
    error::Error,
    kivra::{client::Client, model::{InboxEntry, InboxListing}, session::Session},
    terminal::LoadedTerminal, tui::content,
};

pub fn show(
    client: &impl Client,
    session: &Session,
    terminal: &mut LoadedTerminal,
    inbox: InboxListing,
) -> Result<(), Error> {
    let mut widget_state = TableState::new().with_selected(0);
    let mut expanded = false;
    loop {
        render(terminal, &inbox, &mut widget_state, expanded)?;
        match read()? {
            Event::Key(key) if key.code == KeyCode::Char('q') => {
                return Ok(());
            }

            Event::Key(key) if key.code == KeyCode::Up => {
                let select = match widget_state.selected().unwrap_or(0) {
                    0 => 0,
                    n => n - 1,
                };
                widget_state.select(Some(select));
            }

            Event::Key(key) if key.code == KeyCode::Down => {
                let select = match widget_state.selected().unwrap_or(0) {
                    n if n >= inbox.len() => n,
                    n => n + 1,
                };
                widget_state.select(Some(select));
            }

            Event::Key(key) if key.code == KeyCode::Enter => {
                match widget_state.selected() {
                    None => (),
                    Some(selected) => {
                        let entry = inbox.get(selected)
                            .ok_or(Error::AppError("Selected item out of bounds".to_string()))?;
                        let details = client.get_item_details(session, &entry.item.key)?;
                        content::show(terminal, &entry.item, &details)?;
                    }
                }
            }

            Event::Key(key) if key.code == KeyCode::Tab => {
                expanded = !expanded;
            }
            _ => (),
        }
    }
}

pub fn render(
    terminal: &mut LoadedTerminal,
    inbox: &InboxListing,
    widget_state: &mut TableState,
    expanded: bool,
) -> Result<(), Error> {
    let widget = inbox_widget(inbox, expanded);
    let draw = |frame: &mut Frame| {
        frame.render_stateful_widget(widget, frame.size(), widget_state);
    };
    terminal.draw(draw)?;
    Ok(())
}

fn inbox_widget(inbox: &InboxListing, expanded: bool) -> Table<'static> {
    let rows = inbox.iter().map(inbox_row);
    let max_id_len = inbox
        .iter()
        .map(|i| i.id.to_string().len())
        .max()
        .unwrap_or_default();
    let widths = [
        Constraint::Length(max_id_len as u16),
        Constraint::Max(20),
        Constraint::Fill(1),
        Constraint::Length(16),
    ];

    let highlight_symbol = if expanded { "v " } else { "> " };

    Table::new(rows, widths)
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(highlight_symbol)
        .block(Block::bordered().border_type(BorderType::Rounded))
}

fn inbox_row(entry: &InboxEntry) -> Row<'static> {
    let local_datetime = Local.from_utc_datetime(&entry.item.created_at.naive_utc());
    let cells = [
        entry.id.to_string(),
        entry.item.sender_name.clone(),
        entry.item.subject.clone(),
        local_datetime.format("%Y-%m-%d %H:%M").to_string(),
    ];
    Row::new(cells)
}
