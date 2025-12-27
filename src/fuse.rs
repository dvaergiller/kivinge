use std::{
    cmp::min,
    ffi::OsStr,
    fmt::{Display, Formatter},
    ops::{Shl, Shr},
    path::Path,
    time::{Duration, UNIX_EPOCH},
};

use bytes::Bytes;
use cached::{Cached, SizedCache, TimedCache, TimedSizedCache};
use fuser::{
    mount2, FileAttr, FileType, Filesystem, MountOption, ReplyData,
    ReplyDirectory, Request,
};
use libc::{EFAULT, EINVAL, EISDIR, ENOENT, ENOTDIR};
use thiserror::Error;
use tracing::{debug, error, warn};

use crate::{
    client::Client,
    model::content::{Attachment, InboxEntry, InboxListing, ItemDetails},
};

#[derive(Debug, Clone, Error, PartialEq)]
pub enum Error {
    #[error("not found")]
    NotFound,

    #[error("internal error: {0}")]
    InternalError(String),

    #[error("invalid")]
    Invalid,

    #[error("inode is directory")]
    IsDir,

    #[error("inode is not directory")]
    IsNotDir,
}

impl Error {
    fn error_code(&self) -> i32 {
        match self {
            Error::NotFound => {
                debug!("{}", self);
                ENOENT
            }

            Error::InternalError(_) => {
                error!("{}", self);
                EFAULT
            }

            Error::Invalid => {
                warn!("{}", self);
                EINVAL
            }

            Error::IsDir => {
                debug!("{}", self);
                EISDIR
            }

            Error::IsNotDir => {
                debug!("{}", self);
                ENOTDIR
            }
        }
    }
}

const INBOX_TTL: Duration = Duration::from_secs(60);
const DETAILS_TTL: Duration = Duration::from_mins(60);
const FILESYSTEM_TTL: Duration = Duration::from_secs(60);

pub fn mount(
    client: &mut impl Client,
    mountpoint: &Path,
) -> Result<(), std::io::Error> {
    let filesystem = KivraFS {
        client,
        inbox_cache: TimedSizedCache::with_size_and_lifespan(
            1,
            INBOX_TTL.into(),
        ),
        details_cache: TimedCache::with_lifespan(DETAILS_TTL.into()),
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

#[derive(Clone, Debug)]
enum Inode {
    Root,
    InboxEntry {
        entry: InboxEntry,
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
            Inode::InboxEntry { entry, .. } => (entry.id as u64 + 1).shl(32),
            Inode::Attachment { inbox_entry_id, attachment_id, .. } => {
                (*inbox_entry_id as u64 + 1).shl(32)
                    + (*attachment_id as u64 + 1)
            }
        }
    }

    fn entry_id(inode_id: u64) -> Option<u32> {
        match inode_id.shr(32) as u32 {
            0 => None,
            i => Some(i - 1),
        }
    }

    fn attachment_id(inode_id: u64) -> Option<u32> {
        match inode_id as u32 {
            0 => None,
            i => Some(i - 1),
        }
    }

    fn attr(&self) -> FileAttr {
        let (kind, perm, size, nlink) = match self {
            Inode::Root => (FileType::Directory, 0o500, 0u64, 2),
            Inode::InboxEntry { .. } => (FileType::Directory, 0o500, 0u64, 2),
            Inode::Attachment { attachment, .. } => {
                (FileType::RegularFile, 0o400, attachment.size as u64, 1)
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
            nlink,
            uid: 1000,
            gid: 1001,
            rdev: 0,
            flags: 0,
            blksize,
        }
    }
}

impl Display for Inode {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        format!("{:#016x}", self.to_u64()).fmt(f)
    }
}

struct KivraFS<'a, C: Client> {
    client: &'a mut C,
    inbox_cache: TimedSizedCache<(), InboxListing>,
    details_cache: TimedCache<u32, ItemDetails>,
    attachment_cache: SizedCache<(u32, u32), Bytes>,
}

impl<'a, C: Client> KivraFS<'a, C> {
    fn inbox_listing(&mut self) -> Result<InboxListing, Error> {
        self.inbox_cache
            .cache_try_get_or_set_with((), || {
                self.client
                    .get_inbox_listing()
                    .map_err(|err| Error::InternalError(err.to_string()))
            })
            .cloned()
    }

    fn inbox_entry(&mut self, entry_id: u32) -> Result<InboxEntry, Error> {
        let listing = self.inbox_listing()?;
        let entry = listing
            .iter()
            .find(|entry| entry.id == entry_id)
            .ok_or(Error::NotFound)?;
        Ok(entry.clone())
    }

    fn details(&mut self, entry: &InboxEntry) -> Result<ItemDetails, Error> {
        self.details_cache
            .cache_try_get_or_set_with(entry.id, || {
                self.client
                    .get_item_details(&entry.item.key)
                    .map_err(|err| Error::InternalError(err.to_string()))
            })
            .cloned()
    }

