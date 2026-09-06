use anyhow::Context;
use clap::{CommandFactory, FromArgMatches};
use colored::Colorize;
use nye::args::{Args, DevSubcommandSubcommand, ListSubcommandSubcommand, Subcommand};
use nye::targets::Target;

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
                    nye::commands::dev_init::run(&args, cmd).await?
                }
                DevSubcommandSubcommand::Pack(cmd) => {
                    nye::commands::dev_pack::run(&args, cmd).await?
                }
            },
            Subcommand::Install(cmd) => nye::commands::install::run(&args, cmd).await?,
            Subcommand::Uninstall(cmd) => nye::commands::uninstall::run(&args, cmd).await?,
            Subcommand::List(cmd) => match &cmd.subcommand {
                None => nye::commands::list::run(&args, cmd).await?,
                Some(ListSubcommandSubcommand::Bins(cmd)) => nye::commands::list_bins::run(&args, cmd).await?,
                Some(ListSubcommandSubcommand::Libs(cmd)) => nye::commands::list_libs::run(&args, cmd).await?,
            },
            #[cfg(debug_assertions)]
            Subcommand::Toasty(cmd) => nye::commands::toasty::run(&args, cmd).await?,
        }
    } else {
        command_copy
            .print_help()
            .context("Could not print command help.")?;
    }

    Ok(())
}
