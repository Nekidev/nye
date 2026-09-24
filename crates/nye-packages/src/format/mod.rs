//! Nye's package file format.
//!
//! Initially, `nye` used zip files. However, this comes with a lot of security headaches and few
//! libraries with good ergonomics, so we moved to a custom package file format for better control.
//!
//! Nye package files use the `.nye` file extension.
//!
//! # Format
//!
//! The format is extremely simple. It contains a directory at the top of the file that defines the
//! metadata for the rest of the file, it has no compression support (it can be added on top of it),
//! and only carries the metadata needed by nye.
//!
//! The layout is the following:
//!
//! ```txt
//! +-------------------+
//! | File Signature    |
//! +-------------------+
//! | Directory         |
//! | - File 1 Metadata |
//! | - File 2 Metadata |
//! | ...               |
//! +-------------------+
//! | Manifest          |
//! +-------------------+
//! | File Contents     |
//! | - File 1 Contents |
//! | - File 2 Contents |
//! | ...               |
//! +-------------------+
//! ```
//!
//! All numbers in the file are stored as big endians.
//!
//! ## File Signature
//!
//! Each nye package file starts with the folllowing four bytes:
//!
//! ```txt
//! 110 121 101 0
//! ```
//!
//! Converted to ASCII, it reads "nye", then a `0` for the file format version.
//!
//! If the version is not `0`, make sure to fail parsing the file or implement support for future
//! versions.
//!
//! ## Directory
//!
//! The directory contains all the metadata for the files inside the package file.
//!
//! It begins with 2 bytes, representing the amount of entries in the directory. It DOES NOT count
//! the manifest, meaning the package file will exactly one manifest file + the amount of files
//! these 2 bytes specify.
//!
//! Then, a `u64` specifying the manifest file's size. The manifest file always appears first in
//! the package file for easier analysis of package files.
//!
//! At the end of it, the entries' metadata.
//!
//! ### File Metadata
//!
//! Each file entry's metadata consists of the following:
//!
//! * `size` (`u64`): The size of the file, in bytes.
//! * `type` (`u8`): The file type.
//!     * `0` - A binary file (`bin/`).
//!     * `1` - A library file (`lib/`).
//!     * `2` - An editable text configuration file (`etc/`).
//!     * `3` - A variable data file (`var/`).
//! * `name` (`segments`): The file's name, using nye's segment encoding.
//!
//! #### Segments Encoding
//!
//! Nye uses a custom file path encoding to reduce the amount of invalid states representable.
//!
//! When writing normal file paths, there's multiple undesireable states from the package manager's
//! point of view. `.` segments, `..` segments, double slashes, backslashes, absolute paths, invalid
//! characters, and paths with trailing slashes are just some examples. Nye's segment encoding makes
//! many of those undesireable states not representable, which reduces the amount of additional
//! validation requires and improves the safety of the format.
//!
//! Segment encoding follows the following layout:
//!
//! ```text
//! +--------------------------+
//! | u8: Segment count - 1    |
//! +--------------------------+
//! | u8: Segment 1 length - 1 |
//! |     Segment 1 bytes      |
//! +--------------------------+
//! | u8: Segment 2 length - 1 |
//! |     Segment 2 bytes      |
//! +--------------------------+
//! | ...                      |
//! +--------------------------+
//! ```
//!
//! Segment bytes use the following alphabet:
//!
//! * a-z: 0-25
//! * A-Z: 26-51
//! * 0-9: 52-61
//! * `-`: 62
//! * `_`: 63
//! * `.`: 64
//! * `,`: 65
//! * `@`: 66
//!
//! When converted back to a file path, segments are decoded and joined using `/`.
//!
//! The following segments are not allowed:
//!
//! * `.`
//! * `..`
//!
//! ### File Contents
//!
//! The first bytes after the directory are the manifest. The size of this section will be
//! according to the manifest file size defined in the directory.
//!
//! After the manifest, file contents will be defined sequentially. You can calculate the offset of
//! each file using each file's size. Files go in order, meaning the first file to appear in the
//! directory will be the first file to have its contents defined.
//!
//! For example, given the following example package file data:
//!
//! ```text
//! 3 bytes of nye
//! 1 byte of file format version
//! 2 bytes of the amount of entries in the directory
//! 8 bytes of the manifest section's size
//!     8 bytes of entry 1 size
//!     1 byte of entry 1 type
//!     1 byte of entry 1 name segment count
//!         1 byte of segment length
//!         X bytes of segment bytes
//!         ... do once per segment
//!     ... do once per entry
//! X bytes of manifest data
//! X bytes of entry 1 data
//! X bytes of entry 2 data
//! ...
//! ```
//!
//! # In Rust
//!
//! This module provides a reader and a writer for package files, aiming at ergonomics and safety.
//! Safety is in multiple cases up to you, so there's configurable safety rules for you to use
//! according to your program's needs.

use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::path::PathBuf;
use std::str::FromStr;

use crate::format::reading::decoding::Decodeable;

pub mod reading;
pub mod safety;
pub mod writing;

/// Nye's segments type alphabet.
pub const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.,@";

/// The signature section of a package file, containing the package file format version.
#[derive(Debug, Clone, Copy)]
pub struct NyeFileSignature {
    /// The package file format version.
    pub version: u8,
}

/// The directory section of a package file, containing manifest and entry metadata.
///
/// This type also holds an index of (type, name) -> entry index for fast access by name.
#[derive(Default, Debug, Clone)]
pub struct NyeFileDirectory {
    /// The size of the manifest section in bytes.
    pub manifest_size: u64,

