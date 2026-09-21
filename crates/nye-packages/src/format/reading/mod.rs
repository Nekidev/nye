//! Read and inspect package files.
//!
//! # Reading
//!
//! Reading package files can be done using either [`NyeFileSeekableReader`] or
//! [`NyeFileStreamableReader`]. These enable reading files from either a stream of bytes, like an
//! incoming HTTP request's body, or from a seekable source, like a file in the filesystem.
//!
//! They both share the same methods regarding reading a package file's directory and manifest,
//! since these are parsed immediately after opening a file. They, however, differ in how files are
//! accessed.
//!
//! * When reading from a [`NyeFileSeekableReader`], files can be directly accessed via index or
//!   file type + path.
//! * When reading from a [`NyeFileStreamableReader`], files are accessed in an iterator-like
//!   manner, calling [`NyeFileStreamableReader::get_next_entry()`].
//!
//! You'll need an instance of [`Safety`] to open a package file. You can use nye's defaults with
//! [`Safety::default()`]. For more information about it, check the [`safety`] module.
//!
//! The specifics will be described in each reader's module.
//!
//! ## Manifest
//!
//! To get the package's manifest, use `manifest()`. It's parsed when the file is opened, so
//! calling that method is just grabbing a reference.
//!
//! ## Reading Files
//!
//! There are a few different ways to read files inside the package file, and they depend on whether
//! the reader is [`NyeFileSeekableReader`] or [`NyeFileStreamableReader`].
//!
//! ### With [`NyeFileSeekableReader`]
//!
//! There are two methods you can use to read files when the underlying file is seekable.
//!
//! - [`NyeFileSeekableReader::get_entry_by_path`] - Get the file by file type and name.
//! - [`NyeFileSeekableReader::get_entry_by_index`] - Get the file by its index in the directory.
//!
//! Files are indexed when the file is opened, so both are `O(1)`.
//!
//! [`NyeFileSeekableReader::get_entry_by_path()`] requires a [`Segments`] object to be passed as
//! the path argument. The only way to initialize it is via [`Segments::from_str`].
//!
//! ### With [`NyeFileStreamableReader`]
//!
//! Files only need to implement [`Readable`] to be able to be read. The signature, directory, and
//! manifest are parsed the same way than with [`NyeFileSeekableReader`], so methods depending on
//! those 3 elements will be available just fine.
//!
//! Reading is done in an iterator style, calling [`NyeFileStreamableReader::get_next_entry()`].
//!
//! [`Safety`]: super::safety::Safety
//! [`Safety::default()`]: super::safety::Safety::default
//! [`Segments::from_str`]: std::str::FromStr::from_str

use core::task;
use std::fmt::Debug;
use std::io::{self, SeekFrom};
use std::pin::Pin;

use tokio::io::{AsyncRead, AsyncSeek, ReadBuf};

use crate::Manifest;
use crate::format::{NyeFileDirectory, NyeFileEntryKind, NyeFileSignature, Segments};

pub mod seekable;
pub mod streamable;

pub use seekable::NyeFileSeekableReader;
pub use streamable::NyeFileStreamableReader;

/// A supertrait for [`AsyncRead`] + [`Debug`] + [`Unpin`] + `'static`.
pub trait Readable: AsyncRead + Debug + Unpin + 'static {}
impl<T> Readable for T where T: AsyncRead + Debug + Unpin + 'static {}

/// A supertrait for [`Readable`] + [`AsyncSeek`].
pub trait ReadableSeekable: Readable + AsyncSeek {}
impl<T> ReadableSeekable for T where T: Readable + AsyncSeek {}

/// Utility to parse the signature, directory, and manifest sections of a package file using
/// [`Encodeable`](super::Encodeable).
pub(super) struct NyeFileHeader {
    pub signature: NyeFileSignature,
    pub directory: NyeFileDirectory,
    pub manifest: Manifest,
}

/// A readable file entry.
pub struct NyeFileReadableEntry<'a, F>
where
    F: Readable,
{
    /// The file's name/path.
    pub name: &'a Segments,
    /// The file's kind.
    pub kind: NyeFileEntryKind,
    /// The file's size in bytes.
    pub size: u64,
    /// The package file being read from.
    inner: &'a mut F,
    /// The file's first byte index in the inner file.
    offset: u64,
    /// The current cursor. Points to the first unread byte index.
    cursor: u64,
}

impl<'a, F> AsyncRead for NyeFileReadableEntry<'a, F>
where
    F: Readable,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> task::Poll<io::Result<()>> {
        let file = self.get_mut();
        let inner = Pin::new(&mut file.inner);

        if file.cursor >= file.size {
            return task::Poll::Ready(Ok(()));
        }

        let remaining = file.size.saturating_sub(file.cursor);
        let mut limited = buf.take(remaining as usize);

        let result = inner.poll_read(cx, &mut limited);

        if result.is_ready() {
            let filled = limited.filled().len();
            file.cursor += filled as u64;

            // TODO: I have no clue how to make `buf` see the filled part without cloning it. God
            //       bless whoever finds out.
            let slice = limited.filled().to_vec();
            buf.put_slice(&slice);
        }

        result
    }
}

impl<'a, F> AsyncSeek for NyeFileReadableEntry<'a, F>
where
    F: ReadableSeekable,
{
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> io::Result<()> {
        let file = self.get_mut();
        let inner = Pin::new(&mut file.inner);

        let position = match position {
            SeekFrom::Start(position) => SeekFrom::Start(position + file.offset),
            SeekFrom::End(position) => {
                let end = (file.offset + file.size) as i128;

                if (end + position as i128) < 0 {
                    return Err(io::Error::other("Attempted to seek before byte 0."));
                } else {
                    SeekFrom::Start((end + position as i128) as u64)
                }
            }
            SeekFrom::Current(position) => {
                if (file.cursor as i128 + position as i128) < 0 {
                    return Err(io::Error::other("Attempted to seek before byte 0."));
                } else {
                    SeekFrom::Current(position)
                }
            }
        };

        inner.start_seek(position)
    }

    fn poll_complete(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<io::Result<u64>> {
        let file = self.get_mut();
        let inner = Pin::new(&mut file.inner);

        match inner.poll_complete(cx) {
            task::Poll::Pending => task::Poll::Pending,
            task::Poll::Ready(Ok(position)) => task::Poll::Ready(Ok(position - file.offset)),
            task::Poll::Ready(Err(error)) => task::Poll::Ready(Err(error)),
        }
    }
}
