use std::path::{Path, PathBuf};

use anyhow::Context;
use async_zip::StoredZipEntry;
use async_zip::tokio::read::seek::ZipFileReader;
use tokio::fs::File;
use tokio::io::{AsyncBufRead, AsyncSeek, BufReader};

use crate::manifest::Manifest;
use crate::{extraction, validation};

#[derive(Clone)]
pub struct Package {
    pub manifest: Manifest,
    pub contents: Vec<PackageFile>,
}

#[derive(Clone)]
pub struct PackageFile {
    pub name: String,
    pub link: Option<String>,
    pub kind: PackageFileKind,
    pub size: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PackageFileKind {
    Manifest,
    Bin,
    Lib,
    Etc,
    Var,
}

pub async fn analyze(path: impl AsRef<Path>) -> anyhow::Result<Package> {
    let file = File::open(path)
        .await
        .context("Could not open file at the specified path.")?;
    let mut zip = ZipFileReader::with_tokio(BufReader::new(file))
        .await
        .context("Could not read zip file in path.")?;

    let manifest_string = extraction::read_file_to_string(&mut zip, "nye.toml", 1024 * 1024)
        .await
        .context("Could not read manifest file in package file.")?;
    let manifest: Manifest = toml::from_str(&manifest_string)
        .context("Could not deserialize the manifest file in the package file.")?;

    validation::validate(&mut zip, &manifest).context("The package file was invalid.")?;

    let contents = get_files(&zip, &manifest).context("Could not get package file contents.")?;

    Ok(Package { manifest, contents })
}

fn get_files<T>(zip: &ZipFileReader<T>, manifest: &Manifest) -> anyhow::Result<Vec<PackageFile>>
where
    T: AsyncBufRead + AsyncSeek + Unpin,
{
    let mut files = Vec::with_capacity(zip.file().entries().len());

    for index in 0..zip.file().entries().len() {
        let entry = zip.file().entries().get(index).unwrap();
        let filename = entry.filename().clone().into_string().context(concat!(
            "A file in the package file could not have its name converted to a string. Is it ",
            "using weird characters?"
        ))?;

        if filename == "nye.toml" {
            files.push(get_manifest_file(entry));
            continue;
        }
        if filename.starts_with("bin/") {
            files.push(get_bin_file(filename, entry, manifest));
            continue;
        }
        if filename.starts_with("lib/") {
            files.push(get_lib_file(filename, entry, manifest));
            continue;
        }
        if filename.starts_with("etc/") {
            files.push(get_etc_file(filename, entry));
            continue;
        }
        if filename.starts_with("var/") {
            files.push(get_var_file(filename, entry));
            continue;
        }

        anyhow::bail!("The file `{filename}` is out of place.");
    }

    Ok(files)
}

fn get_manifest_file(entry: &StoredZipEntry) -> PackageFile {
    PackageFile {
        name: String::from("nye.toml"),
        link: None,
        kind: PackageFileKind::Manifest,
        size: entry.uncompressed_size(),
    }
}

fn get_bin_file(filename: String, entry: &StoredZipEntry, manifest: &Manifest) -> PackageFile {
    let mut link = None;

    for bin in &manifest.exposes.bin {
        if PathBuf::from("bin").join(&bin.path) == filename {
            link = Some(bin.link.clone());
        }
    }

    PackageFile {
        name: filename,
        link,
        kind: PackageFileKind::Bin,
        size: entry.uncompressed_size(),
    }
}

fn get_lib_file(filename: String, entry: &StoredZipEntry, manifest: &Manifest) -> PackageFile {
    let mut link = None;

    for lib in &manifest.exposes.lib {
        if PathBuf::from("lib").join(&lib.path) == filename {
            link = Some(lib.link.clone());
        }
    }

    PackageFile {
        name: filename,
        link,
        kind: PackageFileKind::Lib,
        size: entry.uncompressed_size(),
    }
}

fn get_etc_file(filename: String, entry: &StoredZipEntry) -> PackageFile {
    PackageFile {
        name: filename,
        link: None,
        kind: PackageFileKind::Etc,
        size: entry.uncompressed_size(),
    }
}

fn get_var_file(filename: String, entry: &StoredZipEntry) -> PackageFile {
    PackageFile {
        name: filename,
        link: None,
        kind: PackageFileKind::Lib,
        size: entry.uncompressed_size(),
    }
}
