compile_error!(concat!(
    "Keyrings are not persistent, even for the persistent keyring. We'll have to implement a ",
    "secrets backend abstraction that uses libsecret when available and can fall back to plain ",
    "text stores or similar. Thing about libsecret is that it depends on GNOME stuff and a ",
    "daemon so the plan is to make a fallback structure: libsecret, then keyring. In the future ",
    "we could implement our own built-in secrets daemon, though for today it's a good idea to ",
    "keep it libsecret -> keyring. The persistent keyring will keep things up for the session, ",
    "it's better than nothing, and silently falling back to plain text doesn't sound like a good ",
    "idea."
));

use std::fmt::Display;

use anyhow::Context;
use linux_keyutils::{KeyError, KeyRing, KeyRingIdentifier};

pub struct Credentials {
    pub access_token: String,
    pub refresh_token: String,
}

pub fn set_for_registry(name: impl Display, credentials: Credentials) -> anyhow::Result<()> {
    let keyring = KeyRing::get_persistent(KeyRingIdentifier::Process)
        .context("Could not get user's persistent keyring.")?;

    keyring
        .add_key(&format!("nye:registry:{name}:access"), &credentials.access_token)
        .context("Could not store access token in user's persistent keyring.")?;
    keyring
        .add_key(&format!("nye:registry:{name}:refresh"), &credentials.refresh_token)
        .context("Could not store refresh token in user's persistent keyring.")?;

    Ok(())
}

pub fn get_for_registry(name: impl Display) -> anyhow::Result<Option<Credentials>> {
    let keyring = KeyRing::get_persistent(KeyRingIdentifier::Process)
        .context("Could not get user's persistent keyring.")?;

    let access_key = match keyring.search(&format!("nye:registry:{name}:access")) {
        Ok(v) => v,
        Err(e) => match e {
            KeyError::KeyDoesNotExist | KeyError::KeyExpired | KeyError::KeyRevoked => return Ok(None),
            error => return Err(error).context("An error occurred while searching for the access token from the registry's keyring"),
        }
    };
    let refresh_key = match keyring.search(&format!("nye:registry:{name}:refresh")) {
        Ok(v) => v,
        Err(e) => match e {
            KeyError::KeyDoesNotExist | KeyError::KeyExpired | KeyError::KeyRevoked => return Ok(None),
            error => return Err(error).context("An error occurred while searching for the refresh token from the registry's keyring"),
        }
    };

    let access_bytes = access_key
        .read_to_vec()
        .context("Could not read the access token key.")?;
    let refresh_bytes = refresh_key
        .read_to_vec()
        .context("Could not read the refresh token key.")?;

    Ok(Some(Credentials {
        access_token: String::from_utf8_lossy(&access_bytes).to_string(),
        refresh_token: String::from_utf8_lossy(&refresh_bytes).to_string(),
    }))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_keyring_set_credentials() {
        set_for_registry(
            "__nye-internal-test-1",
            Credentials {
                access_token: "abc".into(),
                refresh_token: "def".into(),
            },
        )
        .unwrap();
    }

    #[test]
    fn test_keyring_get_credentials() {
        set_for_registry(
            "__nye-internal-test-2",
            Credentials {
                access_token: "uwu".into(),
                refresh_token: "owo".into(),
            },
        )
        .unwrap();

        let credentials = get_for_registry("__nye-internal-test-2").unwrap().unwrap();

        assert_eq!(credentials.access_token, "uwu");
        assert_eq!(credentials.refresh_token, "owo");
    }
}
