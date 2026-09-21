//! Safety rules for when reading untrusted package files.
//! 
//! Parsing untrusted files introduces security risks that allow a malicious actor to harm your
//! device in a few ways. This module counters these by allowing you to limit the resources used in
//! different parts of the parsing to protect yourself against malicious package files.
//! 
//! The configuration used by nye is the one returned by [`Safety::default()`]. In many cases,
//! these are good enough.
//! 
//! See the [`Safety`] type's documentation to see what does each field do.

/// Safety rules for when reading untrusted package files.
#[derive(Clone, Copy)]
pub struct Safety {
    /// The max amount of entries allowed.
    pub max_files: u16,
    /// The max file size allowed per file in the package file.
    pub max_file_size: u64,
    /// The max file name size allowed per file in the package file.
    pub max_file_name_size: u16,
    /// The manifest's max size.
    pub max_manifest_size: u64,
}

impl Default for Safety {
    fn default() -> Self {
        Self {
            max_files: 512,
            max_file_size: 1024 * 1024 * 10,
            max_file_name_size: 512,
            max_manifest_size: 1024 * 1024,
        }
    }
}
