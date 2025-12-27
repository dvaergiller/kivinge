use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use bytes::Bytes;

use crate::{
    client::{session, Client},
    error::Error,
    model::content::{InboxEntry, InboxItem, InboxListing, ItemDetails},
    tui,
};

pub fn load_session_or_login(
    client: &impl Client,
) -> Result<session::Session, Error> {
    let loaded = session::try_load()?;
    if let Some(session) = loaded {
        return Ok(session);
    }

    let mut terminal = tui::terminal::load()?;
    let mut login_view = tui::login::LoginView::make(client)?;
    match tui::show(&mut login_view, &mut terminal, None)? {
        Some(auth_response) => {
            let session = session::make(
                auth_response.access_token,
                auth_response.id_token,
            )?;
            session::save(&session)?;
            Ok(session)
        }
        None => Err(Error::UserError("Login aborted")),
    }
}

pub fn get_entry_by_id(
    inbox: InboxListing,
    item_id: u32,
) -> Result<InboxEntry, Error> {
    inbox
        .into_iter()
        .find(|i| i.id == item_id)
        .ok_or(Error::UserError("Inbox item does not exist"))
}

fn get_attachment_body(
    client: &impl Client,
    session: &session::Session,
    item: &InboxItem,
    details: &ItemDetails,
    attachment_num: u32,
) -> Result<Bytes, Error> {
    let attachment = details
        .parts
        .get(attachment_num as usize)
        .ok_or(Error::UserError("Inbox item has no such attachment number"))?;

    match (&attachment.key, &attachment.body) {
        (None, None) => Err(Error::AppError(
            "Attachment has no attachment key nor inline body",
        )),
        (Some(key), _) => client.download_attachment(session, &item.key, key),
        (_, Some(body)) => Ok(Bytes::copy_from_slice(body.as_bytes())),
    }
}

pub fn download_attachment(
    client: &impl Client,
    session: &session::Session,
    item: &InboxItem,
    attachment_num: u32,
    download_dir: PathBuf,
) -> Result<PathBuf, Error> {
    let details = client.get_item_details(session, &item.key)?;
    let file =
        get_attachment_body(client, session, item, &details, attachment_num)?;
    let filename = details.attachment_name(attachment_num as usize)?;
    let full_path = Path::new(&download_dir).join(filename);
    File::create_new(&full_path)?.write_all(&file)?;
    Ok(full_path)
}

pub fn open_attachment(
    client: &impl Client,
    session: &session::Session,
    item: &InboxItem,
    attachment_num: u32,
) -> Result<(), Error> {
    let tmp_dir = std::env::temp_dir();
    let path =
        download_attachment(client, session, item, attachment_num, tmp_dir)?;
    opener::open(path)?;
    Ok(())
}
