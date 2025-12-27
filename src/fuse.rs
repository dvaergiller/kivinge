use std::{
    ops::{Shl, Shr},
    path::Path,
    time::{Duration, UNIX_EPOCH},
};

use fuser::{
    mount2, FileAttr, FileType, Filesystem, MountOption, ReplyData,
    ReplyDirectory, Request,
};
use libc::{EFAULT, EINVAL, ENOENT};
use tracing::{debug, error, info, warn};

use crate::{
    client::Client,
    error::Error,
    model::content::InboxListing,
};

pub fn mount(
    client: &mut impl Client,
    mountpoint: &Path,
) -> Result<(), Error> {
    let filesystem =
        KivraFS { client, inbox_listing: InboxListing::default() };
    let mount_options = [
        MountOption::FSName("kivinge".to_string()),
        MountOption::DefaultPermissions,
        MountOption::RO,
        MountOption::NoAtime,
    ];
    mount2(filesystem, mountpoint, &mount_options)?;
    Ok(())
}

enum Inode {
    Root,
    InboxEntry { inbox_entry_id: u32 },
    Attachment { inbox_entry_id: u32, attachment_id: u32, size: u64 },
}

impl Inode {
    fn to_u64(&self) -> u64 {
        match self {
            Inode::Root => 1,
            Inode::InboxEntry { inbox_entry_id } => *inbox_entry_id as u64 + 1,
            Inode::Attachment { inbox_entry_id, attachment_id, .. } => {
                (*inbox_entry_id as u64 + 1).shl(32) + (*attachment_id as u64)
            }
        }
    }

    fn from_u64(number: u64) -> Inode {
        if number == 1 {
            Inode::Root
        } else if number.shr(32) == (0 as u64) {
            Inode::InboxEntry { inbox_entry_id: number as u32 - 1 }
        } else {
            Inode::Attachment {
                inbox_entry_id: number.shr(32) as u32 - 1,
                attachment_id: (number & 0xFFFFFFFF) as u32,
                size: 0,
            }
        }
    }

    fn attr(&self) -> FileAttr {
        let (kind, perm, size) = match self {
            Inode::Root => (FileType::Directory, 0o500, 0u64),
            Inode::InboxEntry { inbox_entry_id: _ } => {
                (FileType::Directory, 0o500, 0u64)
            }
            Inode::Attachment { inbox_entry_id: _, attachment_id: _, size } => {
                (FileType::RegularFile, 0o400, *size)
            }
        };
        FileAttr {
            ino: self.to_u64(),
            size,
            blocks: 1,
            atime: UNIX_EPOCH, // 1970-01-01 00:00:00
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind,
            perm,
            nlink: 1,
            uid: 1000,
            gid: 1001,
            rdev: 0,
            flags: 0,
            blksize: 512,
        }
    }
}

struct KivraFS<'a, C: Client> {
    client: &'a mut C,
    inbox_listing: InboxListing,
}

const TTL: Duration = Duration::from_secs(60);

impl<'a, C: Client> Filesystem for KivraFS<'a, C> {
    fn lookup(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
        let parent_inode = Inode::from_u64(parent);
        match parent_inode {
            Inode::Root => {
                let entry = self
                    .inbox_listing
                    .iter()
                    .find(|e| name.to_str() == Some(&e.item.name()));

                if let Some(e) = entry {
                    let inode = Inode::InboxEntry { inbox_entry_id: e.id };
                    reply.entry(&TTL, &inode.attr(), 0);
                } else {
                    reply.error(ENOENT);
                }
            }

            Inode::InboxEntry { inbox_entry_id } => {
                info!("Getting inbox entry {}", inbox_entry_id);
                let entry_lookup =
                    self.inbox_listing.iter().find(|e| e.id == inbox_entry_id);

                if entry_lookup.is_none() {
                    warn!("Inode does not exist");
                    reply.error(ENOENT);
                    return;
                }

                let entry = entry_lookup.unwrap();
                let details_res = self.client.get_item_details(&entry.item.key);
                if let Err(e) = details_res {
                    error!("Failed to fetch details: {}", e);
                    reply.error(EFAULT);
                    return;
                }

                let details = details_res.unwrap();
                let attachment_lookup =
                    details.parts.iter().enumerate().find(|(id, _)| {
                        debug!("Comparing {:?} to {:?}", name, details.attachment_name(*id));
                        name.to_str()
                            == details.attachment_name(*id).ok().as_deref()
                    });

                if attachment_lookup.is_none() {
                    warn!("No attachment with name {name:?}");
                    reply.error(ENOENT);
                    return;
                }
                let attachment = attachment_lookup.unwrap();
                let inode = Inode::Attachment {
                    inbox_entry_id,
                    attachment_id: attachment.0 as u32,
                    size: attachment.1.size as u64,
                };
                reply.entry(&TTL, &inode.attr(), 0);
            }

            Inode::Attachment { .. } => {
                reply.error(EINVAL);
            }
        }
    }

