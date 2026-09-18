use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use anyhow::Context;

pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

/// Returns the permissions the current user has for a specified path.
///
/// Arguments:
/// * `path` - The file to check for.
///
/// Returns:
/// * `Ok(Permissions)` - Read, write, and execute permissions for the current user.
/// * `Err(Error)` - If an error occurred while getting the path's metadata.
pub fn get_current_user_permissions(path: impl AsRef<Path>) -> anyhow::Result<Permissions> {
    let meta = fs::metadata(path).context("Could not read the metadata of the specified path.")?;
    let mode = meta.permissions().mode();

    let current_uid = users::get_current_uid();
    let current_gid = users::get_current_gid();

    let owner_uid = meta.uid();
    let owner_gid = meta.gid();

    match (current_uid == owner_uid, current_gid == owner_gid) {
        (true, _) => Ok(Permissions {
            read: mode & 0o400 != 0,
            write: mode & 0o200 != 0,
            execute: mode & 0o100 != 0,
        }),
        (false, true) => Ok(Permissions {
            read: mode & 0o040 != 0,
            write: mode & 0o020 != 0,
            execute: mode & 0o010 != 0,
        }),
        (false, false) => Ok(Permissions {
            read: mode & 0o001 != 0,
            write: mode & 0o001 != 0,
            execute: mode & 0o001 != 0,
        }),
    }
}
