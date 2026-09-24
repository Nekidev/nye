//! Create and write to package files.
//!
//! Unlike the [`reading`] module, this module does not provide separate seekable/streamable
//! variants. Nye packages are efficient to analyze at the cost of being a pain to write.
//!
//! This module allows writing to a file both from memory and from the filesystem. Writing from the
//! filesystem is way more memory-efficient than doing so from memory, as loading from memory
//! usually requires to load the whole file to memory.
//!
//! # Creating a File
//!
//! Creating the file writer is simple. Initialize a [`NyeFileWriter`] using
//! [`NyeFileWriter::new()`]. You'll need to have your [`Manifest`] built already.
//!
//! ```
//! let mut writer = NyeFileWriter::new(manifest);
//! ```
//!
//! # Inserting Files
//!
//! [`NyeFileWriteableEntry`] allows you write custom file loaders so you don't have to load all
//! data to memory. Since the directory has to be finished before being able to write file contents,
//! the flow is the following:
//!
//! 1. Initialize entries and loaders,
//! 2. Finish the package file definition,
//! 3. Write all file definitions to the package file's directory,
//! 4. Write all file contents to the package file, loading the reader and reading them one by one.
//!
//! Loaders can be either of the following:
//!
//! * A type implementing [`AsyncRead`],
//! * A future returning a type implementing [`AsyncRead`],
//! * A future returning a result holding either a type implementing [`AsyncRead`] or an error
//!   conversible into [`anyhow::Error`],
//! * A function implementing [`FnOnce`] that returns a type implementing [`AsyncRead`],
//! * A function implementing [`FnOnce`] that returns a result holding either a type implementing
//!   [`AsyncRead`] or an error conversible into [`anyhow::Error`],
//! * A function implementing [`AsyncFnOnce`] that returns a type implementing [`AsyncRead`],
//! * A function implementing [`AsyncFnOnce`] that returns a result holding either a type
//!   implementing [`AsyncRead`] or an error conversible into [`anyhow::Error`].
//!
//! This allows you to load file contents dynamically in many different ways. These are some
//! examples:
//!
//! ```
//! let file = File::open("busybox").await?;
//!
//! async fn load_direct() -> File {
//!     File::open("busybox").await.unwrap()
//! }
//! async fn load_result() -> Result<File, io::Error> {
//!     File::open("busybox").await
//! }
//!
//! let entry = NyeFileWriteableEntry::new(NyeFileEntryKind::Bin, "busybox", file)?;
//! let entry = NyeFileWriteableEntry::new(NyeFileEntryKind::Bin, "busybox", move || file)?;
//! let entry = NyeFileWriteableEntry::new(NyeFileEntryKind::Bin, "busybox", File::open("busybox"))?;
//! let entry = NyeFileWriteableEntry::new(NyeFileEntryKind::Bin, "busybox", || File::open("busybox"))?;
//! let entry = NyeFileWriteableEntry::new(NyeFileEntryKind::Bin, "busybox", async || File::open("busybox").await)?;
//! let entry = NyeFileWriteableEntry::new(NyeFileEntryKind::Bin, "busybox", Cursor::new(Vec::new()))?;
//! let entry = NyeFileWriteableEntry::new(NyeFileEntryKind::Bin, "busybox", || Cursor::new(Vec::new()))?;
//! let entry = NyeFileWriteableEntry::new(NyeFileEntryKind::Bin, "busybox", load_direct)?;
//! let entry = NyeFileWriteableEntry::new(NyeFileEntryKind::Bin, "busybox", load_result)?;
//! let entry = NyeFileWriteableEntry::new(NyeFileEntryKind::Bin, "busybox", load_direct())?;
//! let entry = NyeFileWriteableEntry::new(NyeFileEntryKind::Bin, "busybox", load_result())?;
//! ```
//!
//! To add an entry to the writer, use [`NyeFileWriter::insert()`]. It will check for name
//! collisions before writing it. Files will be written to the package file in the order you insert
//! them.
//!
//! # Finishing the Package File
//!
//! Once all entries are inserted and loaders are ready, it's time to write the [`NyeFileWriter`] to
//! a file.
//!
//! You can write the final package file to any type implementing both [`AsyncWrite`] and
//! [`AsyncSeek`].
//! 
//! ```
//! writer.write(File::open("output.nye").await).await?;
//! ```
//!
//! [`reading`]: super::reading
//! [`Manifest`]: crate::Manifest
//! [`AsyncRead`]: tokio::io::AsyncRead

use std::collections::HashSet;
use std::io::SeekFrom;
use std::marker::PhantomData;

use anyhow::Context;
use tokio::io::{AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt};

use crate::format::writing::encoding::Encodeable;
use crate::format::writing::loadable::{Loadable, LoadableWithoutKind, LoadableWrapper};
use crate::format::{NyeFileEntry, NyeFileEntryKind, NyeFileSignature, Segments};
use crate::manifest::Manifest;

pub mod encoding;
pub mod loadable;

