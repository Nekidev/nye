use std::path::{Path, PathBuf};

use anyhow::Context as AnyhowContext;
use async_zip::tokio::write::ZipFileWriter;
use async_zip::{Compression, ZipEntryBuilder};
use tokio::fs::{self, File};
use tokio_util::compat::FuturesAsyncWriteCompatExt;

use crate::packages;
use crate::projects::TargetOrShared;
use crate::projects::context::Context;
use crate::targets::Target;
use crate::validation::Validate;

pub async fn package(ctx: &Context, target: Target) -> anyhow::Result<PathBuf> {
    ctx.manifest.validate().context(concat!(
        "The project's manifest was not valid. Fix any validity issues before packaging the ",
        "project."
    ))?;

    if !ctx
        .manifest
        .targets
        .contains_key(&TargetOrShared::Target(target))
    {
        anyhow::bail!("The specified target to package is not configured in nye.toml.");
    }

    fs::create_dir_all(ctx.path.join("dist"))
        .await
        .context("Could not ensure dist directory existed in project directory.")?;

    let output_file_path = ctx.get_dist_package_path(target);
    let mut zip = create_zip_file(&output_file_path)
        .await
        .context("Could not create zip file for package.")?;

    let mut paths =
        insert_target_contents_into_zip(ctx, &mut zip, &TargetOrShared::Target(target), &[])
            .await
            .context("Could not insert target's files into zip file.")?;

    if ctx.manifest.targets.contains_key(&TargetOrShared::Shared) {
        paths.extend(
            insert_target_contents_into_zip(ctx, &mut zip, &TargetOrShared::Shared, &paths)
                .await
                .context("Could not write shared files to zip file.")?,
        );
    }

    let manifest = insert_manifest_into_zip(ctx, target, &mut zip)
        .await
        .context("Could not insert nye.toml manifest into package zip file.")?;

    validate_manifest(&manifest, &paths).context(
        "The resulting package was invalid due to an issue with the project's nye.toml maninfest.",
    )?;

    zip.close()
        .await
        .context("Could not close pack zip file.")?;

    Ok(output_file_path)
}

async fn create_zip_file(path: &Path) -> anyhow::Result<ZipFileWriter<File>> {
    let output_file = File::create(&path)
        .await
        .context("Could not create pack file.")?;
    let output_file_zip = ZipFileWriter::with_tokio(output_file);

    Ok(output_file_zip)
}

async fn insert_target_contents_into_zip(
    ctx: &Context,
    zip: &mut ZipFileWriter<File>,
    target_or_shared: &TargetOrShared,
    skip: &[PathBuf],
) -> anyhow::Result<Vec<PathBuf>> {
    let Some(target_config) = ctx.manifest.targets.get(target_or_shared) else {
        anyhow::bail!("The target attempted to be packed was not configured in nye.toml.");
    };
    let target_path = ctx.path.join(&target_config.source);

    let paths = insert_directory_into_zip(zip, &target_path, skip)
        .await
        .context("Could not write directory's contents to zip file.")?;

    Ok(paths)
}

async fn insert_directory_into_zip(
    zip: &mut ZipFileWriter<File>,
    path: &Path,
    skip: &[PathBuf],
) -> anyhow::Result<Vec<PathBuf>> {
    let alan = globwalk::GlobWalkerBuilder::from_patterns(
        path,
        &["bin/**/*", "etc/**/*", "lib/**/*", "var/**/*"],
    )
    .follow_links(false)
    .build()
    .context("Could not initialize glob walker.")?;

    let mut paths = Vec::new();

    for dir_entry in alan {
        let dir_entry = dir_entry.context("An error occurred while finding an entry via glob.")?;

        if dir_entry.file_type().is_file() {
            let path = pathdiff::diff_paths(dir_entry.path(), path)
                .context("Could not get relative path of dir entry.")?;

            if skip.contains(&path) {
                continue;
            }

            let zip_entry =
                ZipEntryBuilder::new(path.display().to_string().into(), Compression::Stored)
                    .build();
            let writer = zip
                .write_entry_stream(zip_entry)
                .await
                .context("Could not insert entry into pack zip.")?;
            let mut writer = writer.compat_write();

            let mut reader = File::open(dir_entry.path())
                .await
                .context("Could not open file to pack.")?;

            tokio::io::copy(&mut reader, &mut writer)
                .await
                .context("Could not write file to zip.")?;

            writer
                .into_inner()
                .close()
                .await
                .context("Could not flush file entry to zip file.")?;

            paths.push(path);
        }
    }

    Ok(paths)
}

async fn insert_manifest_into_zip(
    ctx: &Context,
    target: Target,
    zip: &mut ZipFileWriter<File>,
) -> anyhow::Result<packages::Manifest> {
    let manifest = packages::Manifest::from_project_manifest(ctx.manifest.clone(), target);
    let manifest_string = toml::to_string_pretty(&manifest)
        .context("Could not serialize package manifest to string.")?;

    let zip_entry = ZipEntryBuilder::new("nye.toml".into(), Compression::Stored).build();
    zip.write_entry_whole(zip_entry, manifest_string.as_bytes())
        .await
        .context("Could not insert manifest into package zip.")?;

    Ok(manifest)
}

fn validate_manifest(manifest: &packages::Manifest, paths: &[PathBuf]) -> anyhow::Result<()> {
    for exposed_bin in &manifest.exposes.bin {
        let expected_path = PathBuf::from("bin").join(&exposed_bin.path);
        if !paths.contains(&expected_path) {
            anyhow::bail!(
                "The exposed binary `{}` was expected to be in package at `{}`, but that path did not exist.",
                exposed_bin.link,
                exposed_bin.path.display()
            );
        }
    }

    Ok(())
}
