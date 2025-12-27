use chrono::{Local, TimeZone};

use crate::{error::Error, model::content::ItemDetails};

pub fn format(details: ItemDetails) -> Result<String, Error> {
    let local_datetime = Local
        .from_utc_datetime(&details.created_at.naive_utc())
        .format("%Y-%m-%d %H:%M")
        .to_string();

    let mut output = vec![
        format!("Sender:   {}\n", details.sender_name),
        format!("Subject:  {}\n", details.subject),
        format!("Created:  {}\n\n", local_datetime),
        format!("Attachments:\n"),
    ];

    for i in 0..(details.parts.len()) {
        output.push(format!("  {}: {}\n", i, details.attachment_name(i)?));
    }

    Ok(output.concat())
}
