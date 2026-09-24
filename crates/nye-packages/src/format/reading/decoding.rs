//! Parsing for package file contents.

use std::collections::HashMap;

use anyhow::Context;
use tokio::io::AsyncReadExt;

use crate::format::reading::{NyeFileHeader, Readable};
use crate::format::safety::Safety;
use crate::format::{
    ALPHABET, NyeFileDirectory, NyeFileEntry, NyeFileEntryKind, NyeFileSignature, Segment, Segments,
};

/// Implemented by decodeable structs, like [`Segments`] and [`NyeFileDirectory`].
pub trait Decodeable: Sized {
    /// Parses and returns itself.
    ///
    /// This function leaves the file cursor at the first byte after the parsed bytes.
    #[allow(async_fn_in_trait)]
    async fn decode<F>(file: &mut F, safety: &Safety) -> anyhow::Result<Self>
    where
        F: Readable;

    /// Returns the amount of bytes this element occupies in a package file.
    fn size(&self) -> u64;
}

impl Decodeable for NyeFileHeader {
    fn size(&self) -> u64 {
        self.signature.size() + self.directory.size() + self.directory.manifest_size
    }

    async fn decode<F>(file: &mut F, safety: &Safety) -> anyhow::Result<Self>
    where
        F: Readable,
    {
        let signature = NyeFileSignature::decode(file, safety)
            .await
            .context("Could not parse the package file's signature.")?;

        if signature.version != 0 {
            anyhow::bail!(
                "The package file was made using an unsupported version format, v{}.",
                signature.version
            );
        }

        let directory = NyeFileDirectory::decode(file, safety)
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

impl Decodeable for NyeFileSignature {
    fn size(&self) -> u64 {
        4
    }

    async fn decode<F>(file: &mut F, _safety: &Safety) -> anyhow::Result<Self>
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

impl Decodeable for NyeFileDirectory {
    fn size(&self) -> u64 {
        // Start with the manifest's size (8 bytes) and the amount of entries accounted for (2
        // bytes).
        let mut size = 10;

        for entry in &self.entries {
            size += entry.size();
        }

        size
    }

    async fn decode<F>(file: &mut F, safety: &Safety) -> anyhow::Result<Self>
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
            let entry = NyeFileEntry::decode(file, safety)
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

impl Decodeable for NyeFileEntry {
    fn size(&self) -> u64 {
        // 8 bytes for the file size, 1 byte for the file type, and the name's size.
        8 + self.kind.size() + self.name.size()
    }

    async fn decode<F>(file: &mut F, safety: &Safety) -> anyhow::Result<Self>
    where
        F: Readable,
    {
        let size = file
            .read_u64()
            .await
            .context("Could not read file entry size.")?;
        let kind = NyeFileEntryKind::decode(file, safety)
            .await
            .context("Could not read file entry kind.")?;
        let name = Segments::decode(file, safety)
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

impl Decodeable for NyeFileEntryKind {
    fn size(&self) -> u64 {
        1
    }

    async fn decode<F>(file: &mut F, _safety: &Safety) -> anyhow::Result<Self>
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

impl Decodeable for Segments {
    fn size(&self) -> u64 {
        // 1 byte for the amount of segments + the size of each segment.
        let mut size = 1;

        for segment in &self.0 {
            size += segment.size();
        }

        size
    }

    async fn decode<F>(file: &mut F, safety: &Safety) -> anyhow::Result<Self>
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
            let segment = Segment::decode(file, safety)
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

impl Decodeable for Segment {
    fn size(&self) -> u64 {
        // The amount of bytes plus the byte indicating the amount of bytes.
        self.0.len() as u64 + 1
    }

    async fn decode<F>(file: &mut F, _safety: &Safety) -> anyhow::Result<Self>
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
