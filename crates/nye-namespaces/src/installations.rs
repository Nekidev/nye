// TODO: Add support for installing/uninstalling packages. I'm thinking that collisions and
//       conflicts will be better checked all at once before installing any packages. The flow
//       could be something like
//
//       Namespace::install() -> Installation::validate() -> Installation::execute()
//
//       It would allow to configure many installation parameters, such as excluding artifacts from
//       being exposed. Maybe even make `Installation` an interface to a single package's
//       installation and expose an API to validate, install, uninstall, and update the
//       installation. That could be really good.
//
// TODO: When installing a package, lets extract entries from last to first so we can truncate the
//       package file as we extract them. This will allow to extract big package files without
//       duplicating them on the system in most cases. Files can be truncated with `set_len()`.

use std::path::PathBuf;

use nye_packages::format::reading::{NyeFileSeekableReader, ReadableSeekable};
use nye_schemas::semver::Semver;

use crate::Namespace;

pub struct Installation {
    pub path: PathBuf,

    pub package_name: String,
    pub package_version: Semver,
}

pub struct InstallationBuilder<R>
where
    R: ReadableSeekable,
{
    pub package: NyeFileSeekableReader<R>,
    pub namespace: Namespace,
}

impl<R> InstallationBuilder<R>
where
    R: ReadableSeekable,
{
    pub fn new(namespace: Namespace, package: NyeFileSeekableReader<R>) -> Self {
        Self { package, namespace }
    }
}
