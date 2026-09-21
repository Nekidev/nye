//! A crate to inspect, create, extract, and validate `nye` package files.
//!
//! This crate is internally used by `nye`. It is, however, meant to be used as a standalone library
//! other people can use, and is structured as such.

compile_error!("TODO: Implement writers.");
compile_error!("TODO: Validate manifests against themselves and against package file directories.");
compile_error!("TODO: Implement Package::inspect() to provide a nicer API than handling files by hand.");

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
