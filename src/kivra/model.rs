use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub company_onboard_complete_oauth_client_id: String,
    pub oauth_default_client_id: String,
}
