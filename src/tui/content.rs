use std::fmt::Display;

use crossterm::event::{read, Event, KeyCode};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Borders, List, ListDirection, Paragraph};
use ratatui::{symbols, Frame};

use crate::{
    error::Error,
    kivra::model::{ItemDetails, InboxItem},
    terminal::LoadedTerminal,
};

pub fn show(
    terminal: &mut LoadedTerminal,
    content_spec: &InboxItem,
    item_details: &ItemDetails,
) -> Result<(), Error> {
    loop {
        render(terminal, content_spec, item_details)?;
        match read()? {
            Event::Key(key) if key.code == KeyCode::Char('q') => {
                return Ok(());
            }
            _ => (),
        }
    }
}

fn indent(n: usize, s: impl Display) -> String {
    format!("\n{:indent$}{}", "", s, indent = n)
}

pub fn render(
    terminal: &mut LoadedTerminal,
    content_spec: &InboxItem,
    content_details: &ItemDetails,
) -> Result<(), Error> {
    let draw = |frame: &mut Frame| {
        let main_layout = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints(vec![
                Constraint::Length(4),
                Constraint::Length(5),
                Constraint::Min(5),
            ])
            .split(frame.size());

        let top_layout = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
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
        let sender_text = indent(2, &content_spec.sender_name);
        let sender_widget = Paragraph::new(sender_text).block(sender_block);
        frame.render_widget(sender_widget, top_layout[0]);

        let created_block = Block::new()
            .borders(Borders::TOP | Borders::RIGHT)
            .title("Created at:")
            .title_style(Style::new().bold());
        let created_text = indent(2, content_spec.created_at.format("%Y-%m-%d %H:%M"));
        let created_widget = Paragraph::new(created_text).block(created_block);
        frame.render_widget(created_widget, top_layout[1]);

        let subject_block = Block::new()
            .border_set(symbols::border::Set {
                top_left: symbols::line::VERTICAL_RIGHT,
                top_right: symbols::line::VERTICAL_LEFT,
                ..symbols::border::PLAIN
            })
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .title("Subject:")
            .title_style(Style::new().bold());
        let subject_text = indent(2, &content_spec.subject);
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
        let attachments: Vec<String> = content_details
            .parts
            .iter()
            .map(|d| indent(2, &d.name))
            .collect();
        let subject_widget = List::new(attachments)
            .block(attachments_block)
            .direction(ListDirection::TopToBottom);
        frame.render_widget(subject_widget, main_layout[2]);
    };
    terminal.draw(draw)?;
    Ok(())
}
