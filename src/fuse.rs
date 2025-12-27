use std::{
    cmp::min, ffi::OsStr, ops::{Shl, Shr}, path::Path, time::{Duration, UNIX_EPOCH}
};

use bytes::Bytes;
use cached::{SizedCache, TimedCache, TimedSizedCache, Cached};
use fuser::{
    mount2, FileAttr, FileType, Filesystem, MountOption, ReplyData,
    ReplyDirectory, Request,
};
use libc::{EFAULT, EINVAL, EISDIR, ENOENT, ENOTDIR};
use tracing::{debug, warn};

use crate::{
    client::Client,
    model::content::{Attachment, InboxEntry, InboxListing, ItemDetails}
};

#[derive(Debug, Clone, Copy)]
pub enum Error {
    NotFound = ENOENT as isize,
    InternalError = EFAULT as isize,
    Invalid = EINVAL as isize,
    IsDir = EISDIR as isize,
    IsNotDir = ENOTDIR as isize,
}

impl Error {
    fn error_code(&self) -> i32 {
        match self {
            Error::NotFound => debug!("not found"),
            Error::InternalError => warn!("internal error"),
            Error::Invalid => warn!("invalid"),
            Error::IsDir => debug!("inode is directory"),
            Error::IsNotDir => debug!("inode is not directory"),
        }
        *self as i32
    }
}

pub fn mount(
    client: &mut impl Client,
    mountpoint: &Path,
) -> Result<(), std::io::Error> {
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
    // CurrentDir,
    // ParentDir,
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
            // Inode::CurrentDir => 1,
            // Inode::ParentDir => 1,
            Inode::InboxEntry { entry, .. } =>
                (entry.id as u64 + 1).shl(32),
            Inode::Attachment { inbox_entry_id, attachment_id, .. } =>
                (*inbox_entry_id as u64 + 1).shl(32) +
                (*attachment_id as u64),
        }
    }

    fn entry_id(inode_id: u64) -> u32 {
        inode_id.shr(32) as u32 - 1
    }

    fn attachment_id(inode_id: u64) -> u32 {
        inode_id as u32
    }

    fn attr(&self) -> FileAttr {
        let (kind, perm, size) = match self {
            Inode::Root => (FileType::Directory, 0o500, 0u64),
            // Inode::CurrentDir => (FileType::Directory, 0o500, 0u64),
            // Inode::ParentDir => (FileType::Directory, 0o500, 0u64),
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

    fn inbox_entry(&mut self, entry_id: u32) -> Result<InboxEntry, Error> {
        let listing = self.inbox_listing()?;
        let entry = listing
            .iter()
            .find(|entry| entry.id == entry_id)
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
        entry_id: u32,
        attachment_id: u32
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
        self.attachment_cache.cache_try_get_or_set_with(
            (entry_id, attachment_id),
            || {
                match (attachment.body, attachment.key) {
                    (Some(inline_body), _) => {
                        Ok(inline_body.into_bytes().into())
                    }
                    (_, Some(attachment_key)) => {
                        Ok(self
                           .client
                           .download_attachment(item_key, &attachment_key)
                           .map_err(|_| Error::InternalError)?
                        )
                    }
                    (None, None) => {
                        Err(Error::Invalid)
                    }
                }
            }
        ).cloned()
    }

    fn inode(&mut self, inode_id: u64) -> Result<Inode, Error> {
        match inode_id {
            // 0 => return Ok(Inode::CurrentDir),
            1 => return Ok(Inode::Root),
            _ => (),
        }

        let entry_id = Inode::entry_id(inode_id);
        let attachment_id = Inode::attachment_id(inode_id);
        let entry = self.inbox_entry(entry_id)?;

        if attachment_id == 0 {
            Ok(Inode::InboxEntry { entry })
        }
        else {
            let attachment = self.attachment(entry_id, attachment_id)?;
            Ok(Inode::Attachment {
                inbox_entry_id: entry_id,
                item_key: entry.item.key,
                attachment_id,
                attachment,
            })
        }
    }

    fn inode_children(
        &mut self,
        parent_id: u64
    ) -> Result<Vec<(String, Inode)>, Error> {
        match self.inode(parent_id)? {
            Inode::Root => {
                Ok(self
                   .inbox_listing()?
                   .into_iter()
                   .map(|entry|
                        (entry.item.name(), Inode::InboxEntry { entry })
                   )
                   .collect())
            },
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
            },
            Inode::Attachment { .. } => {
                Err(Error::IsNotDir)
            }
            // _ => {
            //     Err(Error::Invalid)
            // }
        }
    }

    fn inode_by_name(
        &mut self,
        parent_id: u64,
        name: &str
    ) -> Result<Inode, Error> {
        self
            .inode_children(parent_id)?
            .into_iter()
            .find_map(|(child_name, inode)| {
                (child_name == name).then_some(inode)
            })
            .ok_or(Error::NotFound)
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
        debug!("Looking up {name:?} in {parent:#018x}");
        match self.inode_by_name(parent, &name.to_string_lossy()) {
            Ok(inode) => {
                reply.entry(&TTL, &inode.attr(), 0);
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
        debug!("getattr: {ino}");
        match self.inode(ino) {
            Ok(inode) => reply.attr(&TTL, &inode.attr()),
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
        debug!("read: {ino}");
        match self.inode(ino) {
            Err(error) => reply.error(error.error_code()),
            Ok(Inode::Attachment { inbox_entry_id, item_key, attachment_id, .. }) => {
                let res = self.attachment_contents(
                    inbox_entry_id,
                    &item_key,
                    attachment_id
                );
                match res {
                    Ok(data) => {
                        let start = offset as usize;
                        let end = min(data.len(), start + size as usize) - 1;
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
        debug!("readdir: {ino:#018x}");
        let children = match self.inode_children(ino) {
            Err(error) => {
                reply.error(error.error_code());
                return;
            },
            Ok(children) => {
                children
            }
        };

        debug!(
            "Listing directory {} with {} children from offset {}",
            ino,
            children.len(),
            offset
        );

        let special_files = [
            // (".".to_string(), Inode::CurrentDir),
            // ("..".to_string(), Inode::ParentDir),
        ];

        let result = special_files
            .into_iter()
            .chain(children.into_iter())
            .enumerate()
            .skip(offset as usize);

        for (idx, (name, inode)) in result {
            let ino = inode.to_u64();
            if reply.add(
                ino,
                (idx + 1) as i64,
                inode.attr().kind,
                OsStr::new(&name)) {
                break;
            }
        }
        reply.ok();
    }
}
