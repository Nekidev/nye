use std::path::PathBuf;

use clap::ArgAction;

/// Nyeki's package manager.
#[derive(clap::Parser)]
#[clap(version)]
#[command(
    disable_help_flag = true,
    disable_version_flag = true,
    // disable_help_subcommand = true
)]
pub struct Args {
    #[arg(short, long, help = "Display the current system target.")]
    pub target: bool,

    #[arg(
        short,
        long,
        help = "Run the command using the system's installation context."
    )]
    pub system: bool,

    #[arg(short, long, help = "Display instructions on how to use nye.", action = ArgAction::Help)]
    pub help: Option<bool>,

    #[arg(short, long, help = "Display the current nye version.", action = ArgAction::Version)]
    pub version: Option<bool>,

    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,
}

#[derive(clap::Subcommand)]
pub enum Subcommand {
    /// Create, pack, and publish packages.
    #[command(visible_alias = "d")]
    Dev(DevSubcommandArgs),

    /// Install one or more packages.
    #[command(visible_alias = "i")]
    Install,

    /// Uninstall one or more packages.
    #[command(visible_alias = "u")]
    Uninstall,
}

#[derive(clap::Parser)]
pub struct DevSubcommandArgs {
    #[command(subcommand)]
    pub subcommand: DevSubcommandSubcommand,
}

#[derive(clap::Subcommand)]
pub enum DevSubcommandSubcommand {
    /// Initialize a new package project.
    #[command(visible_alias = "i")]
    Init(DevSubcommandInitSubcommandArgs),
}

#[derive(clap::Parser)]
pub struct DevSubcommandInitSubcommandArgs {
    /// The directory to use for the new package project.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// The name to give the package project. Defaults to the current directory's name.
    #[arg(short, long)]
    pub name: Option<String>,
}
