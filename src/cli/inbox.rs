use chrono::{Local, TimeZone};
use tabled::builder::Builder;
use tabled::settings::Style;

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
    builder.build().with(Style::modern()).to_string()
}
