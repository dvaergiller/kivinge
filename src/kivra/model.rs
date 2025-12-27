use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ops::Deref};

use crate::error::Error;

pub type UserId = String;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub company_onboard_complete_oauth_client_id: String,
    pub oauth_endpoint_url: String,
    pub oauth_default_client_id: String,
    pub oauth_default_redirect_uri: String,
    pub oauth_grant_type: String,
    pub oauth_response_type: String,
}

pub type CodeVerifier = Vec<u8>;
pub type AuthCode = String;

#[derive(Serialize)]
pub struct AuthRequest {
    pub response_type: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub scope: String,
    pub client_id: String,
    pub redirect_uri: String,
}

#[derive(Deserialize, Debug)]
pub struct AuthResponse {
    pub auto_start_token: String,
    pub qr_data: Vec<String>,
    pub qr_code: String,
    pub code: AuthCode,
    pub next_poll_url: String,
}

#[derive(Deserialize, Debug)]
pub struct AuthStatus {
    pub status: String,
    pub progress_status: String,
    pub message_code: String,
    pub qr_code: String,
    pub ssn: Option<String>,
    pub retry_after: Option<u32>,
    pub next_poll_url: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct AuthTokenRequest {
    pub client_id: String,
    pub code: String,
    pub code_verifier: String,
    pub grant_type: String,
    pub redirect_uri: String,
}

#[derive(Deserialize, Debug)]
pub struct AuthTokenResponse {
    pub access_token: String,
    pub expires_in: u32,
    pub id_token: String,
    pub scope: String,
    pub token_type: String,
}

#[derive(Serialize, Debug)]
pub struct RevokeRequest {
    pub token: String,
    pub token_type_hint: String,
}

pub type ContentKey = String;
pub type SenderKey = String;
pub type AgreementKey = String;
pub type ContentLabels = BTreeMap<String, bool>;

#[derive(Debug)]
pub struct Date(pub chrono::NaiveDate);

impl<'a> Deserialize<'a> for Date {
    fn deserialize<Des: serde::Deserializer<'a>>(d: Des) -> Result<Date, Des::Error> {
        let mut date_string = String::deserialize(d)?.clone();
        let _removed = date_string.split_off(10);
        let date = NaiveDate::parse_from_str(&date_string, "%Y-%m-%d")
            .map_err(serde::de::Error::custom)?;
        Ok(Date(date))
    }
}

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
    pub body: Option<Vec<u8>>,
}
