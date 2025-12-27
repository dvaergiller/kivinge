use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use std::{collections::BTreeMap, ops::Deref};

use super::Date;
use crate::error::Error;

pub type ContentKey = String;
pub type SenderKey = String;
pub type AgreementKey = String;
pub type ContentLabels = BTreeMap<String, bool>;

#[derive(Deserialize, Debug)]
pub struct InboxItem {
    pub key: ContentKey,
    pub sender: SenderKey,
    pub sender_name: String,
    pub created_at: DateTime<Utc>,
    // This can be empty. Let's worry about that if we need the field:
    // pub generated_at: DateTime,
    pub subject: String,
    pub status: String, // Might be an enum later
    pub labels: ContentLabels,
    pub indexed_at: DateTime<Utc>,
    #[serde(default)]
    pub payable: bool,
    pub amount: Option<Decimal>,
    pub input_amount: Option<Decimal>, // Unsure what this refers to
    pub currency: Option<String>,
    pub payment_status: Option<String>,
    pub pay_date: Option<Date>,
    pub due_date: Option<Date>,
    pub agreement_key: Option<AgreementKey>,
    pub agreement_status: Option<String>,
    pub variable_amount: Option<bool>,
    #[serde(rename = "type")]
    pub content_type: String,
    pub has_multiple_options: bool,
    pub sender_icon_url: String,
    // Do not know how to decode these yet
    // pub tags: // null
    // pub form: //null
}

#[derive(Debug)]
pub struct InboxEntry {
    pub id: u32,
    pub item: InboxItem,
}

pub struct InboxListing(Vec<InboxEntry>);

impl Deref for InboxListing {
    type Target = Vec<InboxEntry>;
    fn deref(&self) -> &Self::Target {
        let InboxListing(listing) = self;
        listing
    }
}

impl IntoIterator for InboxListing {
    type Item = InboxEntry;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl InboxListing {
    pub fn from_content_specs(mut vec: Vec<InboxItem>) -> InboxListing {
        vec.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        let listing = vec
            .into_iter()
            .zip(1..)
            .map(|(item, id)| InboxEntry { id, item })
            .collect();
        InboxListing(listing)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ItemDetails {
    pub subject: String,
    pub sender_name: String,
    pub created_at: DateTime<Utc>,
    pub parts: Vec<Attachment>,
}

impl ItemDetails {
    pub fn attachment_name(&self, index: usize) -> Result<String, Error> {
        let attachment = self.parts.get(index).ok_or(Error::AppError(
            "Attachment index out of bounds".to_string(),
        ))?;
        let file_extension = match attachment.content_type.as_str() {
            "application/pdf" => "pdf",
            "text/html" => "html",
            _ => "txt",
        };

        Ok(format!(
            "{}-{}-{}-{}.{}",
            self.created_at.to_rfc3339(),
            self.sender_name,
            self.subject,
            index,
            file_extension
        )
        .replace(' ', "_"))
    }
}

pub type AttachmentKey = String;

#[derive(Clone, Debug, Deserialize)]
pub struct Attachment {
    pub content_type: String,
    pub size: usize,
    pub key: Option<AttachmentKey>,
    pub body: Option<String>,
}
