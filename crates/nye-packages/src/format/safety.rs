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
    /// The package name's max length.
    pub max_package_name_size: u64,
    /// The package version's max length.
    pub max_package_version_size: u64,
    /// The max amount of exposed links (cross artifact type).
    pub max_links: u64,
    /// A link's (artifact alias) max length.
    pub max_link_size: u64,
    /// The max amount of exposed binaries.
    pub max_exposed_bins: u64,
    /// The max amount of exposed libraries.
    pub max_exposed_libs: u64,
    /// The max amount of exposed env variable values.
    pub max_exposed_vars: u64,
    /// The max amount of consumed environment variables.
    pub max_consumed_vars: u64,
    /// The max exposed & consumed environment variable name length.
    pub max_var_name_size: u64,
    /// The max exposed & consumed environment variable value length.
    pub max_var_value_size: u64,
    /// The max consumed environment variable separator length.
    pub max_var_separator_size: u64,
    /// Restricts the target to the current system's target.
    pub only_current_target: bool,
    /// Restrict the targets to nye-supported targets.
    pub only_supported_targets: bool,
}

impl Default for Safety {
    fn default() -> Self {
        Self {
            max_files: 512,
            max_file_size: 1024 * 1024 * 10,
            max_file_name_size: 512,
            max_manifest_size: 1024 * 1024,
            max_package_name_size: 32,
            max_package_version_size: 64,
            max_links: 128,
            max_link_size: 32,
            max_exposed_bins: 32,
            max_exposed_libs: 32,
            max_exposed_vars: 32,
            max_consumed_vars: 32,
            max_var_name_size: 32,
            max_var_value_size: 512,
            max_var_separator_size: 8,
            only_current_target: true,
            only_supported_targets: true,
        }
    }
}
