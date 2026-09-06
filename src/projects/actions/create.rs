use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Context;
use tokio::fs;

use crate::projects::ManifestExposes;
use crate::projects::manifest::{Manifest, ManifestConsumes, ManifestPackage, ManifestTarget, TargetOrShared};
use crate::semver::Semver;
use crate::targets::Target;

/// Creates a new package project at the specified directory.
///
/// Arguments:
/// * `path` - The path where the project will be created. All parent directories required to
///   create the project will be created.
/// * `name` - The name to give to the project. Defaults to the path's directory name.
///
/// Returns:
/// * [`Manifest`] - The project's manifest on success.
/// * [`anyhow::Error`] - If an error occurred while initializing the project.
pub async fn create(path: PathBuf, name: Option<String>) -> anyhow::Result<Manifest> {
    if fs::try_exists(path.join("nye.toml"))
        .await
        .context("Could not check if project conflicted with an existing one.")?
    {
        anyhow::bail!(
            "There's already a nye project in {}. Choose a different path or delete the existing project.",
            path.display()
        );
    }

    let current_target = Target::get_current().context("Could not get current system context.")?;

    let dirs = [
        path.clone(),
        path.join("src").join("shared").join("bin"),
        path.join("src").join("shared").join("lib"),
        path.join("src").join("shared").join("etc"),
        path.join("src")
            .join(current_target.to_string())
            .join("bin"),
        path.join("src")
            .join(current_target.to_string())
            .join("lib"),
        path.join("src")
            .join(current_target.to_string())
            .join("etc"),
        path.join("dist"),
    ];

    for dir in dirs {
        fs::create_dir_all(dir)
            .await
            .context("Could not create initial directories for project.")?;
    }

    let canonical_path = fs::canonicalize(path)
        .await
        .context("Could not canonicalize project path.")?;

    let name = if let Some(name) = name {
        name
    } else {
        canonical_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string()
    };

    let manifest = Manifest {
        package: ManifestPackage {
            name,
            version: Semver::from((0, 0, 0)),
        },
        targets: HashMap::from([
            (
                TargetOrShared::Shared,
                ManifestTarget {
                    source: PathBuf::from("src/shared"),
                },
            ),
            (
                TargetOrShared::Target(current_target),
                ManifestTarget {
                    source: format!("src/{current_target}").into(),
                },
            ),
        ]),
        exposes: ManifestExposes::default(),
        consumes: ManifestConsumes::default(),
    };
    let manifest_string =
        toml::to_string_pretty(&manifest).context("Could not write project manifest to string.")?;
    fs::write(canonical_path.join("nye.toml"), manifest_string)
        .await
        .context("Could not create manifest in project directory.")?;

    let gitignore = dedent::dedent!(
        r#"
            dist/
        "#
    );
    fs::write(canonical_path.join(".gitignore"), gitignore)
        .await
        .context("Could not write gitignore file.")?;

    Ok(manifest)
}
