use chrono::{Local, TimeZone};
use crossterm::event::{read, Event, KeyCode};
use ratatui::{
    layout::Constraint,
    style::{Modifier, Style},
    widgets::{Row, Table, TableState},
    Frame,
};

use crate::{
    error::Error,
    kivra::model::{InboxEntry, InboxListing},
    terminal::LoadedTerminal,
};

pub fn show(terminal: &mut LoadedTerminal, inbox: InboxListing) -> Result<(), Error> {
    let mut widget_state = TableState::new().with_selected(0);

    loop {
        render(terminal, &inbox, &mut widget_state)?;
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
            _ => (),
        }
    }
}

pub fn render(
    terminal: &mut LoadedTerminal,
    inbox: &InboxListing,
    widget_state: &mut TableState,
) -> Result<(), Error> {
    let widget = inbox_widget(inbox);
    let draw = |frame: &mut Frame| {
        frame.render_stateful_widget(widget, frame.size(), widget_state);
    };
    terminal.draw(draw)?;
    Ok(())
}

fn inbox_widget(inbox: &InboxListing) -> Table<'static> {
    let rows = inbox.iter().map(inbox_row);
    let widths = [
        Constraint::Max(20),
        Constraint::Fill(1),
        Constraint::Length(16),
    ];
    Table::new(rows, widths)
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">>")
}

fn inbox_row(entry: &InboxEntry) -> Row<'static> {
    let local_datetime = Local.from_utc_datetime(&entry.item.created_at.naive_utc());
    let cells = [
        entry.item.sender_name.clone(),
        entry.item.subject.clone(),
        local_datetime.format("%Y-%m-%d %H:%M").to_string(),
    ];
    Row::new(cells)
}
