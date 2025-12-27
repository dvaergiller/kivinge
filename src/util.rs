use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use bytes::Bytes;

use crate::{
    client::Client,
    error::Error,
    model::content::{InboxEntry, InboxItem, InboxListing, ItemDetails},
};

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
    client: &mut impl Client,
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
        (Some(key), _) => Ok(client.download_attachment(&item.key, key)?),
        (_, Some(body)) => Ok(Bytes::copy_from_slice(body.as_bytes())),
    }
}

pub fn download_attachment(
    client: &mut impl Client,
    item: &InboxItem,
    attachment_num: u32,
    download_dir: PathBuf,
) -> Result<PathBuf, Error> {
    let details = client.get_item_details(&item.key)?;
    let file = get_attachment_body(client, item, &details, attachment_num)?;
    let filename = details.attachment_name(attachment_num as usize)?;
    let full_path = Path::new(&download_dir).join(filename);
    File::create_new(&full_path)?.write_all(&file)?;
    Ok(full_path)
}

pub fn open_attachment(
    client: &mut impl Client,
    item: &InboxItem,
    attachment_num: u32,
) -> Result<(), Error> {
    let tmp_dir = std::env::temp_dir();
    let path = download_attachment(client, item, attachment_num, tmp_dir)?;
    opener::open(path)?;
    Ok(())
}
