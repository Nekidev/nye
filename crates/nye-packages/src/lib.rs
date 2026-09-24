//! A crate to inspect, create, extract, and validate `nye` package files.
//!
//! This crate is internally used by `nye`. It is, however, meant to be used as a standalone library
//! other people can use, and is structured as such.
//! 
//! There are two main submodules where you'll find lots of documentation:
//! 
//! * [`format`]: Everything about the package file format file.
//! * [`manifest`]: Everything about the package manifest.

// compile_error!("TODO: Validate manifests against themselves and against package file directories.");
// compile_error!("TODO: Add compression support on the file.");

pub mod format;
pub mod manifest;
