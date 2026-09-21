//! Read nye package files from seekable sources, like local files.
//!
//! This module hosts the [`NyeFileSeekableReader`] nye package file reader interface. It requires
//! the underlying source to implement all the bounds required by [`ReadableSeekable`], which in
//! short are tokio's [`AsyncRead`] and [`AsyncSeek`].
//!
//! # Open a File
//!
//! To initialize the reader, use [`NyeFileSeekableReader::open`] like follows:
//!
//! ```
//! use tokio::fs::File;
//!
//! use nye_packages::format::safety::Safety;
//! use nye_packages::format::reading::NyeFileSeekableReader;
//!
//! let file = File::open("package.nye").await?;
//! let reader = NyeFileSeekableReader::open(file, Safety::default()).await?;
//! ```
//!
//! Opening a file with `open()` will automatically index the file's directory and read the
//! manifest.
//!
//! [`Safety`] specifies the limits that prevent from malicious files to use too many resources. It,
//! among other rules, allows you to specify max file sizes, max file name sizes, and max entries.
//!
//! # Read the Manifest
//!
//! The manifest is read when `open()` is called. You can access a reference to it using
//! [`NyeFileSeekableReader::manifest()`].
//!
//! The manifest is not currently validated against the directory's contents. Make sure to apply
//! any validation by hand.
//!
//! # Read the Files
//!
//! Given the package file is also called file, files inside it are called entries. These are
//! indexed when the package file is opened and available through a few different methods:
//!
//! * [`NyeFileSeekableReader::get_entry_by_path()`] - Returns a wrapper over the package file that
//!   implements [`AsyncRead`] and [`AsyncSeek`] bound to the file's contents, queried by entry path
//!   and type. Since it wraps and seeks the underlying file, it's more expensive than its
//!   metadata-only counterpart.
//! * [`NyeFileSeekableReader::get_entry_by_index()`] - Returns a wrapper over the package file that
//!   implements [`AsyncRead`] and [`AsyncSeek`] bound to the file's contents, queried by entry
//!   index in the directory. Since it wraps and seeks the underlying file, it's more expensive than
//!   its metadata-only counterpart.
//! * [`NyeFileSeekableReader::get_entry_metadata_by_path()`] - Returns the metadata of the file
//!   declared in the package file's directory, queried by file type and name.
//! * [`NyeFileSeekableReader::get_entry_metadata_by_index()`] - Returns the metadata of the file
//!   declared in the package file's directory, queried by index in the package file's directory.
//!
//! Entry metadata is also available in [`NyeFileSeekableReader::entries()`]. However, since
//! internally these entries are indexed, finding specific entries is way cheaper when when using
//! `get_entry_metadata_by_*` methods than when iterating through every entry.
//!
//! ## Iterating Through Entries
//!
//! To iterate through entries to read their contents, you'll have an easier day using the following
//! pattern:
//!
//! ```
//! let mut i = 0;
//! while let Some(entry) = reader.get_entry_by_index(i) {
//!     // ...
//! }
//! ```
//!
//! ## Initializing [`Segments`]
//!
//! Queries by path don't take a [`String`], rather a [`Segments`]. To initialize it, use
//! [`Segments::from_str`]. For example,
//!
//! ```
//! reader.get_entry_by_path(NyeFileEntryKind::Bin, Segments::from_str("a/b/c")?);
//! ```
//!
//! # Package File Size Validation
//!
//! Only the bytes declared as used by the directory will be read. The total amount of bytes used is
//! returned by [`NyeFileSeekableReader::size()`]. In most cases, you want to make sure the total
//! package file size matches the returned value exactly, as mismatching sizes mean either
//! incomplete files or files with trailing trash bytes.
//!
//! [`AsyncRead`]: tokio::io::AsyncRead
//! [`AsyncSeek`]: tokio::io::AsyncSeek
//! [`Segments::from_str`]: std::str::FromStr::from_str

use std::fmt::Debug;
use std::io::SeekFrom;

use anyhow::Context;
use tokio::io::AsyncSeekExt;

use crate::Manifest;
use crate::format::encoding::Encodeable;
use crate::format::reading::{NyeFileHeader, NyeFileReadableEntry, ReadableSeekable};
use crate::format::safety::Safety;
use crate::format::{NyeFileDirectory, NyeFileEntry, NyeFileEntryKind, NyeFileSignature, Segments};

/// A nye package file reader.
#[derive(Debug)]
pub struct NyeFileSeekableReader<F>
where
    F: ReadableSeekable,
{
    /// The file object being read.
    inner: F,
    /// The manifest file in the package file.
    manifest: Manifest,
    /// The package file's signature.
    signature: NyeFileSignature,
    /// The nye file's directory.
    directory: NyeFileDirectory,
}

