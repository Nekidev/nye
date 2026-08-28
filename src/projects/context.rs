use std::path::PathBuf;

use anyhow::Context as AnyhowContext;
use tokio::fs;

use crate::projects::manifest::Manifest;
use crate::targets::Target;

pub struct Context {
    pub path: PathBuf,
    pub manifest: Manifest,
}

impl Context {
    /// Returns the project context for the current working directory.
    pub async fn get_current() -> anyhow::Result<Context> {
        let mut directory = std::env::current_dir()
            .context("Could not get current working directory to get package context.")?;

        loop {
            if fs::try_exists(directory.join("nye.toml"))
                .await
                .context("Could not check if nye.toml existed in directory.")?
            {
                let manifest_string = fs::read_to_string(directory.join("nye.toml"))
                    .await
                    .context("Could not read nye.toml file.")?;
                let manifest: Manifest = toml::from_str(&manifest_string)
                    .context("Could not parse nye.toml manifest file.")?;

                return Ok(Context {
                    path: directory,
                    manifest,
                });
            }

            if let Some(parent) = directory.parent() {
                directory = parent.to_path_buf();
            } else {
                anyhow::bail!(
                    "No nye.toml manifest was found in the current path nor any parent directories."
                )
            }
        }
    }

    pub fn get_dist_package_path(&self, target: Target) -> PathBuf {
        self.path.join("dist").join(format!(
            "nye-{}-v{}-for-{}-pack.zip",
            self.manifest.package.name, self.manifest.package.version, target
        ))
    }
}
