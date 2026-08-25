use anyhow::Context;
use clap::{CommandFactory, FromArgMatches};
use colored::Colorize;

use crate::{
    args::{Args, DevSubcommandSubcommand, Subcommand},
    targets::Target,
};

mod args;
mod commands;
mod projects;
mod semver;
mod targets;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
        println!(
            "Your current system's target is {}.",
            current_target.to_string().blue()
        );

        if !current_target.is_supported() {
            println!();
            eprintln!(
                "{}",
                "YOUR CURRENT SYSTEM TARGET IS NOT SUPPORTED. USE AT YOUR OWN RISK.".red()
            )
        }

        return Ok(());
    }

    if let Some(subcommand) = &args.subcommand {
        match subcommand {
            Subcommand::Dev(subcommand) => match &subcommand.subcommand {
                DevSubcommandSubcommand::Init(cmd) => commands::dev_init::run(&args, cmd).await?,
            },
            Subcommand::Install => anyhow::bail!("not implemented"),
            Subcommand::Uninstall => anyhow::bail!("not implemented"),
        }
    } else {
        command_copy
            .print_help()
            .context("Could not print command help.")?;
    }

    Ok(())
}
