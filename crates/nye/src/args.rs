use std::path::PathBuf;

use clap::ArgAction;
use nye_schemas::targets::Target;
use url::Url;

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

#[allow(clippy::large_enum_variant)]
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

    /// Sign into a registry.
    Signin(SigninSubcommandArgs),

    /// Sign up for a registry.
    Signup(SignupSubcommandArgs),

    /// Toasty development migration commands.
    #[cfg(debug_assertions)]
    #[command(visible_alias = "t")]
    Toasty(ToastySubcommandArgs),
}

#[derive(clap::Args)]
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

#[derive(clap::Args)]
pub struct DevSubcommandInitSubcommandArgs {
    /// The directory to use for the new package project.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// The name to give the package project. Defaults to the path's directory
    /// name.
    #[arg(short, long)]
    pub name: Option<String>,

    /// Display instructions on how to use nye dev init.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Args)]
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

#[derive(clap::Args)]
pub struct InstallSubcommandArgs {
    /// The path to one or more installable package files.
    #[arg(short, long)]
    pub path: Vec<PathBuf>,

    /// Display instructions on how to use nye install.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Args)]
pub struct UninstallSubcommandArgs {
    /// The names of the packages to uninstall.
    pub packages: Vec<String>,

    /// Display instructions on how to use nye uninstall.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Args)]
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

#[derive(clap::Args)]
pub struct ListSubcommandBinsSubcommandArgs {
    /// Display instructions on how to use nye list bins.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Args)]
pub struct ListSubcommandLibsSubcommandArgs {
    /// Display instructions on how to use nye list libs.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Args)]
pub struct SigninSubcommandArgs {
    /// The URL of the registry to sign into. If it's not configured in registries.toml,
    /// it'll be added automatically.
    pub registry: Url,

    /// The username to sign in with.
    #[arg(short, long)]
    pub username: Option<String>,
    /// The password to sign in with.
    #[arg(short, long)]
    pub password: Option<String>,

    /// Display instructions on how to use nye signin.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Args)]
pub struct SignupSubcommandArgs {
    /// The URL of the registry to sign up for. If it's not configured in registries.toml,
    /// it'll be added automatically.
    pub registry: Url,

    /// The email to sign up with.
    #[arg(short, long)]
    pub email: Option<String>,
    /// The username to sign up with.
    #[arg(short, long)]
    pub username: Option<String>,
    /// The password to sign up with.
    #[arg(short, long)]
    pub password: Option<String>,

    /// Display instructions on how to use nye signup.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[cfg(debug_assertions)]
#[derive(clap::Args)]
pub struct ToastySubcommandArgs {
    /// The arguments to pass to the toasty command.
    pub args: Vec<String>,

    /// The SQLite database URL to use to generate migrations.
    #[arg(short, long, default_value = "sqlite://state.db")]
    pub database_url: Url,

    /// Display instructions on how to use nye toasty.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}
