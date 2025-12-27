use chrono::{Local, TimeZone};

use crate::error::Error;
use crate::kivra::model::ContentSpec;

pub fn print(inbox: &Vec<ContentSpec>) -> Result<(), Error> {
    for item in inbox {
        let local_datetime = Local.from_utc_datetime(&item.created_at.naive_utc());
        println!(
            "{} - {} - {}",
            item.sender_name,
            item.subject,
            local_datetime.format("%Y-%m-%d %H:%M")
        );
    }
    Ok(())
}
