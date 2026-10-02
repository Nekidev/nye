//! # The Nye Project
//!
//! Nye is a package manager for a custom Linux distribution. It is based on tidiness, simplicity,
//! and predictability.
//!
//! A few things that differenciate it from other package managers (and their distros):
//!
//! * User-specific and system-wide packages,
//! * Root is not treated as a special user,
//! * Namespace-based filesystem layout,
//! * The user only sees what they installed, not the dependencies,
//! * TOML-first,
//! * Minimal yet capable by design.
//!
//! The Linux distribution has not been started yet (a few things were developed for it but it'll be
//! worked on from scratch).
//!
//! ## Examples of Nye
//!
//! Nye uses the filesystem to scope package installations into what are called "namespaces".
//!
//! ```text
//! /                System-wide namespace's root
//!   /pkg           System-wide namespace's package state and data
//!   /usr           User-specific namespaces
//!     /{username}  User-specific namespace's root
//!       /pkg       User-specific namespace's package state and data
//! ```
//!
//! As mentioned above, root is not treated as a special user. This means it gets its own
//! user-specific namespace. Given its special permissions, it is the only user allowed to manage
//! the system-wide namespace. In essence,
//!
//! * System-wide namespace contains packages usable by every user,
//! * User-specific namespaces contain packages usable by each specific user.
//!
//! The user's "home", commonly located at `/home/{username}`, was moved to `/usr/{username}/room`.
//!
//! Applications commonly store all user-specific data under `$HOME` in one way or another.
//! Configuration files under `~/.config`, local data under `~/.local`, `~/.cache`, `~/.cargo`, and
//! more. This is wrong.
//!
//! Every namespace holds the following directories:
//!
//! * `bin`
//! * `lib`
//! * `env`
//! * `etc`
//!
//! This means no more `.config`, `.local`, nor `.share`. `room` is meant to hold only the user's
//! files, not application data nor configurations.
//!
//! The directories above hold symlinks to package-defined data. Each package gets its own `bin`,
//! `etc`, `lib`, `var`, and other directories to work on.
//!
//! ## The Package Manager
//!
//! The nye package manager, whose CLI is declared by this package, is also meant to be extremely
//! simple. You already know the commands:
//!
//! * `nye install busybox`
//! * `nye uninstall busybox`
//!
//! The other commands are basic management ones:
//!
//! * `nye dev init` - A new package project.
//! * `nye dev pack` - Create package files from a project.
//! * `nye list` - List installed packages and their artifacts.
//! * `nye signin` - Log into a package registry.
//! * `nye signup` - Create an account in a package registry.
//!
//! That's pretty much it for now.
//!
//! ## The Crate Ecosystem
//!
//! The Nye project is split into multiple individually usable crates. They are all written as
//! standalone packages, with good crate-specific documentation. It's easy to build your own stuff
//! on top of Nye by grabbing its parts and plugging them into your own applications.
//!
//! For example, to install a package:
//!
//! ```
//! let file = File::open("package.nye").await?;
//! let reader = NyeFileSeekableReader::open(file, Safety::default()).await?;
//! let namespace = Namespace::get_for_current_user().await?;
//!
//! let installer = Installation::build(namespace, reader);
//! installer.validate().await?;
//! installer.install().await?;
//! ```
//!
//! Get creative, go wild, and don't forget to share what you've built! All nye crates are licensed
//! under AGPLv3. You can read all the dos and don'ts [here](https://github.com/Nekidev/nye/blob/rewrite/LICENSE).
//!
//! The following crates are currently available:
//!
//! * `nye` - The package manager's CLI binary,
//! * `nye-environment` - Read and use `NYE_` environment variables,
//! * `nye-keyring-daemon` - The client-side package registry's credential storage daemon binary,
//! * `nye-keyring-daemon-protocol` - The `nye-keyring-daemon`'s protocol,
//! * `nye-namespaces` - Manage namespaces and their packages (installing, uninstalling,
//!   inspecting),
//! * `nye-namespaces-toasty` - The [Toasty CLI](//docs.rs/toasty-cli) binary for the namespace
//!   state database,
//! * `nye-packages` - Reading and writing package file and manifests,
//! * `nye-projects` - Creating, packaging, and managing package projects,
//! * `nye-registry` - Generic client interfaces to registries and configs,
//! * `nye-registry-http-client` - HTTP registry client,
//! * `nye-registry-http-server` - HTTP registry server,
//! * `nye-registry-local-client` - Local, file-based registry client,
//! * `nye-schemas` - Serde types used across the project, like `Username`, `Semver`, and `Email`,
//! * `nye-utils` - A bunch of tiny utilities that have not been abstracted to their own package
//!   yet,
//! * `nye-validation` - A few validation utilities, like `is_kebab_case()` and `is_env_var_name()`.
//!
//! ## Contributing
//!
//! Contributions are more than welcome! There are a few TODOs around the repository's code, running
//! `rg "TODO"` (ripgrep) in the repository will give you a few places to get started.
//!
//! If you find any bugs, have any suggestions, or see an issue with Nye's documentation, open an
//! issue in [Nye's GitHub repository](https://github.com/Nekidev/nye).
//!
//! Nye is currently a work in progress. Many things are being worked on or unfinished.

