use serde::{Deserialize, Serialize};

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
