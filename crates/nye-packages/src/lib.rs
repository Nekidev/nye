pub mod analysis;
pub mod extraction;
pub mod manifest;
pub mod validation;

use std::path::{Path, PathBuf};

pub use analysis::*;
use anyhow::Context;
use async_zip::tokio::write::ZipFileWriter;
use async_zip::{Compression, ZipEntryBuilder};
pub use manifest::*;
use tokio::fs::File;
use tokio_util::compat::FuturesAsyncWriteCompatExt;

pub struct PackageWriter {
    zip: ZipFileWriter<File>,
    manifest: Manifest,
}

impl PackageWriter {
    /// Creates a new empty package file.
    ///
    /// Arguments:
    /// * `path` - The path where to write the file.
    /// * `meta` - The package's metadata.
    pub async fn create_new(path: impl AsRef<Path>, meta: ManifestPackage) -> anyhow::Result<Self> {
        let file = File::create_new(path)
            .await
            .context("Could not create a new package file. Did it already exist?")?;

        let manifest = Manifest {
            package: meta,
            exposes: ManifestExposes::default(),
            consumes: ManifestConsumes::default(),
        };

        Ok(Self {
            zip: ZipFileWriter::with_tokio(file),
            manifest,
        })
    }

    /// Insert a binary file into the package.
    ///
    /// Arguments:
    /// * `in_path` - The path to the file to the insert.
    /// * `out_path` - The path the binary will be inserted as, without the `bin/` prefix.
    /// * `links` - The links the binary will be exposed as, if any.
    pub async fn insert_bin(
        &mut self,
        in_path: impl AsRef<Path>,
        out_path: impl AsRef<Path>,
        links: Vec<String>,
    ) -> anyhow::Result<()> {
        let entry = ZipEntryBuilder::new(
            PathBuf::from("bin")
                .join(&out_path)
                .to_str()
                .context("The specified filename could not be converted to a string.")?
                .into(),
            Compression::Stored,
        )
        .build();

        let mut file = File::open(&in_path)
            .await
            .context("Could not open specified file.")?;
        let mut stream = self
            .zip
            .write_entry_stream(entry)
            .await
            .context("Could not create entry write stream for file.")?
            .compat_write();

        tokio::io::copy(&mut file, &mut stream)
            .await
            .context("Could not write file into package file.")?;

        stream
            .into_inner()
            .close()
            .await
            .context("Could not finish writing file to package file.")?;

        for link in links {
            self.manifest.exposes.bin.push(ManifestExposesArtifact {
                link,
                path: out_path.as_ref().into(),
            });
        }

        Ok(())
    }

    /// Writes the manifest into the package file.
    async fn insert_manifest(&mut self) -> anyhow::Result<()> {
        let entry = ZipEntryBuilder::new("nye.toml".into(), Compression::Stored).build();

        let manifest = toml::to_string_pretty(&self.manifest)
            .context("Could not serialize the manifest into TOML.")?;

        self.zip
            .write_entry_whole(entry, manifest.as_bytes())
            .await
            .context("Could not write the manifest into the package file.")?;

        Ok(())
    }

    /// Writes the manifest to the file and closes it.
    pub async fn finish(mut self) -> anyhow::Result<()> {
        self.insert_manifest()
            .await
            .context("Could not insert manifest to package file.")?;

        self.zip
            .close()
            .await
            .context("Could not close the zip file.")?;

        Ok(())
    }
}