impl<F> NyeFileSeekableReader<F>
where
    F: ReadableSeekable,
{
    /// Reads a `.nye` file.
    ///
    /// It parses the file directory before returning. Use [`Safety`] to define how the file will
    /// be parsed and what conditions will cause the file to fail.
    ///
    /// Arguments:
    /// * `file` - The file to read.
    pub async fn open(mut file: F, safety: Safety) -> anyhow::Result<Self>
    where
        F: ReadableSeekable,
    {
        let header = NyeFileHeader::parse(&mut file, &safety)
            .await
            .context("Could not parse package file's header.")?;

        Ok(Self {
            inner: file,
            manifest: header.manifest,
            signature: header.signature,
            directory: header.directory,
        })
    }

    /// The usable part of the file.
    ///
    /// In many cases, you'll want to reject any package files that are not exactly this size,
    /// since it means they contain unused bytes.
    ///
    /// Returns:
    /// [`u64`] - The usable part of the file, in bytes.
    pub fn size(&self) -> u64 {
        let mut size = self.signature.size() + self.directory.size() + self.directory.manifest_size;

        for entry in self.entries() {
            size += entry.size;
        }

        size
    }

    /// Returns the package file format version this file was encoded with.
    pub fn version(&self) -> u8 {
        self.signature.version
    }

    /// Returns the metadata about each file, in order.
    ///
    /// These files don't contain the manifest. To read the manifest, use
    /// [`NyeFileSeekableReader::manifest()`] instead.
    ///
    /// In case you need to find a specific entry's metadata, check out the following methods
    /// instead:
    ///
    /// * [`NyeFileSeekableReader::get_entry_metadata_by_path`]
    /// * [`NyeFileSeekableReader::get_entry_metadata_by_index`]
    pub fn entries(&self) -> &[NyeFileEntry] {
        &self.directory.entries
    }

    /// Returns the package file's manifest.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Get an entry's metadata by its type and path, without a reader.
    pub async fn get_entry_metadata_by_path(
        &self,
        kind: NyeFileEntryKind,
        path: impl Into<Segments>,
    ) -> Option<&NyeFileEntry> {
        let key = (kind, path.into());

        self.directory
            .index
            .get(&key)
            .and_then(|i| self.directory.entries.get(*i))
    }

    /// Get an entry's metadata by its index in the directory, without a reader.
    pub async fn get_entry_metadata_by_index(&self, index: usize) -> Option<&NyeFileEntry> {
        self.directory.entries.get(index)
    }

    /// Returns a readable file entry by its kind and path.
    pub async fn get_entry_by_path<'a>(
        &'a mut self,
        kind: NyeFileEntryKind,
        path: impl Into<Segments>,
    ) -> anyhow::Result<Option<NyeFileReadableEntry<'a, F>>> {
        let key = (kind, path.into());

        if let Some(index) = self.directory.index.get(&key) {
            let entry = self
                .directory
                .entries
                .get(*index)
                .context("The index was corrupted. This is a bug.")?;
            let offset = self
                .directory
                .get_entry_location_by_index(*index)
                .context("The index was corrupted. This is a bug.")?
                .0;

            self.inner
                .seek(SeekFrom::Start(offset))
                .await
                .context("Could not seek inner file.")?;

            Ok(Some(NyeFileReadableEntry {
                name: &entry.name,
                kind: entry.kind,
                size: entry.size,
                inner: &mut self.inner,
                offset,
                cursor: 0,
            }))
        } else {
            Ok(None)
        }
    }

    /// Returns a readable file entry by its index in the directory.
    pub async fn get_entry_by_index<'a>(
        &'a mut self,
        index: usize,
    ) -> anyhow::Result<Option<NyeFileReadableEntry<'a, F>>> {
        if let Some(entry) = self.directory.entries.get(index) {
            let offset = self
                .directory
                .get_entry_location_by_index(index)
                .context("The index was corrupted. This is a bug.")?
                .0;

            self.inner
                .seek(SeekFrom::Start(offset))
                .await
                .context("Could not seek inner file.")?;

            Ok(Some(NyeFileReadableEntry {
                name: &entry.name,
                kind: entry.kind,
                size: entry.size,
                inner: &mut self.inner,
                offset,
                cursor: 0,
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;
    use std::str::FromStr;

    use tokio::io::AsyncReadExt;

    use super::*;

    #[tokio::test]
    async fn test_nye_file_reader() -> anyhow::Result<()> {
        #[rustfmt::skip]
        let file = Cursor::new(&[
            // Signature
            110, 121, 101, 0,

            // Directory
            0, 2,                     // Amount of entries
            0, 0, 0, 0, 0, 0, 0, 68,  // Manifest size
            
                // First entry
                0, 0, 0, 0, 0, 0, 0, 3,  // File size
                0,                       // File type
                0, 0, 0,                 // Single-segment name, "a".

                // Second entry
                0, 0, 0, 0, 0, 0, 0, 6,  // File size
                0,                       // File type
                1, 0, 1, 0, 2,           // Double-segment name, "b/c".

            // Manifest's contents
            91, 112, 97, 99, 107, 97, 103, 101, 93, 10, 110, 97, 109, 101, 32, 61, 32, 34, 101,
            120, 97, 109, 112, 108, 101, 34, 10, 118, 101, 114, 115, 105, 111, 110, 32, 61, 32,
            34, 49, 46, 48, 46, 48, 34, 10, 116, 97, 114, 103, 101, 116, 32, 61, 32, 34, 108,
            105, 110, 117, 120, 45, 120, 56, 54, 95, 54, 52, 34,

            // First entry's contents
            0, 1, 2,

            // Second entry's contents
            3, 4, 5, 6, 7, 8
        ][..]);
        let mut reader = NyeFileSeekableReader::open(file, Safety::default())
            .await
            .context("Could not open file.")?;

        let mut entry = reader
            .get_entry_by_path(NyeFileEntryKind::Bin, Segments::from_str("a")?)
            .await
            .context("Could not get entry 1.")?
            .context("Entry 1 was not found.")?;
        let mut buffer = Vec::new();
        entry
            .read_to_end(&mut buffer)
            .await
            .context("Could not read first entry's bytes.")?;
        assert_eq!(buffer, Vec::from([0, 1, 2]));

        let mut entry = reader
            .get_entry_by_path(NyeFileEntryKind::Bin, Segments::from_str("b/c")?)
            .await
            .context("Could not get entry 2.")?
            .context("Entry 2 was not found.")?;
        let mut buffer = Vec::new();
        entry
            .read_to_end(&mut buffer)
            .await
            .context("Could not read second entry's bytes.")?;
        assert_eq!(buffer, Vec::from([3, 4, 5, 6, 7, 8]));

        Ok(())
    }
}
