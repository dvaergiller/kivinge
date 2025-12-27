pub mod auth;
pub mod content;

use chrono::NaiveDate;
use serde::Deserialize;

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
