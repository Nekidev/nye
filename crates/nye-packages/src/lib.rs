//! A crate to inspect, create, extract, and validate `nye` package files.
//!
//! This crate is internally used by `nye`. It is, however, meant to be used as a standalone library
//! other people can use, and is structured as such.

use crate::manifest::Manifest;

pub mod format;
pub mod manifest;
mod packages;

/// A nye package file.
pub struct Package {
    /// The package's manifest.
    ///
    /// Contains all the declarations of this package.
    pub manifest: Manifest,
}

impl Package {}