    /// An index table of file type + name to index in the file.
    pub index: HashMap<(NyeFileEntryKind, Segments), usize>,
    /// Metadata of each entry in the directory section.
    pub entries: Vec<NyeFileEntry>,
}

impl NyeFileDirectory {
    /// Returns the start and end of the contents of the directory section in the package file.
    ///
    /// The returned value is returned as `[a; b)`, meaning the first index is inclusive and the
    /// second one is exclusive.
    pub fn get_directory_location(&self) -> (u64, u64) {
        (4, 4 + self.size())
    }

    /// Returns the start and end of the contents of the manifest in the package file.
    ///
    /// The returned value is returned as `[a; b)`, meaning the first index is inclusive and the
    /// second one is exclusive.
    pub fn get_manifest_location(&self) -> (u64, u64) {
        let offset = self.get_directory_location().1;

        (offset, offset + self.manifest_size)
    }

    /// Returns the start and end of the contents of an entry in the package file.
    ///
    /// Returns:
    /// * `Some((u64, u64))` - The file's location, as `[a; b)` (first index inclusive, second index
    ///   exclusive).
    /// * `None` - The index specified does not exist.
    pub fn get_entry_location_by_index(&self, index: usize) -> Option<(u64, u64)> {
        if let Some(entry) = self.entries.get(index) {
            let mut offset = self.get_manifest_location().1;

            for (i, entry) in self.entries.iter().enumerate() {
                if i == index {
                    break;
                }

                offset += entry.size;
            }

            Some((offset, entry.size))
        } else {
            None
        }
    }

    /// Returns a directory entry by its position in the directory.
    pub fn get_entry_by_index(&self, index: usize) -> Option<&NyeFileEntry> {
        self.entries.get(index)
    }

    /// Returns a directory entry by its file type and name.
    /// 
    /// Arguments:
    /// * `kind` - The file kind. E.g. bin, lib.
    /// * `path` - The file's path.
    pub fn get_entry_by_path(&self, kind: NyeFileEntryKind, path: impl Into<Segments>) -> Option<&NyeFileEntry> {
        if let Some(index) = self.index.get(&(kind, path.into())) {
            self.entries.get(*index)
        } else {
            None
        }
    }
}

/// An individual entry's metadata.
///
/// Defined in the directory section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NyeFileEntry {
    /// The file name.
    pub name: Segments,
    /// The size of the file in bytes.
    pub size: u64,
    /// The type of file.
    pub kind: NyeFileEntryKind,
}

/// A file's artifact type.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum NyeFileEntryKind {
    /// A binary file.
    Bin,
    /// A library file.
    Lib,
    /// An editable text configuration file.
    Etc,
    /// A variable data file.
    Var,
}

impl Display for NyeFileEntryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            NyeFileEntryKind::Bin => write!(f, "bin"),
            NyeFileEntryKind::Lib => write!(f, "lib"),
            NyeFileEntryKind::Etc => write!(f, "etc"),
            NyeFileEntryKind::Var => write!(f, "var"),
        }
    }
}

/// An array of segments.
///
/// Manipulating these is easy. You can convert it to a [`String`] or [`PathBuf`] representation
/// using [`Segments::to_string()`], [`Segments::into()`], and [`Segments::to_path_buf()`].
///
/// To create a new [`Segments`] instance, parse a representable path using
/// [`Segments::from_str()`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Segments(Vec<Segment>);

impl Segments {
    /// Joins the segments into a [`PathBuf`].
    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf::from(self.to_string())
    }

    /// Returns a slice over this path's segments.
    pub fn segments(&self) -> &[Segment] {
        &self.0
    }
}

impl FromStr for Segments {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let str_segments: Vec<_> = s.split("/").collect();
        let mut segments = Vec::with_capacity(str_segments.len());

        for segment in str_segments {
            if segment.is_empty() {
                anyhow::bail!("There is an empty segment in the string. They're invalid.");
            }

            segments.push(Segment::from_str(segment)?);
        }

        Ok(Self(segments))
    }
}

impl TryFrom<&str> for Segments {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl From<Segments> for String {
    fn from(value: Segments) -> Self {
        value.to_string()
    }
}

impl Display for Segments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let segments: Vec<_> = self.0.iter().map(|s| s.to_string()).collect();
        let full = segments.join("/");

        write!(f, "{full}")
    }
}

/// An individual nye file path segment.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Segment(Vec<u8>);

impl Segment {
    /// Converts the segment to a [`PathBuf`]
    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf::from(self.to_string())
    }

    /// Returns the amount of bytes this segment uses.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether the current segment is empty.
    ///
    /// This should never be true, as empty segments are invalid.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl FromStr for Segment {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            anyhow::bail!("Empty segments are invalid.");
        }

        let mut bytes = Vec::with_capacity(s.len());

        'chars: for char in s.chars() {
            for (index, achar) in ALPHABET.chars().enumerate() {
                if char == achar {
                    bytes.push(index as u8);
                    continue 'chars;
                }
            }

            anyhow::bail!(
                "A character in the segment did not correspond to any character in the alphabet."
            );
        }

        Ok(Self(bytes))
    }
}

impl From<Segment> for String {
    fn from(value: Segment) -> Self {
        value.to_string()
    }
}

impl Display for Segment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        'bytes: for byte in &self.0 {
            for (index, char) in ALPHABET.chars().enumerate() {
                if *byte as usize == index {
                    write!(f, "{char}")?;
                    continue 'bytes;
                }
            }

            // The byte was invalid.
            return Err(std::fmt::Error);
        }

        Ok(())
    }
}