/// Supertrait for types implementing [`AsyncWrite`], [`AsyncSeek`], and [`Unpin`].
pub trait Writeable: AsyncWrite + AsyncSeek + Unpin {}
impl<T> Writeable for T where T: AsyncWrite + AsyncSeek + Unpin {}

/// A package file writer.
pub struct NyeFileWriter {
    manifest: Manifest,
    entries: Vec<NyeFileWriteableEntry>,
    segments: HashSet<(NyeFileEntryKind, Segments)>,
}

impl NyeFileWriter {
    pub fn new(manifest: Manifest) -> Self {
        Self {
            manifest,
            entries: Vec::new(),
            segments: HashSet::new(),
        }
    }

    pub fn insert(&mut self, entry: NyeFileWriteableEntry) -> anyhow::Result<()> {
        self.segments
            .insert((entry.kind, entry.name.clone()))
            .ok_or(anyhow::anyhow!(concat!(
                "There's already a file in the directory with this name. Make sure the file ",
                "names you insert are unique per file type."
            )))?;
        self.entries.push(entry);

        Ok(())
    }

    pub async fn write(self, mut output: impl Writeable) -> anyhow::Result<()> {
        let signature = NyeFileSignature { version: 0 };
        let signature_bytes = signature.encode();
        output
            .write_all(&signature_bytes)
            .await
            .context("Could not write signature to file.")?;

        output
            .write_u16(self.entries.len() as u16)
            .await
            .context("Could not write the amount of files in the package file to directory.")?;

        let manifest_bytes =
            toml::to_string_pretty(&self.manifest).context("Could not serialize the manifest.")?;
        output
            .write_u64(manifest_bytes.len() as u64)
            .await
            .context("Could not write manifest length to directory.")?;

        // We can't know beforehand how big the file entry sizes will be (since they're loaded from
        // an [`AsyncRead`] loader that doesn't have `.len()`). Therefore, we'll write the
        // directory with every file set to 0 bytes and overwrite those sizes as we finish writing
        // entries.
        //
        // This vector stores the loaders from `self.entries` in order, each stored next to the
        // offset at which they had their size placeholder written.
        //
        // According to the format, file sizes are a u64.
        let mut offsets_and_loaders = Vec::new();
        for entry in self.entries {
            let meta = NyeFileEntry {
                name: entry.name,
                kind: entry.kind,
                size: 0,
            };
            let bytes = meta.encode();

            let offset = output
                .seek(SeekFrom::Current(0))
                .await
                .context("Could not get current offset for entry in directory.")?;

            output
                .write_all(&bytes)
                .await
                .context("Could not write directory entry placeholder.")?;

            offsets_and_loaders.push((offset, entry.loader));
        }

        output
            .write_all(manifest_bytes.as_bytes())
            .await
            .context("Could not write manifest package file.")?;

        for (offset, loader) in offsets_and_loaders {
            let cursor = output
                .seek(SeekFrom::Current(0))
                .await
                .context("Could not get offset in file before writing file contents.")?;

            let mut loader = loader.load().await.context("Could not load loader.")?;
            let written = tokio::io::copy(&mut loader, &mut output)
                .await
                .context("Could not write entry's contents to package file.")?;

            output
                .seek(SeekFrom::Start(offset))
                .await
                .context("Could not seek to file size offset.")?;
            output
                .write_u64(written)
                .await
                .context("Could not overwrite entry size in directory.")?;
            output
                .seek(SeekFrom::Start(cursor))
                .await
                .context("Could not recover position in file after writing entry size.")?;
        }

        Ok(())
    }
}

pub struct NyeFileWriteableEntry {
    name: Segments,
    kind: NyeFileEntryKind,
    loader: Box<dyn LoadableWithoutKind>,
}

impl NyeFileWriteableEntry {
    pub fn new<S, T, K, E>(kind: NyeFileEntryKind, name: S, loader: T) -> anyhow::Result<Self>
    where
        S: TryInto<Segments, Error = E>,
        T: Loadable<K> + 'static,
        K: 'static,
        E: Into<anyhow::Error> + Send + Sync + 'static,
    {
        let name = name
            .try_into()
            .map_err(Into::into)
            .context("Could not parse the entry's name into a segment.")?;

        let loader = Box::new(LoadableWrapper {
            inner: loader,
            _phantom: PhantomData::<K>,
        });

        Ok(Self { name, kind, loader })
    }
}

#[cfg(test)]
mod test {
    use tokio::fs::File;

    use super::*;

    #[tokio::test]
    async fn test_impl() -> anyhow::Result<()> {
        let file = File::open("busybox").await?;

        NyeFileWriteableEntry::new(NyeFileEntryKind::Bin, "busybox", file.try_clone().await?)?;
        NyeFileWriteableEntry::new(NyeFileEntryKind::Bin, "busybox", move || file)?;
        NyeFileWriteableEntry::new(NyeFileEntryKind::Bin, "busybox", File::open("busybox"))?;
        NyeFileWriteableEntry::new(NyeFileEntryKind::Bin, "busybox", || File::open("busybox"))?;
        NyeFileWriteableEntry::new(NyeFileEntryKind::Bin, "busybox", async || {
            File::open("busybox").await
        })?;

        Ok(())
    }
}
