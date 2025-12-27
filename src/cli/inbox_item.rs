use chrono::{Local, TimeZone};

use crate::kivra::model::ItemDetails;

pub fn print(details: ItemDetails) {
    let local_datetime = Local
        .from_utc_datetime(&details.created_at.naive_utc())
        .format("%Y-%m-%d %H:%M")
        .to_string();

    println!("Sender:   {}", details.sender_name);
    println!("Subject:  {}", details.subject);
    println!("Created:  {}", local_datetime);
    println!("\nAttachments:");
    for (part, i) in details.parts.iter().zip(1..) {
        println!("  {}: {}", i, part.name);
    }
}
