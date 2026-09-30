//! Nye's registry credential "keyring" daemon.
//!
//! This package provides a small binary that stores registry credentials as plain text in
//! root-owned files.
//!
//! This daemon provides little extra protection in practice other than not storing the
//! plain-text credential files under user-owned files. In practice, any program that understands
//! this daemon's protocol will be able to read the current user's credentials just fine, just not
//! another user's (provided they don't have sudo access). It may, and likely will, provide more
//! robust security in the future.
//!
//! This daemon uses the [`nye_keyring_daemon_protocol`] library to communicate with clients, and
//! so can do clients to communicate with this daemon.
//!
//! # Installation
//!
//! You can install this daemon by running the following cargo command in your terminal:
//!
//! ```sh
//! cargo install nye-keyring-daemon
//! ```
//!
//! # Usage
//!
//! The binary's interface will be mostly self-describing using the `--help` command. It has only
//! one CLI flag available and it's only there in debug builds to skip the root user checks.
//! Otherwise, running `sudo nye-keyring-daemon` in your terminal will start the process.
//!
//! In most cases, you'll want to have this binary as a process handled by a service that starts
//! with your system.

use anyhow::Context;
use chrono::{Duration, Utc};
use clap::Parser;
use colored::Colorize;
use nye_keyring_daemon_protocol::messages::{
    ClientGetAuthForRegistry, ClientMessage, ClientSetAuthForRegistry, ServerMessage,
    ServerRegistryAuth,
};
use tokio::net::{UnixListener, UnixStream};

use crate::args::Args;
use crate::keyring::RegistryCredentials;

mod args;
mod keyring;

fn main() {
    let result = main_inner();

    match result {
        Ok(()) => {}
        Err(error) => {
            eprintln!("{}", format!("MAIN ERROR: {error:?}").red());
        }
    }
}

#[tokio::main]
async fn main_inner() -> anyhow::Result<()> {
    let args = Args::parse();

    #[cfg(debug_assertions)]
    let skip_check = args.no_root;
    #[cfg(not(debug_assertions))]
    let skip_check = false;

    if users::get_effective_uid() != 0 && !skip_check {
        anyhow::bail!(
            "You're logged in as `{}`, yet you need to be logged in as `{}` to be able to run the keyring daemon.",
            users::get_current_username()
                .unwrap()
                .into_string()
                .unwrap(),
            users::get_user_by_uid(0).unwrap().name().to_str().unwrap()
        );
    }

    let socket = UnixListener::bind("/run/nye-keyring-daemon.sock")
        .context("Could not bind to nye keyring daemon sock.")?;

    loop {
        let (stream, _) = socket
            .accept()
            .await
            .context("Could not accept an incoming socket connection.")?;

        tokio::spawn(handle(stream));
    }
}

async fn handle(stream: UnixStream) {
    let result = handle_inner(stream).await;

    match result {
        Ok(()) => {}
        Err(error) => {
            eprintln!("{}", format!("HANDLE ERROR: {error:?}").yellow());
        }
    }
}

async fn handle_inner(mut stream: UnixStream) -> anyhow::Result<()> {
    loop {
        let (message_id, message) = nye_keyring_daemon_protocol::server::recv(&mut stream)
            .await
            .context("Could not receive message from client.")?;

        let result = match message {
            ClientMessage::GetAuthForRegistry(message) => {
                handle_get_auth_for_registry(&mut stream, message, message_id).await
            }
            ClientMessage::SetAuthForRegistry(message) => {
                handle_set_auth_for_registry(&mut stream, message, message_id).await
            }
        };

        if let Err(error) = result {
            nye_keyring_daemon_protocol::server::send(
                &mut stream,
                message_id,
                ServerMessage::Error(error.to_string()),
            )
            .await
            .context(format!(
                "Could not send error message to client. The error to send was: {error}"
            ))?;

            return Err(error);
        }
    }
}

async fn handle_get_auth_for_registry(
    stream: &mut UnixStream,
    message: ClientGetAuthForRegistry,
    message_id: u64,
) -> anyhow::Result<()> {
    let peer = stream
        .peer_cred()
        .context("Could not get peer credentials.")?;

    let credentials = keyring::load(peer.uid(), message.registry_url)
        .await
        .context("Could not get registry credentials.")?;

    match credentials {
        Some(credentials) => nye_keyring_daemon_protocol::server::send(
            stream,
            message_id,
            ServerRegistryAuth {
                access_token: credentials.access_token,
            },
        )
        .await
        .context("Could not send registry credentials to client.")?,
        None => {
            nye_keyring_daemon_protocol::server::send(stream, message_id, ServerMessage::Nothing)
                .await
                .context("Could not send empty registry credentials to client.")?
        }
    }

    Ok(())
}

async fn handle_set_auth_for_registry(
    stream: &mut UnixStream,
    message: ClientSetAuthForRegistry,
    message_id: u64,
) -> anyhow::Result<()> {
    let peer = stream
        .peer_cred()
        .context("Could not get peer credentials.")?;

    keyring::save(
        peer.uid(),
        message.registry_url,
        RegistryCredentials {
            access_token: message.access_token,
            access_token_expires_at: Utc::now()
                + Duration::seconds(message.access_token_expires_in as i64),
            refresh_token: message.refresh_token,
            refresh_token_expires_at: Utc::now()
                + Duration::seconds(message.refresh_token_expires_in as i64),
        },
    )
    .await
    .context("Could not store registry credentials.")?;

    nye_keyring_daemon_protocol::server::send(stream, message_id, ServerMessage::Nothing)
        .await
        .context("Could not send empty response to client.")?;

    Ok(())
}
