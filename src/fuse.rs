use std::{
    ops::{Shl, Shr},
    path::Path,
    time::{Duration, UNIX_EPOCH},
};

use bytes::Bytes;
use cached::{SizedCache, TimedCache, TimedSizedCache, Cached};
use fuser::{
    mount2, FileAttr, FileType, Filesystem, MountOption, ReplyData,
    ReplyDirectory, Request,
};
use libc::{EFAULT, EINVAL, EISDIR, ENOENT};
use tracing::{debug, error, warn};

use crate::{
    client::Client, model::content::{Attachment, InboxEntry, InboxItem, InboxListing, ItemDetails}
};

enum Error {
    NotFound,
    InternalError,
    Invalid,
    IsDir,
}

pub fn mount(
    client: &mut impl Client,
    mountpoint: &Path,
) -> Result<(), Error> {
    let filesystem =
        KivraFS {
            client,
            inbox_cache: TimedSizedCache::with_size_and_lifespan(1, TTL.into()),
            details_cache: TimedCache::with_lifespan(TTL.into()),
            attachment_cache: SizedCache::with_size(10),
        };
    let mount_options = [
        MountOption::FSName("kivinge".to_string()),
        MountOption::DefaultPermissions,
        MountOption::RO,
        MountOption::NoAtime,
    ];
    mount2(filesystem, mountpoint, &mount_options)?;
    Ok(())
}

#[derive(Clone)]
enum Inode {
    Root,
    InboxEntry {
        entry: InboxEntry,
        details: Option<ItemDetails>,
    },
    Attachment {
        inbox_entry_id: u32,
        item_key: String,
        attachment_id: u32,
        attachment: Attachment,
    },
}

impl Inode {
    fn to_u64(&self) -> u64 {
        match self {
            Inode::Root => 1,
            Inode::InboxEntry { entry, .. } => entry.id as u64 + 1,
            Inode::Attachment { inbox_entry_id, attachment_id, .. } => {
                (*inbox_entry_id as u64 + 1).shl(32) + (*attachment_id as u64)
            }
        }
    }

    fn entry_id(inode_id: u64) -> u32 {
        inode_id.shr(32) as u32
    }

    fn attachment_id(inode_id: u64) -> u32 {
        inode_id as u32
    }

    fn attr(&self) -> FileAttr {
        let (kind, perm, size) = match self {
            Inode::Root => (FileType::Directory, 0o500, 0u64),
            Inode::InboxEntry { .. } => {
                (FileType::Directory, 0o500, 0u64)
            }
            Inode::Attachment { attachment, .. } => {
                (FileType::RegularFile, 0o400, attachment.size as u64)
            }
        };
        let blksize = 512u32;
        FileAttr {
            ino: self.to_u64(),
            size,
            blocks: size.div_ceil(blksize as u64),
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
            blksize,
        }
    }
}

struct KivraFS<'a, C: Client> {
    client: &'a mut C,
    inbox_cache: TimedSizedCache<(), InboxListing>,
    details_cache: TimedCache<u32, ItemDetails>,
    attachment_cache: SizedCache<(u32, u32), Bytes>,
}

