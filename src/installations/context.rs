use std::path::PathBuf;

use anyhow::Context as AnyhowContext;

#[derive(Clone)]
pub struct Context {
    /// The root directory of the context's namespace.
    ///
    /// This will be:
    /// * When the command is being run as the system, `/`.
    /// * When the command is being run as a user, `/usr/{username}/`.
    pub root: PathBuf,

    /// Whether the namespace is the system's root, i.e. /.
    pub is_system: bool,
}

impl Context {
    pub async fn get_current(is_system: bool) -> anyhow::Result<Context> {
        let uid = users::get_effective_uid();

        if is_system && uid != 0 {
            anyhow::bail!(concat!(
                "You tried to run a restricted command on the system installation (by passing ",
                "`--system`) without being root. Use sudo or log in as root before retrying."
            ))
        }

        if is_system {
            Ok(Context {
                root: PathBuf::from("/"),
                is_system,
            })
        } else {
            let username = users::get_effective_username()
                .context("Could not get current user's username to get context root path.")?;
            let username = username.to_str().context(concat!(
                "Could not get the current user's username as a string. Does it have any weird ",
                "characters?"
            ))?;

            Ok(Context {
                root: PathBuf::from(format!("/usr/{username}/")),
                is_system,
            })
        }
    }
}
