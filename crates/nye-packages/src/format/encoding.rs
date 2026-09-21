//! Encoding and decoding of package file contents.
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

use std::collections::HashMap;

use anyhow::Context;
use tokio::io::AsyncReadExt;

use crate::format::reading::{NyeFileHeader, Readable};
use crate::format::safety::Safety;
use crate::format::{
    NyeFileDirectory, NyeFileEntry, NyeFileEntryKind, NyeFileSignature, Segment, Segments,
};

/// Nye's segments type alphabet.
pub const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.,@";

/// Implemented by encodeable/decodeable structs, like [`Segments`] and [`NyeFileDirectory`].
pub trait Encodeable: Sized {
    /// Parses and returns itself.
    ///
    /// This function leaves the file cursor at the first byte after the parsed bytes.
    #[allow(async_fn_in_trait)]
    async fn parse<F>(file: &mut F, safety: &Safety) -> anyhow::Result<Self>
    where
        F: Readable;

    /// Returns the amount of bytes this element occupies in a package file.
    fn size(&self) -> u64;
}

impl Encodeable for NyeFileHeader {
    fn size(&self) -> u64 {
        self.signature.size() + self.directory.size() + self.directory.manifest_size
    }

    async fn parse<F>(file: &mut F, safety: &Safety) -> anyhow::Result<Self>
    where
        F: Readable,
    {
        let signature = NyeFileSignature::parse(file, safety)
            .await
            .context("Could not parse the package file's signature.")?;

        if signature.version != 0 {
            anyhow::bail!(
                "The package file was made using an unsupported version format, v{}.",
                signature.version
            );
        }

        let directory = NyeFileDirectory::parse(file, safety)
            .await
            .context("Could not parse the package file's directory.")?;

        let mut manifest_buffer = vec![0u8; directory.manifest_size as usize];
        file.read_exact(&mut manifest_buffer)
            .await
            .context("Could not read manifest from package file.")?;
        let manifest = toml::from_slice(&manifest_buffer)
            .context("The manifest in the package file was invalid.")?;

        Ok(Self {
            signature,
            directory,
            manifest,
        })
    }
}

impl Encodeable for NyeFileSignature {
    fn size(&self) -> u64 {
        4
    }

    async fn parse<F>(file: &mut F, _safety: &Safety) -> anyhow::Result<Self>
    where
        F: Readable,
    {
        let mut signature = [0u8; 4];
        file.read_exact(&mut signature)
            .await
            .context("Could not read the first 4 bytes of the file.")?;

        if &signature[0..3] != "nye".as_bytes() {
            anyhow::bail!("The package file did not start with a nye package file signature.");
        }

        Ok(Self {
            version: signature[3],
        })
    }
}

impl Encodeable for NyeFileDirectory {
    fn size(&self) -> u64 {
        // Start with the manifest's size (8 bytes) and the amount of entries accounted for (2
        // bytes).
        let mut size = 10;

        for entry in &self.entries {
            size += entry.size();
        }

        size
    }

    async fn parse<F>(file: &mut F, safety: &Safety) -> anyhow::Result<Self>
    where
        F: Readable,
    {
        let entries_count = file
            .read_u16()
            .await
            .context("Could not read amount of entries in nye package file.")?;
        let manifest_size = file
            .read_u64()
            .await
            .context("Could not read the manifest's file size.")?;

        if manifest_size > safety.max_manifest_size {
            anyhow::bail!(
                "The manifest is bigger than the max allowed size. The manifest is {} bytes while the max allowed is {} bytes.",
                manifest_size,
                safety.max_manifest_size,
            );
        }

        if entries_count > safety.max_files {
            anyhow::bail!(
                "The nye package file has more declared entries than allowed by the safety rules (has {} files, max is {}).",
                entries_count,
                safety.max_files
            );
        }

        let mut directory = NyeFileDirectory {
            manifest_size,
            index: HashMap::with_capacity(entries_count as usize),
            entries: Vec::with_capacity(entries_count as usize),
        };

        for i in 0..entries_count {
            let entry = NyeFileEntry::parse(file, safety)
                .await
                .context("An entry in the directory was invalid.")?;
            let kind = entry.kind;

            if directory
                .index
                .insert((kind, entry.name.clone()), i as usize)
                .is_some()
            {
                anyhow::bail!(
                    "The file `{}/{}` is duplicate in the package file.",
                    kind,
                    entry.name
                );
            }

            directory.entries.push(entry);
        }

        Ok(directory)
    }
}