// TODO: `nye-validation` is not used in any way that wouldn't fit `nye-schemas`, we could merge
// them.
// TODO: `nye-registry*` packages have not been adapted from the old monolithic codebase yet.

use anyhow::Context;
use clap::{CommandFactory, FromArgMatches};
use colored::Colorize;
use nye_schemas::targets::Target;

use crate::args::{Args, DevSubcommandSubcommand, Subcommand};

pub mod args;
pub mod commands;
pub mod display;

fn main() {
    let result = main_inner();

    match result {
        Ok(()) => {}
        Err(error) => {
            eprintln!("{}", format!("{error:?}").red());
        }
    }
}

#[tokio::main]
async fn main_inner() -> anyhow::Result<()> {
    dotenvy::dotenv_override().ok();

    let current_target = Target::get_current()?;

    let mut command = Args::command();

    if !current_target.is_supported() {
        command = command.after_help(
            "YOUR CURRENT SYSTEM TARGET IS NOT SUPPORTED. USE AT YOUR OWN RISK."
                .red()
                .to_string(),
        );
    }

    let mut command_copy = command.clone();
    let matches = command.get_matches();
    let args = Args::from_arg_matches(&matches).context("Could not parse CLI arguments.")?;

    if args.target {
        println!("Your current system's target is {}.", current_target.to_string().blue());

        if !current_target.is_supported() {
            println!();
            eprintln!(
                "{}",
                "YOUR CURRENT SYSTEM TARGET IS NOT SUPPORTED. USE AT YOUR OWN RISK.".red()
            )
        }

        return Ok(());
    }

    if args.system && users::get_effective_uid() != 0 {
        anyhow::bail!(
            "You're logged in as `{}`, yet you need to be logged in as `{}` to be able to run commands on the system installation.",
            users::get_current_username()
                .unwrap()
                .into_string()
                .unwrap(),
            users::get_user_by_uid(0).unwrap().name().to_str().unwrap()
        );
    }

    if let Some(subcommand) = &args.subcommand {
        match subcommand {
            Subcommand::Dev(subcommand) => match &subcommand.subcommand {
                DevSubcommandSubcommand::Init(cmd) => {
                    crate::commands::dev_init::run(&args, cmd).await?
                }
                DevSubcommandSubcommand::Pack(cmd) => {
                    crate::commands::dev_pack::run(&args, cmd).await?
                }
            },
            Subcommand::Install(cmd) => crate::commands::install::run(&args, cmd).await?,
            Subcommand::Uninstall(cmd) => crate::commands::uninstall::run(&args, cmd).await?,
            _ => eprintln!("IN THE WORKS!"),
            // Subcommand::List(subcommand) => match &subcommand.subcommand {
            //     None => nye::commands::list::run(&args, subcommand).await?,
            //     Some(ListSubcommandSubcommand::Bins(cmd)) => {
            //         nye::commands::list_bins::run(&args, cmd).await?
            //     }
            //     Some(ListSubcommandSubcommand::Libs(cmd)) => {
            //         nye::commands::list_libs::run(&args, cmd).await?
            //     }
            // },
            // Subcommand::Signin(cmd) => nye::commands::signin::run(&args, cmd).await?,
            // Subcommand::Signup(cmd) => nye::commands::signup::run(&args, cmd).await?,
        }
    } else {
        command_copy
            .print_help()
            .context("Could not print command help.")?;
    }

    Ok(())
}
