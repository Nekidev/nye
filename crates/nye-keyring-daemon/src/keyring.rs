use std::io::ErrorKind;

use anyhow::Context;
use chrono::{DateTime, Utc};
use hex::ToHex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RegistryCredentials {
    pub access_token: String,
    pub access_token_expires_at: DateTime<Utc>,
    pub refresh_token: String,
    pub refresh_token_expires_at: DateTime<Utc>,
}

fn get_file_name(registry_url: &String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(registry_url);
    let hex: String = hasher.finalize().encode_hex();

    format!("{hex}.toml")
}

pub async fn load(
    user_id: u32,
    registry_url: String,
) -> anyhow::Result<Option<RegistryCredentials>> {
    let directory = nye_environment::get_env_etc()
        .context("Could not get NYE_INSTALLATION for etc directory.")?
        .join("credentials")
        .join(user_id.to_string());
    let filename = directory.join(get_file_name(&registry_url));

    let contents = match fs::read_to_string(&filename).await {
        Ok(contents) => contents,
        Err(error) => match error.kind() {
            ErrorKind::NotFound => return Ok(None),
            _ => {
                return Err(error).context(format!(
                    "Could not read from credentials file at `{}`.",
                    filename.display()
                ));
            }
        },
    };

    let credentials: RegistryCredentials = toml::from_str(&contents).context(format!(
        "Could not deserialize credentials in credentials file at {}.",
        filename.display()
    ))?;

    Ok(Some(credentials))
}

pub async fn save(
    user_id: u32,
    registry_url: String,
    credentials: RegistryCredentials,
) -> anyhow::Result<()> {
    let directory = nye_environment::get_env_etc()
        .context("Could not get NYE_INSTALLATION for etc directory.")?
        .join("credentials")
        .join(user_id.to_string());
    let filename = directory.join(get_file_name(&registry_url));

    fs::create_dir_all(&directory)
        .await
        .context("Could not create directories to store registry credentials.")?;

    let contents = toml::to_string_pretty(&credentials)
        .context("Could not serialize credentials into TOML.")?;

    fs::write(directory.join(filename), contents)
        .await
        .context("Could not write credentials to credentials file.")?;

    Ok(())
}
