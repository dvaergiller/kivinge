use chrono::{Local, TimeZone};

use crate::{error::Error, kivra::model::ItemDetails};

pub fn print(details: ItemDetails) -> Result<(), Error> {
    let local_datetime = Local
        .from_utc_datetime(&details.created_at.naive_utc())
        .format("%Y-%m-%d %H:%M")
        .to_string();

    println!("Sender:   {}", details.sender_name);
    println!("Subject:  {}", details.subject);
    println!("Created:  {}", local_datetime);

    println!("\nAttachments:");
    for i in 0..(details.parts.len()) {
        println!("  {}: {}", i + 1, details.attachment_name(i)?);
    }

    Ok(())
}