    fn getattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        reply: fuser::ReplyAttr,
    ) {
        reply.attr(&TTL, &Inode::from_u64(ino).attr());
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        match Inode::from_u64(ino) {
            Inode::Root => reply.error(EINVAL),
            Inode::InboxEntry { inbox_entry_id: _ } => reply.error(EINVAL),
            Inode::Attachment { inbox_entry_id, attachment_id, .. } => {
                let entry_lookup =
                    self.inbox_listing.iter().find(|e| e.id == inbox_entry_id);
                if entry_lookup.is_none() {
                    warn!("Already here");
                    reply.error(ENOENT);
                    return;
                }
                let entry = entry_lookup.unwrap();
                let details_res = self.client.get_item_details(&entry.item.key);
                if let Err(e) = details_res {
                    error!("Failed to fetch details: {}", e);
                    reply.error(EFAULT);
                    return;
                }
                let details = details_res.unwrap();
                let attachment_lookup = details
                    .parts
                    .into_iter()
                    .enumerate()
                    .find(|(id, _)| *id as u32 == attachment_id);

                if attachment_lookup.is_none() {
                    warn!("No here");
                    reply.error(ENOENT);
                    return;
                }
                let attachment = attachment_lookup.unwrap().1;

                let data_res = self.client.download_attachment(
                    &entry.item.key,
                    &attachment.key.unwrap(),
                );
                if let Err(e) = data_res {
                    error!("Error downloading attachment: {}", e);
                    reply.error(EFAULT);
                    return;
                }

                let data = data_res.unwrap();
                if data.is_empty() {
                    reply.data(&[]);
                } else {
                    let start = offset as usize;
                    let end =
                        std::cmp::min(data.len(), start + size as usize) - 1;
                    reply.data(&data[start..end]);
                }
            }
        }
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let entries: Vec<(Inode, String)> = match Inode::from_u64(ino) {
            Inode::Root => {
                self.inbox_listing =
                    match self.client.get_inbox_listing() {
                        Ok(listing) => listing,
                        Err(err) => {
                            error!("Failed to get inbox listing: {}", err);
                            reply.error(EFAULT);
                            return;
                        }
                    };
                self.inbox_listing
                    .iter()
                    .map(|entry| {
                        (
                            Inode::InboxEntry { inbox_entry_id: entry.id },
                            entry.item.name(),
                        )
                    })
                    .collect()
            }

            Inode::InboxEntry { inbox_entry_id } => {
                let entry_lookup = self
                    .inbox_listing
                    .iter()
                    .find(|entry| entry.id == inbox_entry_id);
                if entry_lookup.is_none() {
                    warn!("Error here");
                    reply.error(ENOENT);
                    return;
                }
                let entry = entry_lookup.unwrap();
                let details_res =
                    self.client.get_item_details(&entry.item.key);

                if let Err(e) = details_res {
                    error!("Failed to get item details: {}", e);
                    reply.error(EFAULT);
                    return;
                }

                let details = details_res.unwrap();
                details
                    .parts
                    .iter()
                    .enumerate()
                    .map(|(i, part)| {
                        (
                            Inode::Attachment {
                                inbox_entry_id: entry.id as u32,
                                attachment_id: i as u32,
                                size: part.size as u64,
                            },
                            details.attachment_name(i).unwrap(),
                        )
                    })
                    .collect()
            }

            Inode::Attachment { .. } => {
                reply.error(EINVAL);
                return;
            }
        };

        let dotlinks = vec![
            (Inode::InboxEntry { inbox_entry_id: 1 }, ".".to_string()),
            (Inode::InboxEntry { inbox_entry_id: 1 }, "..".to_string()),
        ];

        let contents = (&dotlinks)
            .into_iter()
            .chain(entries.iter())
            .filter(|(inode, _)| inode.to_u64() as i64 > offset);

        for (inode, name) in contents {
            if reply.add(
                inode.to_u64(),
                inode.to_u64() as i64 + 1,
                inode.attr().kind,
                name,
            ) {
                break;
            }
        }
        reply.ok();
    }
}
// asociety
//     afry