    fn attachment(
        &mut self,
        entry_id: u32,
        attachment_id: u32,
    ) -> Result<Attachment, Error> {
        let entry = self.inbox_entry(entry_id)?;
        let details = self.details(&entry)?;
        details
            .parts
            .get(attachment_id as usize)
            .ok_or(Error::NotFound)
            .cloned()
    }

    fn attachment_contents(
        &mut self,
        entry_id: u32,
        item_key: &str,
        attachment_id: u32,
    ) -> Result<Bytes, Error> {
        let attachment = self.attachment(entry_id, attachment_id)?;
        self.attachment_cache
            .cache_try_get_or_set_with((entry_id, attachment_id), || {
                match (attachment.body, attachment.key) {
                    (Some(inline_body), _) => {
                        Ok(inline_body.into_bytes().into())
                    }
                    (_, Some(attachment_key)) => Ok(self
                        .client
                        .download_attachment(item_key, &attachment_key)
                        .map_err(|err| {
                            Error::InternalError(err.to_string())
                        })?),
                    (None, None) => Err(Error::Invalid),
                }
            })
            .cloned()
    }

    fn inode(&mut self, inode_id: u64) -> Result<Inode, Error> {
        match (Inode::entry_id(inode_id), Inode::attachment_id(inode_id)) {
            (None, _) => Ok(Inode::Root),
            (Some(entry_id), None) => {
                Ok(Inode::InboxEntry { entry: self.inbox_entry(entry_id)? })
            }
            (Some(entry_id), Some(attachment_id)) => {
                let entry = self.inbox_entry(entry_id)?;
                let attachment = self.attachment(entry_id, attachment_id)?;
                Ok(Inode::Attachment {
                    inbox_entry_id: entry_id,
                    item_key: entry.item.key,
                    attachment_id,
                    attachment,
                })
            }
        }
    }

    fn inode_children(
        &mut self,
        parent_id: u64,
    ) -> Result<Vec<(String, Inode)>, Error> {
        match self.inode(parent_id)? {
            Inode::Root => Ok(self
                .inbox_listing()?
                .into_iter()
                .map(|entry| (entry.item.name(), Inode::InboxEntry { entry }))
                .collect()),
            Inode::InboxEntry { entry } => {
                let details = self.details(&entry)?;
                Ok(details
                    .parts
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, attachment)| {
                        let name = details.attachment_name(idx).ok()?;
                        let inode = Inode::Attachment {
                            inbox_entry_id: entry.id,
                            item_key: entry.item.key.clone(),
                            attachment_id: idx as u32,
                            attachment: attachment.clone(),
                        };
                        Some((name, inode))
                    })
                    .collect())
            }
            Inode::Attachment { .. } => Err(Error::IsNotDir),
        }
    }

    fn inode_by_name(
        &mut self,
        parent_id: u64,
        name: &str,
    ) -> Result<Inode, Error> {
        self.inode_children(parent_id)?
            .into_iter()
            .find_map(|(child_name, inode)| {
                (child_name == name).then_some(inode)
            })
            .ok_or(Error::NotFound)
    }
}

impl<'a, C: Client> Filesystem for KivraFS<'a, C> {
    fn lookup(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
        match self.inode_by_name(parent, &name.to_string_lossy()) {
            Ok(inode) => {
                debug!("found inode {inode } by name {name:?}");
                reply.entry(&FILESYSTEM_TTL, &inode.attr(), 0);
            }
            Err(error) => {
                reply.error(error.error_code());
            }
        }
    }

    fn getattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        reply: fuser::ReplyAttr,
    ) {
        match self.inode(ino) {
            Ok(inode) => reply.attr(&FILESYSTEM_TTL, &inode.attr()),
            Err(error) => reply.error(error.error_code()),
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
            Err(error) => reply.error(error.error_code()),
            Ok(Inode::Attachment {
                inbox_entry_id,
                item_key,
                attachment_id,
                ..
            }) => {
                let res = self.attachment_contents(
                    inbox_entry_id,
                    &item_key,
                    attachment_id,
                );
                match res {
                    Ok(data) => {
                        let start = offset as usize;
                        let end = min(data.len(), start + size as usize);
                        reply.data(&data[start..end]);
                    }
                    Err(error) => reply.error(error.error_code()),
                }
            }
            Ok(_) => {
                reply.error(Error::IsDir.error_code());
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
        let children = match self.inode_children(ino) {
            Err(error) => {
                reply.error(error.error_code());
                return;
            }
            Ok(children) => children,
        };

        let after_offset = &children[(offset as usize)..];
        debug!(
            "{} children, offset {}, {} entries left",
            children.len(),
            offset,
            after_offset.len(),
        );
        for (idx, (name, inode)) in after_offset.iter().enumerate() {
            let add_offset = idx as i64 + offset + 1;
            debug!("adding entry on offset {add_offset}");

            if let Inode::InboxEntry { entry } = inode {
                debug!("Adding inbox entry '{}'", entry.item.name());
            }

            if reply.add(
                inode.to_u64(),
                add_offset,
                inode.attr().kind,
                OsStr::new(&name),
            ) {
                debug!("output buffer full, stopping");
                break;
            }
        }
        reply.ok();
    }
}