impl Encodeable for NyeFileEntry {
    fn size(&self) -> u64 {
        // 8 bytes for the file size, 1 byte for the file type, and the name's size.
        8 + self.kind.size() + self.name.size()
    }

    async fn parse<F>(file: &mut F, safety: &Safety) -> anyhow::Result<Self>
    where
        F: Readable,
    {
        let size = file
            .read_u64()
            .await
            .context("Could not read file entry size.")?;
        let kind = NyeFileEntryKind::parse(file, safety)
            .await
            .context("Could not read file entry kind.")?;
        let name = Segments::parse(file, safety)
            .await
            .context("A file's name was invalid.")?;

        if size > safety.max_file_size {
            anyhow::bail!(
                "The file `{kind}/{name}` was bigger than the max allowed size (file is {} bytes, max is {}).",
                size,
                safety.max_file_size
            );
        }

        Ok(NyeFileEntry { name, size, kind })
    }
}

impl Encodeable for NyeFileEntryKind {
    fn size(&self) -> u64 {
        1
    }

    async fn parse<F>(file: &mut F, _safety: &Safety) -> anyhow::Result<Self>
    where
        F: Readable,
    {
        let byte = file
            .read_u8()
            .await
            .context("Could not read the file type byte.")?;

        match byte {
            0 => Ok(NyeFileEntryKind::Bin),
            1 => Ok(NyeFileEntryKind::Lib),
            2 => Ok(NyeFileEntryKind::Etc),
            3 => Ok(NyeFileEntryKind::Var),
            _ => anyhow::bail!("The specified file type, {byte}, is not supported."),
        }
    }
}

impl Encodeable for Segments {
    fn size(&self) -> u64 {
        // 1 byte for the amount of segments + the size of each segment.
        let mut size = 1;

        for segment in &self.0 {
            size += segment.size();
        }

        size
    }

    async fn parse<F>(file: &mut F, safety: &Safety) -> anyhow::Result<Self>
    where
        F: Readable,
    {
        let segments_count_minus_one = file
            .read_u8()
            .await
            .context("Could not read amount of segments.")?;
        let mut segments = Vec::with_capacity(segments_count_minus_one as usize + 1);
        let mut segments_len = 0;

        for _ in 0..=segments_count_minus_one {
            let segment = Segment::parse(file, safety)
                .await
                .context("A segment was invalid.")?;

            segments_len += segment.len();
            if segments_len > safety.max_file_name_size as usize {
                anyhow::bail!(
                    "The file name `{}` is longer than the max allowed file name size ({} bytes).",
                    Segments(segments),
                    safety.max_file_name_size
                );
            }

            segments.push(segment);
        }

        Ok(Self(segments))
    }
}

impl Encodeable for Segment {
    fn size(&self) -> u64 {
        // The amount of bytes plus the byte indicating the amount of bytes.
        self.0.len() as u64 + 1
    }

    async fn parse<F>(file: &mut F, _safety: &Safety) -> anyhow::Result<Self>
    where
        F: Readable,
    {
        let length_minus_one = file
            .read_u8()
            .await
            .context("Could not read segment length.")?;
        let mut bytes = vec![0u8; length_minus_one as usize + 1];

        file.read_exact(&mut bytes)
            .await
            .context("Could not read segment bytes.")?;

        for byte in &bytes {
            if *byte as usize > ALPHABET.len() {
                anyhow::bail!("A byte in the segment, {byte}, is not in the alphabet.");
            }
        }

        if bytes == ".".as_bytes() {
            anyhow::bail!("Segment is `.`, which is not allowed.");
        }
        if bytes == "..".as_bytes() {
            anyhow::bail!("Segment is `..`, which is not allowed.");
        }

        Ok(Self(bytes))
    }
}