impl<'a, C: Client> KivraFS<'a, C> {
    fn inode(&mut self, inode_id: u64) -> Result<Inode, Error> {
        let entry_id = Inode::entry_id(inode_id);
        let attachment_id = Inode::attachment_id(inode_id);

        if entry_id == 0 {
            return Inode::Root;
        }

        let entry = self.inbox_entry(entry_id)?;
        let details = self.details(&entry)?;

        if attachment_id == 0 {
            Inode::InboxEntry { entry, details }
        }
        else {
            let attachment = self.
        }
            (0, 1) => Inode::Root,
            (entry_id, 0) => {
                let entry = self.inbox_entry(entry_id)?;
                let details = self.details(&entry)?;
                Inode::InboxEntry { entry, details }
            },
            (entry_id, attachment_id) => {
                let entry = self.inbox_entry(entry_id)?;
                let details = self.details(&entry)?;
                
            }
        }
    }

    fn inbox_listing(&mut self) -> Result<InboxListing, Error> {
        self.inbox_cache.cache_try_get_or_set_with(
            (),
            || {
                self
                    .client
                    .get_inbox_listing()
                    .map_err(|_| Error::InternalError)
            }
        ).cloned()
    }

    fn inbox_entry(&mut self, item_id: u32) -> Result<InboxEntry, Error> {
        let listing = self.inbox_listing()?;
        let entry = listing
            .iter()
            .find(|entry| entry.id == item_id)
            .ok_or(Error::NotFound)?;
        Ok(entry.clone())
    }

    fn details(&mut self, entry: &InboxEntry) -> Result<ItemDetails, Error> {
        self.details_cache.cache_try_get_or_set_with(
            entry.id,
            || {
                self
                    .client
                    .get_item_details(&entry.item.key)
                    .map_err(|_| Error::InternalError)
            }
        ).cloned()
    }

    fn attachment(
        &mut self,
        item_id: u32,
        attachment_id: u32
    ) -> Result<Bytes, Error> {
        let entry = self.inbox_entry(item_id)?;
        let details = self.details(&entry)?;
        self.details_cache.cache_try_get_or_set_with(
            (entry.id, attachment_id),
            || {
                let attachment = details
                    .parts
                    .get(attachment_id as usize)
                    .ok_or(Error::NotFound)?;
                match (attachment.key, attachment.body) {
                    (None, None) => {
                        Err(Error::Invalid)
                    },
                    (Some(attachment_key), _) => {
                        self.client.download_attachment(
                            &entry.item.key,
                            &attachment.key
                        ).map_err(|_| Error::InternalError)
                    },
                    (_, Some(body)) => {
                        body.as_bytes().into()
                    }
                }
            }
        ).cloned()
    }
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
        let parent_inode = self.inode(parent);
        let entry_id = Inode::entry_id(parent_inode);
        let attachment_id = Inode::attachment_id(parent_inode);
        // match (entry_idInode:: {
        //     Some(Inode::Root) => {
        //         let entry = self
        //             .inbox_listing()
        //             .into_iter()
        //             .find(|e| name.to_str() == Some(&e.item.name()));

        //         if let Some(e) = entry {
        //             let inode = Inode::InboxEntry {
        //                 entry: e,
        //                 details: None,
        //             };
        //             reply.entry(&TTL, &inode.attr(), 0);
        //         } else {
        //             reply.error(ENOENT);
        //         }
        //     }

        //     Some(Inode::InboxEntry { entry, .. }) => {
        //         debug!("Getting inbox entry {}", entry.id);
        //         let details_res = self.client.get_item_details(&entry.item.key);
        //         if let Err(e) = details_res {
        //             error!("Failed to fetch details: {}", e);
        //             reply.error(EFAULT);
        //             return;
        //         }

        //         let details = details_res.unwrap();
        //         let attachment_lookup =
        //             details.parts.into_iter().enumerate().find(|(id, _)| {
        //                 debug!("Comparing {:?} to {:?}", name, details.attachment_name(*id));
        //                 name.to_str()
        //                     == details.attachment_name(*id).ok().as_deref()
        //             });

        //         if attachment_lookup.is_none() {
        //             debug!("No attachment with name {name:?}");
        //             reply.error(ENOENT);
        //             return;
        //         }
        //         let attachment = attachment_lookup.unwrap();
        //         let inode = Inode::Attachment {
        //             inbox_entry_id: entry.id,
        //             attachment_id: attachment.0 as u32,
        //             attachment: attachment.1,
        //         };
        //         reply.entry(&TTL, &inode.attr(), 0);
        //     }

        //     Some(Inode::Attachment { .. }) => {
        //         reply.error(EINVAL);
        //     }

        //     None => {
        //         reply.error(ENOENT);
        //     }
        // }
    }

    fn getattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        reply: fuser::ReplyAttr,
    ) {
        match self.inode(ino) {
            Some(inode) => reply.attr(&TTL, &inode.attr()),
            None => reply.error(ENOENT),
        }
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
        match self.inode(ino) {
            None => reply.error(ENOENT),
            Some(Inode::Root) => reply.error(EISDIR),
            Some(Inode::InboxEntry { .. }) => reply.error(EISDIR),
            Some(Inode::Attachment { inbox_entry_id, attachment, .. }) => {
                let entry_lookup =
                    self.inbox_listing.iter().find(|e| e.id == inbox_entry_id);
                if entry_lookup.is_none() {
                    warn!("Already here");
                    reply.error(ENOENT);
                    return;
                }
                let entry = entry_lookup.unwrap();

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
        let entries: Vec<(Inode, String)> = match self.inode(ino) {
            Some(Inode::Root) => {
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
                            Inode::InboxEntry {
                                inbox_entry_id: entry.id,
                                item_key: entry.item.key.clone(),
                            },
                            entry.item.name(),
                        )
                    })
                    .collect()
            }

            Some(Inode::InboxEntry { inbox_entry_id, item_key }) => {
                let details_res = self.client.get_item_details(&item_key);

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
                                inbox_entry_id,
                                attachment_id: i as u32,
                                size: part.size as u64,
                                item_key: item_key.clone(),
                                attachment_key: part.key.clone(),
                            },
                            details.attachment_name(i).unwrap(),
                        )
                    })
                    .collect()
            }

            Some(Inode::Attachment { .. }) => {
                reply.error(EINVAL); // Not a directory
                return;
            }

            None => {
                reply.error(ENOENT);
                return;
            }
        };

        let dotlinks = vec![
            (Inode::InboxEntry { inbox_entry_id: 1, item_key: String::new() }, ".".to_string()),
            (Inode::InboxEntry { inbox_entry_id: 1, item_key: String::new() }, "..".to_string()),
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
