use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::PathBuf;

use super::error::Error;
use super::model::UserId;

#[derive(Clone, Deserialize, Debug)]
pub struct UserInfo {
    pub kivra_user_id: UserId,
    pub name: String,
    pub first_name: String,
    pub last_name: String,
    pub ssn: String,
    pub email: String,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub user_info: UserInfo,
    pub access_token: String,
    pub id_token: String,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
struct StoredSession {
    access_token: String,
    id_token: String,
}

impl TryInto<Session> for StoredSession {
    type Error = Error;
    fn try_into(self) -> Result<Session, Error> {
        let user_info = extract_user_info(&self.id_token)?;
        Ok(Session {
            user_info,
            access_token: self.access_token,
            id_token: self.id_token,
        })
    }
}

impl From<Session> for StoredSession {
    fn from(session: Session) -> StoredSession {
        StoredSession {
            access_token: session.access_token,
            id_token: session.id_token,
        }
    }
}

fn default_session_path() -> Result<PathBuf, Error> {
    let mut path = dirs::data_local_dir().ok_or(Error::AppError(
        "Failed to determine data local dir for saving session data".to_string(),
    ))?;
    path.push("kivinge.session");
    Ok(path)
}

pub fn try_load() -> Result<Option<Session>, Error> {
    let session_path = default_session_path()?;
    if !session_path.exists() {
        return Ok(None);
    }

    let session_file = File::open(session_path)?;
    let stored_session: StoredSession = serde_json::from_reader(session_file)?;
    Ok(Some(stored_session.try_into()?))
}

pub fn save(session: &Session) -> Result<(), Error> {
    let session_path = default_session_path()?;
    let session_file = File::create(session_path)?;
    let stored_session: StoredSession = session.clone().into();
    serde_json::to_writer(session_file, &stored_session)?;
    Ok(())
}

pub fn delete_saved() -> Result<(), Error> {
    let session_path = default_session_path()?;
    Ok(std::fs::remove_file(session_path)?)
}

pub fn make(access_token: String, id_token: String) -> Result<Session, Error> {
    let user_info = extract_user_info(&id_token)?;
    Ok(Session {
        user_info,
        access_token,
        id_token,
    })
}

fn extract_user_info(id_token: &str) -> Result<UserInfo, Error> {
    let sections = id_token.split('.').collect::<Vec<&str>>();
    let claims_base64 = sections.get(1).ok_or(Error::AppError(
        "Malformed JWT returned by server: Too few sections".to_string(),
    ))?;
    let claims_json = URL_SAFE_NO_PAD.decode(claims_base64)?;
    Ok(serde_json::from_slice(claims_json.as_slice())?)
}
