//! Read nye package files from byte streams, such as HTTP request payloads.
//!
//! This module hosts the [`NyeFileStreamableReader`] nye package file reader interface. It requires
//! the underlying source to implement all the bounds required by [`Readable`], which in short is
//! tokio's [`AsyncRead`].
//!
//! This reader is slightly more inconvenient to use than the seekable alternative, as entry
//! contents have to be accessed in order and, in many cases, data has to be read before reaching
//! the next entry, which is likely to be meaningfully slower when entries are big.
//!
//! # Open a File
//!
//! To initialize the reader, use [`NyeFileStreamableReader::open`] like follows:
//!
//! ```
//! use tokio::fs::File;
//!
//! use nye_packages::format::safety::Safety;
//! use nye_packages::format::reading::NyeFileStreamableReader;
//!
//! let file = File::open("package.nye").await?;
//! let reader = NyeFileStreamableReader::open(file, Safety::default()).await?;
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
//! [`NyeFileStreamableReader::manifest()`].
//!
//! The manifest is not currently validated against the directory's contents. Make sure to apply
//! any validation by hand.
//!
//! # Read the Files
//!
//! Given the package file is also called file, files inside it are called entries. These are
//! indexed when the package file is opened and available through a few different methods:
//!
//! * [`NyeFileStreamableReader::get_next_entry()`] - Returns the a wrapper that implements
//!   [`AsyncRead`] over the package file bound to the entry's contents. Reaching the next entry
//!   without finishing reading the current entry's contents will require this method to read and
//!   discard any remaining data of the previous entry before returning. If you only need entry
//!   metadata, without needing to read the entry's contents, use
//!   [`NyeFileStreamableReader::entries()`] or either of the methods below.
//! * [`NyeFileStreamableReader::get_entry_metadata_by_path()`] - Returns the metadata of the file
//!   declared in the package file's directory, queried by file type and name.
//! * [`NyeFileStreamableReader::get_entry_metadata_by_index()`] - Returns the metadata of the file
//!   declared in the package file's directory, queried by index in the package file's directory.
//!
//! Entry metadata is also available in [`NyeFileStreamableReader::entries()`]. However, since
//! internally these entries are indexed, finding specific entries is way cheaper when when using
//! `get_entry_metadata_by_*` methods than when iterating through every entry.
//!
//! ## Initializing [`Segments`]
//!
//! Queries by path don't take a [`String`], rather a [`Segments`]. To initialize it, use
//! [`Segments::from_str`]. For example,
//!
//! ```
//! reader.get_entry_metadata_by_path(NyeFileEntryKind::Bin, Segments::from_str("a/b/c")?);
//! ```
//!
//! # Package File Size Validation
//!
//! Only the bytes declared as used by the directory will be read. The total amount of bytes used is
//! returned by [`NyeFileStreamableReader::size()`]. In most cases, you want to make sure the total
//! package file size matches the returned value exactly, as mismatching sizes mean either
//! incomplete files or files with trailing trash bytes.
//!
//! [`AsyncRead`]: tokio::io::AsyncRead
//! [`AsyncSeek`]: tokio::io::AsyncSeek
//! [`Segments::from_str`]: std::str::FromStr::from_str

use core::task;
use std::io::{self};
use std::pin::Pin;

use anyhow::Context;
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};

use crate::Manifest;
use crate::format::encoding::Encodeable;
use crate::format::reading::{NyeFileHeader, NyeFileReadableEntry, Readable};
use crate::format::safety::Safety;
use crate::format::{NyeFileDirectory, NyeFileEntry, NyeFileEntryKind, NyeFileSignature, Segments};

/// A wrapper over a [`Readable`] object that keeps track of the cursor's position.
#[derive(Debug)]
pub struct ReadWithMeta<T>
where
    T: Readable,
{
    /// The file being read from.
    file: T,
    /// Points to the first unread byte index.
    cursor: u64,
}

