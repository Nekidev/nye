//! Manage Nye namespaces.
//!
//! Nye namespaces are the environments created and managed by Nye. These usually consist of a
//! `/pkg` directory in the namespace's root (system-wide, `/`; or user-specific,
//! `/usr/{username}/`), artifact directories within that namespace (`/bin`, `/lib`, etc.), the
//! environment directory (`/env`), and a few more.
//!
//! This package allows you to manage those namespaces by allowing you to query installations, read
//! states, and install and uninstall packages.
//!
//! This package is provided under the GPL v3 license.
//!
//! # Installation
//!
//! To add this package to your cargo project, run the following command in your terminal:
//!
//! ```sh
//! cargo install nye-namespaces
//! ```
//!
//! # Usage
//!
//! You can create a new namespace using [`Namespace::create()`]. In many cases, though, you'll want
//! to interact with an existing namespace. For that, you can use either of the various `get_*`
//! associated functions of [`Namespace`].
//!
//! * [`Namespace::get()`] - Get a namespace by its path.
//! * [`Namespace::get_unchecked()`] - Get a namespace by its path without verifying its integrity.
//! * [`Namespace::get_for_system()`] - Get the system-wide namespace.
//! * [`Namespace::get_for_user_id()`] - Get a user-specific namespace by user ID.
//! * [`Namespace::get_for_user_name()`] - Get a user-specific namespace by user name.
//! * [`Namespace::get_for_current_user()`] - Get the current user's user-specific namespace.
//!
//! Namespace paths are the path to the namespace's root. For example, for a system-wide namespace,
//! the path is `/`. You can find `/pkg` and `/bin` in it. For a user-specific namespace, the path
//! is `/usr/{username}`. You can find user-specific `/pkg` and `/bin` in there.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::Context;
use tokio::fs;

pub mod installations;
mod database;

/// An namespace manager.
pub struct Namespace {
    /// The path to the namespace's root.
    path: PathBuf,
}

impl Namespace {
    /// Create a new namespace.
    ///
    /// When creating a namespace for a different user, change the ownership of the namespace's
    /// directories to that user.
    ///
    /// Any non-existent segments of the specified path will be created.
    ///
    /// Arguments:
    /// * `path` - The absolute path to the namespace.
    ///
    /// Returns:
    /// [`Namespace`] - The newly created namespace.
    pub async fn create(path: impl AsRef<Path>) -> anyhow::Result<Namespace> {
        let dirs = [
            path.as_ref().join("bin"),
            path.as_ref().join("lib"),
            path.as_ref().join("var"),
            path.as_ref().join("etc"),
            path.as_ref().join("env"),
            path.as_ref().join("pkg").join("store"),
        ];

        for dir in dirs {
            fs::create_dir_all(&dir)
                .await
                .context(format!("Could not create `{}`.", dir.display()))?;
        }

        Ok(Namespace {
            path: path.as_ref().to_path_buf(),
        })
    }

    /// Get a namespace by its root/namespace path.
    ///
    /// Arguments:
    /// * `path` - The path to the namespace's root.
    ///
    /// Returns:
    /// * `Ok(Namespace)` - The namespace at the specified path.
    /// * `Err(Error)` - If the namespace was corrupt.
    pub async fn get(path: impl AsRef<Path>) -> anyhow::Result<Namespace> {
        let namespace = Namespace {
            path: path.as_ref().to_path_buf(),
        };

        namespace
            .verify_integrity()
            .await
            .context("The specified namespace was corrupt.")?;

        Ok(namespace)
    }

    /// Get a namespace by its root path without validating it for integrity.
    ///
    /// Arguments:
    /// * `path` - The namespace's root path.
    pub fn get_unchecked(path: impl AsRef<Path>) -> Namespace {
        Namespace {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Get a user's namespace by their ID.
    ///
    /// This method will fail if the namespace is corrupt.
    ///
    /// Arguments:
    /// * `user_id` - The ID of the user.
    ///
    /// Returns:
    /// * `Ok(Namespace)` - The user's namespace.
    /// * `Err(Error)` - If the user did not exist or the namespace was corrupt.
    pub async fn get_for_user_id(user_id: u32) -> anyhow::Result<Namespace> {
        let user =
            users::get_user_by_uid(user_id).context("There is no user with the specified ID.")?;

        Self::get(PathBuf::from("/usr/").join(user.name())).await
    }

    /// Get a user's namespace by their user name.
    ///
    /// This method will fail if the namespace is corrupt.
    ///
    /// Arguments:
    /// * `user_name` - The user's name.
    ///
    /// Returns:
    /// * `Ok(Namespace)` - The user's namespace.
    /// * `Err(Error)` - If the user did not exist or the namespace was corrupt.
    pub async fn get_for_user_name(user_name: impl AsRef<OsStr>) -> anyhow::Result<Namespace> {
        let user = users::get_user_by_name(&user_name)
            .context("There is no user with the specified name.")?;

        Self::get(PathBuf::from("/usr/").join(user.name())).await
    }

    /// Get the user-specific namespace for the current user.
    ///
    /// This method will fail if the namespace is corrupt.
    ///
    /// Returns:
    /// * `Ok(Namespace)` - The user's namespace.
    /// * `Err(Error)` - If the namespace was corrupt.
    pub async fn get_for_current_user() -> anyhow::Result<Namespace> {
        let user_id = users::get_current_uid();

        Self::get_for_user_id(user_id).await
    }

    /// Get the system-wide namespace.
    ///
    /// It is an alias for `Namespace::get("/")`.
    ///
    /// This method will fail if the namespace is corrupt.
    ///
    /// Returns:
    /// * `Ok(Namespace)` - The system-wide namespace.
    /// * `Err(Error)` - If the namespace was corrupt.
    pub async fn get_for_system() -> anyhow::Result<Namespace> {
        Self::get("/").await
    }

    /// Ensures the namespace is not broken.
    ///
    /// This function is internally called by all `get_*` methods.
    ///
    /// If it returns an error, the validation failed. Therefore, it is quite convenient to use it
    /// the following way:
    ///
    /// ```
    /// namespace.verify_integrity().context("The namespace was corrupted.")?;
    /// ```
    pub async fn verify_integrity(&self) -> anyhow::Result<()> {
        let dirs = [
            self.path.join("bin"),
            self.path.join("lib"),
            self.path.join("var"),
            self.path.join("etc"),
            self.path.join("env"),
            self.path.join("pkg").join("store"),
        ];

        for dir in dirs {
            if !fs::try_exists(&dir)
                .await
                .context("Could not check if directory existed.")?
            {
                anyhow::bail!(
                    "`{}` does not exist. It must in a valid installation.",
                    dir.display()
                );
            }
        }

        Ok(())
    }
}
