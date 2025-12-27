use serde::{Serialize, Deserialize};
use time::OffsetDateTime;
use rust_decimal::Decimal;

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
    pub code: String,
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

#[derive(Deserialize, Debug)]
pub struct ContentLabels {
    pub viewed: bool,
    pub trashed: bool,
    pub agreement: bool,
    pub paid: bool
}

pub type ContentKey = String;
pub type SenderKey = String;
pub type AgreementKey = String;

#[derive(Deserialize, Debug)]
pub struct ContentSpec {
    pub key: ContentKey,
    pub sender: SenderKey,
    pub sender_name: String,
    pub created_at: OffsetDateTime,
    pub generated_at: OffsetDateTime,
    pub subject: String,
    pub status: String, // Might be an enum later
    pub labels: ContentLabels,
    pub indexed_at: OffsetDateTime,
    pub payable: bool,
    pub amount: Decimal,
    pub input_amount: Decimal, // Unsure what this refers to
    pub currency: String,
    pub payment_status: Option<String>,
    pub pay_date: Option<OffsetDateTime>,
    pub due_date: Option<OffsetDateTime>,
    pub agreement_key: Option<AgreementKey>,
    pub agreement_status: Option<String>,
    pub variable_amount: bool,
    #[serde(rename = "type")]
    pub content_type: String,
    pub has_multiple_options: bool,
    pub sender_icon_url: String,

    // Do not know how to decode these yet
    // pub tags: // null
    // pub form: //null
}
