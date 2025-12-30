use chrono::{Local, TimeZone};
use crossterm::terminal;
use tabled::builder::Builder;
use tabled::settings::{object::Columns, width::Width, Modify, Style};

use crate::model::content::InboxListing;

pub fn format(inbox: InboxListing) -> String {
    let mut builder = Builder::default();
    builder.push_record(["Id", "Sender", "Subject", "Created At"]);
    for entry in inbox {
        let local_datetime = Local
            .from_utc_datetime(&entry.item.created_at.naive_utc())
            .format("%Y-%m-%d %H:%M")
            .to_string();
        builder.push_record([
            &entry.id.to_string(),
            &entry.item.sender_name,
            &entry.item.subject,
            &local_datetime,
        ]);
    }

    let mut table = builder.build();
    table.with(Style::modern());

    // Table border overhead for modern style with 4 columns:
    // - 5 separators (one before each column + one at end)
    // - 8 padding spaces (1 space on each side of 4 columns)
    const COLUMNS: usize = 4;
    const BORDER_OVERHEAD: usize = (COLUMNS + 1) + (COLUMNS * 2); // 13

    const ID_WIDTH: usize = 4;
    const CREATED_AT_WIDTH: usize = 18;
    const FIXED_WIDTH: usize = ID_WIDTH + CREATED_AT_WIDTH + BORDER_OVERHEAD;
    const MIN_FLEX_WIDTH: usize = 20;

    let term_width = terminal::size().map(|(w, _)| w as usize).unwrap_or(150);

    // Split remaining space equally between Sender and Subject
    let flex_width =
        term_width.saturating_sub(FIXED_WIDTH).max(MIN_FLEX_WIDTH * 2) / 2;

    table
        .with(
            Modify::new(Columns::single(1))
                .with(Width::truncate(flex_width).suffix("…")),
        )
        .with(
            Modify::new(Columns::single(2))
                .with(Width::truncate(flex_width).suffix("…")),
        );

    table.to_string()
}
