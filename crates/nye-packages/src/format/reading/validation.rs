//! Validate a package file's manifest against the file's contents.

use std::collections::HashSet;
use std::str::FromStr;

use anyhow::Context;
use nye_schemas::targets::Target;

use crate::format::reading::safety::Safety;
use crate::format::{NyeFileDirectory, NyeFileEntryKind, Segments};
use crate::manifest::{Manifest, ManifestConsumesEnv, ManifestExposesArtifact};

/// Validates a package file manifest against a package file directory.
///
/// Arguments:
/// * `manifest` - The package file's manifest.
/// * `directory` - The package file's directory.
/// * `safety` - The safety rules and limits.
pub fn validate(
    manifest: &Manifest,
    directory: &NyeFileDirectory,
    safety: &Safety,
) -> anyhow::Result<()> {
    validate_package_meta(manifest, safety)
        .context("The package's metadata was incorrectly configured.")?;
    validate_package_exposes(manifest, directory, safety)
        .context("The package's exposed artifacts were incorrectly configured.")?;
    validate_package_consumes(manifest, safety)
        .context("The package's consumed artifacts were incorrectly configured.")?;

    Ok(())
}

fn validate_package_meta(manifest: &Manifest, safety: &Safety) -> anyhow::Result<()> {
    if manifest.package.name.len() as u64 > safety.max_package_name_size {
        anyhow::bail!("The package's name was longer than {} bytes.", safety.max_package_name_size);
    }
    if manifest.package.name.is_empty() {
        anyhow::bail!("The package had no name (empty string).");
    }

    nye_validation::is_kebab_case(&manifest.package.name)
        .context("The package's name was not kebab case.")?;

    if manifest.package.version.to_string().len() as u64 > safety.max_package_version_size {
        anyhow::bail!("The package's version string was longer than the max allowed.");
    }

    if !manifest.package.target.is_supported() && safety.only_supported_targets {
        anyhow::bail!(
            "The package's target, {}, is not supported by nye.",
            manifest.package.target
        );
    }

    let current_target = Target::get_current()?;
    if safety.only_current_target && current_target != manifest.package.target {
        anyhow::bail!(
            "Only the current system's target, {}, is allowed, but this package was made for {}.",
            current_target,
            manifest.package.target
        );
    }

    Ok(())
}

fn validate_package_exposes(
    manifest: &Manifest,
    directory: &NyeFileDirectory,
    safety: &Safety,
) -> anyhow::Result<()> {
    if manifest.exposes.bin.len() as u64 > safety.max_exposed_bins {
        anyhow::bail!("The manifest exposed more binaries than allowed.");
    }
    if manifest.exposes.lib.len() as u64 > safety.max_exposed_libs {
        anyhow::bail!("The manifest exposed more libraries than allowed.");
    }

    validate_exposed_artifacts(&manifest.exposes.bin, NyeFileEntryKind::Bin, directory, safety)
        .context("One or more exposed binaries were misconfigured.")?;
    validate_exposed_artifacts(&manifest.exposes.lib, NyeFileEntryKind::Lib, directory, safety)
        .context("One or more exposed binaries were misconfigured.")?;

    Ok(())
}

fn validate_exposed_artifacts(
    artifacts: &[ManifestExposesArtifact],
    artifact_kind: NyeFileEntryKind,
    directory: &NyeFileDirectory,
    safety: &Safety,
) -> anyhow::Result<()> {
    let mut names = HashSet::with_capacity(artifacts.len());
    for artifact in artifacts {
        validate_exposed_artifact(artifact, artifact_kind, directory, safety)
            .context("An exposed artifact was misconfigured.")?;

        let is_new = names.insert(&artifact.link);
        if !is_new {
            anyhow::bail!("The {artifact_kind} {} is declared twice.", artifact.link);
        }
    }

    Ok(())
}

fn validate_exposed_artifact(
    artifact: &ManifestExposesArtifact,
    artifact_kind: NyeFileEntryKind,
    directory: &NyeFileDirectory,
    safety: &Safety,
) -> anyhow::Result<()> {
    let path = artifact
        .path
        .to_str()
        .context("The exposed artifact's path could not be interpreted as a string.")?;

    if path.len() > safety.max_file_name_size as usize {
        anyhow::bail!("The exposed artifact's path was longer than allowed.");
    }
    if path.is_empty() {
        anyhow::bail!("The exposed artifact's path was empty.");
    }
    if artifact.link.len() > safety.max_link_size as usize {
        anyhow::bail!("The exposed artifact's link was longer than allowed.");
    }
    if artifact.link.is_empty() {
        anyhow::bail!("The exposed artifact's link was empty.");
    }

    nye_validation::is_safe_path(&artifact.path)
        .context("The exposed artifact's path was unsafe.")?;
    nye_validation::is_safe_path_component(&artifact.link)
        .context("The exposed artifact's link was unsafe or invalid.")?;

    let entry = directory.get_entry_by_path(
        artifact_kind,
        Segments::from_str(path).context(concat!(
            "The path specified by the exposed artifact could not be converted to its segments ",
            "representation."
        ))?,
    );

    if entry.is_none() {
        anyhow::bail!(
            "The exposed artifact's path did not point to an existing entry in the package file."
        );
    }

    Ok(())
}

fn validate_package_consumes(manifest: &Manifest, safety: &Safety) -> anyhow::Result<()> {
    validate_package_consumes_env(manifest, safety)
        .context("The package's consumed env vars were invalid.")?;
    validate_package_consumes_box(manifest, safety)
        .context("The package's consumed boxes were invalid.")?;

    Ok(())
}

fn validate_package_consumes_env(manifest: &Manifest, safety: &Safety) -> anyhow::Result<()> {
    for var in &manifest.consumes.env {
        if var.name().is_empty() {
            anyhow::bail!("A consumed environment variable's name was empty.");
        }
        if var.name().len() as u64 > safety.max_var_name_size {
            anyhow::bail!("A consumed environment varibale's name was longer than allowed.");
        }
        nye_validation::is_env_var_name(var.name())
            .context("A consumed environment variable did not have a valid name.")?;

        if var.name().to_ascii_lowercase().starts_with("nye") {
            anyhow::bail!("Consumed environment variable names cannot start with NYE.");
        }

        match &var {
            ManifestConsumesEnv::List { name, separator } => {
                if separator.len() as u64 > safety.max_var_separator_size {
                    anyhow::bail!(
                        "A consumed environment variable, {name}, had its separator longer than allowed."
                    );
                }
            }
            ManifestConsumesEnv::Value { name, value } => {
                if value.len() as u64 > safety.max_var_value_size {
                    anyhow::bail!(
                        "A consumed environment variable, {name}, had its value longer than allowed."
                    );
                }
            }
        }
    }

    Ok(())
}

fn validate_package_consumes_box(manifest: &Manifest, safety: &Safety) -> anyhow::Result<()> {
    for r#box in &manifest.consumes.r#box {
        if r#box.name.is_empty() {
            anyhow::bail!("A consumed box name was empty.");
        }
        if r#box.name.len() as u64 > safety.max_box_name_size {
            anyhow::bail!("A consumed box's name was longer than allowed.");
        }
        nye_validation::is_kebab_case(&r#box.name)
            .context("A consumed box did not have a valid name.")?;
    }

    Ok(())
}
