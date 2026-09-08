#[cfg(feature = "registry")]
use std::net::SocketAddr;
use std::path::PathBuf;

use clap::ArgAction;

use crate::targets::Target;

/// Nyeki's package manager.
#[derive(clap::Parser)]
#[clap(version)]
#[command(
    disable_help_flag = true,
    disable_version_flag = true,
    disable_help_subcommand = true
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
    Install(InstallSubcommandArgs),

    /// Uninstall one or more packages.
    #[command(visible_alias = "u")]
    Uninstall(UninstallSubcommandArgs),

    /// Lists all installed packages.
    #[command(visible_alias = "l")]
    List(ListSubcommandArgs),

    /// Registry HTTP server commands.
    #[cfg(feature = "registry")]
    #[command(visible_alias = "r")]
    Registry(RegistrySubcommandArgs),

    /// Toasty development migration commands.
    #[cfg(debug_assertions)]
    #[command(visible_alias = "t")]
    Toasty(ToastySubcommandArgs),
}

#[derive(clap::Parser)]
pub struct DevSubcommandArgs {
    #[command(subcommand)]
    pub subcommand: DevSubcommandSubcommand,

    /// Display instructions on how to use nye dev and its subcommands.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Subcommand)]
pub enum DevSubcommandSubcommand {
    /// Initialize a new package project.
    #[command(visible_alias = "i")]
    Init(DevSubcommandInitSubcommandArgs),

    /// Package the current project into an installable file.
    #[command(visible_alias = "p")]
    Pack(DevSubcommandPackSubcommandArgs),
}

#[derive(clap::Parser)]
pub struct DevSubcommandInitSubcommandArgs {
    /// The directory to use for the new package project.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// The name to give the package project. Defaults to the path's directory name.
    #[arg(short, long)]
    pub name: Option<String>,

    /// Display instructions on how to use nye dev init.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Parser)]
pub struct DevSubcommandPackSubcommandArgs {
    /// Filter the supported targets to package.
    #[arg(short, long = "target")]
    pub targets: Vec<Target>,

    /// Overwrite existing packages in the dist folder.
    #[arg(short, long)]
    pub overwrite: bool,

    /// Display instructions on how to use nye dev pack.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Parser)]
pub struct InstallSubcommandArgs {
    /// The path to one or more installable package files.
    #[arg(short, long)]
    pub path: Vec<PathBuf>,

    /// Display instructions on how to use nye install.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Parser)]
pub struct UninstallSubcommandArgs {
    /// The names of the packages to uninstall.
    pub packages: Vec<String>,

    /// Display instructions on how to use nye uninstall.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Parser)]
pub struct ListSubcommandArgs {
    #[command(subcommand)]
    pub subcommand: Option<ListSubcommandSubcommand>,

    /// Display instructions on how to use nye list and its subcommands.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Subcommand)]
pub enum ListSubcommandSubcommand {
    /// Lists all exposed binaries by installed packages.
    #[clap(visible_alias = "b")]
    Bins(ListSubcommandBinsSubcommandArgs),

    /// Lists all exposed libraries by installed packages.
    #[clap(visible_alias = "l")]
    Libs(ListSubcommandLibsSubcommandArgs),
}

#[derive(clap::Parser)]
pub struct ListSubcommandBinsSubcommandArgs {
    /// Display instructions on how to use nye list bins.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Parser)]
pub struct ListSubcommandLibsSubcommandArgs {
    /// Display instructions on how to use nye list libs.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[cfg(feature = "registry")]
#[derive(clap::Parser)]
pub struct RegistrySubcommandArgs {
    /// Display instructions on how to use nye registry.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,

    #[command(subcommand)]
    pub subcommand: RegistrySubcommandSubcommand,
}

#[cfg(feature = "registry")]
#[derive(clap::Subcommand)]
pub enum RegistrySubcommandSubcommand {
    /// Run an HTTP registry server.
    #[clap(visible_alias = "r")]
    Run(RegistrySubcommandRunSubcommandArgs),
}

#[cfg(feature = "registry")]
#[derive(clap::Parser)]
pub struct RegistrySubcommandRunSubcommandArgs {
    /// The address to listen for incoming connections at.
    #[arg(env = "NYE_REGISTRY_BIND", default_value = "127.0.0.1:3000")]
    pub bind: SocketAddr,

    #[clap(flatten)]
    pub duckity: Option<RegistrySubcommandRunSubcommandArgsDuckity>,

    /// Display instructions on how to use nye registry run.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[cfg(feature = "registry")]
#[derive(clap::Args)]
#[group(
    requires = "application_secret",
    requires = "signin_protection_profile_id",
    requires = "signup_protection_profile_id"
)]
pub struct RegistrySubcommandRunSubcommandArgsDuckity {
    /// The Duckity application secret to use to protect sign in and sign up endpoints.
    #[arg(
        required = false,
        short,
        long = "duckity-application-secret",
        env = "NYE_REGISTRY_DUCKITY_APPLICATION_SECRET"
    )]
    pub application_secret: String,

    /// The Duckity protection profile ID to use in the sign in endpoint.
    #[arg(
        required = false,
        short = 'i',
        long = "duckity-singin-protection-profile-id",
        env = "NYE_REGISTRY_DUCKITY_SIGNIN_PROTECTION_PROFILE_ID"
    )]
    pub signin_protection_profile_id: String,

    /// The Duckity protection profile ID to use in the sign up endpoint.
    #[arg(
        required = false,
        short = 'u',
        long = "duckity-singup-protection-profile-id",
        env = "NYE_REGISTRY_DUCKITY_SIGNUP_PROTECTION_PROFILE_ID"
    )]
    pub signup_protection_profile_id: String,
}

#[cfg(debug_assertions)]
#[derive(clap::Parser)]
pub struct ToastySubcommandArgs {
    /// The arguments to pass to the toasty command.
    pub args: Vec<String>,
}