impl<T> ReadWithMeta<T>
where
    T: Readable,
{
    /// Initializes a new file wrapper with the cursor set to 0.
    fn new(file: T) -> Self {
        Self { file, cursor: 0 }
    }

    /// Seeks the cursor by reading and discarding.
    ///
    /// Arguments:
    /// * `amount` - The amount of bytes to seek forward (discard).
    async fn seek(&mut self, mut amount: usize) -> anyhow::Result<()> {
        loop {
            if amount == 0 {
                break;
            }

            let mut take = (&mut self.file).take(amount as u64);
            let read = take
                .read(&mut vec![0u8; 8192.min(amount)])
                .await
                .context("Could not read from.")?;

            amount -= read;
        }

        Ok(())
    }
}

impl<T> AsyncRead for ReadWithMeta<T>
where
    T: Readable,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> task::Poll<io::Result<()>> {
        let wrapper = self.get_mut();
        let file = Pin::new(&mut wrapper.file);

        let before = buf.filled().len();
        let result = file.poll_read(cx, buf);
        let after = buf.filled().len();

        wrapper.cursor += (after - before) as u64;
        result
    }
}

/// A nye package file reader.
#[derive(Debug)]
pub struct NyeFileStreamableReader<F>
where
    F: Readable,
{
    /// The file object being read.
    inner: ReadWithMeta<F>,
    /// The next entry to read.
    cursor: usize,
    /// The manifest file in the package file.
    manifest: Manifest,
    /// The package file's signature.
    signature: NyeFileSignature,
    /// The nye file's directory.
    directory: NyeFileDirectory,
}

impl<F> NyeFileStreamableReader<F>
where
    F: Readable,
{
    /// Reads a `.nye` file.
    ///
    /// It parses the file directory before returning. Use [`Safety`] to define how the file will
    /// be parsed and what conditions will cause the file to fail.
    ///
    /// Arguments:
    /// * `file` - The file to read.
    pub async fn open(file: F, safety: Safety) -> anyhow::Result<Self>
    where
        F: Readable,
    {
        let mut file = ReadWithMeta::new(file);

        let header = NyeFileHeader::parse(&mut file, &safety)
            .await
            .context("Could not parse package file's header.")?;

        Ok(Self {
            inner: file,
            cursor: 0,
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

    /// Returns the next entry in the package file, if any.
    pub async fn get_next_entry<'a>(
        &'a mut self,
    ) -> anyhow::Result<Option<NyeFileReadableEntry<'a, ReadWithMeta<F>>>> {
        let Some(entry) = self.directory.entries.get(self.cursor) else {
            return Ok(None);
        };

        if let Some((start, _end)) = self.directory.get_entry_location_by_index(self.cursor) {
            if self.inner.cursor > start {
                anyhow::bail!(concat!(
                    "You've already read past this entry. You cannot go back when using ",
                    "NyeFileStreamableReader. Use NyeFileSeekableReader instead if possible, ",
                    "reorder your access to entries, or cache entry values for later use."
                ));
            }

            let remaining = self.inner.cursor - start;

            self.inner
                .seek(remaining as usize)
                .await
                .context("Could not seek to next file's first byte.")?;

            self.cursor += 1;

            Ok(Some(NyeFileReadableEntry {
                name: &entry.name,
                kind: entry.kind,
                size: entry.size,
                inner: &mut self.inner,
                offset: start,
                cursor: 0,
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod test {
    use tokio::io::{AsyncReadExt, BufReader};

    use super::*;

    #[tokio::test]
    async fn test_nye_file_reader() -> anyhow::Result<()> {
        #[rustfmt::skip]
        let file = BufReader::new(&[
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
        let mut reader = NyeFileStreamableReader::open(file, Safety::default())
            .await
            .context("Could not open file.")?;

        let mut entry = reader
            .get_next_entry()
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
            .get_next_entry()
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
