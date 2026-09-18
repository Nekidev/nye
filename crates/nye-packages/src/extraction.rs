use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Context;
use async_zip::tokio::read::seek::ZipFileReader;
use tokio::io::{AsyncBufRead, AsyncReadExt, AsyncSeek};
use tokio_util::compat::FuturesAsyncReadCompatExt;

pub fn read_paths<Z>(zip: &mut ZipFileReader<Z>) -> anyhow::Result<HashSet<PathBuf>>
where
    Z: AsyncBufRead + AsyncSeek + Unpin,
{
    let mut paths = HashSet::new();

    for index in 0..zip.file().entries().len() {
        let entry = zip.file().entries().get(index).unwrap();
        let filename = entry.filename().clone().into_string().context(concat!(
            "A file in the package file could not have its name converted to a string. Is it ",
            "using weird characters?"
        ))?;
        let filename = PathBuf::from(filename);

        paths.insert(filename);
    }

    Ok(paths)
}

pub async fn read_file_to_string<Z, P>(
    zip: &mut ZipFileReader<Z>,
    path: P,
    max_size: u64,
) -> anyhow::Result<String>
where
    Z: AsyncBufRead + AsyncSeek + Unpin,
    P: AsRef<Path>,
{
    let path = path.as_ref().display().to_string();

    if !zip.file().entries().is_empty() {
        for index in 0..zip.file().entries().len() {
            let entry = zip.file().entries().get(index).unwrap();
            let filename = entry
                .filename()
                .as_str()
                .context("Could not decode package file name.")?;

            if filename == path {
                if entry.uncompressed_size() > max_size {
                    anyhow::bail!(
                        "The file in the package file size was bigger than the size limit set for this file, {max_size} bytes."
                    );
                }

                let reader = zip.reader_without_entry(index).await.context(
                    "Could not get reader for manifest file in package file. This is weird.",
                )?;

                let mut string = String::new();
                reader
                    .compat()
                    .read_to_string(&mut string)
                    .await
                    .context("Could not read nye.toml manifest in package file to string. Are its contents correct?")?;

                return Ok(string);
            }
        }
    }

    anyhow::bail!("There was no such file in the zip file.")
}
